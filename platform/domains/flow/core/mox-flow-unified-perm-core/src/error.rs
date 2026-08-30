// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 权限错误类型

use thiserror::Error;

/// 权限系统结果类型
pub type PermResult<T> = Result<T, PermError>;

/// 权限错误
#[derive(Debug, Error)]
pub enum PermError {
    /// 访问被拒绝
    #[error("access denied: {0}")]
    AccessDenied(String),

    /// 未认证
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),

    /// 未找到
    #[error("not found: {0}")]
    NotFound(String),

    /// 已存在
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// 租户不匹配
    #[error("tenant mismatch: {0}")]
    TenantMismatch(String),

    /// 无效参数
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// 策略错误
    #[error("policy error: {0}")]
    PolicyError(String),

    /// Token 无效
    #[error("invalid token: {0}")]
    InvalidToken(String),

    /// Token 过期
    #[error("token expired")]
    TokenExpired,

    /// SSO 错误
    #[error("sso error: {0}")]
    SsoError(String),

    /// 内部错误
    #[error("internal error: {0}")]
    InternalError(String),
}

impl PermError {
    /// 获取错误码
    pub fn code(&self) -> &'static str {
        match self {
            PermError::AccessDenied(_) => "ACCESS_DENIED",
            PermError::Unauthenticated(_) => "UNAUTHENTICATED",
            PermError::NotFound(_) => "NOT_FOUND",
            PermError::AlreadyExists(_) => "ALREADY_EXISTS",
            PermError::TenantMismatch(_) => "TENANT_MISMATCH",
            PermError::InvalidArgument(_) => "INVALID_ARGUMENT",
            PermError::PolicyError(_) => "POLICY_ERROR",
            PermError::InvalidToken(_) => "INVALID_TOKEN",
            PermError::TokenExpired => "TOKEN_EXPIRED",
            PermError::SsoError(_) => "SSO_ERROR",
            PermError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            PermError::AccessDenied("test".to_string()).code(),
            "ACCESS_DENIED"
        );
        assert_eq!(
            PermError::Unauthenticated("test".to_string()).code(),
            "UNAUTHENTICATED"
        );
        assert_eq!(
            PermError::NotFound("test".to_string()).code(),
            "NOT_FOUND"
        );
        assert_eq!(PermError::TokenExpired.code(), "TOKEN_EXPIRED");
        assert_eq!(
            PermError::InternalError("test".to_string()).code(),
            "INTERNAL_ERROR"
        );
    }

    #[test]
    fn test_error_display() {
        let err = PermError::AccessDenied("no permission".to_string());
        assert!(err.to_string().contains("access denied"));
    }
}
