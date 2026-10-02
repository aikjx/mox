//! 消息推送 API 端点
//!
//! 支持站内信 / 邮件 / 短信 / 飞书 / 钉钉 / 企业微信 等多渠道消息推送

use super::repository::{InboxRepository, StoreError};
use crate::{alliance::experts_common::TenantId, auth::ApiAuth, message_center::*};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// 消息中心状态
pub struct MessageCenterState {
    pub repository: InboxRepository,
    pub templates: Arc<RwLock<HashMap<String, MessageTemplate>>>,
    pub channel_configs: Arc<RwLock<HashMap<String, ChannelConfig>>>,
}

impl MessageCenterState {
    pub fn new() -> Self {
        Self::with_db_path(std::path::PathBuf::from(crate::store_json::store_db_path()))
    }

    pub fn with_db_path(path: std::path::PathBuf) -> Self {
        let mut templates = HashMap::new();
        for tpl in builtin_templates() {
            templates.insert(tpl.template_id.clone(), tpl);
        }
        Self {
            repository: InboxRepository::new(path),
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
        json!({ "code": c.as_str(), "name": c.display_name(), "available": *c == MessageChannel::InApp })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/enterprise/message/messages —— 获取消息列表
pub async fn list_messages_handler(
    State(state): State<Arc<MessageCenterState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if params.get("receiver_id").is_some_and(|id| id != &user.id) {
        return Json(json!({"code":0, "data":[], "total":0})).into_response();
    }
    let limit = match page_parameter(&params, "limit", 50, 1, 200) {
        Ok(value) => value,
        Err(message) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"code":400,"message":message})))
                .into_response()
        },
    };
    let offset = match page_parameter(&params, "offset", 0, 0, i64::MAX) {
        Ok(value) => value,
        Err(message) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"code":400,"message":message})))
                .into_response()
        },
    };
    let kind = params.get("message_type").cloned();
    let status = params.get("status").cloned();
    match run_store(&state, move |store| {
        store.list(&tenant, &user.id, limit, offset, kind.as_deref(), status.as_deref())
    })
    .await
    {
        Ok((list, total)) => {
            Json(json!({"code":0,"data":list,"total":total,"limit":limit,"offset":offset}))
                .into_response()
        },
        Err(error) => storage_error(error),
    }
}

/// GET /api/enterprise/message/messages/:id —— 获取消息详情
pub async fn get_message_handler(
    State(state): State<Arc<MessageCenterState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    match run_store(&state, move |store| store.get(&tenant, &user.id, &id)).await {
        Ok(Some(message)) => Json(json!({"code":0,"data":message})).into_response(),
        Ok(None) => missing_message(),
        Err(error) => storage_error(error),
    }
}

/// POST /api/enterprise/message/send —— 发送消息
pub async fn send_message_handler(
    State(state): State<Arc<MessageCenterState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    headers: HeaderMap,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    if req.channels.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code": 400, "message": "发送渠道不能为空"})),
        )
            .into_response();
    }
    if req.channels.iter().any(|channel| *channel != MessageChannel::InApp)
        || req.scheduled_at.is_some()
    {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"code": 501,
            "message": "外部渠道或定时发送适配器未接入，未发送消息"})),
        )
            .into_response();
    }
    let receivers: std::collections::HashSet<_> =
        req.receiver_ids.clone().unwrap_or_default().into_iter().collect();
    // User directory routing has not been connected: only a verified self inbox is available.
    if receivers.len() != 1 || !receivers.contains(&user.id) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"code": 501,
            "message": "跨用户收件人目录校验未接入；当前仅支持当前用户站内收件箱"})),
        )
            .into_response();
    }
    let key = match headers.get("idempotency-key") {
        Some(value) => match value.to_str() {
            Ok(value)
                if !value.is_empty()
                    && value.len() <= 128
                    && value.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c)) =>
            {
                Some(value.to_owned())
            },
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"code":400,"message":"幂等键格式无效"})),
                )
                    .into_response()
            },
        },
        None => None,
    };
    let fingerprint = match serde_json::to_value(&req).and_then(|mut value| {
        canonicalize(&mut value);
        serde_json::to_vec(&value)
    }) {
        Ok(bytes) => hex::encode(Sha256::digest(bytes)),
        Err(error) => return storage_error(StoreError::Unavailable(error.to_string())),
    };
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
            return (StatusCode::NOT_FOUND, Json(json!({"code":404,"message":"消息模板不存在"})))
                .into_response();
        }
    } else {
        (req.title, req.content)
    };

    let message = Message {
        message_id: message_id.clone(),
        tenant_id: tenant,
        message_type: req.message_type,
        title,
        content,
        html_content: req.html_content,
        priority: req.priority.unwrap_or(MessagePriority::Normal),
        channels: req.channels,
        sender_id: user.id.clone(),
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

    // A committed local inbox entry is not an external-channel delivery receipt.
    let mut msg = message.clone();
    msg.status = MessageStatus::Sent;
    msg.sent_at = Some(chrono::Utc::now().to_rfc3339());

    let record = MessageSendRecord {
        record_id: format!("rec_{}", uuid::Uuid::new_v4().simple()),
        message_id: message_id.clone(),
        receiver_id: user.id,
        channel: MessageChannel::InApp,
        status: MessageStatus::Sent,
        error_message: None,
        retry_count: 0,
        sent_at: Some(chrono::Utc::now().to_rfc3339()),
        read_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let message_id = match run_store(&state, move |store| {
        store.send(&msg, &record, key.as_deref(), &fingerprint)
    })
    .await
    {
        Ok(id) => id,
        Err(error) => return storage_error(error),
    };

    Json(json!({
        "code": 0,
        "message": "当前用户站内消息与回执已事务提交",
        "data": { "message_id": message_id }
    }))
    .into_response()
}

/// POST /api/enterprise/message/messages/:id/read —— 标记消息已读
pub async fn mark_read_handler(
    State(state): State<Arc<MessageCenterState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    match run_store(&state, move |store| store.mark_read(&tenant, &user.id, &id)).await {
        Ok(true) => Json(json!({"code":0,"message":"已读状态已持久化"})).into_response(),
        Ok(false) => missing_message(),
        Err(error) => storage_error(error),
    }
}

/// GET /api/enterprise/message/templates —— 获取消息模板列表
pub async fn list_templates_handler(State(state): State<Arc<MessageCenterState>>) -> Response {
    let templates = state.templates.read().await;
    let list: Vec<&MessageTemplate> = templates.values().collect();
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/enterprise/message/stats —— 获取消息统计
pub async fn message_stats_handler(
    State(state): State<Arc<MessageCenterState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    match run_store(&state, move |store| store.stats(&tenant, &user.id)).await {
        Ok(stats) => Json(json!({"code":0,"data":stats})).into_response(),
        Err(error) => storage_error(error),
    }
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

async fn run_store<T: Send + 'static>(
    state: &MessageCenterState,
    operation: impl FnOnce(InboxRepository) -> Result<T, StoreError> + Send + 'static,
) -> Result<T, StoreError> {
    let store = state.repository.clone();
    tokio::task::spawn_blocking(move || operation(store))
        .await
        .map_err(|error| StoreError::Unavailable(error.to_string()))?
}

fn storage_error(error: StoreError) -> Response {
    match error {
        StoreError::Conflict => {
            (StatusCode::CONFLICT, Json(json!({"code":409,"message":"幂等键已用于不同发送请求"})))
                .into_response()
        },
        StoreError::Unavailable(detail) => {
            tracing::error!(error = %detail, "Inbox transaction failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"code":503,"message":"消息存储暂不可用，操作未确认成功"})),
            )
                .into_response()
        },
    }
}

fn missing_message() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({"code":404,"message":"消息不存在"}))).into_response()
}

fn page_parameter(
    params: &HashMap<String, String>,
    name: &str,
    default: i64,
    min: i64,
    max: i64,
) -> Result<i64, String> {
    match params.get(name) {
        None => Ok(default),
        Some(value) => match value.parse::<i64>() {
            Ok(value) if (min..=max).contains(&value) => Ok(value),
            _ => Err(format!("分页参数 {name} 无效")),
        },
    }
}

// Sort nested JSON object keys so HashMap iteration order cannot change retry identity.
fn canonicalize(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for value in map.values_mut() {
                canonicalize(value);
            }
            map.sort_keys();
        },
        serde_json::Value::Array(values) => {
            for value in values {
                canonicalize(value);
            }
        },
        _ => {},
    }
}
