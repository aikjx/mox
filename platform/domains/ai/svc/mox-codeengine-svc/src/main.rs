// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! mox-codeengine-svc —— 全自研 AI 代码引擎 HTTP 服务（:3210）
//!
//! 以「开发专家联盟处理模式」管线对外提供出码服务（含 code agent 式自修复循环
//! 与 diff-first 增量补丁）：
//! - `POST /api/codeengine/run`   —— 需求文本 → 联盟诊断 → 裁决 → 自修复收敛 → 三证闸门 → 代码包（+补丁）
//! - `POST /api/codeengine/patch` —— 对基线文件应用 SEARCH/REPLACE 编辑块（确定性匹配，歧义拒绝）
//! - `GET  /api/codeengine/health` —— 健康检查（专家阵容 / 阶段清单 / 能力位）
//! - `GET  /api/codeengine/cases`  —— Learn 阶段经验库（交付率 / 平均收敛轮数）

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use mox_codeengine_core::{
    parse_blocks, apply_blocks, CodeEngine, EditBlock, EngineConfig, EngineResult, VerdictPolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct AppState {
    engine: Mutex<CodeEngine>,
}

#[derive(Deserialize)]
struct RunRequest {
    /// 需求文本（中文/英文，多步骤以 。；; 换行分隔）
    text: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    /// 自动注入合规护栏（默认 true）
    #[serde(default = "default_true")]
    auto_guard: bool,
    /// 裁决通过线（默认 0.75）
    #[serde(default)]
    threshold: Option<f64>,
    /// code agent 式自修复循环（默认 true）
    #[serde(default = "default_true")]
    self_repair: bool,
    /// 自修复最大轮数（默认 3）
    #[serde(default)]
    max_repair_rounds: Option<usize>,
    /// 基线文件（path → content）：同名交付物产出 SEARCH/REPLACE 增量补丁
    #[serde(default)]
    known_files: Option<BTreeMap<String, String>>,
    /// 是否随响应返回完整代码包（默认 true；false 只返回摘要）
    #[serde(default = "default_true")]
    with_bundle: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct RunResponse {
    delivered: bool,
    approved: bool,
    vetoed: bool,
    verdict_score: f64,
    rounds: usize,
    repairs: Vec<String>,
    gates: Vec<(String, bool, String)>,
    digest: String,
    events: Vec<serde_json::Value>,
    patches: Vec<EditBlock>,
    files: Option<Vec<serde_json::Value>>,
    result: Option<EngineResult>,
}

async fn run_handler(
    State(st): State<Arc<AppState>>,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    if req.text.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "text 不能为空".into()));
    }
    let cfg = EngineConfig {
        auto_guard: req.auto_guard,
        policy: VerdictPolicy {
            threshold: req.threshold.unwrap_or(0.75),
            ..Default::default()
        },
        self_repair: req.self_repair,
        max_repair_rounds: req.max_repair_rounds.unwrap_or(3).max(1),
        ..Default::default()
    };
    let mut guard = st
        .engine
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "引擎锁中毒".to_string()))?;
    guard.config = cfg;
    let (id, name) = (
        req.id.clone().unwrap_or_else(|| "api-req".into()),
        req.name.clone().unwrap_or_else(|| {
            let n: String = req.text.chars().take(20).collect();
            n
        }),
    );
    let result = match &req.known_files {
        Some(files) if !files.is_empty() => {
            guard.run_with_context(&id, &name, &req.text, files)
        }
        _ => guard.run_named(&id, &name, &req.text),
    };
    let files_json = if req.with_bundle {
        result.bundle.as_ref().map(|b| {
            b.files
                .iter()
                .map(|f| serde_json::json!({"path": f.path, "content": f.content}))
                .collect()
        })
    } else {
        None
    };
    let resp = RunResponse {
        delivered: result.delivered,
        approved: result.verdict.approved,
        vetoed: result.verdict.vetoed,
        verdict_score: result.verdict.score,
        rounds: result.rounds,
        repairs: result.repairs.clone(),
        gates: result
            .gates
            .checks
            .iter()
            .map(|c| (c.name.to_string(), c.passed, c.detail.clone()))
            .collect(),
        digest: result.digest(),
        events: result
            .events
            .iter()
            .map(|e| serde_json::json!({"stage": e.stage, "ok": e.ok, "note": e.note}))
            .collect(),
        patches: result.patches.clone(),
        files: files_json,
        result: req.with_bundle.then_some(result),
    };
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct PatchRequest {
    /// 基线文件 path → content
    base: BTreeMap<String, String>,
    /// 结构化编辑块（与 blocks_text 二选一，块可混用）
    #[serde(default)]
    blocks: Vec<EditBlock>,
    /// Aider 式 SEARCH/REPLACE 文本格式
    #[serde(default)]
    blocks_text: Option<String>,
}

async fn patch_handler(
    Json(req): Json<PatchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut blocks = req.blocks.clone();
    if let Some(text) = &req.blocks_text {
        match parse_blocks(text) {
            Ok(parsed) => blocks.extend(parsed),
            Err(e) => return Err((StatusCode::BAD_REQUEST, format!("编辑块解析失败: {e}"))),
        }
    }
    if blocks.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "未提供任何编辑块".into()));
    }
    let (files, report) = apply_blocks(&req.base, &blocks);
    Ok(Json(serde_json::json!({
        "ok": report.ok(),
        "applied": report.applied,
        "failed": report.failed,
        "results": report.results,
        "files": files,
    })))
}

#[derive(Serialize)]
struct HealthResponse {
    engine: &'static str,
    mode: &'static str,
    capabilities: Vec<&'static str>,
    stages: Vec<&'static str>,
    experts: Vec<(&'static str, &'static str)>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        engine: "mox-codeengine-core",
        mode: "dev-expert-alliance",
        capabilities: vec![
            "alliance-panel",
            "weighted-verdict",
            "auditor-veto",
            "triple-conservation-gates",
            "self-repair-loop",
            "diff-first-patching",
        ],
        stages: vec![
            "Intent", "Build", "Solve", "Team", "Diagnose", "Verdict", "Generate", "Gate",
            "Refine", "Deliver", "Learn",
        ],
        experts: vec![
            ("analyst", "需求分析师"),
            ("builder", "构建专家"),
            ("auditor", "合规审计师(一票否决)"),
            ("coordinator", "协调官"),
        ],
    })
}

async fn cases(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let guard = match st.engine.lock() {
        Ok(g) => g,
        Err(_) => return Json(serde_json::json!({"error": "engine poisoned"})),
    };
    let avg_rounds = if guard.book.cases.is_empty() {
        0.0
    } else {
        guard.book.cases.iter().map(|c| c.rounds).sum::<usize>() as f64
            / guard.book.cases.len() as f64
    };
    Json(serde_json::json!({
        "total": guard.book.cases.len(),
        "delivery_rate": guard.book.delivery_rate(),
        "avg_rounds_to_converge": avg_rounds,
        "cases": guard.book.cases,
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("MOX_CODEENGINE_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3210);
    let state = Arc::new(AppState::default());
    let app = Router::new()
        .route("/api/codeengine/run", post(run_handler))
        .route("/api/codeengine/patch", post(patch_handler))
        .route("/api/codeengine/health", get(health))
        .route("/api/codeengine/cases", get(cases))
        .with_state(state);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("mox-codeengine-svc listening on {addr} (dev-expert-alliance mode)");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use mox_codeengine_core::{CodeEngine, EngineConfig};

    #[test]
    fn engine_runs_in_alliance_mode_offline() {
        // 服务层冒烟：不依赖 HTTP，直接驱动引擎（自修复开）
        let mut engine = CodeEngine::default();
        let r = engine.run("读取 input.csv 文件；把解析结果写入 db.orders 表；导出报表 output.xlsx");
        assert!(r.delivered);
        assert!(r.verdict.approved);
    }

    #[test]
    fn engine_self_repair_converges() {
        let cfg = EngineConfig {
            auto_guard: false,
            self_repair: true,
            ..Default::default()
        };
        let mut engine = CodeEngine::new(cfg);
        let r = engine.run("查询 db.citizen_idcard 表并导出报表 output.xlsx");
        assert_eq!(r.rounds, 2);
        assert!(!r.repairs.is_empty());
        assert!(r.delivered);
    }
}
