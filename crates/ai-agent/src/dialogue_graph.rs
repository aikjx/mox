//! # 对话知识图谱自动整理模块
//!
//! 实现"对话内容自动通过知识图谱整理、优化布局"的全自动流水线：
//! 1. 对话落库（SQLite，单文件可整包迁移）
//! 2. 对话完成后自动调用 LLM 抽取实体/关系，写入知识图谱
//! 3. 自动触发图谱布局优化（PageRank 重算 + 社区发现）
//! 4. 统一搜索（对话 + 图谱节点）
//! 5. 一键导入导出（对话 + 图谱打包为单个 JSON 迁移文件）

use anyhow::{anyhow, Result};
use chrono::Utc;
use graph_algorithms::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
use rusqlite::params;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::LLMClient;

/// 抽取结果的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    #[serde(default)]
    pub weight: f64,
}

/// 抽取结果的关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub source: String,
    pub target: String,
    pub relation: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

/// LLM 返回的抽取结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LlmExtract {
    entities: Vec<ExtractedEntity>,
    relations: Vec<ExtractedRelation>,
}

/// 对话持久化 + 图谱同步器
pub struct DialogueGraphSyncer {
    /// SQLite 连接（对话落库）。rusqlite::Connection 非 Sync，用 std Mutex 包裹以跨线程安全共享。
    pub db: Arc<Mutex<Connection>>,
    /// 共享知识图谱（与 graph-algorithms 同一实例）
    graph: Arc<RwLock<KnowledgeGraph>>,
    /// LLM 客户端（用于智能抽取）
    llm: Arc<RwLock<LLMClient>>,
    /// 是否全自动同步（默认 true）
    auto_sync: Arc<RwLock<bool>>,
}

impl DialogueGraphSyncer {
    pub fn new(
        db_path: &str,
        graph: Arc<RwLock<KnowledgeGraph>>,
        llm: Arc<RwLock<LLMClient>>,
    ) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS dialogue_sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS dialogue_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_msg_session ON dialogue_messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_msg_content ON dialogue_messages(content);
            "#,
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            graph,
            llm,
            auto_sync: Arc::new(RwLock::new(true)),
        })
    }

    /// 内存模式（无 SQLite）：SQLite 不可用时降级使用，对话不落库但图谱同步仍生效
    pub fn new_in_memory(
        graph: Arc<RwLock<KnowledgeGraph>>,
        llm: Arc<RwLock<LLMClient>>,
    ) -> Self {
        Self {
            db: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            graph,
            llm,
            auto_sync: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn set_auto_sync(&self, enabled: bool) {
        *self.auto_sync.write().await = enabled;
    }

    pub async fn is_auto_sync(&self) -> bool {
        *self.auto_sync.read().await
    }

    /// 新建会话
    pub async fn create_session(&self, title: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.db.lock().unwrap().execute(
            "INSERT INTO dialogue_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, now, now],
        )?;
        Ok(id)
    }

    /// 追加一条消息，并（在全自动模式下）自动同步进知识图谱
    pub async fn append_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String> {
        let msg_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        {
            let db = self.db.lock().unwrap();
            db.execute(
                "INSERT INTO dialogue_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![msg_id, session_id, role, content, now],
            )?;
            db.execute(
                "UPDATE dialogue_sessions SET updated_at = ?1 WHERE id = ?2",
                params![now, session_id],
            )?;
        }

        // 全自动：仅对 user 消息做图谱抽取（避免 bot 回声污染）
        if *self.auto_sync.read().await && role == "user" {
            if let Err(e) = self.sync_message_to_graph(session_id, content).await {
                tracing::warn!("对话自动同步图谱失败（不影响对话）: {e}");
            }
        }
        Ok(msg_id)
    }

    /// 用 LLM 从一段对话内容中抽取实体/关系并写入图谱（含布局优化）
    pub async fn sync_message_to_graph(&self, session_id: &str, content: &str) -> Result<usize> {
        let extracted = self.extract_with_llm(content).await?;
        if extracted.entities.is_empty() {
            return Ok(0);
        }

        let mut graph = self.graph.write().await;
        let session_node = format!("dialogue:{}", session_id);
        // 每个会话一个对话节点，作为被抽取概念的归属锚点
        graph.add_node(KnowledgeNode {
            id: session_node.clone(),
            label: format!("对话#{}", &session_id[..8.min(session_id.len())]),
            node_type: "dialogue".to_string(),
            properties: serde_json::json!({ "session_id": session_id }),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });

        for ent in &extracted.entities {
            let node_id = sanitize_id(&ent.name);
            graph.add_node(KnowledgeNode {
                id: node_id.clone(),
                label: ent.name.clone(),
                node_type: ent.entity_type.clone(),
                properties: serde_json::json!({}),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
            // 概念 -> 对话 锚点，权重体现归属强度
            let _ = graph.add_edge(KnowledgeEdge {
                source: node_id,
                target: session_node.clone(),
                weight: ent.weight.max(0.5),
                relation_type: "mentioned_in".to_string(),
                properties: serde_json::json!({}),
            });
        }

        for rel in &extracted.relations {
            let src = sanitize_id(&rel.source);
            let tgt = sanitize_id(&rel.target);
            let _ = graph.add_edge(KnowledgeEdge {
                source: src,
                target: tgt,
                weight: rel.weight,
                relation_type: rel.relation.clone(),
                properties: serde_json::json!({}),
            });
        }

        // 自动布局优化：重算中心性 + 社区发现，结果回写 metadata 供前端布局
        let pr = graph.centrality_metrics();
        for (id, score) in pr.pagerank {
            if let Some(node) = graph.get_node_mut(&id) {
                node.metadata.insert("pagerank".to_string(), format!("{score:.4}"));
            }
        }
        let communities = graph.detect_communities(8);
        for (ci, comm) in communities.iter().enumerate() {
            for nid in &comm.nodes {
                if let Some(node) = graph.get_node_mut(nid) {
                    node.metadata
                        .insert("community".to_string(), ci.to_string());
                }
            }
        }
        Ok(extracted.entities.len())
    }

    /// 调用 LLM 抽取实体/关系；LLM 不可用时降级为关键词规则抽取
    async fn extract_with_llm(&self, content: &str) -> Result<LlmExtract> {
        let llm = self.llm.read().await;
        if llm.is_enabled() {
            let prompt = build_extract_prompt(content);
            match llm
                .chat(vec![crate::llm_client::LLMChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                }])
                .await
            {
                Ok(text) => {
                    if let Ok(parsed) = parse_llm_json::<LlmExtract>(&text) {
                        return Ok(parsed);
                    }
                    tracing::warn!("LLM 抽取结果解析失败，降级规则抽取");
                }
                Err(e) => tracing::warn!("LLM 抽取调用失败，降级规则抽取: {e}"),
            }
        }
        Ok(rule_based_extract(content))
    }

    /// 统一搜索：对话内容 + 图谱节点标签
    pub async fn search(&self, query: &str, limit: usize) -> Result<SearchResult> {
        let q = format!("%{}%", query);
        let mut dialogues = Vec::new();
        {
            let db = self.db.lock().unwrap();
            let mut stmt = db.prepare(
                "SELECT s.id, s.title, m.content, m.role, m.created_at \
                 FROM dialogue_messages m JOIN dialogue_sessions s ON s.id = m.session_id \
                 WHERE m.content LIKE ?1 OR s.title LIKE ?1 \
                 ORDER BY m.created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![q, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            for r in rows {
                let (sid, title, content, role, ts) = r?;
                dialogues.push(SearchHit {
                    kind: "dialogue".to_string(),
                    id: sid,
                    title,
                    snippet: truncate(&content, 120),
                    role,
                    ts,
                });
            }
        }

        let graph = self.graph.read().await;
        let nodes = graph
            .nodes()
            .iter()
            .filter(|n| {
                n.label.to_lowercase().contains(&query.to_lowercase())
                    || n.id.to_lowercase().contains(&query.to_lowercase())
                    || n.node_type.to_lowercase().contains(&query.to_lowercase())
            })
            .take(limit)
            .map(|n| SearchHit {
                kind: "graph".to_string(),
                id: n.id.clone(),
                title: n.label.clone(),
                snippet: format!("类型: {}", n.node_type),
                role: n.node_type.clone(),
                ts: String::new(),
            })
            .collect();

        Ok(SearchResult {
            dialogues,
            graph_nodes: nodes,
        })
    }

    /// 导出：对话 + 图谱 打包为单个迁移文件
    pub async fn export_bundle(&self) -> Result<ExportBundle> {
        let mut sessions = Vec::new();
        {
            let db = self.db.lock().unwrap();
            let mut sstmt = db.prepare("SELECT id, title, created_at, updated_at FROM dialogue_sessions")?;
            let srows = sstmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for s in srows {
                let (id, title, ca, ua) = s?;
                let mut mstmt = db.prepare(
                    "SELECT id, role, content, created_at FROM dialogue_messages WHERE session_id = ?1 ORDER BY created_at ASC",
                )?;
                let mrows = mstmt.query_map(params![id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?;
                let mut messages = Vec::new();
                for m in mrows {
                    let (mid, role, content, ts) = m?;
                    messages.push(ExportMessage { id: mid, role, content, created_at: ts });
                }
                sessions.push(ExportSession {
                    id,
                    title,
                    created_at: ca,
                    updated_at: ua,
                    messages,
                });
            }
        }

        let graph = self.graph.read().await;
        let bundle = ExportBundle {
            version: "1.0".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            sessions,
            graph_nodes: graph.nodes().into_iter().cloned().collect(),
            graph_edges: graph.edges(),
        };
        Ok(bundle)
    }

    /// 导入：校验后合并进当前对话库与图谱（幂等：相同 id 覆盖）
    pub async fn import_bundle(&self, bundle: ExportBundle) -> Result<ImportReport> {
        {
            let db = self.db.lock().unwrap();
            for s in &bundle.sessions {
                db.execute(
                    "INSERT OR REPLACE INTO dialogue_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    params![s.id, s.title, s.created_at, s.updated_at],
                )?;
                for m in &s.messages {
                    db.execute(
                        "INSERT OR REPLACE INTO dialogue_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![m.id, s.id, m.role, m.content, m.created_at],
                    )?;
                }
            }
        }

        let mut graph = self.graph.write().await;
        let mut node_n = 0usize;
        for n in bundle.graph_nodes {
            graph.add_node(n);
            node_n += 1;
        }
        let mut edge_n = 0usize;
        for e in bundle.graph_edges {
            if graph.get_node(&e.source).is_some() && graph.get_node(&e.target).is_some() {
                let _ = graph.add_edge(e);
                edge_n += 1;
            }
        }
        // 重新布局优化
        let pr = graph.centrality_metrics();
        for (id, score) in pr.pagerank {
            if let Some(node) = graph.get_node_mut(&id) {
                node.metadata.insert("pagerank".to_string(), format!("{score:.4}"));
            }
        }
        Ok(ImportReport {
            sessions: bundle.sessions.len(),
            messages: bundle.sessions.iter().map(|s| s.messages.len()).sum(),
            nodes: node_n,
            edges: edge_n,
        })
    }

    /// 列出会话
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, title, created_at, updated_at FROM dialogue_sessions ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (id, title, ca, ua) = r?;
            out.push(SessionSummary {
                id,
                title,
                created_at: ca,
                updated_at: ua,
            });
        }
        Ok(out)
    }
}

use std::collections::HashMap;

/// 会话摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 搜索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub role: String,
    pub ts: String,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub dialogues: Vec<SearchHit>,
    pub graph_nodes: Vec<SearchHit>,
}

/// 导出消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// 导出会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<ExportMessage>,
}

/// 完整迁移包（对话 + 图谱）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: String,
    pub exported_at: String,
    pub sessions: Vec<ExportSession>,
    pub graph_nodes: Vec<KnowledgeNode>,
    pub graph_edges: Vec<KnowledgeEdge>,
}

/// 导入报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub sessions: usize,
    pub messages: usize,
    pub nodes: usize,
    pub edges: usize,
}

// ---------- 抽取辅助 ----------

fn build_extract_prompt(content: &str) -> String {
    format!(
        r#"请从以下对话内容中抽取知识图谱的实体与关系。
只关注算子、算法、数学概念、系统能力、业务流程等可复用知识。
严格以如下 JSON 返回，不要任何额外文字：
{{
  "entities": [{{"name":"实体名","entity_type":"operator|algorithm|concept|capability|workflow","weight":1.0}}],
  "relations": [{{"source":"实体A","target":"实体B","relation":"依赖|包含|实现|属于","weight":1.0}}]
}}
对话内容：
{content}"#
    )
}

/// 容错解析 LLM 返回的 JSON（允许包裹在 markdown 代码块中）
fn parse_llm_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    let trimmed = text.trim();
    let json_str = if let Some(start) = trimmed.find('{') {
        let end = trimmed.rfind('}').unwrap_or(trimmed.len() - 1);
        &trimmed[start..=end]
    } else {
        return Err(anyhow!("无 JSON 内容"));
    };
    Ok(serde_json::from_str(json_str)?)
}

/// LLM 不可用时的降级：从内容中匹配已知算子/算法关键词
fn rule_based_extract(content: &str) -> LlmExtract {
    let known: &[(&str, &str)] = &[
        ("线性变换", "operator"),
        ("激活函数", "operator"),
        ("归一化", "operator"),
        ("卷积", "operator"),
        ("注意力", "operator"),
        ("注意力机制", "algorithm"),
        ("PageRank", "algorithm"),
        ("最短路径", "algorithm"),
        ("社区发现", "algorithm"),
        ("工作流", "workflow"),
        ("插件", "capability"),
        ("资源管理", "capability"),
        ("浏览器自动化", "capability"),
    ];
    let mut entities = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (kw, ty) in known {
        if content.contains(kw) && seen.insert(*kw) {
            entities.push(ExtractedEntity {
                name: kw.to_string(),
                entity_type: ty.to_string(),
                weight: 1.0,
            });
        }
    }
    LlmExtract {
        entities,
        relations: Vec::new(),
    }
}

fn sanitize_id(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == ':' { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        format!("ent_{}", &Uuid::new_v4().to_string()[..8])
    } else {
        cleaned
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut end = 0;
        for (i, _) in s.char_indices() {
            if i >= max {
                break;
            }
            end = i;
        }
        format!("{}…", &s[..end])
    }
}
