// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟会话持久化域（Experts Session）HTTP 路由
//!
//! 提供企业级专家会话的全生命周期管理：创建、列表、统计、详情、更新、删除、
//! 消息追加、会话内相似搜索、全域语义搜索、导出、归档。
//!
//! 路径前缀：`/api/experts/sessions/*` 及 `/api/experts/semantic-search`
//!
//! 设计原则：
//! - 所有写操作后立即 `save_sessions()` 事务化持久化到 SQLite（`data/experts.db`，
//!   WAL 并发 + 消息规范化投影，历史 JSON 启动时自动迁移）
//! - 列表接口剥离完整 messages，仅返回 `message_count`，避免响应体膨胀
//! - 相似搜索基于 `experts_common::text_similarity()`（字符 bigram Jaccard）
//! - 响应信封统一 `mox_api_protocol::{ApiResponse, api_ok, api_error}`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use mox_api_protocol::ApiResponse;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

use super::experts_common::*;
use mox_audit::{AuditAction, AuditOutcome};

// =====================================================================
// 一、请求体定义
// =====================================================================

/// 创建会话请求体
#[derive(Debug, Deserialize)]
pub struct CreateSessionBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub expert_ids: Option<Vec<String>>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub session_type: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, Value>>,
}

/// 更新会话请求体（合并式更新，所有字段可选）
#[derive(Debug, Deserialize)]
pub struct UpdateSessionBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, Value>>,
}

/// 追加消息请求体
#[derive(Debug, Deserialize)]
pub struct AppendMessageBody {
    pub role: String,
    #[serde(default)]
    pub sender_id: Option<String>,
    #[serde(default)]
    pub sender_name: Option<String>,
    pub content: String,
    #[serde(default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<Value>>,
    #[serde(default)]
    pub rating: Option<u8>,
}

/// 会话内相似搜索请求体
#[derive(Debug, Deserialize)]
pub struct SimilarSearchBody {
    pub query: String,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub min_score: Option<f64>,
}

/// 全域语义搜索请求体
#[derive(Debug, Deserialize)]
pub struct SemanticSearchBody {
    pub query: String,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub session_type: Option<String>,
    #[serde(default)]
    pub expert_id: Option<String>,
}

// =====================================================================
// 二、内部工具：将会话转为列表视图（剥离 messages，附加 message_count）
// =====================================================================

fn session_to_list_view(session: &ExpertSession) -> Value {
    json!({
        "id": session.id,
        "title": session.title,
        "expert_ids": session.expert_ids,
        "user_id": session.user_id,
        "session_type": session.session_type,
        "status": session.status,
        "topic": session.topic,
        "message_count": session.messages.len(),
        "tags": session.tags,
        "metadata": session.metadata,
        "created_at": session.created_at,
        "last_active_at": session.last_active_at,
        "archived_at": session.archived_at,
    })
}

/// 从 RFC3339 字符串提取日期部分（YYYY-MM-DD），用于"今日"判断
fn date_part(ts: &str) -> &str {
    if ts.len() >= 10 {
        &ts[..10]
    } else {
        ts
    }
}

/// 解析两个 RFC3339 时间戳的分钟差（失败返回 None）
fn duration_minutes(start: &str, end: &str) -> Option<f64> {
    let s = chrono::DateTime::parse_from_rfc3339(start).ok()?;
    let e = chrono::DateTime::parse_from_rfc3339(end).ok()?;
    Some((e - s).num_seconds() as f64 / 60.0)
}

// =====================================================================
// 三、端点 Handler
// =====================================================================

// ---------------------------------------------------------------------
// 1. POST /api/experts/sessions — 创建会话
// ---------------------------------------------------------------------
async fn create_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<CreateSessionBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let session_id = gen_id("sess");

    let session = ExpertSession {
        id: session_id.clone(),
        title: body.title.unwrap_or_default(),
        expert_ids: body.expert_ids.unwrap_or_default(),
        user_id: body.user_id.unwrap_or_default(),
        session_type: body.session_type.unwrap_or_else(|| "single".into()),
        status: "active".into(),
        topic: body.topic.unwrap_or_default(),
        messages: Vec::new(),
        tags: body.tags.unwrap_or_default(),
        metadata: body.metadata.unwrap_or_default(),
        created_at: now.clone(),
        last_active_at: now.clone(),
        archived_at: None,
    };

    {
        let mut sessions = state.sessions.lock();
        sessions.insert(session_id.clone(), session.clone());
        save_sessions(&sessions);
    }

    emit_audit(&state, AuditAction::Unknown("session.create".into()), "session", &session_id, AuditOutcome::Success, Some(&format!("type={}", session.session_type)));

    ok(json!(session))
}

// ---------------------------------------------------------------------
// 2. GET /api/experts/sessions — 会话列表（分页 + 过滤 + 搜索）
// ---------------------------------------------------------------------
async fn list_sessions(
    State(state): State<Arc<ExpertsSharedState>>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let (offset, page_size) = parse_pagination(&params);

    let filter_status = params.get("status").map(|s| s.as_str());
    let filter_type = params.get("session_type").map(|s| s.as_str());
    let filter_expert = params.get("expert_id").map(|s| s.as_str());
    let filter_user = params.get("user_id").map(|s| s.as_str());
    let search = params.get("search").map(|s| s.to_lowercase());

    let sessions = state.sessions.lock();
    let mut filtered: Vec<&ExpertSession> = sessions
        .values()
        .filter(|s| {
            if let Some(st) = filter_status {
                if s.status != st {
                    return false;
                }
            }
            if let Some(t) = filter_type {
                if s.session_type != t {
                    return false;
                }
            }
            if let Some(eid) = filter_expert {
                if !s.expert_ids.iter().any(|id| id == eid) {
                    return false;
                }
            }
            if let Some(uid) = filter_user {
                if s.user_id != uid {
                    return false;
                }
            }
            if let Some(q) = &search {
                let title_match = s.title.to_lowercase().contains(q);
                let topic_match = s.topic.to_lowercase().contains(q);
                if !title_match && !topic_match {
                    return false;
                }
            }
            true
        })
        .collect();

    // 按 created_at 降序（RFC3339 字符串可直接字典序比较）
    filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = filtered.len();
    let page_items: Vec<Value> = filtered
        .into_iter()
        .skip(offset)
        .take(page_size)
        .map(|s| session_to_list_view(s))
        .collect();

    let page = if page_size > 0 { offset / page_size + 1 } else { 1 };

    ok(json!({
        "sessions": page_items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

// ---------------------------------------------------------------------
// 3. GET /api/experts/sessions/stats — 会话统计
// ---------------------------------------------------------------------
async fn session_stats(
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let sessions = state.sessions.lock();
    let now_str = now_iso();
    let today = date_part(&now_str);

    let total_sessions = sessions.len();
    let mut active_sessions = 0u64;
    let mut archived_sessions = 0u64;
    let mut closed_sessions = 0u64;
    let mut total_messages = 0u64;
    let mut sessions_today = 0u64;
    let mut duration_sum = 0.0f64;
    let mut duration_count = 0u64;
    let mut expert_counts: HashMap<String, u64> = HashMap::new();
    let mut type_dist: HashMap<String, u64> = HashMap::new();

    for s in sessions.values() {
        match s.status.as_str() {
            "active" => active_sessions += 1,
            "archived" => archived_sessions += 1,
            "closed" => closed_sessions += 1,
            _ => {}
        }
        total_messages += s.messages.len() as u64;

        if date_part(&s.created_at) == today {
            sessions_today += 1;
        }

        if !s.last_active_at.is_empty() && !s.created_at.is_empty() {
            if let Some(dur) = duration_minutes(&s.created_at, &s.last_active_at) {
                if dur >= 0.0 {
                    duration_sum += dur;
                    duration_count += 1;
                }
            }
        }

        for eid in &s.expert_ids {
            *expert_counts.entry(eid.clone()).or_insert(0) += 1;
        }

        *type_dist.entry(s.session_type.clone()).or_insert(0) += 1;
    }

    let avg_messages = if total_sessions > 0 {
        total_messages as f64 / total_sessions as f64
    } else {
        0.0
    };
    let avg_duration = if duration_count > 0 {
        duration_sum / duration_count as f64
    } else {
        0.0
    };

    // top_experts_by_sessions：按计数降序取前 10
    let mut top_experts: Vec<(String, u64)> = expert_counts.into_iter().collect();
    top_experts.sort_by(|a, b| b.1.cmp(&a.1));
    let top_experts_list: Vec<Value> = top_experts
        .into_iter()
        .take(10)
        .map(|(id, count)| json!({ "expert_id": id, "count": count }))
        .collect();

    ok(json!({
        "total_sessions": total_sessions,
        "active_sessions": active_sessions,
        "archived_sessions": archived_sessions,
        "closed_sessions": closed_sessions,
        "total_messages": total_messages,
        "avg_messages_per_session": avg_messages,
        "avg_session_duration_minutes": avg_duration,
        "sessions_today": sessions_today,
        "top_experts_by_sessions": top_experts_list,
        "session_type_distribution": type_dist,
        "ts": now_iso(),
    }))
}

// ---------------------------------------------------------------------
// 4. GET /api/experts/sessions/:id — 获取单个会话详情（含完整 messages）
// ---------------------------------------------------------------------
async fn get_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let sessions = state.sessions.lock();
    match sessions.get(&id) {
        Some(session) => ok(json!(session)),
        None => err(404, format!("session not found: {id}")),
    }
}

// ---------------------------------------------------------------------
// 5. PUT /api/experts/sessions/:id — 更新会话（合并式）
// ---------------------------------------------------------------------
async fn update_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSessionBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    {
        let mut sessions = state.sessions.lock();
        match sessions.get_mut(&id) {
            Some(session) => {
                if let Some(title) = body.title {
                    session.title = title;
                }
                if let Some(status) = body.status {
                    session.status = status;
                }
                if let Some(topic) = body.topic {
                    session.topic = topic;
                }
                if let Some(tags) = body.tags {
                    session.tags = tags;
                }
                if let Some(metadata) = body.metadata {
                    for (k, v) in metadata {
                        session.metadata.insert(k, v);
                    }
                }
                session.last_active_at = now.clone();
                let response = ok(json!(session.clone()));
                save_sessions(&sessions);
                response
            }
            None => err(404, format!("session not found: {id}")),
        }
    }
}

// ---------------------------------------------------------------------
// 6. DELETE /api/experts/sessions/:id — 删除会话
// ---------------------------------------------------------------------
async fn delete_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    {
        let mut sessions = state.sessions.lock();
        if sessions.remove(&id).is_some() {
            save_sessions(&sessions);
            emit_audit(&state, AuditAction::Unknown("session.delete".into()), "session", &id, AuditOutcome::Success, None);
            return ok(json!({
                "deleted": true,
                "session_id": id,
            }));
        }
    }
    err(404, format!("session not found: {id}"))
}

// ---------------------------------------------------------------------
// 7. POST /api/experts/sessions/:id/messages — 追加消息
// ---------------------------------------------------------------------
async fn append_message(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<AppendMessageBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let msg_id = gen_id("msg");

    let message = SessionMessage {
        id: msg_id.clone(),
        role: body.role,
        sender_id: body.sender_id.unwrap_or_default(),
        sender_name: body.sender_name.unwrap_or_default(),
        content: body.content,
        msg_type: body.msg_type.unwrap_or_else(|| "text".into()),
        attachments: body.attachments.unwrap_or_default(),
        rating: body.rating,
        created_at: now.clone(),
    };

    {
        let mut sessions = state.sessions.lock();
        match sessions.get_mut(&id) {
            Some(session) => {
                session.messages.push(message.clone());
                session.last_active_at = now.clone();
                save_sessions(&sessions);
                emit_audit(&state, AuditAction::Unknown("session.append_message".into()), "session", &id, AuditOutcome::Success, Some(&format!("msg_id={}", msg_id)));
                ok(json!(message))
            }
            None => err(404, format!("session not found: {id}")),
        }
    }
}

// ---------------------------------------------------------------------
// 8. POST /api/experts/sessions/:id/similar-search — 会话内相似消息搜索
// ---------------------------------------------------------------------
async fn similar_search(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<SimilarSearchBody>,
) -> ApiResponse<Value> {
    let top_k = body.top_k.unwrap_or(5);
    let min_score = body.min_score.unwrap_or(0.1);

    let sessions = state.sessions.lock();
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => return err(404, format!("session not found: {id}")),
    };

    let mut scored: Vec<(&SessionMessage, f64)> = session
        .messages
        .iter()
        .map(|m| {
            let score = text_similarity(&body.query, &m.content);
            (m, score)
        })
        .filter(|(_, score)| *score >= min_score)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total_found = scored.len();
    let results: Vec<Value> = scored
        .into_iter()
        .take(top_k)
        .enumerate()
        .map(|(idx, (msg, score))| {
            json!({
                "message": msg,
                "similarity_score": score,
                "rank": idx + 1,
            })
        })
        .collect();

    ok(json!({
        "query": body.query,
        "session_id": id,
        "results": results,
        "total_found": total_found,
    }))
}

// ---------------------------------------------------------------------
// 9. POST /api/experts/semantic-search — 全域语义搜索（跨会话）
// ---------------------------------------------------------------------
async fn semantic_search(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<SemanticSearchBody>,
) -> ApiResponse<Value> {
    let top_k = body.top_k.unwrap_or(10);
    let filter_type = body.session_type.as_deref();
    let filter_expert = body.expert_id.as_deref();

    let sessions = state.sessions.lock();
    let mut total_sessions_scanned = 0u64;
    let mut total_messages_scanned = 0u64;
    let mut scored: Vec<(String, String, &SessionMessage, f64)> = Vec::new();

    for session in sessions.values() {
        // 会话级过滤
        if let Some(t) = filter_type {
            if session.session_type != t {
                continue;
            }
        }
        if let Some(eid) = filter_expert {
            if !session.expert_ids.iter().any(|id| id == eid) {
                continue;
            }
        }

        total_sessions_scanned += 1;
        for msg in &session.messages {
            total_messages_scanned += 1;
            let score = text_similarity(&body.query, &msg.content);
            if score > 0.0 {
                scored.push((session.id.clone(), session.title.clone(), msg, score));
            }
        }
    }

    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    let results: Vec<Value> = scored
        .into_iter()
        .take(top_k)
        .map(|(sid, stitle, msg, score)| {
            json!({
                "session_id": sid,
                "session_title": stitle,
                "message": msg,
                "similarity_score": score,
            })
        })
        .collect();

    ok(json!({
        "query": body.query,
        "results": results,
        "total_sessions_scanned": total_sessions_scanned,
        "total_messages_scanned": total_messages_scanned,
    }))
}

// ---------------------------------------------------------------------
// 10. GET /api/experts/sessions/:id/export — 导出会话
// ---------------------------------------------------------------------
async fn export_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let sessions = state.sessions.lock();
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => return err(404, format!("session not found: {id}")),
    };

    let content = json!(session);
    let message_count = session.messages.len();

    ok(json!({
        "session_id": id,
        "format": "json",
        "exported_at": now_iso(),
        "content": content,
        "download_url": null,
        "message_count": message_count,
    }))
}

// ---------------------------------------------------------------------
// 11. POST /api/experts/sessions/:id/archive — 归档会话
// ---------------------------------------------------------------------
async fn archive_session(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let now = now_iso();
    {
        let mut sessions = state.sessions.lock();
        match sessions.get_mut(&id) {
            Some(session) => {
                session.status = "archived".into();
                session.archived_at = Some(now.clone());
                session.last_active_at = now.clone();
                let message_count = session.messages.len();
                save_sessions(&sessions);
                emit_audit(&state, AuditAction::Unknown("session.archive".into()), "session", &id, AuditOutcome::Success, Some(&format!("message_count={}", message_count)));
                ok(json!({
                    "session_id": id,
                    "status": "archived",
                    "archived_at": now,
                    "message_count": message_count,
                }))
            }
            None => err(404, format!("session not found: {id}")),
        }
    }
}

// =====================================================================
// 四、路由装配
// =====================================================================

pub fn build_experts_session_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        // 创建 + 列表（同路径不同方法，合并 MethodRouter）
        .route(
            "/api/experts/sessions",
            post(create_session).get(list_sessions),
        )
        // 统计（必须在 /:id 之前注册，避免 stats 被捕获为路径参数）
        .route("/api/experts/sessions/stats", get(session_stats))
        // 详情 + 更新 + 删除（同路径不同方法）
        .route(
            "/api/experts/sessions/:id",
            get(get_session).put(update_session).delete(delete_session),
        )
        // 追加消息
        .route("/api/experts/sessions/:id/messages", post(append_message))
        // 会话内相似搜索
        .route("/api/experts/sessions/:id/similar-search", post(similar_search))
        // 导出
        .route("/api/experts/sessions/:id/export", get(export_session))
        // 归档
        .route("/api/experts/sessions/:id/archive", post(archive_session))
        // 全域语义搜索（独立路径，不在 /sessions/ 下）
        .route("/api/experts/semantic-search", post(semantic_search))
        .with_state(state)
}

// =====================================================================
// 五、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    /// 构造一个干净的测试用共享状态（空注册表 + 空会话）
    fn test_state() -> Arc<ExpertsSharedState> {
        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(ExpertGraph::default())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
            audit: crate::experts_common::build_audit_context(),
        })
    }

    fn make_session(id: &str, title: &str) -> ExpertSession {
        let now = now_iso();
        ExpertSession {
            id: id.into(),
            title: title.into(),
            expert_ids: vec!["exp-001".into()],
            user_id: "user-001".into(),
            session_type: "single".into(),
            status: "active".into(),
            topic: "测试主题".into(),
            messages: Vec::new(),
            tags: vec!["test".into()],
            metadata: HashMap::new(),
            created_at: now.clone(),
            last_active_at: now,
            archived_at: None,
        }
    }

    // --- 测试 1：创建会话 ---
    #[tokio::test]
    async fn test_create_session() {
        let state = test_state();
        let body = CreateSessionBody {
            title: Some(" Rust 架构咨询".into()),
            expert_ids: Some(vec!["exp-arch-001".into()]),
            user_id: Some("user-alice".into()),
            session_type: Some("single".into()),
            topic: Some("微服务拆分".into()),
            tags: Some(vec!["rust".into(), "architecture".into()]),
            metadata: None,
        };
        let resp = create_session(State(state.clone()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["status"], "active");
        assert_eq!(data["title"], " Rust 架构咨询");
        assert_eq!(data["expert_ids"][0], "exp-arch-001");
        assert_eq!(data["messages"], json!([]));
        assert!(data["id"].as_str().unwrap().starts_with("sess-"));
        // 验证已写入 state
        assert_eq!(state.sessions.lock().len(), 1);
    }

    // --- 测试 2：获取单个会话详情 ---
    #[tokio::test]
    async fn test_get_session() {
        let state = test_state();
        let session = make_session("sess-get-001", "获取测试");
        state.sessions.lock().insert("sess-get-001".into(), session);

        let resp = get_session(State(state.clone()), Path("sess-get-001".into())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["id"], "sess-get-001");
        assert_eq!(data["title"], "获取测试");

        // 不存在返回 404
        let resp404 = get_session(State(state.clone()), Path("nonexistent".into())).await;
        assert_eq!(resp404.code, 404);
    }

    // --- 测试 3：列表分页 ---
    #[tokio::test]
    async fn test_list_sessions_pagination() {
        let state = test_state();
        // 插入 5 个会话
        for i in 0..5 {
            let mut s = make_session(&format!("sess-list-{:02}", i), &format!("标题{}", i));
            s.created_at = format!("2026-09-0{:02}T10:00:00Z", i + 1);
            state.sessions.lock().insert(s.id.clone(), s);
        }

        // 第 1 页，page_size=2
        let mut params = HashMap::new();
        params.insert("page".into(), "1".into());
        params.insert("page_size".into(), "2".into());
        let resp = list_sessions(State(state.clone()), Query(params)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["total"], 5);
        assert_eq!(data["page"], 1);
        assert_eq!(data["page_size"], 2);
        assert_eq!(data["sessions"].as_array().unwrap().len(), 2);
        // 按 created_at 降序，第一条应该是 sess-list-04 (2026-09-05)
        assert_eq!(data["sessions"][0]["id"], "sess-list-04");
        // 列表项不含 messages 字段，但含 message_count
        assert!(data["sessions"][0].get("messages").is_none());
        assert_eq!(data["sessions"][0]["message_count"], 0);

        // 第 3 页（最后 1 条）
        let mut params2 = HashMap::new();
        params2.insert("page".into(), "3".into());
        params2.insert("page_size".into(), "2".into());
        let resp2 = list_sessions(State(state.clone()), Query(params2)).await;
        let data2 = resp2.data.unwrap();
        assert_eq!(data2["sessions"].as_array().unwrap().len(), 1);
    }

    // --- 测试 4：追加消息 ---
    #[tokio::test]
    async fn test_append_message() {
        let state = test_state();
        let session = make_session("sess-msg-001", "消息测试");
        state.sessions.lock().insert("sess-msg-001".into(), session);

        let body = AppendMessageBody {
            role: "user".into(),
            sender_id: Some("user-alice".into()),
            sender_name: Some("Alice".into()),
            content: "如何设计高可用系统？".into(),
            msg_type: Some("text".into()),
            attachments: None,
            rating: None,
        };
        let resp = append_message(State(state.clone()), Path("sess-msg-001".into()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["role"], "user");
        assert_eq!(data["content"], "如何设计高可用系统？");
        assert!(data["id"].as_str().unwrap().starts_with("msg-"));

        // 验证会话 messages 已更新
        let sessions = state.sessions.lock();
        let s = sessions.get("sess-msg-001").unwrap();
        assert_eq!(s.messages.len(), 1);
        assert_eq!(s.messages[0].content, "如何设计高可用系统？");
    }

    // --- 测试 5：会话内相似搜索 ---
    #[tokio::test]
    async fn test_similar_search() {
        let state = test_state();
        let mut session = make_session("sess-sim-001", "相似搜索测试");
        session.messages = vec![
            SessionMessage {
                id: "msg-1".into(),
                role: "user".into(),
                sender_id: "".into(),
                sender_name: "".into(),
                content: "如何设计微服务架构的服务拆分策略".into(),
                msg_type: "text".into(),
                attachments: vec![],
                rating: None,
                created_at: now_iso(),
            },
            SessionMessage {
                id: "msg-2".into(),
                role: "expert".into(),
                sender_id: "".into(),
                sender_name: "".into(),
                content: "微服务架构设计需要考虑领域驱动设计和边界上下文".into(),
                msg_type: "text".into(),
                attachments: vec![],
                rating: None,
                created_at: now_iso(),
            },
            SessionMessage {
                id: "msg-3".into(),
                role: "user".into(),
                sender_id: "".into(),
                sender_name: "".into(),
                content: "今天天气真好适合散步".into(),
                msg_type: "text".into(),
                attachments: vec![],
                rating: None,
                created_at: now_iso(),
            },
        ];
        state.sessions.lock().insert("sess-sim-001".into(), session);

        let body = SimilarSearchBody {
            query: "微服务架构设计".into(),
            top_k: Some(2),
            min_score: Some(0.05),
        };
        let resp = similar_search(State(state.clone()), Path("sess-sim-001".into()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["session_id"], "sess-sim-001");
        assert_eq!(data["query"], "微服务架构设计");
        let results = data["results"].as_array().unwrap();
        assert!(!results.is_empty());
        // 第一条应该是最相关的（msg-1 或 msg-2），不应该是 msg-3
        let first_id = results[0]["message"]["id"].as_str().unwrap();
        assert!(first_id == "msg-1" || first_id == "msg-2");
        assert_eq!(results[0]["rank"], 1);
    }

    // --- 测试 6：归档会话 ---
    #[tokio::test]
    async fn test_archive_session() {
        let state = test_state();
        let session = make_session("sess-arch-001", "归档测试");
        state.sessions.lock().insert("sess-arch-001".into(), session);

        let resp = archive_session(State(state.clone()), Path("sess-arch-001".into())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["status"], "archived");
        assert_eq!(data["session_id"], "sess-arch-001");
        assert!(data["archived_at"].is_string());

        // 验证状态已更新
        let sessions = state.sessions.lock();
        let s = sessions.get("sess-arch-001").unwrap();
        assert_eq!(s.status, "archived");
        assert!(s.archived_at.is_some());
    }

    // --- 测试 7：更新会话（合并式） ---
    #[tokio::test]
    async fn test_update_session_merge() {
        let state = test_state();
        let session = make_session("sess-upd-001", "原始标题");
        state.sessions.lock().insert("sess-upd-001".into(), session);

        let body = UpdateSessionBody {
            title: Some("新标题".into()),
            status: None,
            topic: Some("新主题".into()),
            tags: None,
            metadata: Some({
                let mut m = HashMap::new();
                m.insert("priority".into(), json!("high"));
                m
            }),
        };
        let resp = update_session(State(state.clone()), Path("sess-upd-001".into()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["title"], "新标题");
        assert_eq!(data["topic"], "新主题");
        // status 未传，保持原值 active
        assert_eq!(data["status"], "active");
        assert_eq!(data["metadata"]["priority"], "high");
    }

    // --- 测试 8：删除会话 ---
    #[tokio::test]
    async fn test_delete_session() {
        let state = test_state();
        let session = make_session("sess-del-001", "删除测试");
        state.sessions.lock().insert("sess-del-001".into(), session);
        assert_eq!(state.sessions.lock().len(), 1);

        let resp = delete_session(State(state.clone()), Path("sess-del-001".into())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["deleted"], true);
        assert_eq!(data["session_id"], "sess-del-001");
        assert_eq!(state.sessions.lock().len(), 0);
    }

    // --- 测试 9：会话统计 ---
    #[tokio::test]
    async fn test_session_stats() {
        let state = test_state();
        let mut s1 = make_session("sess-stats-001", "统计1");
        s1.status = "active".into();
        s1.session_type = "single".into();
        s1.messages = vec![
            SessionMessage {
                id: "m1".into(), role: "user".into(), sender_id: "".into(),
                sender_name: "".into(), content: "msg1".into(), msg_type: "text".into(),
                attachments: vec![], rating: None, created_at: now_iso(),
            },
        ];
        let mut s2 = make_session("sess-stats-002", "统计2");
        s2.status = "archived".into();
        s2.session_type = "multi".into();
        s2.expert_ids = vec!["exp-001".into(), "exp-002".into()];
        state.sessions.lock().insert("sess-stats-001".into(), s1);
        state.sessions.lock().insert("sess-stats-002".into(), s2);

        let resp = session_stats(State(state.clone())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["total_sessions"], 2);
        assert_eq!(data["active_sessions"], 1);
        assert_eq!(data["archived_sessions"], 1);
        assert_eq!(data["total_messages"], 1);
        assert_eq!(data["avg_messages_per_session"], 0.5);
        assert!(data["top_experts_by_sessions"].is_array());
        assert!(data["session_type_distribution"]["single"] == 1);
        assert!(data["session_type_distribution"]["multi"] == 1);
    }
}
