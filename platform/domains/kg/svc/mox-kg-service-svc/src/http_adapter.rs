//! KG/AI HTTP 适配层（6 KG 接口真实桥接 + 4 AI 引擎接口桩）
//!
//! ## 依赖特性
//! 需启用 feature = "http-adapter" 才会构建：
//! - 引入 `axum` + `mox-framework`（企业级骨架）
//! - 提供 `build_kg_ai_router()`：返回 axum Router，可挂入 gateway
//! - 内嵌 in-memory `KnowledgeGraph`（demo 数据自动注入：6 节点 8 边 P0-P12 业务链）
//!
//! ## 6 KG 接口（100% 桥接 `mox-kg-algo-core` 新算法 API）
//! 1. GET /kg/v1/neighborhood       → KnowledgeGraph::neighborhood_subgraph
//! 2. GET /kg/v1/path               → KnowledgeGraph::find_paths (Yen's k-shortest)
//! 3. GET /kg/v1/shortest-path      → KnowledgeGraph::shortest_path (Dijkstra)
//! 4. GET /kg/v1/centrality         → KnowledgeGraph::centrality_metrics (4指标 + 公式)
//! 5. GET /kg/v1/communities        → KnowledgeGraph::detect_communities (CNM 模块度)
//! 6. GET /kg/v1/stats              → KnowledgeGraph::stats (含密度解读 + 公式文档)
//!
//! ## 4 AI 引擎接口（与归一化总纲 §AIS·AI 对齐，路由桩）
//! 1. POST /ai/engine/process       → 自动意图识别 → 能力路由
//! 2. POST /ai/engine/analyze       → 显式能力执行
//! 3. GET  /ai/engine/capabilities  → 能力矩阵自描述（7 类基准任务）
//! 4. GET  /ai/engine/metrics       → 成功率/降级率/延迟指标
#![cfg(feature = "http-adapter")]

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use mox_framework::FrameworkResult;
use mox_kg_algo_core::{
    CentralityMetrics, Community, KnowledgeGraph, KnowledgeGraphBuilder, KnowledgeEdge,
    NeighborhoodResult, PathResult, GraphStats,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// 共享状态：线程安全内嵌 in-memory 知识图谱（demo 自动注入 P0-P12 业务链）
// ============================================================================
pub struct KgAiState {
    pub graph: Arc<RwLock<KnowledgeGraph>>,
    pub started_unix_ms: i64,
}

impl KgAiState {
    pub fn new_demo() -> Self {
        let mut g = KnowledgeGraphBuilder::new()
            // 6 类核心实体（跨阶段可追溯链的最小业务示例）
            .add_node("P0-REQ-001",  "需求收集·考勤系统", "Requirement")
            .add_node("P2-ARCH-001", "架构设计·微服务", "Design")
            .add_node("P3-UI-001",   "UI设计·考勤页",   "UIDesign")
            .add_node("P4-CODE-001", "代码·考勤service","Code")
            .add_node("P8-TEST-001", "测试报告·SIT",   "TestReport")
            .add_node("P10-RUN-001", "运行·生产v1.2",   "Deployment")
            // 8 条关系（形成 P0→P2→P3→P4→P8→P10 主链路 + 2 条回环反馈）
            .add_edge_typed("P0-REQ-001",  "P2-ARCH-001", 1.0, "derive")
            .add_edge_typed("P2-ARCH-001", "P3-UI-001",   1.0, "derive")
            .add_edge_typed("P3-UI-001",   "P4-CODE-001", 1.0, "derive")
            .add_edge_typed("P4-CODE-001", "P8-TEST-001", 1.0, "verify")
            .add_edge_typed("P8-TEST-001", "P10-RUN-001", 1.0, "promote")
            .add_edge_typed("P8-TEST-001", "P4-CODE-001", 0.8, "bug_fix") // 测试→代码 反馈
            .add_edge_typed("P10-RUN-001", "P2-ARCH-001", 0.6, "refactor") // 运维→架构 反馈
            .add_edge_typed("P4-CODE-001", "P2-ARCH-001", 0.4, "tech_debt")// 技术债务回溯
            .build();

        // 补双向展开（让无向语义算法在 DiGraph 上正确：度/介数/紧密/社区 统一）
        let extra_edges: Vec<KnowledgeEdge> = g
            .edges()
            .iter()
            .filter(|e| e.source != e.target)
            .map(|e| KnowledgeEdge {
                source: e.target.clone(),
                target: e.source.clone(),
                weight: e.weight,
                relation_type: format!("rev_{}", e.relation_type),
                properties: json!({}),
            })
            .collect();
        for e in extra_edges {
            let _ = g.add_edge(e);
        }

        Self {
            graph: Arc::new(RwLock::new(g)),
            started_unix_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

// ============================================================================
// Query Params
// ============================================================================
#[derive(Debug, Deserialize)]
pub struct NeighborhoodQuery {
    pub center: String,
    #[serde(default = "d2")]
    pub depth: usize,
    #[serde(default = "n500")]
    pub limit: usize,
}
fn d2() -> usize { 2 }
fn n500() -> usize { 500 }

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    pub source: String,
    pub target: String,
    #[serde(default = "k3")]
    pub k: usize,
}
fn k3() -> usize { 3 }

#[derive(Debug, Deserialize)]
pub struct CommunityQuery {
    #[serde(default = "iter100")]
    pub iterations: usize,
}
fn iter100() -> usize { 100 }

// ============================================================================
// 统一包装：附加「调用路径说明 + 算法公式」
// ============================================================================
#[derive(Debug, Serialize)]
struct ApiEnvelope<T> {
    ok: bool,
    elapsed_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'static str>,
    data: T,
}

fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

// ============================================================================
// 6 KG 接口·真实 handler
// ============================================================================
async fn kg_neighborhood(
    State(s): State<Arc<KgAiState>>,
    Query(q): Query<NeighborhoodQuery>,
) -> FrameworkResult<Json<ApiEnvelope<NeighborhoodResult>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.neighborhood_subgraph(&q.center, q.depth, q.limit)
        .map_err(|e| mox_framework::FrameworkError::not_found(format!("{}", e)))?;
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("BFS hop=depth 双向(入+出)扩展，Cytoscape.js 兼容 nodes+edges 结构"),
        data,
    }))
}

async fn kg_find_paths(
    State(s): State<Arc<KgAiState>>,
    Query(q): Query<PathQuery>,
) -> FrameworkResult<Json<ApiEnvelope<Vec<PathResult>>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.find_paths(&q.source, &q.target, q.k)
        .map_err(|e| mox_framework::FrameworkError::not_found(format!("{}", e)))?;
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("Yen's k-最短路径：第1条Dijkstra，后续按偏离点禁边跑Dijkstra，按总权升序"),
        data,
    }))
}

async fn kg_shortest_path(
    State(s): State<Arc<KgAiState>>,
    Query(q): Query<PathQuery>,
) -> FrameworkResult<Json<ApiEnvelope<Option<PathResult>>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.shortest_path(&q.source, &q.target)
        .map_err(|e| mox_framework::FrameworkError::not_found(format!("{}", e)))?;
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("Dijkstra 加权最短路（权=边权重），反向 predecessor 数组回溯路径"),
        data,
    }))
}

async fn kg_centrality(
    State(s): State<Arc<KgAiState>>,
) -> FrameworkResult<Json<ApiEnvelope<CentralityMetrics>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.centrality_metrics();
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("4指标：度中心性 C_D/介数 Brandes C_B/紧密 Harmonic C_H/PageRank PR；公式含于人读字段返回"),
        data,
    }))
}

async fn kg_communities(
    State(s): State<Arc<KgAiState>>,
    Query(q): Query<CommunityQuery>,
) -> FrameworkResult<Json<ApiEnvelope<Vec<Community>>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.detect_communities(q.iterations);
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("CNM 模块度贪心凝聚：初始每节点一社区，反复合并 ΔQ 最大的相邻社区对；确定性平局=字典序最小"),
        data,
    }))
}

async fn kg_stats(
    State(s): State<Arc<KgAiState>>,
) -> FrameworkResult<Json<ApiEnvelope<GraphStats>>> {
    let t0 = now_ms();
    let g = s.graph.read();
    let data = g.stats();
    Ok(Json(ApiEnvelope {
        ok: true,
        elapsed_ms: now_ms() - t0,
        note: Some("密度解读等级：0=稀疏(<20%) 1=中等(20~50%) 2=高度稠密(>50%)；centrality_formulas 含 5 大算法人读公式"),
        data,
    }))
}

// ============================================================================
// 4 AI 引擎接口·桩（说明后续如何桥接 mox-ai-intent-core + mox-ai-orchestrator-svc）
// ============================================================================
#[derive(Debug, Deserialize)]
struct AiProcessReq {
    #[serde(default)]
    query: String,
    #[serde(default)]
    context: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AiAnalyzeReq {
    capability: String,
    #[serde(default)]
    payload: serde_json::Value,
}

async fn ai_process(
    State(_s): State<Arc<KgAiState>>,
    Json(req): Json<AiProcessReq>,
) -> FrameworkResult<Json<serde_json::Value>> {
    // 未来挂接点：
    //   1) mox_ai_intent_core::classify_intent(req.query.as_bytes(), rules)
    //   2) 个性化 PageRank(d=0.85, 30 轮) → 取 top 能力
    //   3) 调 mox-ai-expert-svc 打分联盟匹配
    //   4) 执行 + 审计链落 lamport 块
    Ok(Json(json!({
        "ok": true,
        "stub": true,
        "pipeline": [
            "意图识别 A5: Activation Diffusion PPR",
            "能力路由: 7 类能力(数学/逻辑/知识/代码/中文/时效性/指令)",
            "专家联盟匹配: score_alliance_candidates",
            "执行: capability_driver(payload)",
            "审计链: DengBaoHashChain.append",
        ],
        "echo": {
            "query_len": req.query.len(),
            "context_is_object": req.context.is_object(),
        }
    })))
}

async fn ai_analyze(
    State(_s): State<Arc<KgAiState>>,
    Json(req): Json<AiAnalyzeReq>,
) -> FrameworkResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "ok": true,
        "stub": true,
        "capability": req.capability,
        "note": "显式能力执行：不做意图识别，直接按 capability 字段派发",
        "payload_keys": req.payload.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
    })))
}

async fn ai_capabilities(
    State(_s): State<Arc<KgAiState>>,
) -> FrameworkResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "ok": true,
        "capabilities": [
            {"id":"math",     "name":"数学推理",   "benchmark":"GSM8K / AIME"},
            {"id":"logic",    "name":"逻辑推理",   "benchmark":"LogicalDeduction"},
            {"id":"knowledge","name":"知识问答",   "benchmark":"HotpotQA 多跳"},
            {"id":"code",     "name":"代码生成",   "benchmark":"HumanEval+ MBPP"},
            {"id":"chinese",  "name":"中文理解",   "benchmark":"CMMLU / C-Eval"},
            {"id":"timely",   "name":"时效性检索", "benchmark":"FreshQA"},
            {"id":"follow",   "name":"指令跟随",   "benchmark":"IFEval"},
        ],
        "optimization": "CEM 交叉熵 (σ̄<0.06 或 3 轮无改进停止)",
        "multi_objective_score": "0.55×quality + 0.20×speed + 0.10×token_efficiency + 0.15×stability",
    })))
}

async fn ai_metrics(
    State(s): State<Arc<KgAiState>>,
) -> FrameworkResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "ok": true,
        "started_unix_ms": s.started_unix_ms,
        "gauges": ["success_rate", "degrade_rate", "latency_p50_ms", "latency_p99_ms", "tokens_input_total", "tokens_output_total"],
        "slo_targets": {
            "success_rate_min": 0.985,
            "degrade_rate_max": 0.05,
            "latency_p99_max_ms": 3000,
        },
        "note": "真实环境从 Prometheus Registry 采集，此处为 schema 桩",
    })))
}

// ============================================================================
// 路由装配入口
// ============================================================================
pub fn build_kg_ai_router() -> Router {
    let state = Arc::new(KgAiState::new_demo());
    Router::new()
        // ===== KG 6 接口（真实桥接 algo-core） =====
        .route("/kg/v1/neighborhood",  get(kg_neighborhood))
        .route("/kg/v1/path",          get(kg_find_paths))
        .route("/kg/v1/shortest-path", get(kg_shortest_path))
        .route("/kg/v1/centrality",    get(kg_centrality))
        .route("/kg/v1/communities",   get(kg_communities))
        .route("/kg/v1/stats",         get(kg_stats))
        // ===== AI 引擎 4 接口（桩，挂接说明附于内部） =====
        .route("/ai/engine/process",      post(ai_process))
        .route("/ai/engine/analyze",      post(ai_analyze))
        .route("/ai/engine/capabilities", get(ai_capabilities))
        .route("/ai/engine/metrics",      get(ai_metrics))
        .with_state(state)
}
