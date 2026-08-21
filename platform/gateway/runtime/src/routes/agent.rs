//! AI Agent 路由
//!
//! 挂载 `/api/agent/*` 下的 AI Agent 引擎任务端点。

use crate::handlers::agent;
use ai_agent::AIAgent;
use axum::{
    routing::post,
    Router,
};
use std::sync::Arc;

pub fn agent_routes() -> Router<Arc<AIAgent>> {
    Router::new()
        .route("/run", post(agent::run_engine_task_handler))
}