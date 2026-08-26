//! HTTP 接口层：把中枢能力暴露为 `/api/kg/*`，供治理台与前端消费。
//!
//! 挂载方式（在 `crates/mox_platform_orchestrator_svc` 的 Router 上合并）：
//! ```ignore
//! let hub = mox_kg_hub_svc::api::shared(mox_kg_hub_svc::KgHub::new("default"));
//! let app = Router::new().merge(mox_kg_hub_svc::api::routes(hub));
//! ```
//!
//! 全部接口只读或幂等写（接入为 upsert），因此不做额外事务；
//! 并发下用 `RwLock` 保证读多写少场景的吞吐——检索远多于接入。

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    index::HybridQuery,
    ingest::{InfoGraphConnector, KnowledgeBaseConnector, KnowledgeItem},
    KgHub,
};

/// 共享中枢句柄
pub type SharedHub = Arc<RwLock<KgHub>>;

pub fn shared(hub: KgHub) -> SharedHub {
    Arc::new(RwLock::new(hub))
}

/// 统一响应包装：成功与失败结构一致，前端无需分支解析。
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResp<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResp<T> {
    pub fn ok(data: T) -> Json<Self> {
        Json(Self {
            ok: true,
            data: Some(data),
            error: None,
        })
    }
    pub fn err(msg: impl Into<String>) -> Json<Self> {
        Json(Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        })
    }
}

/// 路由表：中枢全部对外能力
pub fn routes(hub: SharedHub) -> Router {
    Router::new()
        // ── 总览与健康 ──
        .route("/api/kg/health", get(health))
        .route("/api/kg/overview", get(overview))
        // ── 检索（智能层）──
        .route("/api/kg/search", post(search))
        .route("/api/kg/search/quick", get(quick_search))
        // ── 推理（关联分析）──
        .route("/api/kg/impact/:id", get(impact))
        .route("/api/kg/trace/:id", get(trace))
        .route("/api/kg/hotspots", get(hotspots))
        .route("/api/kg/isolated", get(isolated))
        // ── 治理（闸门）──
        .route("/api/kg/governance", get(governance))
        .route("/api/kg/deviation", get(deviation))
        .route("/api/kg/gate", get(gate))
        // ── 接入（知识库连接器）──
        .route("/api/kg/ingest/info-graph", post(ingest_info_graph))
        .route("/api/kg/ingest/knowledge-base", post(ingest_kb))
        // ── 智能闭环 ──
        .route("/api/kg/loop/run", post(run_loop))
        // ── 导出 ──
        .route("/api/kg/export/mermaid", get(export_mermaid))
        .with_state(hub)
}

// ───────────────────────── 总览 ─────────────────────────

async fn health() -> Json<ApiResp<serde_json::Value>> {
    ApiResp::ok(serde_json::json!({
        "service": "kg-hub",
        "status": "healthy",
        "capability": "统一关图中枢：三图归一 / 混合检索 / 关联推理 / 治理闸门 / 八段闭环"
    }))
}

async fn overview(State(h): State<SharedHub>) -> Json<ApiResp<crate::HubOverview>> {
    ApiResp::ok(h.read().await.overview())
}

// ───────────────────────── 检索 ─────────────────────────

async fn search(
    State(h): State<SharedHub>,
    Json(q): Json<HybridQuery>,
) -> Json<ApiResp<Vec<crate::SearchHit>>> {
    ApiResp::ok(h.read().await.search(&q))
}

#[derive(Debug, Deserialize)]
struct QuickQ {
    q: String,
    #[serde(default)]
    top_k: Option<usize>,
    /// 图扩散跳数
    #[serde(default)]
    hops: Option<usize>,
}

async fn quick_search(
    State(h): State<SharedHub>,
    Query(p): Query<QuickQ>,
) -> Json<ApiResp<Vec<crate::SearchHit>>> {
    let q = HybridQuery {
        text: p.q,
        top_k: p.top_k.unwrap_or(10),
        expand_hops: p.hops.unwrap_or(0),
        ..Default::default()
    };
    ApiResp::ok(h.read().await.search(&q))
}

// ───────────────────────── 推理 ─────────────────────────

#[derive(Debug, Deserialize)]
struct HopQ {
    #[serde(default)]
    hops: Option<usize>,
}

async fn impact(
    State(h): State<SharedHub>,
    Path(id): Path<String>,
    Query(p): Query<HopQ>,
) -> Json<ApiResp<crate::ImpactReport>> {
    ApiResp::ok(h.read().await.impact(&id, p.hops.unwrap_or(2)))
}

async fn trace(
    State(h): State<SharedHub>,
    Path(id): Path<String>,
) -> Json<ApiResp<crate::TraceReport>> {
    ApiResp::ok(h.read().await.trace(&id))
}

#[derive(Debug, Deserialize)]
struct TopQ {
    #[serde(default)]
    top: Option<usize>,
}

async fn hotspots(
    State(h): State<SharedHub>,
    Query(p): Query<TopQ>,
) -> Json<ApiResp<Vec<crate::Hotspot>>> {
    ApiResp::ok(h.read().await.hotspots(p.top.unwrap_or(10)))
}

async fn isolated(State(h): State<SharedHub>) -> Json<ApiResp<Vec<String>>> {
    ApiResp::ok(h.read().await.isolated())
}

// ───────────────────────── 治理 ─────────────────────────

async fn governance(State(h): State<SharedHub>) -> Json<ApiResp<crate::GovernanceSummary>> {
    ApiResp::ok(h.read().await.governance())
}

async fn deviation(State(h): State<SharedHub>) -> Json<ApiResp<crate::DeviationReport>> {
    ApiResp::ok(crate::govern::detect_deviation(h.read().await.graph()))
}

async fn gate(State(h): State<SharedHub>) -> Json<ApiResp<crate::GateReport>> {
    ApiResp::ok(crate::govern::gate_report(h.read().await.graph()))
}

// ───────────────────────── 接入 ─────────────────────────

#[derive(Debug, Deserialize)]
struct IngestGraphReq {
    /// 直接给 JSON 内容
    #[serde(default)]
    graph_json: Option<String>,
    /// 或给服务端可读路径
    #[serde(default)]
    path: Option<String>,
}

async fn ingest_info_graph(
    State(h): State<SharedHub>,
    Json(req): Json<IngestGraphReq>,
) -> (StatusCode, Json<ApiResp<crate::IngestStat>>) {
    let conn = match (req.graph_json, req.path) {
        (Some(j), _) => InfoGraphConnector::from_str(j),
        (None, Some(p)) => match InfoGraphConnector::from_path(&p) {
            Ok(c) => c,
            Err(e) => return (StatusCode::BAD_REQUEST, ApiResp::err(e.to_string())),
        },
        (None, None) => {
            return (
                StatusCode::BAD_REQUEST,
                ApiResp::err("需提供 graph_json 或 path 之一"),
            )
        }
    };
    match h.write().await.ingest(&conn) {
        Ok(st) => (StatusCode::OK, ApiResp::ok(st)),
        Err(e) => (StatusCode::BAD_REQUEST, ApiResp::err(e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct IngestKbReq {
    source: String,
    items: Vec<KnowledgeItem>,
}

async fn ingest_kb(
    State(h): State<SharedHub>,
    Json(req): Json<IngestKbReq>,
) -> (StatusCode, Json<ApiResp<crate::IngestStat>>) {
    let conn = KnowledgeBaseConnector {
        source: req.source,
        items: req.items,
    };
    match h.write().await.ingest(&conn) {
        Ok(st) => (StatusCode::OK, ApiResp::ok(st)),
        Err(e) => (StatusCode::BAD_REQUEST, ApiResp::err(e.to_string())),
    }
}

// ───────────────────────── 闭环 ─────────────────────────

#[derive(Debug, Serialize)]
struct LoopResp {
    decision: String,
    decision_reason: String,
    persisted: bool,
    node_count: usize,
    edge_count: usize,
    coverage: f64,
    traces: Vec<crate::StageTrace>,
    mermaid: String,
}

/// 对当前事实源即时跑一轮校验闭环（不引入新知识源），
/// 用于治理台"立即体检"按钮。
async fn run_loop(State(h): State<SharedHub>) -> Json<ApiResp<LoopResp>> {
    let hub = h.read().await;
    let (decision, gov) = crate::loop_engine::verify_graph(hub.graph());
    let resp = LoopResp {
        decision: decision.zh().to_string(),
        decision_reason: format!(
            "闸门{} / 偏离{}",
            if gov.gate.passed { "通过" } else { "失败" },
            if gov.deviation.passed {
                "通过"
            } else {
                "失败"
            }
        ),
        persisted: decision != crate::Decision::Reject,
        node_count: gov.node_count,
        edge_count: gov.edge_count,
        coverage: gov.deviation.coverage,
        traces: Vec::new(),
        mermaid: hub.to_mermaid(),
    };
    ApiResp::ok(resp)
}

async fn export_mermaid(State(h): State<SharedHub>) -> Json<ApiResp<String>> {
    ApiResp::ok(h.read().await.to_mermaid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::InfoGraphConnector;

    fn sample() -> &'static str {
        r#"{"nodes":[
            {"id":"Requirement:D01","kind":"Requirement","name":"D01","path":"REQ/D01","summary":"","external":false},
            {"id":"CodeFile:a.rs","kind":"CodeFile","name":"a.rs","path":"a.rs","summary":"","external":false}],
          "edges":[{"id":"e1","from":"Requirement:D01","to":"CodeFile:a.rs","kind":"Bind","label":"","evidence":"x"}]}"#
    }

    #[tokio::test]
    async fn routes_build_without_panic() {
        let mut hub = KgHub::new("default");
        hub.ingest(&InfoGraphConnector::from_str(sample())).unwrap();
        let _app = routes(shared(hub));
    }

    #[tokio::test]
    async fn overview_handler_returns_counts() {
        let mut hub = KgHub::new("default");
        hub.ingest(&InfoGraphConnector::from_str(sample())).unwrap();
        let h = shared(hub);
        let Json(r) = overview(State(h)).await;
        assert!(r.ok);
        let d = r.data.unwrap();
        assert_eq!(d.node_count, 2);
        assert_eq!(d.edge_count, 1);
    }

    #[tokio::test]
    async fn ingest_handler_rejects_empty_payload() {
        let h = shared(KgHub::new("default"));
        let (code, Json(r)) = ingest_info_graph(
            State(h),
            Json(IngestGraphReq {
                graph_json: None,
                path: None,
            }),
        )
        .await;
        assert_eq!(code, StatusCode::BAD_REQUEST);
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("graph_json"));
    }

    #[tokio::test]
    async fn ingest_handler_accepts_inline_json() {
        let h = shared(KgHub::new("default"));
        let (code, Json(r)) = ingest_info_graph(
            State(h.clone()),
            Json(IngestGraphReq {
                graph_json: Some(sample().into()),
                path: None,
            }),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(r.data.unwrap().nodes_new, 2);
        assert_eq!(h.read().await.graph().nodes.len(), 2);
    }

    #[tokio::test]
    async fn malformed_json_returns_400_not_500() {
        let h = shared(KgHub::new("default"));
        let (code, Json(r)) = ingest_info_graph(
            State(h),
            Json(IngestGraphReq {
                graph_json: Some("{oops".into()),
                path: None,
            }),
        )
        .await;
        assert_eq!(code, StatusCode::BAD_REQUEST);
        assert!(!r.ok);
    }

    #[tokio::test]
    async fn quick_search_handler_finds_node() {
        let mut hub = KgHub::new("default");
        hub.ingest(&InfoGraphConnector::from_str(sample())).unwrap();
        let Json(r) = quick_search(
            State(shared(hub)),
            Query(QuickQ {
                q: "a.rs".into(),
                top_k: Some(5),
                hops: Some(1),
            }),
        )
        .await;
        assert!(r.ok);
        assert!(!r.data.unwrap().is_empty());
    }

    #[tokio::test]
    async fn governance_and_loop_handlers_agree_on_pass() {
        let mut hub = KgHub::new("default");
        hub.ingest(&InfoGraphConnector::from_str(sample())).unwrap();
        let h = shared(hub);
        let Json(g) = governance(State(h.clone())).await;
        let Json(l) = run_loop(State(h)).await;
        assert!(g.data.unwrap().passed);
        let lr = l.data.unwrap();
        assert_eq!(lr.decision, "放行");
        assert_eq!(lr.coverage, 100.0);
    }

    #[tokio::test]
    async fn trace_handler_grounds_code_to_requirement() {
        let mut hub = KgHub::new("default");
        hub.ingest(&InfoGraphConnector::from_str(sample())).unwrap();
        let id = crate::urn::build_default(
            mox_flow_fusion_svc::Layer::ExecutionRuntime,
            mox_flow_fusion_svc::EntityKind::Code,
            "a.rs",
        );
        let Json(r) = trace(State(shared(hub)), Path(id)).await;
        assert!(r.data.unwrap().grounded);
    }

    #[tokio::test]
    async fn kb_ingest_handler_works() {
        let h = shared(KgHub::new("default"));
        let (code, Json(r)) = ingest_kb(
            State(h),
            Json(IngestKbReq {
                source: "wiki".into(),
                items: vec![KnowledgeItem {
                    key: "kb/a.md".into(),
                    title: "A".into(),
                    body: "内容".into(),
                    kind: None,
                    evidence: String::new(),
                    refs: vec![],
                }],
            }),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(r.data.unwrap().nodes_new, 1);
    }
}
