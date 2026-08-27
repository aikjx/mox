// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 层3 · API 服务层（κ‑τ 引擎对外 HTTP 服务）
//!
//! 实现 `gen/c5.rs` 定义的 REST 契约，把 κ‑τ 引擎、六维溯源、真实持久化层
//! 通过 HTTP 暴露给外部系统：提交需求 → 触发生成 → 查询拓扑 → 冻结资产 → 检索知识库。
//!
//! 端点：
//! - `GET  /api/health`                        探活：返回状态、知识库资产数、累计项目数、拓扑荷 Q
//! - `POST /api/projects`                      提交自然语言需求，跑完整闭环，返回项目/拓扑 ID 与报告
//! - `GET  /api/projects`                      列出全部项目（审计清单：κ/τ/守恒/无环/绑定/Q）
//! - `GET  /api/projects/:id`                  查询单个项目详情
//! - `POST /api/projects/:id/messages`         追加需求描述，作为同源项目的二次涌现
//! - `GET  /api/topologies/:id`                查询某拓扑的 Mermaid 可视化
//! - `POST /api/topologies/:id/regularize`     对该需求重新跑 κτ 自涌现（含 ℛ̂ 正则化）
//! - `POST /api/topologies/:id/freeze`         冻结资产到全域知识库（注荷 Q）
//! - `GET  /api/assets?q=&domain=`            检索知识库沉淀的拓扑资产
//!
//! 服务启动时会从落库（SQLite/内存）**重放**知识库与六维溯源主图（`AppState::replay_from_store`），
//! 使拓扑荷 Q 在进程重启后仍然连续复用（见 `persistence::Persistence::replay_into`）。
//!
//! 设计为「可测试」：通过 [`build_router`] 返回 `axum::Router`，既可由示例 `serve()` 真正监听端口，
//! 也可在无网络下用 `tower::ServiceExt::oneshot` 直接驱动（见集成测试）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::assoc::AssocGraph;
use crate::parse::parse;
use crate::persistence::Persistence;
use crate::runner::{run_pipeline, PipelineReport};
use mox_ai_flow_svc::primitive::{PrimiEngine, ResourceBudget};

/// 共享应用状态（跨请求保持引擎闭环节点状态与六维溯源主图）
pub struct AppState {
    pub engine: Mutex<PrimiEngine>,
    pub master: Mutex<AssocGraph>,
    pub store: Mutex<Persistence>,
    pub out_dir: PathBuf,
    /// 项目 ID → 拓扑 Mermaid
    pub topologies: Mutex<HashMap<String, String>>,
    /// 项目 ID → 最近一次需求描述（供 regularize 重跑）
    pub last_input: Mutex<HashMap<String, String>>,
}

/// 构造应用状态。默认 C=10，空知识库，落盘到 `out_dir` 下的 SQLite（可替换为 Memory）。
pub fn new_state(out_dir: PathBuf, store: Persistence) -> Arc<AppState> {
    Arc::new(AppState {
        engine: Mutex::new(PrimiEngine::new(
            10.0,
            Default::default(),
            ResourceBudget::default(),
        )),
        master: Mutex::new(AssocGraph::new()),
        store: Mutex::new(store),
        out_dir,
        topologies: Mutex::new(HashMap::new()),
        last_input: Mutex::new(HashMap::new()),
    })
}

/// 由自然语言描述跑一个需求的完整闭环，返回（拓扑 ID, 报告）
async fn run_requirement(
    state: &Arc<AppState>,
    description: &str,
) -> Result<(String, PipelineReport), String> {
    let spec = parse(description).to_spec();
    let id = spec.id.clone();
    let engine_req = spec.requirement();
    let policy = spec.policy;

    let report = {
        let mut engine = state.engine.lock().await;
        let mut master = state.master.lock().await;
        run_pipeline(
            &mut engine,
            &engine_req,
            policy,
            &mut master,
            &state.out_dir,
        )
        .map_err(|e| e.to_string())?
    };

    // 持久化（知识库 + 溯源图 + 项目记录）
    {
        let engine = state.engine.lock().await;
        let master = state.master.lock().await;
        let mut store = state.store.lock().await;
        let _ = store.persist_pipeline(&engine, &master, &id, &report);
    }

    // 读取引擎写出的真实涌现 DAG Mermaid
    let topo =
        std::fs::read_to_string(state.out_dir.join(format!("topo_{id}.mmd"))).unwrap_or_default();
    state.topologies.lock().await.insert(id.clone(), topo);
    state
        .last_input
        .lock()
        .await
        .insert(id.clone(), description.to_string());

    Ok((id, report))
}

// ——— 请求/响应结构 ———

#[derive(Deserialize)]
pub struct CreateProjectReq {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct CreateProjectResp {
    pub id: String,
    pub topology_id: String,
    pub report: PipelineReport,
}

#[derive(Deserialize)]
pub struct MessageReq {
    pub content: String,
}

#[derive(Serialize)]
pub struct MessageResp {
    pub project_id: String,
    pub topology_id: String,
    pub report: PipelineReport,
}

#[derive(Serialize)]
pub struct FreezeResp {
    pub project_id: String,
    pub kb_assets: usize,
    pub q: f64,
}

#[derive(Serialize)]
pub struct AssetView {
    pub id: String,
    pub signature: String,
    pub charge: f64,
    pub reuse_count: u64,
}

#[derive(Serialize)]
pub struct AssetsResp {
    pub total: usize,
    pub matched: Vec<AssetView>,
}

/// 项目审计视图（抽取 `ProjectRecord` 的可序列化字段）
#[derive(Serialize)]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub policy: String,
    pub kappa: f64,
    pub tau: f64,
    pub conserved: bool,
    pub acyclic: bool,
    pub reused: usize,
    pub regularized: bool,
    pub q_before: f64,
    pub q_after: f64,
    pub bound_nodes: usize,
    pub bound_edges: usize,
    pub created_at: String,
}

impl From<crate::persistence::ProjectRecord> for ProjectView {
    fn from(r: crate::persistence::ProjectRecord) -> Self {
        Self {
            id: r.id,
            name: r.name,
            policy: r.policy,
            kappa: r.kappa,
            tau: r.tau,
            conserved: r.conserved,
            acyclic: r.acyclic,
            reused: r.reused,
            regularized: r.regularized,
            q_before: r.q_before,
            q_after: r.q_after,
            bound_nodes: r.bound_nodes,
            bound_edges: r.bound_edges,
            created_at: r.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ProjectsListResp {
    pub total: usize,
    pub projects: Vec<ProjectView>,
}

#[derive(Serialize)]
pub struct ProjectDetailResp {
    pub project: ProjectView,
}

#[derive(Deserialize)]
pub struct AssetsQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub domain: String,
}

// ——— 处理器 ———

/// `POST /api/projects`
async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectReq>,
) -> impl IntoResponse {
    match run_requirement(&state, &req.description).await {
        Ok((id, report)) => (
            StatusCode::OK,
            Json(CreateProjectResp {
                topology_id: id.clone(),
                id,
                report,
            }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// `POST /api/projects/:id/messages`
async fn post_message(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<MessageReq>,
) -> impl IntoResponse {
    match run_requirement(&state, &req.content).await {
        Ok((id, report)) => (
            StatusCode::OK,
            Json(MessageResp {
                project_id,
                topology_id: id,
                report,
            }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// `GET /api/topologies/:id`
async fn get_topology(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let topo = state.topologies.lock().await.get(&id).cloned();
    match topo {
        Some(md) => (
            StatusCode::OK,
            [("content-type", "text/plain; charset=utf-8")],
            md,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "拓扑不存在").into_response(),
    }
}

/// `POST /api/topologies/:id/regularize`
async fn regularize(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let input = state.last_input.lock().await.get(&id).cloned();
    match input {
        Some(desc) => match run_requirement(&state, &desc).await {
            Ok((new_id, report)) => (
                StatusCode::OK,
                Json(CreateProjectResp {
                    topology_id: new_id,
                    id,
                    report,
                }),
            )
                .into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        },
        None => (StatusCode::NOT_FOUND, "该拓扑无重跑输入").into_response(),
    }
}

/// `POST /api/topologies/:id/freeze`
async fn freeze(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    let engine = state.engine.lock().await;
    let resp = FreezeResp {
        project_id: id,
        kb_assets: engine.kb.stored.len(),
        q: engine.state.q,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

/// `GET /api/assets?q=&domain=`
async fn list_assets(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AssetsQuery>,
) -> impl IntoResponse {
    let engine = state.engine.lock().await;
    let matched: Vec<AssetView> = engine
        .kb
        .stored
        .iter()
        .filter(|a| {
            let hit_q = q.q.is_empty() || a.signature.contains(&q.q);
            let hit_d = q.domain.is_empty()
                || a.signature
                    .to_lowercase()
                    .contains(&q.domain.to_lowercase());
            hit_q && hit_d
        })
        .map(|a| AssetView {
            id: a.id.clone(),
            signature: a.signature.clone(),
            charge: a.charge,
            reuse_count: a.reuse_count,
        })
        .collect();
    let total = engine.kb.stored.len();
    (StatusCode::OK, Json(AssetsResp { total, matched })).into_response()
}

/// `GET /api/projects`
async fn list_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let store = state.store.lock().await;
    match store.list_projects() {
        Ok(recs) => {
            let total = recs.len();
            let projects = recs.into_iter().map(ProjectView::from).collect();
            (StatusCode::OK, Json(ProjectsListResp { total, projects })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /api/projects/:id`
async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = state.store.lock().await;
    match store.get_project(&id) {
        Ok(Some(rec)) => (
            StatusCode::OK,
            Json(ProjectDetailResp {
                project: ProjectView::from(rec),
            }),
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "项目不存在").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /api/health`
///
/// 运维/前端探活：返回引擎当前状态（知识库资产数、累计项目数、拓扑荷 Q）。
/// 不依赖任何外部系统，始终 200。
async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (kb_assets, q) = {
        let e = state.engine.lock().await;
        (e.kb.stored.len(), e.state.q)
    };
    let projects_total = {
        let s = state.store.lock().await;
        s.list_projects().map(|v| v.len()).unwrap_or(0)
    };
    let body = serde_json::json!({
        "status": "ok",
        "service": "primiflow",
        "version": env!("CARGO_PKG_VERSION"),
        "kb_assets": kb_assets,
        "projects_total": projects_total,
        "q": q,
    });
    (StatusCode::OK, Json(body)).into_response()
}

/// 零依赖 CORS 中间件：为所有响应追加 `Access-Control-Allow-Origin: *`，
/// 并对浏览器预检 `OPTIONS` 直接返回 204，使前端可直接跨域调用本服务。
async fn cors_middleware(req: Request<Body>, next: Next) -> Response {
    if req.method() == Method::OPTIONS {
        return (
            StatusCode::NO_CONTENT,
            [
                ("access-control-allow-origin", "*"),
                ("access-control-allow-methods", "GET, POST, OPTIONS"),
                ("access-control-allow-headers", "content-type"),
                ("access-control-max-age", "86400"),
            ],
        )
            .into_response();
    }
    let mut resp = next.run(req).await;
    let v = HeaderValue::from_static("*");
    resp.headers_mut().insert("access-control-allow-origin", v);
    resp
}

/// 组装路由（无状态依赖，可被示例监听或测试直接驱动）
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/projects", post(create_project))
        .route("/api/projects", get(list_projects))
        .route("/api/projects/:id", get(get_project))
        .route("/api/projects/:id/messages", post(post_message))
        .route("/api/topologies/:id", get(get_topology))
        .route("/api/topologies/:id/regularize", post(regularize))
        .route("/api/topologies/:id/freeze", post(freeze))
        .route("/api/assets", get(list_assets))
        .layer(middleware::from_fn(cors_middleware))
        .with_state(state)
}

impl AppState {
    /// 服务启动时从落库恢复知识库与六维溯源主图，使**跨重启的拓扑荷 Q 连续复用**。
    ///
    /// 例：上一次进程固化了 N 个资产，本进程启动后引擎直接继承，新需求可命中历史资产、
    /// 不用从零探索。失败（如空库/损坏）静默跳过，不影响启动。
    pub async fn replay_from_store(&self) {
        let (kb, g) = {
            let store = self.store.lock().await;
            (store.load_kb().ok(), store.load_graph().ok())
        };
        if let Some(kb) = kb {
            let mut engine = self.engine.lock().await;
            engine.kb = kb;
        }
        if let Some(g) = g {
            let mut master = self.master.lock().await;
            *master = g;
        }
    }
}

/// 真正监听端口（供示例 `server_demo` 调用，阻塞运行）
pub async fn serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    use axum::serve;
    state.replay_from_store().await;
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("PrimiFlow API listening on {addr}");
    serve(listener, app).await?;
    Ok(())
}

/// 非阻塞启动并返回监听地址（供集成测试用 `reqwest` 真正驱动 HTTP 全链路）
pub async fn spawn_serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<std::net::SocketAddr> {
    use axum::serve;
    state.replay_from_store().await;
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = serve(listener, app).await;
    });
    Ok(local)
}

/// 路由信息（供健康检查 / 文档自生成引用）
pub const API_CONTRACT: &[(&str, &str, &str)] = &[
    ("GET", "/api/health", "探活：服务状态/资产/Q"),
    ("POST", "/api/projects", "提交需求，跑 κτ 闭环"),
    ("GET", "/api/projects", "列出全部项目（审计清单）"),
    ("GET", "/api/projects/:id", "查询单个项目详情"),
    ("POST", "/api/projects/:id/messages", "追加需求描述"),
    ("GET", "/api/topologies/:id", "查询拓扑 Mermaid"),
    ("POST", "/api/topologies/:id/regularize", "重跑 κτ 自涌现"),
    ("POST", "/api/topologies/:id/freeze", "冻结资产到知识库"),
    ("GET", "/api/assets", "检索知识库资产"),
];
