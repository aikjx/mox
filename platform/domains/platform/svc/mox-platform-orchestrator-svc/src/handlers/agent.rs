//! AI Agent 引擎任务端点
//!
//! 提供 AI Agent 引擎任务执行能力：
//!
//! - `run_engine_task` — 接收任务描述，调用 AI Agent 引擎执行并返回结果

use mox_ai_agent_svc::engine::EngineResult;
use mox_ai_agent_svc::AIAgent;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct AgentRunRequest {
    pub task: String,
}

#[derive(Debug, Serialize)]
pub struct AgentRunResponse {
    pub success: bool,
    #[serde(flatten)]
    pub result: Option<EngineResult>,
    pub error: Option<String>,
}

pub async fn run_engine_task_handler(
    State(agent): State<Arc<AIAgent>>,
    Json(req): Json<AgentRunRequest>,
) -> (StatusCode, Json<AgentRunResponse>) {
    match agent.run_engine_task(req.task).await {
        Ok(result) => {
            let success = result.success;
            (
                StatusCode::OK,
                Json(AgentRunResponse {
                    success,
                    result: Some(result),
                    error: None,
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AgentRunResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}
