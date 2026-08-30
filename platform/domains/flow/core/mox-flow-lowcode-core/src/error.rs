// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 低代码错误类型

use thiserror::Error;

/// 低代码结果类型
pub type LowcodeResult<T> = Result<T, LowcodeError>;

/// 低代码错误
#[derive(Debug, Error)]
pub enum LowcodeError {
    /// 验证失败
    #[error("validation failed: {0}")]
    ValidationError(String),

    /// 未找到
    #[error("not found: {0}")]
    NotFound(String),

    /// 已存在
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// 无效配置
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// 脚本错误
    #[error("script error: {0}")]
    ScriptError(String),

    /// 表达式错误
    #[error("expression error: {0}")]
    ExpressionError(String),

    /// 类型错误
    #[error("type error: {0}")]
    TypeError(String),

    /// 内部错误
    #[error("internal error: {0}")]
    InternalError(String),
}

impl LowcodeError {
    /// 获取错误码
    pub fn code(&self) -> &'static str {
        match self {
            LowcodeError::ValidationError(_) => "VALIDATION_ERROR",
            LowcodeError::NotFound(_) => "NOT_FOUND",
            LowcodeError::AlreadyExists(_) => "ALREADY_EXISTS",
            LowcodeError::InvalidConfig(_) => "INVALID_CONFIG",
            LowcodeError::ScriptError(_) => "SCRIPT_ERROR",
            LowcodeError::ExpressionError(_) => "EXPRESSION_ERROR",
            LowcodeError::TypeError(_) => "TYPE_ERROR",
            LowcodeError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            LowcodeError::ValidationError("test".to_string()).code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(
            LowcodeError::NotFound("test".to_string()).code(),
            "NOT_FOUND"
        );
        assert_eq!(
            LowcodeError::ScriptError("test".to_string()).code(),
            "SCRIPT_ERROR"
        );
    }
}
