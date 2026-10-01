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
use mox_alliance_scheduler_core::MetricsSnapshot;

use crate::app_state::SchedulerAppState;

/// 构建调度器 HTTP 路由
pub fn build_router(state: SchedulerAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/leadership", get(leadership_handler))
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
        // 内部服务间鉴权：防绕过网关直连（MOX_INTERNAL_TOKEN；未配置则放行）
        .layer(middleware::from_fn(internal_auth_layer))
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
///
/// N7 指标文本化：按 `Accept` 头协商——
/// - `Accept: text/plain`（Prometheus 默认抓取头含 `text/plain;version=0.0.4`）→ 返回
///   `text/plain; version=0.0.4` Prometheus exposition 文本，命名 `mox_alliance_scheduler_*`；
/// - 否则（无 Accept / `application/json`，含既有前端控制台与 http_integration 测试）→
///   保持原 JSON 快照，零破坏。
async fn metrics_handler(
    State(state): State<SchedulerAppState>,
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
            render_scheduler_metrics_prometheus(&snap),
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

/// N7：把 scheduler 指标快照渲染为 Prometheus exposition 0.0.4 文本。
fn render_scheduler_metrics_prometheus(s: &MetricsSnapshot) -> String {
    let mut o = String::new();
    // ── 专家匹配 ──
    prom_metric(&mut o, "专家匹配请求总数", "counter", "mox_alliance_scheduler_match_requests_total", s.match_requests);
    prom_metric(&mut o, "专家匹配错误数", "counter", "mox_alliance_scheduler_match_errors_total", s.match_errors);
    prom_metric(&mut o, "匹配延迟累计（微秒）", "counter", "mox_alliance_scheduler_match_latency_us_sum", s.match_latency_us_sum);
    prom_metric(&mut o, "匹配延迟采样数", "counter", "mox_alliance_scheduler_match_latency_us_count", s.match_latency_us_count);
    prom_metric(&mut o, "平均匹配延迟（微秒）", "gauge", "mox_alliance_scheduler_match_avg_latency_us", s.match_avg_latency_us);
    // ── LLM 调用 ──
    prom_metric(&mut o, "LLM 调用总数", "counter", "mox_alliance_scheduler_llm_calls_total", s.llm_calls);
    prom_metric(&mut o, "LLM 错误数", "counter", "mox_alliance_scheduler_llm_errors_total", s.llm_errors);
    prom_metric(&mut o, "LLM 延迟累计（毫秒）", "counter", "mox_alliance_scheduler_llm_latency_ms_sum", s.llm_latency_ms_sum);
    prom_metric(&mut o, "LLM 延迟采样数", "counter", "mox_alliance_scheduler_llm_latency_ms_count", s.llm_latency_ms_count);
    prom_metric(&mut o, "平均 LLM 延迟（毫秒）", "gauge", "mox_alliance_scheduler_llm_avg_latency_ms", s.llm_avg_latency_ms);
    // ── 结果融合 ──
    prom_metric(&mut o, "融合调用总数", "counter", "mox_alliance_scheduler_fusion_calls_total", s.fusion_calls);
    prom_metric(&mut o, "融合延迟累计（毫秒）", "counter", "mox_alliance_scheduler_fusion_latency_ms_sum", s.fusion_latency_ms_sum);
    prom_metric(&mut o, "融合延迟采样数", "counter", "mox_alliance_scheduler_fusion_latency_ms_count", s.fusion_latency_ms_count);
    prom_metric(&mut o, "平均融合延迟（毫秒）", "gauge", "mox_alliance_scheduler_fusion_avg_latency_ms", s.fusion_avg_latency_ms);
    // ── DAG 执行 ──
    prom_metric(&mut o, "DAG 执行次数", "counter", "mox_alliance_scheduler_dag_executions_total", s.dag_executions);
    prom_metric(&mut o, "DAG 节点执行总次数", "counter", "mox_alliance_scheduler_dag_node_executions_total", s.dag_node_executions);
    prom_metric(&mut o, "平均每次 DAG 执行节点数", "gauge", "mox_alliance_scheduler_dag_avg_nodes_per_execution", s.dag_avg_nodes_per_execution);
    o
}

/// 多活领导权观测（HA 未开启时如实报告"单副本即 leader"，不装作在选主）
async fn leadership_handler(State(state): State<SchedulerAppState>) -> impl IntoResponse {
    match &state.leadership {
        Some(elector) => {
            let mut body = crate::ha::status_json(elector);
            body["ha_enabled"] = serde_json::Value::Bool(true);
            Json(body)
        }
        None => Json(serde_json::json!({
            "ha_enabled": false,
            "is_leader": true,
            "scope": crate::ha::SCOPE,
            "note": "MOX_ALLIANCE_HA_MODE 未开启：本副本为唯一执行者，无租约仲裁",
        })),
    }
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
                .map(|m| {
                    // U2 匹配透明化：把各维得分与本次实际权重逐维透出。
                    // 注意：priority_score 在加权时乘的是 weights.rating（Expert 无独立 rating 字段）。
                    let b = m.score_breakdown;
                    let w = m.weights;
                    ExpertSummary {
                        expert_id: m.expert.expert_id,
                        name: m.expert.name,
                        description: m.expert.description,
                        domains: m.expert.domains,
                        status: m.expert.status,
                        match_score: Some(m.score),
                        match_reason: Some(m.match_reason),
                        scores: Some(mox_alliance_api::dto::ExpertScoreView {
                            domain: mox_alliance_api::dto::ScoreDim {
                                value: b.domain_match,
                                weight: w.domain as f64,
                            },
                            capability: mox_alliance_api::dto::ScoreDim {
                                value: b.capability_match,
                                weight: w.capability as f64,
                            },
                            health: mox_alliance_api::dto::ScoreDim {
                                value: b.health_score,
                                weight: w.health as f64,
                            },
                            priority: mox_alliance_api::dto::ScoreDim {
                                value: b.priority_score,
                                weight: w.rating as f64,
                            },
                            performance: mox_alliance_api::dto::ScoreDim {
                                value: b.performance_score,
                                weight: w.performance as f64,
                            },
                            total: m.score,
                        }),
                    }
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

#[cfg(test)]
mod n7_prometheus_tests {
    use super::*;
    use mox_alliance_scheduler_core::AllianceMetrics;

    #[test]
    fn prometheus_text_contains_expected_metric_lines() {
        let m = AllianceMetrics::new();
        m.record_match(1500, true);
        m.record_llm_call(120, false);
        m.record_dag_execution(4);
        let snap = m.snapshot();
        let body = render_scheduler_metrics_prometheus(&snap);
        assert!(body.contains("# HELP mox_alliance_scheduler_match_requests_total"));
        assert!(body.contains("# TYPE mox_alliance_scheduler_match_requests_total counter"));
        assert!(body.contains("mox_alliance_scheduler_match_requests_total 1"));
        assert!(body.contains("mox_alliance_scheduler_llm_errors_total 1"));
        assert!(body.contains("mox_alliance_scheduler_dag_executions_total 1"));
        // 非注释数据行必须是 <mox_alliance_scheduler_*> <number>
        let mut data = 0;
        for line in body.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            let name = it.next().expect("metric 名");
            let val = it.next().expect("metric 值");
            assert!(name.starts_with("mox_alliance_scheduler_"), "命名前缀不规范: {name}");
            val.parse::<f64>().expect("值必须为数字");
            data += 1;
        }
        assert_eq!(data, 17, "scheduler 应有 17 条指标数据行，实际 {data}");
    }

    #[test]
    fn accept_header_drives_negotiation() {
        let mut h = HeaderMap::new();
        assert!(!wants_prometheus_text(&h), "无 Accept 必须回退 JSON");
        h.insert(axum::http::header::ACCEPT, "application/json".parse().unwrap());
        assert!(!wants_prometheus_text(&h), "Accept: json 仍回退 JSON");
        h.insert(
            axum::http::header::ACCEPT,
            "application/openmetrics-text; version=0.0.1,text/plain;version=0.0.4;q=0.5,*/*;q=0.1"
                .parse()
                .unwrap(),
        );
        assert!(wants_prometheus_text(&h), "Prometheus 默认 Accept 应命中文本");
    }
}
