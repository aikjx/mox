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
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use mox_system::persistence_provider::{PersistenceProvider, SqlValue};
use mox_system::sqlite_provider::SqlitePersistence;

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

/// SQLite 表结构（会话 + 消息 + 索引），幂等创建
const DB_SCHEMA: &str = r#"
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
    "#;

/// LLM 返回的抽取结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LlmExtract {
    entities: Vec<ExtractedEntity>,
    relations: Vec<ExtractedRelation>,
}

/// 对话持久化 + 图谱同步器
pub struct DialogueGraphSyncer {
    /// 持久化 Provider（对话落库）。通过 trait 对象包装，与 rusqlite 解耦。
    pub db: Arc<dyn PersistenceProvider>,
    /// 共享知识图谱（与 graph-algorithms 同一实例）
    graph: Arc<RwLock<KnowledgeGraph>>,
    /// LLM 客户端（用于智能抽取）
    llm: Arc<RwLock<LLMClient>>,
    /// 是否全自动同步（默认 true）
    auto_sync: Arc<RwLock<bool>>,
    /// 布局是否需要重算（企业级：异步/后台触发布局优化）
    layout_dirty: Arc<AtomicBool>,
    /// 布局重算间隔（0=关闭自动，仅显式触发；n>0=每 n 条用户消息重算；默认 1=每条都算）
    layout_interval: Arc<AtomicUsize>,
    /// 已同步消息计数（用于间隔触发）
    msg_count: Arc<AtomicUsize>,
}

impl DialogueGraphSyncer {
    pub fn new(
        db_path: &str,
        graph: Arc<RwLock<KnowledgeGraph>>,
        llm: Arc<RwLock<LLMClient>>,
    ) -> Result<Self> {
        let pvd = SqlitePersistence::file(db_path)?;
        pvd.exec_batch(DB_SCHEMA)?;

        Ok(Self {
            db: Arc::new(pvd),
            graph,
            llm,
            auto_sync: Arc::new(RwLock::new(true)),
            layout_dirty: Arc::new(AtomicBool::new(false)),
            layout_interval: Arc::new(AtomicUsize::new(1)),
            msg_count: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// 内存模式（无 SQLite）：SQLite 不可用时降级使用，对话不落库但图谱同步仍生效
    pub fn new_in_memory(graph: Arc<RwLock<KnowledgeGraph>>, llm: Arc<RwLock<LLMClient>>) -> Self {
        let pvd = SqlitePersistence::memory().unwrap();
        pvd.exec_batch(DB_SCHEMA).unwrap();
        Self {
            db: Arc::new(pvd),
            graph,
            llm,
            auto_sync: Arc::new(RwLock::new(true)),
            layout_dirty: Arc::new(AtomicBool::new(false)),
            layout_interval: Arc::new(AtomicUsize::new(1)),
            msg_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn set_auto_sync(&self, enabled: bool) {
        *self.auto_sync.write().await = enabled;
    }

    pub async fn is_auto_sync(&self) -> bool {
        *self.auto_sync.read().await
    }

    /// 设置布局重算间隔（企业级性能调优）：
    /// - `0` = 关闭自动布局，仅由 `recompute_layout()` 显式/后台触发（配合 debounce）
    /// - `n>0` = 每 `n` 条用户消息重算一次（默认 `1`，即每条都算，等价于旧行为）
    pub fn set_layout_interval(&self, n: usize) {
        self.layout_interval.store(n, Ordering::SeqCst);
    }

    /// 显式重算布局（PageRank + 社区发现），仅当存在未重算增量（脏标记）时执行。
    /// 配合 `set_layout_interval(0)` 使用，可由 debounce / 后台任务周期性调用，避免每条消息全图重算。
    pub async fn recompute_layout(&self) {
        if !self.layout_dirty.load(Ordering::SeqCst) {
            return;
        }
        let mut graph = self.graph.write().await;
        apply_layout(&mut graph);
        self.layout_dirty.store(false, Ordering::SeqCst);
    }

    /// 新建会话
    pub async fn create_session(&self, title: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.db.exec(
            "INSERT INTO dialogue_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            &[
                SqlValue::Text(id.clone()),
                SqlValue::Text(title.to_string()),
                SqlValue::Text(now.clone()),
                SqlValue::Text(now),
            ],
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
        self.db.exec(
            "INSERT INTO dialogue_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                SqlValue::Text(msg_id.clone()),
                SqlValue::Text(session_id.to_string()),
                SqlValue::Text(role.to_string()),
                SqlValue::Text(content.to_string()),
                SqlValue::Text(now.clone()),
            ],
        )?;
        self.db.exec(
            "UPDATE dialogue_sessions SET updated_at = ?1 WHERE id = ?2",
            &[SqlValue::Text(now), SqlValue::Text(session_id.to_string())],
        )?;

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
        let raw = self.extract_with_llm(content).await?;
        let (extracted, dropped) = sanitize_extracted(raw);
        if dropped > 0 {
            tracing::warn!("对话抽取结果含 {dropped} 个越界实体/关系(疑似提示注入), 已丢弃");
        }
        if extracted.entities.is_empty() {
            return Ok(0);
        }

        let mut graph = self.graph.write().await;
        let session_node = format!("dialogue:{}", session_id);
        // 每个会话一个对话锚点节点（幂等：已存在则复用，避免每条消息重复创建）
        if graph.get_node(&session_node).is_none() {
            graph.add_node(KnowledgeNode {
                id: session_node.clone(),
                label: format!("对话#{}", &session_id[..8.min(session_id.len())]),
                node_type: "dialogue".to_string(),
                properties: serde_json::json!({ "session_id": session_id }),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
        }

        for ent in &extracted.entities {
            let node_id = sanitize_id(&ent.name);
            // 幂等：相同概念节点只创建一次（避免跨消息重复入图）
            if graph.get_node(&node_id).is_none() {
                graph.add_node(KnowledgeNode {
                    id: node_id.clone(),
                    label: ent.name.clone(),
                    node_type: ent.entity_type.clone(),
                    properties: serde_json::json!({}),
                    embedding: None,
                    activation: 0.0,
                    metadata: HashMap::new(),
                });
            }
            // 概念 -> 对话 锚点，权重体现归属强度
            if let Err(e) = graph.add_edge(KnowledgeEdge {
                source: node_id,
                target: session_node.clone(),
                weight: ent.weight.max(0.5),
                relation_type: "mentioned_in".to_string(),
                properties: serde_json::json!({}),
            }) {
                tracing::warn!("图谱边(mentioned_in)写入失败(已跳过): {e}");
            }
        }

        for rel in &extracted.relations {
            let src = sanitize_id(&rel.source);
            let tgt = sanitize_id(&rel.target);
            if let Err(e) = graph.add_edge(KnowledgeEdge {
                source: src,
                target: tgt,
                weight: rel.weight,
                relation_type: rel.relation.clone(),
                properties: serde_json::json!({}),
            }) {
                tracing::warn!("图谱关系边({})写入失败(已跳过): {}", rel.relation, e);
            }
        }

        // 自动布局优化（企业级：按 layout_interval 节流，避免每条消息全图重算）
        let count = self.msg_count.fetch_add(1, Ordering::SeqCst) + 1;
        let interval = self.layout_interval.load(Ordering::SeqCst);
        if interval == 0 {
            // 关闭自动布局：仅标记脏，由 recompute_layout() 显式/后台触发
            self.layout_dirty.store(true, Ordering::SeqCst);
        } else if count.is_multiple_of(interval) {
            apply_layout(&mut graph);
            self.layout_dirty.store(false, Ordering::SeqCst);
        } else {
            self.layout_dirty.store(true, Ordering::SeqCst);
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
            let rows = self.db.query(
                "SELECT s.id, s.title, m.content, m.role, m.created_at \
                 FROM dialogue_messages m JOIN dialogue_sessions s ON s.id = m.session_id \
                 WHERE m.content LIKE ?1 OR s.title LIKE ?1 \
                 ORDER BY m.created_at DESC LIMIT ?2",
                &[SqlValue::Text(q), SqlValue::Int(limit as i64)],
            )?;
            for row in rows {
                let get_text = |k: &str| -> Option<String> {
                    match row.get(k) {
                        Some(SqlValue::Text(s)) => Some(s.clone()),
                        _ => None,
                    }
                };
                let sid = get_text("id").unwrap_or_default();
                let title = get_text("title").unwrap_or_default();
                let content = get_text("content").unwrap_or_default();
                let role = get_text("role").unwrap_or_default();
                let ts = get_text("created_at").unwrap_or_default();
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
            let srows = self.db.query(
                "SELECT id, title, created_at, updated_at FROM dialogue_sessions",
                &[],
            )?;
            for s in srows {
                let get_text = |k: &str| -> Option<String> {
                    match s.get(k) {
                        Some(SqlValue::Text(v)) => Some(v.clone()),
                        _ => None,
                    }
                };
                let id = get_text("id").unwrap_or_default();
                let title = get_text("title").unwrap_or_default();
                let ca = get_text("created_at").unwrap_or_default();
                let ua = get_text("updated_at").unwrap_or_default();

                let mrows = self.db.query(
                    "SELECT id, role, content, created_at FROM dialogue_messages WHERE session_id = ?1 ORDER BY created_at ASC",
                    &[SqlValue::Text(id.clone())],
                )?;
                let mut messages = Vec::new();
                for m in mrows {
                    let get_text_m = |k: &str| -> Option<String> {
                        match m.get(k) {
                            Some(SqlValue::Text(v)) => Some(v.clone()),
                            _ => None,
                        }
                    };
                    let mid = get_text_m("id").unwrap_or_default();
                    let role = get_text_m("role").unwrap_or_default();
                    let content = get_text_m("content").unwrap_or_default();
                    let ts = get_text_m("created_at").unwrap_or_default();
                    messages.push(ExportMessage {
                        id: mid,
                        role,
                        content,
                        created_at: ts,
                    });
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
        for s in &bundle.sessions {
            self.db.exec(
                "INSERT OR REPLACE INTO dialogue_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                &[
                    SqlValue::Text(s.id.clone()),
                    SqlValue::Text(s.title.clone()),
                    SqlValue::Text(s.created_at.clone()),
                    SqlValue::Text(s.updated_at.clone()),
                ],
            )?;
            for m in &s.messages {
                self.db.exec(
                    "INSERT OR REPLACE INTO dialogue_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    &[
                        SqlValue::Text(m.id.clone()),
                        SqlValue::Text(s.id.clone()),
                        SqlValue::Text(m.role.clone()),
                        SqlValue::Text(m.content.clone()),
                        SqlValue::Text(m.created_at.clone()),
                    ],
                )?;
            }
        }

        let mut graph = self.graph.write().await;
        let mut node_n = 0usize;
        for n in bundle.graph_nodes {
            // 幂等：已存在的节点复用，避免重复入图（导入导出迁移需可重放）
            if graph.get_node(&n.id).is_none() {
                graph.add_node(n);
                node_n += 1;
            }
        }
        let mut edge_n = 0usize;
        for e in bundle.graph_edges {
            if graph.get_node(&e.source).is_some() && graph.get_node(&e.target).is_some() {
                if let Err(err) = graph.add_edge(e) {
                    tracing::warn!("导入边写入失败(已跳过): {err}");
                    continue;
                }
                edge_n += 1;
            }
        }
        // 重新布局优化
        apply_layout(&mut graph);
        Ok(ImportReport {
            sessions: bundle.sessions.len(),
            messages: bundle.sessions.iter().map(|s| s.messages.len()).sum(),
            nodes: node_n,
            edges: edge_n,
        })
    }

    /// 列出会话
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let rows = self.db.query(
            "SELECT id, title, created_at, updated_at FROM dialogue_sessions ORDER BY updated_at DESC",
            &[],
        )?;
        let mut out = Vec::new();
        for r in rows {
            let get_text = |k: &str| -> Option<String> {
                match r.get(k) {
                    Some(SqlValue::Text(v)) => Some(v.clone()),
                    _ => None,
                }
            };
            let id = get_text("id").unwrap_or_default();
            let title = get_text("title").unwrap_or_default();
            let ca = get_text("created_at").unwrap_or_default();
            let ua = get_text("updated_at").unwrap_or_default();
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
    match crate::util::extract_json_object(text) {
        Some(json) => Ok(serde_json::from_str(json)?),
        None => Err(anyhow!("无 JSON 内容")),
    }
}

/// LLM 不可用时的降级：从内容中匹配已知算子/算法关键词
fn rule_based_extract(content: &str) -> LlmExtract {
    // 已知关键词 → 实体类型（单一事实源：crate::knowledge::DIALOGUE_KNOWN_ENTITIES）
    let known: &[(&str, &str)] = crate::knowledge::DIALOGUE_KNOWN_ENTITIES;
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
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == ':' {
                c
            } else {
                '_'
            }
        })
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

// ---------- 企业级：抽取安全校验与布局节流 ----------

/// 允许的实体类型白名单（与 build_extract_prompt 契约一致）
pub const ALLOWED_ENTITY_TYPES: &[&str] =
    &["operator", "algorithm", "concept", "capability", "workflow"];
/// 允许的关系类型白名单（含内部 mentioned_in）
pub const ALLOWED_RELATIONS: &[&str] = &["依赖", "包含", "实现", "属于", "mentioned_in"];

fn is_allowed_entity_type(t: &str) -> bool {
    ALLOWED_ENTITY_TYPES.contains(&t)
}

fn is_allowed_relation(r: &str) -> bool {
    ALLOWED_RELATIONS.contains(&r)
}

/// 抽取结果安全过滤：丢弃白名单外的 `entity_type` / `relation`，
/// 防御 LLM 提示注入导致的图谱类型污染。返回 (过滤后结果, 丢弃数量)。
fn sanitize_extracted(mut ext: LlmExtract) -> (LlmExtract, usize) {
    let before = ext.entities.len() + ext.relations.len();
    ext.entities
        .retain(|e| is_allowed_entity_type(&e.entity_type));
    ext.relations.retain(|r| is_allowed_relation(&r.relation));
    let dropped = before - (ext.entities.len() + ext.relations.len());
    (ext, dropped)
}

/// 布局优化：重算 PageRank 中心性 + 社区发现，结果回写节点 metadata 供前端力导向布局。
/// 抽离为独立函数，供 per-message 节流与 import 复用。
fn apply_layout(graph: &mut KnowledgeGraph) {
    let pr = graph.centrality_metrics();
    for (id, score) in pr.pagerank {
        if let Some(node) = graph.get_node_mut(&id) {
            node.metadata
                .insert("pagerank".to_string(), format!("{score:.4}"));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::LLMConfig;
    use graph_algorithms::KnowledgeGraph;
    use std::sync::Arc;
    use tokio::sync::RwLock as TokioRwLock;

    fn test_syncer() -> DialogueGraphSyncer {
        let graph = Arc::new(TokioRwLock::new(KnowledgeGraph::new()));
        let llm = Arc::new(TokioRwLock::new(LLMClient::new(LLMConfig::default())));
        DialogueGraphSyncer::new_in_memory(graph, llm)
    }

    #[tokio::test]
    async fn auto_sync_off_skips_graph() {
        let s = test_syncer();
        s.set_auto_sync(false).await;
        let sid = s.create_session("t").await.unwrap();
        s.append_message(&sid, "user", "使用线性变换和注意力机制")
            .await
            .unwrap();
        let g = s.graph.read().await;
        assert_eq!(g.nodes().len(), 0, "auto_sync 关闭不应入图");
    }

    #[tokio::test]
    async fn rule_extract_only_allowed_types() {
        let s = test_syncer();
        let sid = s.create_session("t").await.unwrap();
        s.append_message(
            &sid,
            "user",
            "使用线性变换、激活函数、归一化、注意力机制、PageRank、社区发现、工作流",
        )
        .await
        .unwrap();
        let g = s.graph.read().await;
        assert!(!g.nodes().is_empty(), "应至少抽取到一个实体节点");
        for n in g.nodes() {
            if n.node_type == "dialogue" {
                continue; // 会话锚点节点，类型本就为 dialogue，不参与白名单校验
            }
            assert!(
                ALLOWED_ENTITY_TYPES.contains(&n.node_type.as_str()),
                "越界 node_type: {}",
                n.node_type
            );
        }
    }

    #[tokio::test]
    async fn entity_type_injection_is_blocked() {
        let (ext, dropped) = sanitize_extracted(LlmExtract {
            entities: vec![
                ExtractedEntity {
                    name: "正常算子".into(),
                    entity_type: "operator".into(),
                    weight: 1.0,
                },
                ExtractedEntity {
                    name: "恶意".into(),
                    entity_type: "../../etc/passwd".into(),
                    weight: 1.0,
                },
                ExtractedEntity {
                    name: "脚本".into(),
                    entity_type: "javascript:alert(1)".into(),
                    weight: 1.0,
                },
            ],
            relations: vec![
                ExtractedRelation {
                    source: "A".into(),
                    target: "B".into(),
                    relation: "依赖".into(),
                    weight: 1.0,
                },
                ExtractedRelation {
                    source: "A".into(),
                    target: "B".into(),
                    relation: "DROP".into(),
                    weight: 1.0,
                },
            ],
        });
        assert_eq!(ext.entities.len(), 1, "应仅保留 operator");
        assert_eq!(ext.relations.len(), 1, "应仅保留允许的 relation");
        assert_eq!(dropped, 3);
    }

    #[tokio::test]
    async fn search_finds_dialogue_and_graph() {
        let s = test_syncer();
        let sid = s.create_session("测试会话").await.unwrap();
        s.append_message(&sid, "user", "使用注意力机制做特征提取")
            .await
            .unwrap();
        let r = s.search("注意力", 10).await.unwrap();
        assert!(!r.graph_nodes.is_empty(), "应搜到图谱节点");
        let r2 = s.search("测试会话", 10).await.unwrap();
        assert!(!r2.dialogues.is_empty(), "应搜到对话");
    }

    #[tokio::test]
    async fn export_import_idempotent() {
        let s = test_syncer();
        let sid = s.create_session("t").await.unwrap();
        s.append_message(&sid, "user", "线性变换和卷积")
            .await
            .unwrap();
        let b1 = s.export_bundle().await.unwrap();
        let _ = s.import_bundle(b1.clone()).await.unwrap();
        let b2 = s.export_bundle().await.unwrap();
        assert_eq!(
            b1.graph_nodes.len(),
            b2.graph_nodes.len(),
            "导入导出应幂等(节点数不变)"
        );
    }

    #[tokio::test]
    async fn layout_interval_zero_defers_recompute() {
        let s = test_syncer();
        s.set_layout_interval(0);
        let sid = s.create_session("t").await.unwrap();
        s.append_message(&sid, "user", "线性变换").await.unwrap();
        let g = s.graph.read().await;
        let has_pr = g
            .nodes()
            .iter()
            .any(|n| n.metadata.contains_key("pagerank"));
        assert!(!has_pr, "interval=0 时不应自动重算布局");
        drop(g);
        s.recompute_layout().await;
        let g2 = s.graph.read().await;
        assert!(
            g2.nodes()
                .iter()
                .any(|n| n.metadata.contains_key("pagerank")),
            "显式 recompute 应补算布局"
        );
    }
}
