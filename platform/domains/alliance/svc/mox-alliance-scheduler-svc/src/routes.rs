// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义

use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use tracing::Instrument;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, Task};

use crate::app_state::SchedulerAppState;

/// 构建调度器 HTTP 路由
pub fn build_router(state: SchedulerAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/:task_id", get(get_task).post(handle_task_action))
        // 边界归一化（2026-09）：原 /tasks/:id/nodes、/tasks/:id/result 的执行器读代理端点已移除。
        // 调度器只负责任务排队/计划/匹配/执行器桥接（写路径经 core 的 ExecutorBridge）；
        // 执行状态/节点/融合结果等读路径由网关(:3080)直连执行器(:3200)，
        // 调度器不再做 HTTP 读代理（此前与网关直连执行器重复，且内含逐请求 .expect() 恐慌点）。
        .route("/experts/search", post(search_experts))
        // P1-③：把 x-request-id 读入 tracing span，使请求 id 贯穿 gateway→scheduler→executor 日志。
        .layer(middleware::from_fn(request_tracing_layer))
        // 一键传输加密（MOX_API_CRYPTO=sm4）：统一信封 data gzip+SM4-GCM，详见 mox-api-crypto
        .layer(middleware::from_fn(mox_api_crypto::middleware::crypto_middleware))
        .with_state(state)
}

/// P1-③ 请求可观测层：从 `x-request-id` 读 ID 并写入 tracing span 字段。
///
/// 上游（gateway 3080）已生成/透传 `x-request-id`；本层在 scheduler 进程内把它绑到
/// `info_span!` 上，后续所有 `tracing::info!` 自动携带 `rid=...`，与 gateway/executor 日志对齐。
/// 缺省时生成新 UUID，保证链路不中断。
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
    let span = tracing::info_span!("http.scheduler", rid = %rid, method = %method.as_str(), path = %path);
    async move { next.run(req).await }.instrument(span).await
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

/// 健康检查（liveness：进程存活即 200；body 真实标注下游依赖，不再恒真）
///
/// 探测 `executor_base_url/health` 写入 `dependencies.executor`：
/// 存活探针恒返回 200；readiness / 监控应依据 body 中依赖状态决定是否摘流。
async fn health_check(State(state): State<SchedulerAppState>) -> impl IntoResponse {
    let executor_url = format!(
        "{}/health",
        state.executor_base_url.trim_end_matches('/')
    );
    let executor_up = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
    {
        Ok(client) => client
            .get(&executor_url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false),
        Err(_) => false,
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "service": "mox-alliance-scheduler",
            "dependencies": { "executor": if executor_up { "up" } else { "down" } }
        })),
    )
        .into_response()
}

/// 运行指标快照（供监控抓取 / 面板展示）
async fn metrics_handler(State(state): State<SchedulerAppState>) -> impl IntoResponse {
    Json(state.metrics.snapshot())
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
        idempotency_key: None,
    };

    match state.scheduler.submit_task(submit_req).await {
        Ok(response) => {
            // 一次提交即一次 DAG 执行：此处计执行次数；节点粒度由 executor 侧指标负责
            state.metrics.record_dag_execution(0);
            (
                StatusCode::OK,
                Json(CreateTaskResponse {
                    task_id: response.task.task_id,
                    title: response.task.title,
                    status: response.task.status,
                    created_at: response.task.created_at,
                }),
            )
                .into_response()
        }
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

/// 任务操作（暂停/恢复/取消/标记完成）
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
        TaskAction::Complete => {
            state
                .scheduler
                .complete_task(task_id, tenant_id, req.reason)
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

    let start = std::time::Instant::now();
    match state.matcher.match_experts(query).await {
        Ok(result) => {
            state.metrics.record_match(start.elapsed().as_micros() as u64, true);
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
        Err(e) => {
            state.metrics.record_match(start.elapsed().as_micros() as u64, false);
            error_response(e).into_response()
        }
    }
}

// ─── 执行器桥接说明 ──────────────────────────────────────────────────────────
//
// 调度器→执行器的写路径（提交执行/暂停/恢复/取消）由 core 的 `ExecutorBridge`
// （HttpExecutorBridge）承担，在 `TaskSchedulerImpl` 内部调用；本 HTTP 层不再
// 暴露任何转发到执行器的读代理端点（读路径由网关直连执行器）。

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
