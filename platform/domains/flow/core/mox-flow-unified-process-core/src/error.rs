// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 流程算法错误类型

use thiserror::Error;

/// 流程结果类型
pub type ProcessResult<T> = Result<T, ProcessError>;

/// 流程错误
#[derive(Debug, Error)]
pub enum ProcessError {
    /// 流程未找到
    #[error("process not found: {0}")]
    NotFound(String),

    /// 流程已存在
    #[error("process already exists: {0}")]
    AlreadyExists(String),

    /// 执行错误
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// 规则错误
    #[error("rule error: {0}")]
    RuleError(String),

    /// 无效配置
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// 流程超时
    #[error("process timeout")]
    Timeout,

    /// 流程被取消
    #[error("process cancelled")]
    Cancelled,

    /// 算法调用错误
    #[error("algorithm error: {0}")]
    AlgorithmError(String),

    /// 内部错误
    #[error("internal error: {0}")]
    InternalError(String),
}

impl ProcessError {
    pub fn code(&self) -> &'static str {
        match self {
            ProcessError::NotFound(_) => "NOT_FOUND",
            ProcessError::AlreadyExists(_) => "ALREADY_EXISTS",
            ProcessError::ExecutionError(_) => "EXECUTION_ERROR",
            ProcessError::RuleError(_) => "RULE_ERROR",
            ProcessError::InvalidConfig(_) => "INVALID_CONFIG",
            ProcessError::Timeout => "TIMEOUT",
            ProcessError::Cancelled => "CANCELLED",
            ProcessError::AlgorithmError(_) => "ALGORITHM_ERROR",
            ProcessError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ProcessError::NotFound("x".to_string()).code(), "NOT_FOUND");
        assert_eq!(ProcessError::Timeout.code(), "TIMEOUT");
        assert_eq!(ProcessError::Cancelled.code(), "CANCELLED");
    }
}
