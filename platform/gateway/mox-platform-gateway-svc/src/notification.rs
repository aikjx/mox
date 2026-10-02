//! Compatibility notification routes backed by the scoped transactional inbox.
//! Legacy collections lack ownership and are preserved without assigning them to a caller.
use crate::{
    alliance::experts_common::TenantId,
    auth::ApiAuth,
    message_center::{
        api::{run_user_store, storage_error, MessageCenterState},
        MessageStatus, MessageType,
    },
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct NotificationState {
    inbox: Arc<MessageCenterState>,
}
impl NotificationState {
    pub fn new() -> Self {
        Self::with_inbox(Arc::new(MessageCenterState::new()))
    }
    pub fn with_inbox(inbox: Arc<MessageCenterState>) -> Self {
        Self { inbox }
    }
}
impl Default for NotificationState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct ListQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    limit: Option<i64>,
    unread_only: Option<bool>,
}
async fn list_notifications(
    State(state): State<Arc<NotificationState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Query(query): Query<ListQuery>,
) -> Response {
    let page = query.page.unwrap_or(1);
    let size = query.page_size.or(query.limit).unwrap_or(20);
    let Some(offset) = page
        .checked_sub(1)
        .and_then(|value| value.checked_mul(size))
        .filter(|value| *value >= 0)
    else {
        return (StatusCode::BAD_REQUEST, Json(json!({"code":400,"message":"通知分页参数无效"})))
            .into_response();
    };
    if page < 1 || !(1..=100).contains(&size) {
        return (StatusCode::BAD_REQUEST, Json(json!({"code":400,"message":"通知分页参数无效"})))
            .into_response();
    }
    let status = query.unread_only.unwrap_or(false).then_some("sent");
    match run_user_store(&state.inbox, tenant.clone(), user.id.clone(), move |store| {
        store.list(&tenant, &user.id, size, offset, None, status)
    })
    .await
    {
        Ok((messages, total)) => {
            let items:Vec<_> = messages.into_iter().map(|message| {
                let kind = match message.message_type { MessageType::Task => "task", MessageType::Alert => "alert", _ => "message" };
                json!({"id":message.message_id,"title":message.title,"content":message.content,"description":message.content,
                    "type":kind,"read":matches!(message.status, MessageStatus::Read),"created_at":message.created_at})
            }).collect();
            Json(
                json!({"code":0,"data":{"items":items,"total":total,"page":page,"page_size":size}}),
            )
            .into_response()
        },
        Err(error) => storage_error(error),
    }
}
async fn mark_notification_read(
    State(state): State<Arc<NotificationState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    match run_user_store(&state.inbox, tenant.clone(), user.id.clone(), move |store| {
        store.mark_read(&tenant, &user.id, &id)
    })
    .await
    {
        Ok(true) => Json(json!({"code":0,"data":{"read":true}})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"code":404,"message":"通知不存在"})))
            .into_response(),
        Err(error) => storage_error(error),
    }
}
async fn mark_all_notifications_read(
    State(state): State<Arc<NotificationState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    match run_user_store(&state.inbox, tenant.clone(), user.id.clone(), move |store| {
        store.mark_all_read(&tenant, &user.id)
    })
    .await
    {
        Ok(count) => Json(json!({"code":0,"data":{"updated_count":count}})).into_response(),
        Err(error) => storage_error(error),
    }
}
async fn unread_count(
    State(state): State<Arc<NotificationState>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    match run_user_store(&state.inbox, tenant.clone(), user.id.clone(), move |store| {
        store.unread_counts(&tenant, &user.id)
    })
    .await
    {
        Ok(counts) => {
            Json(json!({"code":0,"data":{"total":counts.values().sum::<i64>(),"by_type":counts}}))
                .into_response()
        },
        Err(error) => storage_error(error),
    }
}
pub fn build_notification_router(state: Arc<NotificationState>) -> Router {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread-count", get(unread_count))
        .route("/api/notifications/:id/read", put(mark_notification_read))
        .route("/api/notifications/read-all", put(mark_all_notifications_read))
        .with_state(state)
}
