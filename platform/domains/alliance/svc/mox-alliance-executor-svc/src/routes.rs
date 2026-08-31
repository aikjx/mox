// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode};
use mox_alliance_executor_proto::DagEngine;

use crate::app_state::ExecutorAppState;

/// 构建执行器 HTTP 路由
pub fn build_router(state: ExecutorAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/tasks/:task_id/status", get(get_execution_status))
        .route("/tasks/:task_id/nodes", get(list_nodes))
        .route("/tasks/:task_id/nodes/:node_id", get(get_node).post(skip_node))
        .with_state(state)
}

/// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mox-alliance-executor"
    }))
}

/// 获取执行状态
async fn get_execution_status(
    State(state): State<ExecutorAppState>,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    let tenant_id = Uuid::nil(); // Phase 1 简化

    match state.engine.get_execution_status(task_id, tenant_id).await {
        Ok(status) => (
            StatusCode::OK,
            Json(ExecutionStatusResponse {
                task_id: status.task_id,
                status: mox_alliance_common_proto::TaskStatus::Running, // 简化
                progress: status.progress,
                total_nodes: status.total_nodes,
                completed_nodes: status.completed_nodes,
                running_nodes: status.running_nodes,
                failed_nodes: status.failed_nodes,
                pending_nodes: status.pending_nodes,
            }),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 获取节点列表
async fn list_nodes(
    State(state): State<ExecutorAppState>,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    let tenant_id = Uuid::nil();

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
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let tenant_id = Uuid::nil();

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
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let tenant_id = Uuid::nil();

    match state
        .engine
        .skip_node(task_id, &node_id, tenant_id, None)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

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
