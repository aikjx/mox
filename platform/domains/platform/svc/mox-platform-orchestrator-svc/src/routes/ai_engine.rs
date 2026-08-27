// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! /ai/engine/* 路由树挂载
//!
//! AC-10 语义（项目记忆）：静态路径优先于参数化路径。
//! 所有已注册路由都是静态（无 ':' 段），注册顺序不改变静态优先级。
//!
//! ## T6/T9 指标出口约定
//!
//! GET /ai/engine/metrics 返回 EngineMetricsResponse JSON，T9 追加了两个一级键：
//! - "alliance":   专家联盟 6 项占位指标（invocations_total / gate_*_rate_7d / p99_latency_ms）
//! - "subservers": 子服务注册表（见 crate::subservers::registered_subservers，FR-GW-05）
//!
//! 上述两个字面键名 "alliance" 与 "subservers" 均为前端运维面板契约的一部分，
//! 任何重命名都需要同步 ChatView/OPS-Dashboard 侧（AIS 接口稳定性 1.5）。

use crate::handlers::ai_engine::{
    analyze_handler, capabilities_handler, metrics_handler, process_handler,
    workflow_execute_handler, AiEngineState,
};
use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        Json,
    },
    routing::{get, post},
    Router,
};
use futures::stream;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub fn ai_engine_routes(state: Arc<AiEngineState>) -> Router {
    Router::new()
        // 4 段静态长路径优先：与 process/analyze 前缀比较时按静态段计数并列，但
        // workflow_execute 不影响其他路由（完全不同的 path 段）。
        .route("/workflow/execute", post(workflow_execute_handler))
        .route(
            "/workflow/templates",
            get({
                let s = state.clone();
                move || async move {
                    // 与 Node 端 /ai/engine/workflow/templates 对齐：透传
                    let resp = s
                        .sidecar
                        .get_passthrough("ai/engine/workflow/templates")
                        .await
                        .unwrap_or_else(|e| {
                            serde_json::json!({
                                "ok": false, "count": 0, "templates": [],
                                "error": format!("sidecar: {e}"),
                            })
                        });
                    axum::response::Json(resp)
                }
            }),
        )
        // 原有四端点（SPEC-6 基线）：T13 新路由不覆盖其语义
        .route("/process", post(process_handler))
        .route("/analyze", post(analyze_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/metrics", get(metrics_handler))
        // ---- T6 专家联盟扩展 ----
        .route("/alliance/full", post(alliance_full_sse_handler))
        .route("/alliance/capabilities", get(alliance_capabilities_handler))
        .route("/alliance/report", get(alliance_report_handler))
        .with_state(state)
}

// ========================
// 专家联盟扩展（T6 新增）
// ========================

/// POST /ai/engine/alliance/full 请求体（前端 ChatView 直接传）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllianceFullRequest {
    pub query: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    #[serde(default)]
    pub enable_llm_debate: bool,
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
    #[serde(default = "default_true")]
    pub retry_on_c: bool,
}
fn default_true() -> bool { true }
fn default_team_size_4() -> usize { 4 }

/// GET /ai/engine/alliance/capabilities 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceCapabilities {
    pub version: &'static str,
    pub phases: [&'static str; 7],
    pub intent_classes_7: [&'static str; 7],
    pub dimensions_14: Vec<(&'static str, i32)>,
    pub hc_params: BTreeMap<&'static str, String>,
    pub audit_events_7: [&'static str; 7],
    pub health: &'static str,
}

/// GET /alliance/capabilities（FR-GW-02）
pub async fn alliance_capabilities_handler(
    State(_state): State<Arc<AiEngineState>>,
) -> Json<AllianceCapabilities> {
    use mox_ai_expert_svc::alliance::constants as c;
    use mox_ai_expert_svc::ir::Dimension;

    // 14 维 + 优先级
    let all_14 = [
        (Dimension::Permission,        "permission"),
        (Dimension::Security,          "security"),
        (Dimension::Architecture,      "architecture"),
        (Dimension::SecurityCode,      "security_code"),
        (Dimension::Resource,          "resource"),
        (Dimension::Data,              "data"),
        (Dimension::CodeQuality,       "code_quality"),
        (Dimension::Performance,       "performance"),
        (Dimension::Algorithm,         "algorithm"),
        (Dimension::Testing,           "testing"),
        (Dimension::Business,          "business"),
        (Dimension::Documentation,     "documentation"),
        (Dimension::Observability,     "observability"),
        (Dimension::Maintainability,   "maintainability"),
    ];
    let dims: Vec<(&'static str, i32)> = all_14
        .iter()
        .map(|(d, name)| (*name, mox_ai_expert_svc::dim_priority(*d)))
        .collect();

    let mut hc = BTreeMap::new();
    hc.insert("HC-2.method",        c::SPREAD_METHOD.to_string());
    hc.insert("HC-2.damping",       format!("{:.2}", c::SPREAD_DAMPING));
    hc.insert("HC-2.rounds",        c::SPREAD_ROUNDS.to_string());
    hc.insert("HC-8.rrf_k",         c::RRF_K.to_string());
    hc.insert("HC-8.spread_weight", format!("{:.2}", c::SPREAD_WEIGHT));
    hc.insert("HC-8.gate_A",        format!("{:.2}", c::GATE_THRESHOLD_A));
    hc.insert("HC-8.gate_B",        format!("{:.2}", c::GATE_THRESHOLD_B));
    hc.insert("HC-8.gate_C",        format!("{:.2}", c::GATE_THRESHOLD_C));
    hc.insert("HC-8.formula",       c::QUALITY_FORMULA.to_string());
    hc.insert("EAF-4.3.timeout_s",  c::EXPERT_TIMEOUT_SECS.to_string());
    hc.insert("EAF-4.3.max_tokens", c::DEBATE_MAX_TOKENS_PER_ROUND.to_string());

    Json(AllianceCapabilities {
        version: "3.0.0-alliance",
        phases: c::PHASE_NAMES,
        intent_classes_7: c::INTENT_CLASSES,
        dimensions_14: dims,
        hc_params: hc,
        audit_events_7: mox_ai_expert_svc::alliance::gate::AUDIT_EVENTS_7,
        health: "GET /ai/engine/alliance/capabilities 返回 200 即健康",
    })
}

/// POST /alliance/full — SSE 流式 7 阶段（FR-GW-01）
pub async fn alliance_full_sse_handler(
    State(_state): State<Arc<AiEngineState>>,
    Json(req_in): Json<AllianceFullRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    use mox_ai_expert_svc::alliance::{AllianceEngine, AllianceOptions, AllianceRequest};

    let engine = AllianceEngine::new();
    let options = AllianceOptions {
        enable_llm_debate: req_in.enable_llm_debate,
        retry_on_c: req_in.retry_on_c,
        team_size: req_in.team_size,
        enable_spread: true,
    };
    let req = AllianceRequest {
        query: req_in.query,
        session_id: req_in.session_id,
        idempotency_key: req_in.idempotency_key,
        context: req_in.context,
        options,
    };

    // 非流式：先全量跑完（Rust 本地管线 <50ms，SSE 做逐步推送效果）
    let result = engine.run_full_analysis(req).await;

    // 逐帧发 SSE
    let frames: Vec<Event> = match result {
        Ok(events) => {
            let mut f = Vec::with_capacity(events.len() + 1);
            for ev in events {
                let payload = serde_json::json!({
                    "phase": ev.phase.name(),
                    "phase_index": ev.phase.index(),
                    "payload": ev.payload,
                    "trace_id": ev.trace_id.to_string(),
                    "latency_ms": ev.latency_ms,
                    "ts": ev.ts.to_rfc3339(),
                    "degraded": ev.degraded.unwrap_or(false),
                    "degrade_reason": ev.degrade_reason,
                });
                f.push(Event::default().event(ev.phase.name()).data(
                    serde_json::to_string(&payload).unwrap_or_default(),
                ));
            }
            f.push(Event::default().event("close").data("[DONE]"));
            f
        }
        Err(e) => {
            let err = serde_json::json!({
                "error": format!("{}", e),
                "phase": "error",
                "phase_index": 99,
            });
            vec![
                Event::default()
                    .event("error")
                    .data(serde_json::to_string(&err).unwrap_or_default()),
                Event::default().event("close").data("[DONE]"),
            ]
        }
    };
    // 立即发射所有帧；SSE keep-alive heartbeat 交给 axum
    let delayed: Vec<Result<Event, Infallible>> = frames.into_iter().map(Ok).collect();
    Sse::new(stream::iter(delayed)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-alliance"),
    )
}

/// GET /alliance/report?trace_id=xxx（FR-GW-03）
/// 若 trace_id 为空，返回"latest 1 条"的骨架；当前版本为占位实现（Rust 单测/前端联调够用）
#[derive(Debug, Clone, Deserialize)]
pub struct ReportQuery {
    #[serde(default)]
    pub trace_id: Option<String>,
}
pub async fn alliance_report_handler(
    State(_state): State<Arc<AiEngineState>>,
    Query(q): Query<ReportQuery>,
) -> Json<serde_json::Value> {
    use mox_ai_expert_svc::alliance::constants as c;
    let tid = q
        .trace_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    Json(serde_json::json!({
        "trace_id": tid,
        "quality_formula": c::QUALITY_FORMULA,
        "report_version": "3.0.0-alliance",
        "note": "完整报告 = Done 阶段 payload + Gate 结构；生产版可从审计 DB 查询 7 事件回灌",
        "hc_snapshot": {
            "spread": {"method": c::SPREAD_METHOD, "d": c::SPREAD_DAMPING, "rounds": c::SPREAD_ROUNDS},
            "rrf_k": c::RRF_K, "spread_weight": c::SPREAD_WEIGHT,
            "gate": {"A": c::GATE_THRESHOLD_A, "B": c::GATE_THRESHOLD_B, "C": c::GATE_THRESHOLD_C},
        },
        "status": "ok",
    }))
}
