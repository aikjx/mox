//! AI 统一查询：/ai/engine/{process,analyze,capabilities,metrics} 四端点 handler
//!
//! 路由决策 pipeline：
//!   意图分类（sidecar → 本地关键词兜底）→ 激活扩散重排（graph-algo 调用，或空 pass）→
//!   语义缓存探测（简化：Node 端内部实现，Gateway 侧先按意图直接过 capability router）→
//!   能力路由（CapRouter）→ 执行（本地等价直调 sidecar / AI 直调 AIAgent / 混合）→ 回填

use crate::ai_router::{CapabilityEntry, CapabilityRouter, ExecutorKind, RouterDecision, RouterTable};
use crate::sidecar::node_sidecar::{
    GraphAlgoReq, IntentReq, IntentResp, NodeSidecarClient,
};
use ai_agent::AIAgent;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ================== 协议：请求 / 响应 ==================

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ProcessOptions {
    #[serde(default)]
    pub prefer: Option<String>, // "local" | "ai" | "hybrid"
    #[serde(default)]
    pub max_latency_ms: Option<u64>,
    #[serde(default)]
    pub explain: Option<bool>,
    #[serde(default)]
    pub compat: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessRequest {
    pub query: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    #[serde(default)]
    pub options: ProcessOptions,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RouteInfo {
    pub intent: String,
    pub capability: String,
    pub executor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct MetricsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    pub sidecar_calls: u64,
    pub sidecar_fail: u64,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ProcessResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<RouteInfo>,
    /// data 段与现状本地接口同 shape：graph_list -> 节点数组、file_list -> 文件数组；老客户端零改消费。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<MetricsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AnalyzeRequest {
    pub capability: String,
    pub query: Option<String>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    #[serde(default)]
    pub options: ProcessOptions,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Capability {
    pub name: String,
    pub executor: String,
    pub category: String,
    pub p95_latency_ms: Option<u64>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct CapabilitiesResponse {
    pub ok: bool,
    pub count: usize,
    pub items: Vec<Capability>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct EngineMetricsResponse {
    pub ok: bool,
    pub requests_total: u64,
    pub ai_hits: u64,
    pub local_hits: u64,
    pub hybrid_hits: u64,
    pub degrade_hits: u64,
    pub sidecar: crate::sidecar::SidecarMetricsSnapshot,
    pub p95_latency_ms: BTreeMap<String, u64>,
}

// ================== 共享状态 ==================

#[derive(Clone)]
pub struct AiEngineState {
    pub agent: Option<Arc<AIAgent>>,
    pub sidecar: Arc<NodeSidecarClient>,
    pub capability_router: Arc<CapabilityRouter>,
    #[allow(dead_code)] // 预留：后续 `/ai/engine/routes` 调试端点与子路径分发会用；当前 lint 保留
    pub path_router: Arc<RouterTable>,
    pub stats: Arc<EngineStats>,
}

#[derive(Debug, Default)]
pub struct EngineStats {
    pub requests_total: AtomicU64,
    pub ai_hits: AtomicU64,
    pub local_hits: AtomicU64,
    pub hybrid_hits: AtomicU64,
    pub degrade_hits: AtomicU64,
    // 简单的 P95：保留一个 ring buffer，O(N) 在 metrics 端点计算。
    latencies: parking_lot::Mutex<Vec<u64>>,
}

impl EngineStats {
    pub fn record_latency(&self, ms: u64) {
        let mut v = self.latencies.lock();
        v.push(ms);
        if v.len() > 2000 {
            let drop = v.len() - 2000;
            v.drain(0..drop);
        }
    }
    pub fn p95(&self) -> u64 {
        let v = self.latencies.lock();
        if v.is_empty() { return 0; }
        let mut sorted: Vec<u64> = v.clone();
        sorted.sort_unstable();
        let idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
        sorted[idx.min(sorted.len().saturating_sub(1))]
    }
}

impl Default for AiEngineState {
    fn default() -> Self {
        let mut router = CapabilityRouter::new();
        router.register("chat", "llm_chat", ExecutorKind::Ai, None);
        router.register("graph_query", "graph_query", ExecutorKind::Hybrid, None);
        router.register("graph_list", "graph_list", ExecutorKind::Local, None);
        router.register("file_search", "file_graph_search", ExecutorKind::Hybrid, None);
        router.register("file_list", "file_list", ExecutorKind::Local, None);
        router.register("kb_search", "kb_search", ExecutorKind::Hybrid, None);
        router.register("graph_bulk_write", "graph_bulk_write", ExecutorKind::Local, None);
        router.register("atlas_trace", "atlas_trace", ExecutorKind::Hybrid, None);

        let mut paths = RouterTable::new();
        paths.register("graph_list", "/graph/nodes");
        paths.register("graph_node_get", "/graph/nodes/:id");
        paths.register("file_list", "/files/list");
        paths.register("file_get", "/files/:id");
        paths.register("kb_search", "/kb/search");
        paths.register("intent_infer", "/internal/intent");

        Self {
            agent: None,
            sidecar: Arc::new(NodeSidecarClient::new("http://127.0.0.1:3010")),
            capability_router: Arc::new(router),
            path_router: Arc::new(paths),
            stats: Arc::new(EngineStats::default()),
        }
    }
}

impl AiEngineState {
    pub fn with_agent(mut self, agent: Arc<AIAgent>) -> Self { self.agent = Some(agent); self }
    pub fn with_sidecar(mut self, s: NodeSidecarClient) -> Self { self.sidecar = Arc::new(s); self }
}

// ================== 端点实现 ==================

fn count_ms(s: std::time::Instant) -> u64 { s.elapsed().as_millis() as u64 }

pub async fn process_handler(
    State(state): State<Arc<AiEngineState>>,
    Json(mut req): Json<ProcessRequest>,
) -> (StatusCode, Json<ProcessResponse>) {
    let started = std::time::Instant::now();
    state.stats.requests_total.fetch_add(1, Ordering::Relaxed);
    if req.options.compat.is_none() { req.options.compat = Some(true); }

    let mut explain: Vec<String> = Vec::new();

    // ① 意图识别：优先用 sidecar /internal/intent
    let IntentResp { intent, capability: maybe_cap, explain: mut intent_explain, .. } = state
        .sidecar
        .intent(IntentReq { query: req.query.clone().unwrap_or_default(), context: req.context.clone() })
        .await
        .unwrap_or(IntentResp {
            ok: true,
            intent: req.intent.clone().unwrap_or_else(|| "chat".to_string()),
            confidence: 0.0,
            capability: req.capability.clone(),
            explain: vec!["sidecar intent err: fallback to request.intent".to_string()],
        });
    explain.append(&mut intent_explain);

    // ② 能力路由
    let (capability, entry): (String, Option<CapabilityEntry>) = match req.capability.clone().or(maybe_cap) {
        Some(c) => (c.clone(), None),
        None => {
            let e = state.capability_router.resolve(&intent).cloned();
            let cap = e.as_ref().map(|e| e.capability.clone()).unwrap_or_else(|| "llm_chat".to_string());
            (cap, e)
        }
    };
    let e = entry.or_else(|| state.capability_router.resolve(&intent).cloned());
    let executor = e.as_ref().map(|x| x.executor).unwrap_or(ExecutorKind::Hybrid);

    // ③ 执行：对 graph_list / file_list 等本地等价能力 → 走 sidecar 的原生 internal 接口
    let mut data: Option<serde_json::Value> = None;
    let mut ai_summary: Option<String> = None;
    let mut local_ms: Option<u64> = None;
    let mut ai_ms: Option<u64> = None;

    let executor_name = match executor {
        ExecutorKind::Local => "local",
        ExecutorKind::Ai => "ai",
        ExecutorKind::Hybrid => "hybrid",
    }.to_string();

    // 路径路由：capability → 请求 sidecar 对应 /internal/* path
    let t_local = std::time::Instant::now();
    match capability.as_str() {
        "graph_list" => {
            let algo = state.sidecar.graph_algo(GraphAlgoReq {
                algorithm: "list_nodes".into(),
                payload: serde_json::to_value(&req.context).unwrap_or(serde_json::Value::Null),
            }).await;
            match algo {
                Ok(r) if r.ok => data = Some(r.result),
                Ok(r) => data = Some(r.result),
                Err(_) => { state.stats.degrade_hits.fetch_add(1, Ordering::Relaxed); },
            }
            local_ms = Some(count_ms(t_local));
            state.stats.local_hits.fetch_add(1, Ordering::Relaxed);
        }
        "file_list" => {
            // 等价行为：从 internal/list-files 路径走（简化为空数组，T8 会做 compat）
            data = Some(serde_json::json!([]));
            local_ms = Some(count_ms(t_local));
            state.stats.local_hits.fetch_add(1, Ordering::Relaxed);
        }
        "llm_chat" => {
            // 走 AI Agent：若未配置 agent → 返回仅 route 段
            let t_ai = std::time::Instant::now();
            if let Some(agent) = state.agent.as_ref() {
                let query = req.query.clone().unwrap_or_default();
                match agent.run_engine_task(query).await {
                    Ok(ar) => {
                        ai_summary = ar.summary();
                    }
                    Err(e) => explain.push(format!("ai err: {e}")),
                }
            } else {
                explain.push("ai agent unconfigured: returning stub summary".to_string());
                ai_summary = Some(format!("[stub] query: {}", req.query.as_deref().unwrap_or("")));
            }
            ai_ms = Some(count_ms(t_ai));
            state.stats.ai_hits.fetch_add(1, Ordering::Relaxed);
        }
        other => {
            // hybrid：本地 sidecar graph-algo + stub ai summary
            let _ = local_ms.insert(count_ms(t_local));
            // 尝试 sidecar 拿 data
            if let Ok(r) = state.sidecar.graph_algo(GraphAlgoReq { algorithm: other.into(), payload: serde_json::Value::Null }).await {
                if r.ok { data = Some(r.result); }
            }
            ai_summary = Some(format!("[hybrid stub] intent={intent} cap={capability}"));
            state.stats.hybrid_hits.fetch_add(1, Ordering::Relaxed);
            ai_ms = Some(1);
        }
    }

    let _ = RouterDecision { intent: intent.clone(), capability: capability.clone(), executor, steps: explain.clone(), route_path_match: None }; // placeholder

    // 响应组装：data 段原样返回（与本地等价 API 同 shape），加 ai_* 增量字段。
    let total_ms = count_ms(started);
    state.stats.record_latency(total_ms);
    let snap = state.sidecar.metrics.snapshot();
    let resp = ProcessResponse {
        ok: true,
        route: Some(RouteInfo {
            intent: intent.clone(),
            capability: capability.clone(),
            executor: executor_name,
            explain: req.options.explain.unwrap_or(false).then_some(explain),
        }),
        data,
        ai_summary,
        metrics: Some(MetricsInfo {
            total_ms: Some(total_ms),
            local_ms,
            ai_ms,
            cache_hit: Some(false),
            sidecar_calls: snap.calls,
            sidecar_fail: snap.fail,
        }),
        error: None,
    };
    (StatusCode::OK, Json(resp))
}

pub async fn analyze_handler(
    State(state): State<Arc<AiEngineState>>,
    Json(req): Json<AnalyzeRequest>,
) -> (StatusCode, Json<ProcessResponse>) {
    let pr = ProcessRequest {
        query: req.query,
        intent: None,
        capability: Some(req.capability),
        context: req.context,
        options: req.options,
        data: req.payload,
    };
    process_handler(State(state), Json(pr)).await
}

pub async fn capabilities_handler(State(state): State<Arc<AiEngineState>>) -> impl IntoResponse {
    let entries = state.capability_router.list();
    let items: Vec<Capability> = entries
        .into_iter()
        .map(|(_intent, e)| Capability {
            name: e.capability.clone(),
            executor: match e.executor {
                ExecutorKind::Local => "local",
                ExecutorKind::Ai => "ai",
                ExecutorKind::Hybrid => "hybrid",
            }.to_string(),
            category: match e.capability.as_str() {
                s if s.starts_with("graph") => "graph",
                s if s.starts_with("file")  => "file",
                s if s.starts_with("kb")    => "kb",
                _ => "ai",
            }.to_string(),
            p95_latency_ms: e.p95_latency_ms,
            description: Some(format!("capability registry entry for {}", e.capability)),
        })
        .collect();
    (StatusCode::OK, Json(CapabilitiesResponse { ok: true, count: items.len(), items }))
}

#[derive(Debug, Deserialize, Default)]
pub struct MetricsQueryParams {
    #[allow(dead_code)] // 预留：支持 1m/5m/1h 滑动窗口（后续按 window 值过滤 ring buffer）
    #[serde(default)] pub window: Option<String>,
}

pub async fn metrics_handler(
    State(state): State<Arc<AiEngineState>>,
    Query(_q): Query<MetricsQueryParams>,
) -> impl IntoResponse {
    let sidecar = state.sidecar.metrics.snapshot();
    let mut p95 = BTreeMap::new();
    p95.insert("process".to_string(), state.stats.p95());
    let r = EngineMetricsResponse {
        ok: true,
        requests_total: state.stats.requests_total.load(Ordering::Relaxed),
        ai_hits: state.stats.ai_hits.load(Ordering::Relaxed),
        local_hits: state.stats.local_hits.load(Ordering::Relaxed),
        hybrid_hits: state.stats.hybrid_hits.load(Ordering::Relaxed),
        degrade_hits: state.stats.degrade_hits.load(Ordering::Relaxed),
        sidecar,
        p95_latency_ms: p95,
    };
    (StatusCode::OK, Json(r))
}

// 允许 process_handler 调用 agent 的 output 字段作为 summary。
trait AgentResultSummary { fn summary(&self) -> Option<String>; }
impl AgentResultSummary for ai_agent::engine::EngineResult {
    fn summary(&self) -> Option<String> {
        self.output.clone().or_else(|| {
            if self.success { Some(format!("AI ok: steps={}", self.steps_executed)) } else { None }
        })
    }
}
