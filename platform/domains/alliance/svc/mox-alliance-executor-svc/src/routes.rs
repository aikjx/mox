// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义
//!
//! 提供两类 API：
//! - 公共 API（/tasks/*）：供用户/前端调用
//! - 内部 API（/internal/*）：供调度器服务调用

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, CollaborationPlan, Task, TaskStatus};
use mox_alliance_executor_proto::{DagEngine, ExecutionOptions, ExecutionStatus};

use crate::app_state::ExecutorAppState;

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

/// 构建执行器 HTTP 路由
pub fn build_router(state: ExecutorAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
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
        .with_state(state)
}

/// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mox-alliance-executor"
    }))
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

    match state.engine.get_fusion_output(task_id, tenant_id) {
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
    let task_id = req.task.task_id;

    match state
        .engine
        .start_execution(&req.task, req.plan, req.options)
        .await
    {
        Ok(_) => {
            tracing::info!(
                "Execution submitted: task_id={}, nodes={}",
                task_id,
                node_count
            );
            (StatusCode::OK, Json(SuccessResponse::default())).into_response()
        }
        Err(e) => error_response(e).into_response(),
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
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
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
                AllianceErrorCode::TaskAlreadyTerminal => StatusCode::CONFLICT,
                AllianceErrorCode::InvalidTaskStatus => StatusCode::CONFLICT,
                AllianceErrorCode::InvalidPlan => StatusCode::BAD_REQUEST,
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
