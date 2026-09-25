// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 对话沉淀模块：AI 对话核心内容 → 知识图谱 / 云盘 / 知识库 自动归类
//!
//! 端到端链路（POST /api/alliance/sediment）：
//! 1. 读取对话核心内容 —— 编排器 dialogue 会话（GET /api/dialogue/sessions/:id/messages）、
//!    专家联盟会话（本地 ExpertsSharedState）、或客户端内联消息；
//! 2. 自动归类 ——
//!    - 知识图谱：新建知识库文档后调用 `KbAnalyzer` 抽取实体/关键词/摘要，
//!      `GraphLinker` 将「文档节点 + 实体节点 + 关系边」挂入图谱（GraphStore 唯一真源）；
//!    - 云盘：把「对话纪要 Markdown」（原文 + AI 摘要/实体/标签）写入 Cloud 存储桶 `dialogue/`；
//!    - 知识库：登记文档（分类 `cat-dialogue` 对话沉淀，状态 linked，含摘要/实体/关系/标签）。

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use mox_api_protocol::{ApiResponse, api_error, api_ok};
use mox_kb_svc::analyze::{KbAnalyzer, chunk_doc};
use mox_kb_svc::link::GraphLinker;
use mox_kb_svc::model::{STATUS_LINKED, KbDocument};
use mox_kb_svc::KbState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::alliance::experts_common::ExpertsSharedState;
use crate::cloud::CloudState;

/// 对话沉淀请求
#[derive(Debug, Deserialize)]
pub struct SedimentRequest {
    /// 来源：dialogue（编排器 AI 对话）/ expert（专家联盟会话）/ inline（直接传消息）
    #[serde(default = "default_source")]
    pub source: String,
    /// 会话 ID（dialogue / expert 必填）
    #[serde(default)]
    pub session_id: Option<String>,
    /// 标题（inline 必填；dialogue / expert 缺省取会话标题）
    #[serde(default)]
    pub title: Option<String>,
    /// 内联消息（source=inline 必填）
    #[serde(default)]
    pub messages: Vec<SedimentMessage>,
    /// 覆盖知识库分类（缺省 cat-dialogue 对话沉淀）
    #[serde(default)]
    pub category: Option<String>,
}

/// 对话单条消息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SedimentMessage {
    pub role: String,
    pub content: String,
}

fn default_source() -> String {
    "dialogue".to_string()
}

/// 对话沉淀共享状态（网关注册中心唯一持有）
#[derive(Clone)]
pub struct SedimentState {
    pub kb: Arc<KbState>,
    pub cloud: CloudState,
    pub experts: Arc<ExpertsSharedState>,
    orchestrator: String,
    service_token: Option<String>,
    http: reqwest::Client,
}

impl SedimentState {
    pub fn new(kb: Arc<KbState>, cloud: CloudState, experts: Arc<ExpertsSharedState>) -> Self {
        let orchestrator = std::env::var("ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());
        // 与网关反代同口径：ORCHESTRATOR_SERVICE_TOKEN → OUS_API_TOKEN → 启动脚本默认 dev-secret-token
        let service_token = std::env::var("ORCHESTRATOR_SERVICE_TOKEN")
            .ok()
            .or_else(|| std::env::var("OUS_API_TOKEN").ok())
            .or_else(|| Some("dev-secret-token".to_string()));
        Self {
            kb,
            cloud,
            experts,
            orchestrator,
            service_token,
            http: reqwest::Client::new(),
        }
    }
}

/// 路由装配（自包含状态，nest 挂到 /api/alliance 下，随网关注册中心 merge）
pub fn build_sediment_router(state: SedimentState) -> Router<()> {
    Router::new()
        .nest(
            "/api/alliance",
            Router::new()
                .route("/sediment", post(sediment_dialogue))
                .with_state(state),
        )
        .with_state(())
}

/// POST /api/alliance/sediment —— 对话核心内容自动归类沉淀
async fn sediment_dialogue(
    State(state): State<SedimentState>,
    Json(req): Json<SedimentRequest>,
) -> ApiResponse<Value> {
    // ===== 1. 读取对话核心内容 =====
    let category = req.category.clone().unwrap_or_else(|| "cat-dialogue".to_string());
    let (session_id, title, messages) = match req.source.as_str() {
        "dialogue" => match &req.session_id {
            Some(sid) => match fetch_dialogue_transcript(&state, sid).await {
                Ok((t, msgs)) => (sid.clone(), t, msgs),
                Err(e) => return api_error(500, format!("读取对话核心内容失败: {e}")),
            },
            None => return api_error(400, "source=dialogue 时必须提供 session_id"),
        },
        "expert" => match &req.session_id {
            Some(sid) => {
                let session = state.experts.sessions.lock().get(sid).cloned();
                match session {
                    Some(s) => {
                        let msgs: Vec<SedimentMessage> = s
                            .messages
                            .iter()
                            .map(|m| SedimentMessage {
                                role: m.role.clone(),
                                content: m.content.clone(),
                            })
                            .collect();
                        if msgs.is_empty() {
                            return api_error(400, format!("专家会话无消息内容: {sid}"));
                        }
                        let title = if s.title.trim().is_empty() {
                            format!("专家会话 {sid}")
                        } else {
                            s.title.clone()
                        };
                        (sid.clone(), title, msgs)
                    }
                    None => return api_error(404, format!("专家会话不存在: {sid}")),
                }
            }
            None => return api_error(400, "source=expert 时必须提供 session_id"),
        },
        "inline" => {
            let title = req
                .title
                .clone()
                .unwrap_or_else(|| "对话沉淀（内联）".to_string());
            let sid = req.session_id.clone().unwrap_or_else(|| {
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                format!("inline-{millis}")
            });
            if req.messages.is_empty() {
                return api_error(400, "source=inline 时必须提供 messages");
            }
            (sid, title, req.messages)
        }
        other => {
            return api_error(
                400,
                format!("未知 source: {other}（支持 dialogue / expert / inline）"),
            )
        }
    };

    // ===== 2. 知识库：创建文档（原文） + 分析（摘要/关键词/实体/关系） + 挂图 =====
    let transcript_md = build_transcript_md(&title, &session_id, &messages);
    let mut doc = match state.kb.docs.create(&title, &transcript_md, Some(&category)).await {
        Ok(d) => d,
        Err(e) => return api_error(500, format!("知识库文档创建失败: {e}")),
    };
    let analysis = match KbAnalyzer.analyze(&mut doc).await {
        Ok(a) => a,
        Err(e) => return api_error(500, format!("对话核心内容分析失败: {e}")),
    };
    if let Err(e) = state.kb.docs.save(&doc).await {
        return api_error(500, format!("知识库文档保存失败: {e}"));
    }
    let chunks = chunk_doc(&doc);
    let link = GraphLinker.link(&state.kb.graph, &doc, &chunks);
    doc.status = STATUS_LINKED.into();
    if let Err(e) = state.kb.docs.save(&doc).await {
        return api_error(500, format!("知识库文档挂图状态保存失败: {e}"));
    }

    // ===== 3. 云盘：对话纪要 Markdown（原文 + AI 摘要/实体/标签）写入 bucket=dialogue =====
    let final_md = build_sediment_md(&title, &doc, &messages);
    let cloud_key = format!("{}.md", sanitize_key(&session_id));
    let cloud_path = match state.cloud.put_object_text("dialogue", &cloud_key, &final_md) {
        Ok(p) => p,
        Err(e) => return api_error(500, format!("云盘写入失败: {e}")),
    };

    // ===== 4. 汇总响应 =====
    let entities: Vec<Value> = analysis
        .entities
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "type": e.entity_type,
                "frequency": e.frequency,
                "snippet": e.snippet,
            })
        })
        .collect();
    let relations = serde_json::to_value(&analysis.relations).unwrap_or(Value::Null);
    api_ok(json!({
        "ok": true,
        "sediment": {
            "source": req.source,
            "session_id": session_id,
            "title": title,
            "summary": analysis.summary,
            "keywords": analysis.keywords,
            "entities": entities,
            "relations": relations,
            "kb": {
                "doc_id": doc.id,
                "category": doc.category,
                "status": doc.status,
                "tags": doc.tags,
                "graph_nodes_added": link.nodes_added,
                "graph_edges_added": link.edges_added,
                "graph_total_nodes": link.graph_nodes,
                "graph_total_edges": link.graph_edges,
            },
            "cloud": {
                "bucket": "dialogue",
                "key": cloud_key,
                "path": cloud_path,
            },
        }
    }))
}

/// 读取编排器 dialogue 会话完整内容（标题 + 全部消息）
async fn fetch_dialogue_transcript(
    state: &SedimentState,
    session_id: &str,
) -> Result<(String, Vec<SedimentMessage>), String> {
    let url = format!(
        "{}/api/dialogue/sessions/{}/messages",
        state.orchestrator, session_id
    );
    let mut rb = state.http.get(&url);
    if let Some(tok) = &state.service_token {
        rb = rb.bearer_auth(tok);
    }
    let resp = rb.send().await.map_err(|e| format!("连接编排器失败: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| format!("编排器响应解析失败: {e}"))?;
    if !status.is_success() {
        return Err(format!("编排器返回 {status}: {body}"));
    }
    let data = body.get("data").cloned().unwrap_or(body);
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("对话沉淀")
        .to_string();
    let mut msgs = Vec::new();
    if let Some(arr) = data.get("messages").and_then(|v| v.as_array()) {
        for m in arr {
            let role = m
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let content = m
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            msgs.push(SedimentMessage { role, content });
        }
    }
    if msgs.is_empty() {
        return Err(format!("会话 {session_id} 无消息内容"));
    }
    Ok((title, msgs))
}

/// 对话原文 Markdown（知识库文档正文）
fn build_transcript_md(title: &str, session_id: &str, messages: &[SedimentMessage]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    out.push_str(&format!("> 会话ID: {session_id}\n\n"));
    for m in messages {
        let who = match m.role.as_str() {
            "user" => "用户",
            "assistant" | "expert" => "AI 专家",
            other => other,
        };
        out.push_str(&format!("### {who}\n{}\n\n", m.content));
    }
    out
}

/// 对话纪要 Markdown（云盘对象：原文 + AI 提取的要点/实体/标签）
fn build_sediment_md(title: &str, doc: &KbDocument, messages: &[SedimentMessage]) -> String {
    let mut out = build_transcript_md(title, &doc.id, messages);
    out.push_str("\n---\n\n## 会话要点（AI 自动提取）\n\n");
    if !doc.summary.trim().is_empty() {
        out.push_str(&format!("**摘要**：{}\n\n", doc.summary));
    }
    if !doc.entities.is_empty() {
        let names: Vec<String> = doc.entities.iter().map(|e| e.name.clone()).collect();
        out.push_str(&format!("**核心实体**：{}\n\n", names.join("、")));
    }
    if !doc.tags.is_empty() {
        out.push_str(&format!("**标签**：{}\n", doc.tags.join("、")));
    }
    out
}

/// 云盘对象 key 白名单（字母/数字/_/-/.，其余替换为 _）
fn sanitize_key(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_md_builds_markdown() {
        let msgs = vec![
            SedimentMessage {
                role: "user".into(),
                content: "帮我规划专家联盟的AI对话".into(),
            },
            SedimentMessage {
                role: "assistant".into(),
                content: "已生成三步规划".into(),
            },
        ];
        let md = build_transcript_md("测试会话", "s1", &msgs);
        assert!(md.contains("# 测试会话"));
        assert!(md.contains("会话ID: s1"));
        assert!(md.contains("帮我规划专家联盟的AI对话"));
        assert!(md.contains("AI 专家"));
    }

    #[test]
    fn sanitize_key_keeps_safe_chars() {
        assert_eq!(sanitize_key("a-b_c.d-123"), "a-b_c.d-123");
        assert_eq!(sanitize_key("对话/会话"), "_____");
        assert_eq!(sanitize_key(""), "");
    }

    #[tokio::test]
    async fn sediment_creates_analyzed_linked_doc() {
        // 独立临时数据目录，避免污染进程级存储
        let dir = std::env::temp_dir().join(format!(
            "sediment-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        let kb = Arc::new(KbState::with_data_dir(dir.clone()));

        let msgs = vec![
            SedimentMessage {
                role: "user".into(),
                content: "请设计一个多专家协同的知识图谱沉淀方案".into(),
            },
            SedimentMessage {
                role: "assistant".into(),
                content: "建议由编排器统一调度，知识抽取后写入图谱、云盘与知识库".into(),
            },
        ];
        let md = build_transcript_md("沉淀测试", "t1", &msgs);
        let mut doc = kb
            .docs
            .create("沉淀测试", &md, Some("cat-dialogue"))
            .await
            .expect("create doc");
        let analysis = KbAnalyzer.analyze(&mut doc).await.expect("analyze doc");
        kb.docs.save(&doc).await.expect("save doc");
        assert!(!analysis.summary.is_empty());
        assert!(!analysis.entities.is_empty());

        let chunks = chunk_doc(&doc);
        let link = GraphLinker.link(&kb.graph, &doc, &chunks);
        assert!(link.nodes_added >= 1, "挂图应新增节点");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
