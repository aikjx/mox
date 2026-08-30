// Copyright (c) 2026 璇玑 RelGraph · AI对话全维自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 错误类型

use thiserror::Error;

pub type AiResult<T> = Result<T, AiError>;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("execution error: {0}")]
    ExecutionError(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("task failed: {0}")]
    TaskFailed(String),

    #[error("agent error: {0}")]
    AgentError(String),

    #[error("tool error: {0}")]
    ToolError(String),

    #[error("timeout")]
    Timeout,

    #[error("internal error: {0}")]
    InternalError(String),
}

impl AiError {
    pub fn code(&self) -> &'static str {
        match self {
            AiError::NotFound(_) => "NOT_FOUND",
            AiError::AlreadyExists(_) => "ALREADY_EXISTS",
            AiError::ExecutionError(_) => "EXECUTION_ERROR",
            AiError::InvalidInput(_) => "INVALID_INPUT",
            AiError::TaskFailed(_) => "TASK_FAILED",
            AiError::AgentError(_) => "AGENT_ERROR",
            AiError::ToolError(_) => "TOOL_ERROR",
            AiError::Timeout => "TIMEOUT",
            AiError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}
