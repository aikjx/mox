// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, Task};

use crate::app_state::SchedulerAppState;

/// 构建调度器 HTTP 路由
pub fn build_router(state: SchedulerAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/:task_id", get(get_task).post(handle_task_action))
        .route("/tasks/:task_id/nodes", get(proxy_task_nodes))
        .route("/tasks/:task_id/result", get(proxy_task_result))
        .route("/experts/search", post(search_experts))
        .with_state(state)
}

/// 从请求头解析租户 ID（X-Tenant-Id），缺省为 nil
fn tenant_from_headers(headers: &HeaderMap) -> Uuid {
    headers
        .get("X-Tenant-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil)
}

/// 从请求头解析用户 ID（X-User-Id），缺省为 nil
fn user_from_headers(headers: &HeaderMap) -> Uuid {
    headers
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil)
}

/// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mox-alliance-scheduler"
    }))
}

/// 创建任务
async fn create_task(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    use mox_alliance_scheduler_proto::{TaskScheduler, TaskSubmitRequest};

    // 租户/用户贯通：从请求头读取，而非硬编码 nil
    let tenant_id = tenant_from_headers(&headers);
    let user_id = user_from_headers(&headers);

    let submit_req = TaskSubmitRequest {
        tenant_id,
        user_id,
        title: req.title,
        description: req.description,
        task_type: req.task_type,
        priority: req.priority,
        mode: req.mode,
        fusion_strategy: req.fusion_strategy,
    };

    match state.scheduler.submit_task(submit_req).await {
        Ok(response) => (
            StatusCode::OK,
            Json(CreateTaskResponse {
                task_id: response.task.task_id,
                title: response.task.title,
                status: response.task.status,
                created_at: response.task.created_at,
            }),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 获取任务详情
async fn get_task(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    use mox_alliance_scheduler_proto::TaskScheduler;

    let tenant_id = tenant_from_headers(&headers);

    match state.scheduler.get_task(task_id, tenant_id).await {
        Ok(task) => (
            StatusCode::OK,
            Json(task_detail_response(&task)),
        )
            .into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 任务列表（按租户，真实返回）
async fn list_tasks(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use mox_alliance_scheduler_proto::TaskScheduler;

    let tenant_id = tenant_from_headers(&headers);

    match state.scheduler.list_tasks(tenant_id).await {
        Ok(tasks) => {
            let items: Vec<TaskDetailResponse> = tasks
                .into_iter()
                .map(|t| task_detail_response(&t))
                .collect();
            Json(TaskListResponse {
                tasks: items.clone(),
                total: items.len(),
                page: 1,
                page_size: 20,
            })
            .into_response()
        }
        Err(e) => error_response(e).into_response(),
    }
}

/// 任务操作（暂停/恢复/取消）
async fn handle_task_action(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
    Json(req): Json<TaskActionRequest>,
) -> impl IntoResponse {
    use mox_alliance_scheduler_proto::TaskScheduler;

    let tenant_id = tenant_from_headers(&headers);

    let result = match req.action {
        TaskAction::Pause => state.scheduler.pause_task(task_id, tenant_id).await,
        TaskAction::Resume => state.scheduler.resume_task(task_id, tenant_id).await,
        TaskAction::Cancel => {
            state
                .scheduler
                .cancel_task(task_id, tenant_id, req.reason)
                .await
        }
    };

    match result {
        Ok(_) => (StatusCode::OK, Json(SuccessResponse::default())).into_response(),
        Err(e) => error_response(e).into_response(),
    }
}

/// 搜索专家
async fn search_experts(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Json(req): Json<ExpertSearchRequest>,
) -> impl IntoResponse {
    let tenant_id = tenant_from_headers(&headers);
    // 空租户时回退到 system（内置领域专家租户）
    let query_tenant = if tenant_id.is_nil() {
        "system".to_string()
    } else {
        tenant_id.to_string()
    };

    let query = mox_alliance_scheduler_proto::ExpertMatchQuery {
        tenant_id: query_tenant,
        task_description: req.query,
        required_domains: req.domains,
        required_capabilities: vec![],
        min_priority: 1,
        max_results: req.limit,
    };

    match state.matcher.match_experts(query).await {
        Ok(result) => {
            let experts: Vec<ExpertSummary> = result
                .matches
                .into_iter()
                .map(|m| ExpertSummary {
                    expert_id: m.expert.expert_id,
                    name: m.expert.name,
                    description: m.expert.description,
                    domains: m.expert.domains,
                    status: m.expert.status,
                })
                .collect();

            (
                StatusCode::OK,
                Json(ExpertSearchResponse {
                    total: result.total_available,
                    experts,
                }),
            )
                .into_response()
        }
        Err(e) => error_response(e).into_response(),
    }
}

// ─── 执行器代理端点 ────────────────────────────────────────────────────────

/// 代理：获取任务节点列表（转发到 executor 服务）
async fn proxy_task_nodes(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    proxy_to_executor(&state, &headers, &format!("/tasks/{}/nodes", task_id)).await
}

/// 代理：获取任务融合结果（转发到 executor 服务）
async fn proxy_task_result(
    State(state): State<SchedulerAppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> impl IntoResponse {
    proxy_to_executor(&state, &headers, &format!("/tasks/{}/result", task_id)).await
}

/// 通用代理：将 GET 请求转发到 executor 服务，透传 X-Tenant-Id
///
/// - executor 不可达 → 503 Service Unavailable
/// - executor 返回错误 → 透传状态码与 body
async fn proxy_to_executor(
    state: &SchedulerAppState,
    headers: &HeaderMap,
    path: &str,
) -> impl IntoResponse {
    let url = format!("{}{}", state.executor_base_url.trim_end_matches('/'), path);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("reqwest client build failed");

    let mut req = client.get(&url);
    if let Some(tenant) = headers.get("X-Tenant-Id") {
        req = req.header("X-Tenant-Id", tenant);
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(body) => {
                    // 尝试解析为 JSON 透传；解析失败则原样返回文本
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(json) => (
                            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
                            Json(json),
                        )
                            .into_response(),
                        Err(_) => (
                            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
                            body,
                        )
                            .into_response(),
                    }
                }
                Err(_) => (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "error": "Failed to read executor response body"
                    })),
                )
                    .into_response(),
            }
        }
        Err(e) => {
            tracing::warn!(
                "Executor proxy failed for {}: {} (executor_base_url={})",
                path,
                e,
                state.executor_base_url
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Executor service unavailable",
                    "detail": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// Task → TaskDetailResponse
fn task_detail_response(task: &Task) -> TaskDetailResponse {
    TaskDetailResponse {
        task_id: task.task_id,
        title: task.title.clone(),
        description: task.description.clone(),
        status: task.status,
        priority: task.priority,
        progress: task.progress,
        mode: task.mode,
        created_at: task.created_at,
        started_at: task.started_at,
        completed_at: task.completed_at,
        duration_ms: task.duration_ms,
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
                AllianceErrorCode::TaskNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::ExpertNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::NodeNotFound => StatusCode::NOT_FOUND,
                AllianceErrorCode::TaskAlreadyTerminal => StatusCode::CONFLICT,
                AllianceErrorCode::InvalidTaskStatus => StatusCode::CONFLICT,
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
