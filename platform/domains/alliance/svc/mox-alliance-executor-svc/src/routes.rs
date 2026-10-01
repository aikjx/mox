// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义
//!
//! 提供两类 API：
//! - 公共 API（/tasks/*）：供用户/前端调用
//! - 内部 API（/internal/*）：供调度器服务调用

use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use tracing::Instrument;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, CollaborationPlan, Task, TaskStatus};
use mox_alliance_executor_proto::{DagEngine, ExecutionOptions, ExecutionStatus};

use crate::app_state::{ExecutorAppState, ExecutorMetricsSnapshot};

/// 从请求头解析租户 ID（X-Tenant-Id），缺省为 nil
fn tenant_from_headers(headers: &HeaderMap) -> Uuid {
    headers
        .get("X-Tenant-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil)
}

/// 由执行计数推导任务整体状态（不再硬编码 Running）
fn status_from_execution(status: &ExecutionStatus) -> TaskStatus {
    if status.total_nodes == 0 {
        return TaskStatus::Pending;
    }
    if status.cancelled_nodes > 0 {
        return TaskStatus::Cancelled;
    }
    if status.failed_nodes > 0 {
        return TaskStatus::Failed;
    }
    let finished =
        status.completed_nodes + status.failed_nodes + status.skipped_nodes + status.cancelled_nodes;
    if finished >= status.total_nodes {
        TaskStatus::Completed
    } else if status.running_nodes > 0 {
        TaskStatus::Running
    } else {
        TaskStatus::Pending
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn cancelled_nodes_never_mean_success() {
        let mut status = ExecutionStatus { task_id: Uuid::nil(), total_nodes: 2,
            completed_nodes: 1, running_nodes: 0, failed_nodes: 0, pending_nodes: 0,
            skipped_nodes: 0, cancelled_nodes: 1, progress: 1.0,
            started_at: None, estimated_remaining_ms: None };
        assert_eq!(status_from_execution(&status), TaskStatus::Cancelled);
        status.cancelled_nodes = 0;
        status.failed_nodes = 1;
        assert_eq!(status_from_execution(&status), TaskStatus::Failed);
        status.failed_nodes = 0;
        status.completed_nodes = 2;
        assert_eq!(status_from_execution(&status), TaskStatus::Completed);
    }
}

/// 构建执行器 HTTP 路由
pub fn build_router(state: ExecutorAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        // === 公共 API ===
        .route("/tasks/:task_id/status", get(get_execution_status))
        .route("/tasks/:task_id/nodes", get(list_nodes))
        .route("/tasks/:task_id/nodes/:node_id", get(get_node).post(skip_node))
        .route("/tasks/:task_id/result", get(get_fusion_result))
        // === 内部 API（供调度器调用）===
        .route("/internal/executions", post(submit_execution))
        .route("/tasks/:task_id/cancel", post(cancel_execution))
        .route("/tasks/:task_id/pause", post(pause_execution))
        .route("/tasks/:task_id/resume", post(resume_execution))
        // P1-③：把 x-request-id 读入 tracing span，与 gateway/scheduler 日志对齐。
        .layer(middleware::from_fn(request_tracing_layer))
        // 一键传输加密（MOX_API_CRYPTO=sm4）：统一信封 data gzip+SM4-GCM，详见 mox-api-crypto
        .layer(middleware::from_fn(mox_api_crypto::middleware::crypto_middleware))
        // 内部服务间鉴权：防绕过网关直连（MOX_INTERNAL_TOKEN；未配置则放行）
        .layer(middleware::from_fn(internal_auth_layer))
        .with_state(state)
}

/// P1-③ 请求可观测层：从 `x-request-id` 读 ID 并写入 tracing span 字段。
///
/// 上游 scheduler 在 proxy_to_executor 中已透传 `x-request-id`；本层在 executor 进程内
/// 把它绑到 `info_span!`，使一个请求 id 贯穿 gateway→scheduler→executor 四进程日志。
/// 缺省生成新 UUID。
async fn request_tracing_layer(req: Request, next: Next) -> Response {
    let rid = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty() && s.len() <= 64)
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let span = tracing::info_span!("http.executor", rid = %rid, method = %method.as_str(), path = %path);
    async move { next.run(req).await }.instrument(span).await
}


/// 内部服务间鉴权：校验网关注入的共享令牌（MOX_INTERNAL_TOKEN，主值）。
///
/// 下游 svc 不直接对前端暴露，唯一入口是网关(:3080)。本层防止绕过网关直连：
/// - 配置 MOX_INTERNAL_TOKEN 后，所有非公开路径必须携带 `Authorization: Bearer <token>`；
/// - 未配置主/备任一（开发/本地默认）则放行，保持向后兼容；
/// - MOX_DEV_MODE=1 强制跳过；
/// - 探活/指标端点（/health、/metrics、/leadership、/api/registry/health）始终放行。
///
/// 双值滚动（零停机换令牌，P0-C 2026-09-29）：
/// - 新增可选 `MOX_INTERNAL_TOKEN_ALT`（备用值）。Bearer 等于主值 **或** 备用值即通过。
/// - 生产滚动流程：① 旧值写入 `MOX_INTERNAL_TOKEN_ALT` → 全集群滚动重启本 svc；
///   ② 网关出站把 `MOX_INTERNAL_TOKEN` 切到新值，全集群滚动重启网关；
///   ③ 观察一个周期后移除 `MOX_INTERNAL_TOKEN_ALT`，完成双窗口滚动。
///   两个窗口内网关旧请求带旧值（命中 ALT）、新请求带新值（命中主值），均不 401。
async fn internal_auth_layer(req: Request, next: Next) -> Result<Response, StatusCode> {
    let dev_mode = std::env::var("MOX_DEV_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(cfg!(debug_assertions));
    let expected = std::env::var("MOX_INTERNAL_TOKEN").unwrap_or_default();
    let expected_alt = std::env::var("MOX_INTERNAL_TOKEN_ALT").unwrap_or_default();
    if dev_mode || (expected.is_empty() && expected_alt.is_empty()) {
        return Ok(next.run(req).await);
    }
    let path = req.uri().path();
    const PUBLIC: &[&str] = &["/health", "/metrics", "/leadership", "/api/registry/health"];
    if PUBLIC.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(req).await);
    }
    let ok = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t == expected || (!expected_alt.is_empty() && t == expected_alt))
        .unwrap_or(false);
    if ok {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// 健康检查
async fn health_check(State(state): State<ExecutorAppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mox-alliance-executor",
        "execution_ready": state.execution_ready,
        "execution_mode": state.execution_mode,
        "message": if state.execution_ready { "模型已配置，调用结果以实际执行为准" } else { "尚未配置真实模型，不能执行专家分析。请配置模型后重启执行器。" }
    }))
}

/// 运行指标快照（纯原子计数 JSON，与调度器 /metrics 同模式）
///
/// N7 指标文本化：按 `Accept` 头协商——`Accept: text/plain` 返回 Prometheus 文本
/// （`mox_alliance_executor_*`），否则保持原 JSON 快照（向后兼容前端/控制台）。
async fn metrics_handler(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
) -> Response {
    let snap = state.metrics.snapshot();
    if wants_prometheus_text(&headers) {
        (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            render_executor_metrics_prometheus(&snap),
        )
            .into_response()
    } else {
        Json(snap).into_response()
    }
}

/// N7：判定请求是否要求 Prometheus 文本格式（Accept 头含 `text/plain`）。
fn wants_prometheus_text(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/plain"))
        .unwrap_or(false)
}

/// N7：写一行 Prometheus 指标（HELP + TYPE + sample）。
fn prom_metric(o: &mut String, help: &str, ty: &str, name: &str, val: u64) {
    o.push_str(&format!("# HELP {name} {help}\n# TYPE {name} {ty}\n{name} {val}\n"));
}

/// N7：把 executor 指标快照渲染为 Prometheus exposition 0.0.4 文本。
fn render_executor_metrics_prometheus(s: &ExecutorMetricsSnapshot) -> String {
    let mut o = String::new();
    prom_metric(&mut o, "累计提交执行任务数", "counter", "mox_alliance_executor_tasks_submitted_total", s.tasks_submitted);
    prom_metric(&mut o, "累计完成任务数", "counter", "mox_alliance_executor_tasks_completed_total", s.tasks_completed);
    prom_metric(&mut o, "累计完成节点数", "counter", "mox_alliance_executor_nodes_completed_total", s.nodes_completed);
    prom_metric(&mut o, "累计错误数", "counter", "mox_alliance_executor_errors_total", s.errors);
    prom_metric(&mut o, "累计取消任务数", "counter", "mox_alliance_executor_tasks_cancelled_total", s.tasks_cancelled);
    o
}

// ─── 公共 API ──────────────────────────────────────────────────────────────

/// 获取执行状态
async fn get_execution_status(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);

    match state.engine.get_execution_status(task_id, tenant_id).await {
        Ok(status) => (
            StatusCode::OK,
            Json(ExecutionStatusResponse {
                task_id: status.task_id,
                status: status_from_execution(&status),
                progress: status.progress,
                total_nodes: status.total_nodes,
                completed_nodes: status.completed_nodes,
                running_nodes: status.running_nodes,
                failed_nodes: status.failed_nodes,
                pending_nodes: status.pending_nodes,
                skipped_nodes: status.skipped_nodes,
                cancelled_nodes: status.cancelled_nodes,
            }),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 获取节点列表
async fn list_nodes(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);

    match state.engine.get_nodes(task_id, tenant_id).await {
        Ok(nodes) => {
            let node_responses: Vec<NodeDetailResponse> = nodes
                .into_iter()
                .map(|n| NodeDetailResponse {
                    node_id: n.node_id,
                    name: n.name,
                    expert_id: n.expert_id,
                    status: n.status,
                    dependencies: n.dependencies,
                    started_at: n.started_at,
                    completed_at: n.completed_at,
                    duration_ms: n.duration_ms,
                    error_message: n.error_message,
                })
                .collect();

            let total = node_responses.len();
            (
                StatusCode::OK,
                Json(NodeListResponse {
                    nodes: node_responses,
                    total,
                }),
            )
                .into_response()
        }
        Err(e) => error_response(e).into_response(),
    }
}

/// 获取单个节点
async fn get_node(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);

    match state.engine.get_node(task_id, &node_id, tenant_id).await {
        Ok(node) => (
            StatusCode::OK,
            Json(NodeDetailResponse {
                node_id: node.node_id,
                name: node.name,
                expert_id: node.expert_id,
                status: node.status,
                dependencies: node.dependencies,
                started_at: node.started_at,
                completed_at: node.completed_at,
                duration_ms: node.duration_ms,
                error_message: node.error_message,
            }),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 跳过节点（人工干预）
async fn skip_node(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);

    match state
        .engine
        .skip_node(task_id, &node_id, tenant_id, None)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 获取任务的融合结果（DAG 尾部按 fusion_strategy 融合后的结论）
async fn get_fusion_result(
    State(state): State<ExecutorAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);

    match state.engine.get_fusion_output(task_id, tenant_id).await {
        Ok(Some(output)) => (StatusCode::OK, Json(output)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                mox_alliance_common_proto::AllianceErrorCode::NotFound as u32,
                "Task has no fusion result yet".to_string(),
            )),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

// ─── 内部 API（供调度器调用）───────────────────────────────────────────────

/// 提交执行请求（内部 API，供调度器调用）
async fn submit_execution(
    State(state): State<ExecutorAppState>,
    Json(req): Json<SubmitExecutionRequest>,
) -> impl IntoResponse {
    let node_count = req.plan.nodes.len();
    if !state.execution_ready && state.execution_mode != "mock" {
        state.metrics.record_error();
        // 统一错误信封（与 error_response / ErrorResponse 一致）：模型未配置视为执行器不可用
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                AllianceErrorCode::ExecutorUnavailable as u32,
                "尚未配置真实模型，无法执行专家分析".to_string(),
            )),
        )
            .into_response();
    }
    let task_id = req.task.task_id;

    match state
        .engine
        .start_execution(&req.task, req.plan, req.options)
        .await
    {
        Ok(_) => {
            state.metrics.record_submit();
            tracing::info!(
                "Execution submitted: task_id={}, nodes={}",
                task_id,
                node_count
            );
            (StatusCode::OK, Json(SuccessResponse::default())).into_response()
        }
        Err(e) => {
            state.metrics.record_error();
            error_response(e).into_response()
        }
    }
}

/// 取消执行（供调度器调用）
async fn cancel_execution(
    State(state): State<ExecutorAppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<CancelExecutionRequest>,
) -> impl IntoResponse {
    let tenant_id = Uuid::parse_str(&req.tenant_id).unwrap_or(Uuid::nil());

    match state
        .engine
        .cancel_execution(task_id, tenant_id, req.reason)
        .await
    {
        Ok(_) => {
            state.metrics.record_cancel();
            (StatusCode::OK, Json(SuccessResponse::default())).into_response()
        }
        Err(e) => {
            state.metrics.record_error();
            error_response(e).into_response()
        }
    }
}

/// 暂停执行（供调度器调用）
async fn pause_execution(
    State(state): State<ExecutorAppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<PauseResumeRequest>,
) -> impl IntoResponse {
    let tenant_id = Uuid::parse_str(&req.tenant_id).unwrap_or(Uuid::nil());

    match state.engine.pause_execution(task_id, tenant_id).await {
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 恢复执行（供调度器调用）
async fn resume_execution(
    State(state): State<ExecutorAppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<PauseResumeRequest>,
) -> impl IntoResponse {
    let tenant_id = Uuid::parse_str(&req.tenant_id).unwrap_or(Uuid::nil());

    match state.engine.resume_execution(task_id, tenant_id).await {
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 提交执行请求体（内部 API）
#[derive(Debug, serde::Deserialize)]
struct SubmitExecutionRequest {
    task: Task,
    plan: CollaborationPlan,
    options: ExecutionOptions,
}

/// 取消执行请求体
#[derive(Debug, serde::Deserialize)]
struct CancelExecutionRequest {
    tenant_id: String,
    reason: Option<String>,
}

/// 暂停/恢复执行请求体
#[derive(Debug, serde::Deserialize)]
struct PauseResumeRequest {
    tenant_id: String,
}

// ─── 通用错误响应 ──────────────────────────────────────────────────────────

/// 统一错误响应
fn error_response(err: AllianceError) -> (StatusCode, Json<ErrorResponse>) {
    let (code, status) = match &err {
        AllianceError::Business { code, .. } => (
            *code as u32,
            match code {
                AllianceErrorCode::NotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
                AllianceErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
                AllianceErrorCode::TenantMismatch => StatusCode::FORBIDDEN,
                AllianceErrorCode::TaskNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::ExpertNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::NodeNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::TaskAlreadyTerminal => StatusCode::CONFLICT,
                AllianceErrorCode::InvalidTaskStatus => StatusCode::CONFLICT,
                AllianceErrorCode::InvalidPlan => StatusCode::BAD_REQUEST,
                AllianceErrorCode::SchedulerFull => StatusCode::SERVICE_UNAVAILABLE,
                AllianceErrorCode::ExecutorUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                AllianceErrorCode::ExpertUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        ),
        AllianceError::Internal(_) => {
            (AllianceErrorCode::Unknown as u32, StatusCode::INTERNAL_SERVER_ERROR)
        }
        AllianceError::Other(_) => {
            (AllianceErrorCode::Unknown as u32, StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    (status, Json(ErrorResponse::new(code, err.to_string())))
}
