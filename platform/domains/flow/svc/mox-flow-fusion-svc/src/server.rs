//! 企业级 REST 服务层（层3 · API）
//!
//! 把 [`crate::platform::PrimiPlatform`] 的统一图、六维注册表、PT-DOC 自生成能力通过
//! HTTP 对外暴露，供治理台 / 前端 / 第三方系统调用。遵循 primiflow 既有 `server.rs` 的
//! `build_router` 模式（可被示例真正监听，也可在测试中用 `oneshot` 直接驱动）。
//!
//! 端点（除 `health`/`version` 外均受 Bearer 鉴权保护）：
//! - `GET  /api/health`                           探活：注册表统计 + 全局闸门状态
//! - `GET  /api/version`                          服务版本
//! - `POST /api/v1/synthesize`                   提交需求，跑一体化合成并导出 PT-DOC
//! - `GET  /api/v1/registry/by-code?code=`       code→需求 溯源反查
//! - `GET  /api/v1/registry/by-requirement?req=` 按需求 id 查询绑定
//! - `GET  /api/v1/registry/stats`               注册表统计
//! - `POST /api/v1/persist`                      落盘注册表（JSON）
//! - `GET  /api/v1/gate`                          跑全局治理闸门
//! - `GET  /api/v1/docs`                          列出已导出 PT-DOC
//! - `GET  /api/v1/docs/:id`                     读取某 PT-DOC 内容
//!
//! 设计为「可测试」：通过 [`build_router`] 返回 `axum::Router`，既可由 [`serve`] 真正监听端口，
//! 也可在无网络下用 `tower::ServiceExt::oneshot` 直接驱动（见 `tests/server_test.rs`）。

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderValue, Method, Request, Response, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::observability;
use crate::platform::{PlatformReport, PrimiPlatform};
use crate::sixdim::SixDimBinding;
use crate::unified::PlatformGate;
use mox_flow_primiflow_svc::gen::c1::OrchestrationStatus;

/// 共享应用状态（跨请求持有平台闭环节点状态与六维注册表）
pub struct AppState {
    pub platform: Mutex<PrimiPlatform>,
    pub config: Config,
}

/// 由配置构造应用状态。
pub fn new_state(config: Config) -> Arc<AppState> {
    let platform = match &config.persistence_path {
        Some(p) => PrimiPlatform::with_persistence(p.clone()),
        None => PrimiPlatform::new(),
    };
    Arc::new(AppState {
        platform: Mutex::new(platform),
        config,
    })
}

// ——— 请求/响应结构 ———

#[derive(Deserialize)]
pub struct SynthesizeReq {
    pub requirement: String,
    #[serde(default = "default_slider")]
    pub slider_s: f64,
}

fn default_slider() -> f64 {
    0.5
}

#[derive(Serialize)]
pub struct SynthesizeResp {
    pub req_id: String,
    pub status: String,
    pub gate_passed: bool,
    pub registered: usize,
    pub ptdocs: usize,
    pub kappa: f64,
    pub tau: f64,
    pub c: f64,
    pub q: f64,
    pub docs_dir: String,
}

#[derive(Deserialize)]
pub struct CodeQuery {
    pub code: String,
}

#[derive(Deserialize)]
pub struct ReqQuery {
    pub req: String,
}

#[derive(Serialize)]
pub struct DocMeta {
    pub name: String,
    pub size: u64,
}

// 治理闸门可序列化摘要（避免依赖内部类型的 Serialize 派生）
#[derive(Serialize)]
pub struct GateSummary {
    pub passed: bool,
    pub error_count: usize,
    pub conservation: ConservationSummary,
    pub binding: BindingSummary,
    pub governance: GovernanceSummary,
}

#[derive(Serialize)]
pub struct ConservationSummary {
    pub passed: bool,
    pub total_c: f64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct BindingSummary {
    pub passed: bool,
    pub six_dim_nodes: usize,
    pub orphans: Vec<String>,
}

#[derive(Serialize)]
pub struct GovernanceSummary {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

fn gate_to_summary(g: &PlatformGate) -> GateSummary {
    GateSummary {
        passed: g.passed,
        error_count: g.error_count,
        conservation: ConservationSummary {
            passed: g.conservation.passed,
            total_c: g.conservation.total_c,
            errors: g.conservation.errors.clone(),
            warnings: g.conservation.warnings.clone(),
        },
        binding: BindingSummary {
            passed: g.binding.passed,
            six_dim_nodes: g.binding.six_dim_nodes,
            orphans: g.binding.orphans.clone(),
        },
        governance: GovernanceSummary {
            passed: g.governance.passed,
            errors: g.governance.errors.clone(),
            warnings: g.governance.warnings.clone(),
        },
    }
}

// ——— 处理器 ———

/// `GET /api/health`
async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let p = state.platform.lock().await;
    let stats = p.registry.stats();
    let gate = p.graph.full_gate();
    drop(p);
    Json(serde_json::json!({
        "status": "ok",
        "service": "primiflow-fusion",
        "version": env!("CARGO_PKG_VERSION"),
        "registry": {
            "total": stats.total,
            "completed": stats.completed,
            "rejected": stats.rejected,
            "sum_c": stats.sum_c,
            "sum_q": stats.sum_q,
        },
        "gate_passed": gate.passed,
        "auth_enabled": state.config.auth_token.is_some(),
    }))
}

/// `GET /api/version`
async fn version() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "primiflow-fusion",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `POST /api/v1/synthesize`
async fn synthesize(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SynthesizeReq>,
) -> impl IntoResponse {
    let mut p = state.platform.lock().await;
    let rep: PlatformReport =
        p.synthesize_and_emit_docs(&req.requirement, req.slider_s, &state.config.docs_dir);
    let req_id = p
        .registry
        .bindings
        .last()
        .map(|b| b.req_id.clone())
        .unwrap_or_default();
    let gate_passed = rep.gate.passed;
    let registered = rep.registered;
    let ptdocs = rep.ptdocs;
    let status_str = match rep.orchestration.status {
        OrchestrationStatus::Completed => "Completed",
        OrchestrationStatus::RejectedDomain => "RejectedDomain",
        OrchestrationStatus::SmokeFailed => "SmokeFailed",
        // 闸门消费约束下：闸门未通过时保持待闸门态（未冻结、未回灌）
        OrchestrationStatus::CompletedPendingGate => "GateRejected",
    };
    let (k, t, q) = rep
        .orchestration
        .state
        .as_ref()
        .map(|s| (s.kappa, s.tau, s.q))
        .unwrap_or((0.0, 0.0, 0.0));
    let c = (k * k + t * t).sqrt();
    drop(p);

    Json(SynthesizeResp {
        req_id,
        status: status_str.to_string(),
        gate_passed,
        registered,
        ptdocs,
        kappa: k,
        tau: t,
        c,
        q,
        docs_dir: state.config.docs_dir.display().to_string(),
    })
}

/// `GET /api/v1/registry/by-code?code=`
async fn by_code(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CodeQuery>,
) -> impl IntoResponse {
    let p = state.platform.lock().await;
    let found: Vec<SixDimBinding> = p.registry.by_code(&q.code).into_iter().cloned().collect();
    drop(p);
    Json(found)
}

/// `GET /api/v1/registry/by-requirement?req=`
async fn by_requirement(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ReqQuery>,
) -> impl IntoResponse {
    let p = state.platform.lock().await;
    let found: Option<SixDimBinding> = p.registry.by_requirement(&q.req).cloned();
    drop(p);
    match found {
        Some(b) => Json(serde_json::json!({ "found": true, "binding": b })).into_response(),
        None => (StatusCode::NOT_FOUND, "未找到该需求绑定").into_response(),
    }
}

/// `GET /api/v1/registry/stats`
async fn registry_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let p = state.platform.lock().await;
    let s = p.registry.stats();
    drop(p);
    Json(serde_json::json!({
        "total": s.total,
        "completed": s.completed,
        "rejected": s.rejected,
        "sum_kappa": s.sum_kappa,
        "sum_tau": s.sum_tau,
        "sum_c": s.sum_c,
        "sum_q": s.sum_q,
    }))
}

/// `POST /api/v1/persist`
async fn persist(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = match &state.config.persistence_path {
        Some(p) => p.clone(),
        None => {
            return (StatusCode::BAD_REQUEST, "未配置 persistence_path，无法落盘").into_response()
        }
    };
    let p = state.platform.lock().await;
    match p.registry.save(&path) {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "saved": path.display().to_string() })),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /api/v1/gate`
async fn gate(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let p = state.platform.lock().await;
    let summary = gate_to_summary(&p.graph.full_gate());
    drop(p);
    Json(summary)
}

/// `GET /api/v1/docs`
async fn list_docs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let dir = &state.config.docs_dir;
    let mut docs: Vec<DocMeta> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) == Some("md") {
                if let Ok(meta) = e.metadata() {
                    docs.push(DocMeta {
                        name: path.file_name().unwrap().to_string_lossy().into_owned(),
                        size: meta.len(),
                    });
                }
            }
        }
    }
    docs.sort_by(|a, b| a.name.cmp(&b.name));
    Json(docs)
}

/// `GET /api/v1/docs/:id`
async fn get_doc(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    // 仅允许纯文件名，防止目录穿越
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return (StatusCode::BAD_REQUEST, "非法文档 id").into_response();
    }
    let dir = &state.config.docs_dir;
    let path = dir.join(&id);
    match std::fs::read_to_string(&path) {
        Ok(s) => (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/markdown; charset=utf-8",
            )],
            s,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "文档不存在").into_response(),
    }
}

// ——— 中间件 ———

/// Bearer 鉴权中间件：除 `health`/`version` 外，要求 `Authorization: Bearer <token>`。
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path().to_string();
    if path == "/api/health" || path == "/api/version" {
        return next.run(req).await;
    }
    if let Some(expected) = &state.config.auth_token {
        let ok = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v == format!("Bearer {expected}"))
            .unwrap_or(false);
        if !ok {
            return (StatusCode::UNAUTHORIZED, "缺少或非法的 Authorization 头").into_response();
        }
    }
    next.run(req).await
}

/// 零依赖 CORS 中间件：为所有响应追加 `Access-Control-Allow-Origin: *`，
/// 并对浏览器预检 `OPTIONS` 直接返回 204，使前端可直接跨域调用本服务。
async fn cors_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    if req.method() == Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("access-control-allow-origin", "*")
            .header("access-control-allow-methods", "GET, POST, OPTIONS")
            .header(
                "access-control-allow-headers",
                "content-type, authorization",
            )
            .header("access-control-max-age", "86400")
            .body(Body::empty())
            .unwrap_or_else(|_| StatusCode::NO_CONTENT.into_response());
    }
    let mut resp = next.run(req).await;
    let v = HeaderValue::from_static("*");
    resp.headers_mut().insert("access-control-allow-origin", v);
    resp
}

/// 组装路由（无状态依赖，可被示例监听或测试直接驱动）
pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/api/v1/synthesize", post(synthesize))
        .route("/api/v1/registry/by-code", get(by_code))
        .route("/api/v1/registry/by-requirement", get(by_requirement))
        .route("/api/v1/registry/stats", get(registry_stats))
        .route("/api/v1/persist", post(persist))
        .route("/api/v1/gate", get(gate))
        .route("/api/v1/docs", get(list_docs))
        .route("/api/v1/docs/:id", get(get_doc));

    Router::new()
        .route("/api/health", get(health))
        .route("/api/version", get(version))
        .merge(api)
        .layer(middleware::from_fn(cors_middleware))
        .layer(middleware::from_fn(observability::request_span))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

/// 真正监听端口（供 `main.rs` 调用，阻塞运行）
pub async fn serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    use axum::serve;
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("PrimiFlow-Fusion API listening on {addr}");
    serve(listener, app).await?;
    Ok(())
}

/// 路由信息（供健康检查 / 文档自生成引用）
pub const API_CONTRACT: &[(&str, &str, &str)] = &[
    ("GET", "/api/health", "探活：注册表统计/闸门状态"),
    ("GET", "/api/version", "服务版本"),
    (
        "POST",
        "/api/v1/synthesize",
        "提交需求，跑一体化合成并导出 PT-DOC",
    ),
    ("GET", "/api/v1/registry/by-code", "code→需求 溯源反查"),
    (
        "GET",
        "/api/v1/registry/by-requirement",
        "按需求 id 查询绑定",
    ),
    ("GET", "/api/v1/registry/stats", "注册表统计"),
    ("POST", "/api/v1/persist", "落盘注册表"),
    ("GET", "/api/v1/gate", "跑全局治理闸门"),
    ("GET", "/api/v1/docs", "列出已导出 PT-DOC"),
    ("GET", "/api/v1/docs/:id", "读取某 PT-DOC 内容"),
];
