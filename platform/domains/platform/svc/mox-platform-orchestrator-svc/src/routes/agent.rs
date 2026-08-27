// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AI Agent 路由
//!
//! 挂载 `/api/agent/*` 下的 AI Agent 引擎任务端点。

use crate::handlers::agent;
use mox_ai_agent_svc::AIAgent;
use axum::{routing::post, Router};
use std::sync::Arc;

pub fn agent_routes() -> Router<Arc<AIAgent>> {
    Router::new().route("/run", post(agent::run_engine_task_handler))
}
