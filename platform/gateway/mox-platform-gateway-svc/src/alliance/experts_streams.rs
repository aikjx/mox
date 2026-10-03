// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # T4 事件总线对外出口：SSE 事件帧 + Webhook 订阅 HTTP 面
//!
//! 本模块把 T4 进程内总线（[`super::experts_events::EventBus`]）暴露成**对外可订阅**的两条通道：
//!
//! 1. **SSE 事件帧**：[`GET /api/alliance/events/stream`](alliance_events_stream)。
//!    按 [`TenantId`]（认证身份）订阅本租户事件，每事件转一帧：
//!    `event: <Kind>` + `data: <信封 JSON>`（UTF-8）。15s KeepAlive 心跳。
//! 2. **Webhook 订阅**：[`POST/GET/DELETE /api/alliance/events/webhooks`]，登记 URL +
//!    事件类型过滤，总线派发器（[`super::experts_events::spawn_webhook_dispatcher`]）在真实
//!    事件发生时真实 HTTP POST 送达。
//!
//! ## 与既有「任务日志流」SSE 的关系（独立新增，不统一）
//!
//! 既有 `GET /api/alliance/tasks/:id/logs/stream`（sdk `alliance.rs::task_logs_stream`）
//! 推送的是**单任务执行日志帧**（`AllianceGatewayState.log_tx`，data-only）。本模块推送的是
//! **域业务事件帧**（`ExpertsSharedState.events`，带 `event:` 命名 + 结构化事件信封）。
//!
//! 二者**刻意不合并为一条统一流**，理由：
//! - 状态源不同（任务执行日志 vs 业务事件总线），合并要在一个 handler 里同时订阅两条
//!   channel 并做帧格式归一，改动面大且易回归；
//! - 既有日志流的消费方（前端 `getExecutionLogsSSE`，data-only 无 `event:` 名）已按「无命名
//!   事件的数据帧」解析，一旦强行加 `event:` 字段会改变其帧语义；独立通道对既有消费方零影响。
//! - 语义分层：日志流回答「这个任务跑了什么」，事件流回答「这个租户的业务发生了什么」。
//!
//! 租户隔离：SSE 帧流在服务端按连接租户过滤，**绝不**跨租户下发；webhook 同样按租户隔离。

use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use mox_api_protocol::{api_error, api_ok, ApiResponse};

use super::{
    experts_common::{parse_pagination, ExpertsSharedState, OptionalAuthUser, TenantId},
    experts_rbac::{enforce_admin_or_respond, RbacAction},
};

// =====================================================================
// SSE 事件帧：GET /api/alliance/events/stream
// =====================================================================

/// GET /api/alliance/events/stream —— 按租户的 T4 业务事件帧 SSE 流。
///
/// - 经认证中间件注入的 `UserInfo` 解出 [`TenantId`]（无身份 401，与既有读面一致）；
/// - 订阅 `state.events` broadcast，**服务端过滤**只下发本租户事件；
/// - 每事件一帧：`event: <Kind>`（PlanCreated / PlanStatusChanged / ExpertRegistered /
///   ExpertDisabled）+ `data: <信封 JSON>`；
/// - 15s KeepAlive 心跳（沿用既有 SSE 日志流配置）；连接断开 receiver 自动 drop，流自然结束。
pub async fn alliance_events_stream(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> Response {
    let rx = state.events.subscribe();

    let tenant_owned = tenant;
    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(
        stream::unfold((rx, tenant_owned), |(mut rx, t)| async move {
            loop {
                match rx.recv().await {
                    Ok(ev) => {
                        // 租户隔离：非本租户事件跳过，不下发
                        if ev.tenant != t {
                            continue;
                        }
                        let name = ev.kind.type_name().to_string();
                        let Ok(data) = serde_json::to_string(&ev) else {
                            tracing::error!(event_id = %ev.id, "alliance event serialization failed");
                            continue;
                        };
                        return Some((
                            Ok(Event::default().id(ev.id).event(name).data(data)),
                            (rx, t),
                        ));
                    },
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        return Some((
                            // Global lag must not disclose other tenants' event counts.
                            Ok(Event::default()
                                .event("StreamGap")
                                .data(r#"{"reason":"lagged","action":"refresh"}"#)),
                            (rx, t),
                        ));
                    },
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }),
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

// =====================================================================
// Webhook 管理：管理员授权 + SQLite 提交 + 内存投递投影
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateWebhookBody {
    /// 目标 URL（须 http/https）
    pub url: String,
    /// 事件类型过滤：空 = 全部；否则仅投递列出的类型名（与 AllianceEventKind::type_name 对齐）
    #[serde(default)]
    pub event_types: Vec<String>,
}

/// POST /api/alliance/events/webhooks —— 登记一个 webhook 订阅（按认证租户隔离）。
pub async fn create_webhook(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateWebhookBody>,
) -> ApiResponse<Value> {
    if let Err(response) =
        enforce_admin_or_respond(&state, &user, &tenant, RbacAction::ManageWebhooks)
    {
        return response;
    }
    let url = match super::webhook_policy::allowed_target(body.url.trim()) {
        Ok(url) => url.to_string(),
        Err(message) => return api_error(400, message),
    };
    let allowed_types = ["PlanCreated", "PlanStatusChanged", "ExpertRegistered", "ExpertDisabled"];
    if body.event_types.len() > allowed_types.len()
        || body.event_types.iter().any(|t| !allowed_types.contains(&t.as_str()))
    {
        return api_error(400, "event_types 仅支持四种联盟事件，空数组表示全部");
    }
    match state.events.register_webhook(tenant, url, body.event_types) {
        Ok(wh) => api_ok(json!({ "webhook": wh })),
        Err(error) => {
            tracing::error!(%error, "webhook create commit failed");
            api_error(503, "订阅未保存，请稍后重试")
        },
    }
}

/// GET /api/alliance/events/webhooks —— 列出本租户全部订阅。
pub async fn list_webhooks(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResponse<Value> {
    if let Err(response) =
        enforce_admin_or_respond(&state, &user, &tenant, RbacAction::ManageWebhooks)
    {
        return response;
    }
    let mut list = state.events.list_webhooks(&tenant);
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    let total = list.len();
    let (offset, limit) = parse_pagination(&query);
    let items: Vec<_> = list.into_iter().skip(offset).take(limit).collect();
    api_ok(json!({ "webhooks": items, "total": total }))
}

/// DELETE /api/alliance/events/webhooks/:id —— 删除本租户的一个订阅。
pub async fn delete_webhook(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    if let Err(response) =
        enforce_admin_or_respond(&state, &user, &tenant, RbacAction::ManageWebhooks)
    {
        return response;
    }
    match state.events.delete_webhook(&tenant, &id) {
        Ok(true) => api_ok(json!({ "deleted": id })),
        Ok(false) => api_error(404, format!("webhook {id} 不存在或不属于本租户")),
        Err(error) => {
            tracing::error!(%error, "webhook delete commit failed");
            api_error(503, "订阅未删除，请稍后重试")
        },
    }
}

// =====================================================================
// 路由装配
// =====================================================================

/// 构建「T4 事件总线对外出口」路由（SSE 事件帧 + webhook CRUD）。
///
/// 与既有 `tasks/:id/logs/stream` 独立（见模块头注释），由 modules.rs 合并进受保护路由组，
/// 复用同一份 [`ExpertsSharedState`]（故订阅的就是真实业务 emit 的那条总线）。
pub fn build_experts_streams_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        .route("/api/alliance/events/stream", get(alliance_events_stream))
        .route("/api/alliance/events/webhooks", post(create_webhook).get(list_webhooks))
        .route("/api/alliance/events/webhooks/:id", delete(delete_webhook))
        .with_state(state)
}
