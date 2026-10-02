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
    pub iam: Option<Arc<mox_platform_iam_core::IamRepository>>,
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
            iam: None,
            templates: Arc::new(RwLock::new(templates)),
            channel_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_iam(mut self, iam: Arc<mox_platform_iam_core::IamRepository>) -> Self {
        self.iam = Some(iam);
        self
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
    match run_user_store(&state, tenant.clone(), user.id.clone(), move |store| {
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
    match run_user_store(&state, tenant.clone(), user.id.clone(), move |store| {
        store.get(&tenant, &user.id, &id)
    })
    .await
    {
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
    Json(mut req): Json<SendMessageRequest>,
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
    if req.receiver_ids.as_ref().is_none_or(|ids| ids.is_empty() || ids.len() > 100) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":400,"message":"收件人数量必须为 1–100"})),
        )
            .into_response();
    }
    if req
        .receiver_ids
        .as_ref()
        .is_some_and(|ids| ids.iter().any(|id| id.trim().is_empty() || id.chars().count() > 128))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":400,"message":"收件人 ID 必须非空且不超过 128 个字符"})),
        )
            .into_response();
    }
    let receivers: std::collections::BTreeSet<_> =
        req.receiver_ids.clone().unwrap_or_default().into_iter().collect();
    // Standalone instances without IAM retain only the trusted self-inbox capability.
    if state.iam.is_none() && (receivers.len() != 1 || !receivers.contains(&user.id)) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"code": 501,
            "message": "跨用户收件人目录校验未接入；当前仅支持当前用户站内收件箱"})),
        )
            .into_response();
    }
    // Preserve the original self-send request fingerprint for pre-v2 retries, including duplicates.
    if receivers.len() != 1 || !receivers.contains(&user.id) {
        req.receiver_ids = Some(receivers.iter().cloned().collect());
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
    if title.trim().is_empty()
        || title.chars().count() > 200
        || content.trim().is_empty()
        || content.chars().count() > 10000
    {
        return (StatusCode::BAD_REQUEST, Json(json!({"code":400,"message":"标题须非空且不超过 200 个字符；正文须非空且不超过 10000 个字符"}))).into_response();
    }

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
        receiver_id: user.id.clone(),
        channel: MessageChannel::InApp,
        status: MessageStatus::Sent,
        error_message: None,
        retry_count: 0,
        sent_at: Some(chrono::Utc::now().to_rfc3339()),
        read_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let records: Vec<_> = receivers
        .into_iter()
        .map(|receiver_id| {
            let mut record = record.clone();
            record.record_id = format!("rec_{}", uuid::Uuid::new_v4().simple());
            record.receiver_id = receiver_id;
            record
        })
        .collect();
    let iam = state.iam.clone();
    let message_id = match run_store(&state, move |store| {
        let deliver = || store.send_many(&msg, &records, key.as_deref(), &fingerprint);
        match iam {
            Some(iam) => super::authorization::authorize_delivery(
                &iam,
                &msg.tenant_id,
                &msg.sender_id,
                &msg.receiver_ids,
                deliver,
            ),
            None => deliver(),
        }
    })
    .await
    {
        Ok(id) => id,
        Err(error) => return storage_error(error),
    };

    Json(json!({
        "code": 0,
        "message": "站内消息与全部收件人回执已事务提交",
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
    match run_user_store(&state, tenant.clone(), user.id.clone(), move |store| {
        store.mark_read(&tenant, &user.id, &id)
    })
    .await
    {
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
    match run_user_store(&state, tenant.clone(), user.id.clone(), move |store| {
        store.stats(&tenant, &user.id)
    })
    .await
    {
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

pub(crate) async fn run_store<T: Send + 'static>(
    state: &MessageCenterState,
    operation: impl FnOnce(InboxRepository) -> Result<T, StoreError> + Send + 'static,
) -> Result<T, StoreError> {
    let store = state.repository.clone();
    tokio::task::spawn_blocking(move || operation(store))
        .await
        .map_err(|error| StoreError::Unavailable(error.to_string()))?
}

pub(crate) async fn run_user_store<T: Send + 'static>(
    state: &MessageCenterState,
    tenant: String,
    user: String,
    operation: impl FnOnce(InboxRepository) -> Result<T, StoreError> + Send + 'static,
) -> Result<T, StoreError> {
    let iam = state.iam.clone();
    run_store(state, move |store| match iam {
        Some(iam) => super::authorization::authorize_delivery(
            &iam,
            &tenant,
            &user,
            std::slice::from_ref(&user),
            || operation(store),
        ),
        None => operation(store),
    })
    .await
}

pub(crate) fn storage_error(error: StoreError) -> Response {
    match error {
        StoreError::Forbidden => (
            StatusCode::FORBIDDEN,
            Json(json!({"code":403,"message":"发送身份、权限或收件人不可用"})),
        )
            .into_response(),
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
