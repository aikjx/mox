//! 消息推送 API 端点
//!
//! 支持站内信 / 邮件 / 短信 / 飞书 / 钉钉 / 企业微信 等多渠道消息推送

use crate::message_center::*;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 消息中心状态
pub struct MessageCenterState {
    pub messages: Arc<RwLock<HashMap<String, Message>>>,
    pub send_records: Arc<RwLock<Vec<MessageSendRecord>>>,
    pub templates: Arc<RwLock<HashMap<String, MessageTemplate>>>,
    pub channel_configs: Arc<RwLock<HashMap<String, ChannelConfig>>>,
}

impl MessageCenterState {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        for tpl in builtin_templates() {
            templates.insert(tpl.template_id.clone(), tpl);
        }
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
            send_records: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(templates)),
            channel_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MessageCenterState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/message/channels —— 获取支持的渠道列表
pub async fn list_channels_handler() -> Response {
    let channels = supported_channels();
    let result: Vec<serde_json::Value> = channels.iter().map(|c| {
        json!({ "code": c.as_str(), "name": c.display_name() })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/enterprise/message/messages —— 获取消息列表
pub async fn list_messages_handler(
    State(state): State<Arc<MessageCenterState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let messages = state.messages.read().await;
    let mut list: Vec<&Message> = messages.values().collect();
    if let Some(receiver) = params.get("receiver_id") {
        list.retain(|m| m.receiver_ids.contains(receiver));
    }
    if let Some(mtype) = params.get("message_type") {
        list.retain(|m| format!("{:?}", m.message_type).to_lowercase() == *mtype);
    }
    if let Some(status) = params.get("status") {
        list.retain(|m| format!("{:?}", m.status).to_lowercase() == *status);
    }
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/enterprise/message/messages/:id —— 获取消息详情
pub async fn get_message_handler(
    State(state): State<Arc<MessageCenterState>>,
    Path(id): Path<String>,
) -> Response {
    let messages = state.messages.read().await;
    match messages.get(&id) {
        Some(m) => Json(json!({ "code": 0, "data": m })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "消息不存在" }))).into_response(),
    }
}

/// POST /api/enterprise/message/send —— 发送消息
pub async fn send_message_handler(
    State(state): State<Arc<MessageCenterState>>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    // 如果使用模板，渲染模板内容
    let (title, content) = if let Some(template_id) = &req.template_id {
        let templates = state.templates.read().await;
        if let Some(tpl) = templates.get(template_id) {
            let mut title = tpl.title_template.clone();
            let mut content = tpl.content_template.clone();
            if let Some(vars) = &req.template_vars {
                for (k, v) in vars {
                    title = title.replace(&format!("{{{{{}}}}}", k), v);
                    content = content.replace(&format!("{{{{{}}}}}", k), v);
                }
            }
            (title, content)
        } else {
            (req.title, req.content)
        }
    } else {
        (req.title, req.content)
    };

    let message = Message {
        message_id: message_id.clone(),
        tenant_id: "default".to_string(),
        message_type: req.message_type,
        title,
        content,
        html_content: req.html_content,
        priority: req.priority.unwrap_or(MessagePriority::Normal),
        channels: req.channels,
        sender_id: "system".to_string(),
        receiver_ids: req.receiver_ids.unwrap_or_default(),
        receiver_emails: req.receiver_emails,
        receiver_phones: req.receiver_phones,
        action_url: req.action_url,
        extra_data: req.extra_data,
        scheduled_at: req.scheduled_at,
        expires_at: None,
        status: MessageStatus::Pending,
        created_at: now.clone(),
        updated_at: now,
        sent_at: None,
    };

    // 模拟发送（实际实现中需要调用各渠道API）
    let mut msg = message.clone();
    msg.status = MessageStatus::Sent;
    msg.sent_at = Some(chrono::Utc::now().to_rfc3339());
    state.messages.write().await.insert(message_id.clone(), msg);

    // 记录发送记录
    let record = MessageSendRecord {
        record_id: format!("rec_{}", uuid::Uuid::new_v4().simple()),
        message_id: message_id.clone(),
        receiver_id: "user_001".to_string(),
        channel: MessageChannel::InApp,
        status: MessageStatus::Sent,
        error_message: None,
        retry_count: 0,
        sent_at: Some(chrono::Utc::now().to_rfc3339()),
        read_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.send_records.write().await.push(record);

    Json(json!({
        "code": 0,
        "message": "消息发送成功",
        "data": { "message_id": message_id }
    })).into_response()
}

/// POST /api/enterprise/message/messages/:id/read —— 标记消息已读
pub async fn mark_read_handler(
    State(state): State<Arc<MessageCenterState>>,
    Path(id): Path<String>,
) -> Response {
    let mut messages = state.messages.write().await;
    if let Some(msg) = messages.get_mut(&id) {
        msg.status = MessageStatus::Read;
        msg.updated_at = chrono::Utc::now().to_rfc3339();
    }
    Json(json!({ "code": 0, "message": "标记已读成功" })).into_response()
}

/// GET /api/enterprise/message/templates —— 获取消息模板列表
pub async fn list_templates_handler(
    State(state): State<Arc<MessageCenterState>>,
) -> Response {
    let templates = state.templates.read().await;
    let list: Vec<&MessageTemplate> = templates.values().collect();
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/enterprise/message/stats —— 获取消息统计
pub async fn message_stats_handler(
    State(state): State<Arc<MessageCenterState>>,
) -> Response {
    let messages = state.messages.read().await;
    let mut stats = MessageStats::default();
    stats.total = messages.len() as i64;
    for msg in messages.values() {
        match msg.status {
            MessageStatus::Sent => stats.sent += 1,
            MessageStatus::Read => stats.read += 1,
            MessageStatus::Failed => stats.failed += 1,
            MessageStatus::Pending => stats.pending += 1,
            _ => {}
        }
    }
    Json(json!({ "code": 0, "data": stats })).into_response()
}

/// 构建消息中心路由（泛型版本）
pub fn build_message_center_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<MessageCenterState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/channels", get(list_channels_handler))
        .route("/messages", get(list_messages_handler))
        .route("/messages/:id", get(get_message_handler))
        .route("/messages/:id/read", post(mark_read_handler))
        .route("/send", post(send_message_handler))
        .route("/templates", get(list_templates_handler))
        .route("/stats", get(message_stats_handler))
}
