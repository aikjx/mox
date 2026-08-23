//! /ai/engine/* 路由树挂载
//!
//! AC-10 语义（项目记忆）：静态路径优先于参数化路径。
//! 所有已注册路由都是静态（无 ':' 段），注册顺序不改变静态优先级。

use crate::handlers::ai_engine::{
    AiEngineState, analyze_handler, capabilities_handler, metrics_handler, process_handler,
    workflow_execute_handler,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn ai_engine_routes(state: Arc<AiEngineState>) -> Router {
    Router::new()
        // 4 段静态长路径优先：与 process/analyze 前缀比较时按静态段计数并列，但
        // workflow_execute 不影响其他路由（完全不同的 path 段）。
        .route("/workflow/execute", post(workflow_execute_handler))
        .route("/workflow/templates", get({
            let s = state.clone();
            move || async move {
                // 与 Node 端 /ai/engine/workflow/templates 对齐：透传
                let resp = s.sidecar.get_passthrough("ai/engine/workflow/templates").await
                    .unwrap_or_else(|e| serde_json::json!({
                        "ok": false, "count": 0, "templates": [],
                        "error": format!("sidecar: {e}"),
                    }));
                axum::response::Json(resp)
            }
        }))
        // 原有四端点（SPEC-6 基线）：T13 新路由不覆盖其语义
        .route("/process", post(process_handler))
        .route("/analyze", post(analyze_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
