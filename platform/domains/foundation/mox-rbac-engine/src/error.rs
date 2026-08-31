// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RBAC 引擎错误类型
//!
//! 错误码段：US（User 域）· 模块 03（RBAC 引擎）
//! - US03001 ~ US03099 预留为 RBAC 引擎使用

use std::fmt;

/// RBAC 引擎错误
#[derive(Debug)]
pub enum RbacError {
    /// 角色未定义
    RoleNotFound(String),
    /// 循环继承检测
    CyclicInheritance(String),
    /// 策略加载失败
    PolicyLoadFailed(String),
    /// 审计写入失败（不阻断权限检查，仅记录）
    AuditWriteFailed(String),
    /// 策略初始化失败
    PolicyInitFailed(String),
}

impl RbacError {
    /// 错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::RoleNotFound(_) => "US03001",
            Self::CyclicInheritance(_) => "US03002",
            Self::PolicyLoadFailed(_) => "US03003",
            Self::AuditWriteFailed(_) => "US03004",
            Self::PolicyInitFailed(_) => "US03005",
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            Self::RoleNotFound(_) => 404,
            Self::CyclicInheritance(_) => 500,
            Self::PolicyLoadFailed(_) => 500,
            Self::AuditWriteFailed(_) => 500,
            Self::PolicyInitFailed(_) => 500,
        }
    }
}

impl fmt::Display for RbacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoleNotFound(r) => write!(f, "[{}] RBAC: role not found '{}'", self.code(), r),
            Self::CyclicInheritance(r) => write!(
                f,
                "[{}] RBAC: cyclic inheritance detected in role '{}'",
                self.code(),
                r
            ),
            Self::PolicyLoadFailed(msg) => {
                write!(f, "[{}] RBAC: policy load failed — {}", self.code(), msg)
            }
            Self::AuditWriteFailed(msg) => write!(
                f,
                "[{}] RBAC: audit write failed — {} (non-blocking)",
                self.code(),
                msg
            ),
            Self::PolicyInitFailed(msg) => {
                write!(f, "[{}] RBAC: policy init failed — {}", self.code(), msg)
            }
        }
    }
}

impl std::error::Error for RbacError {}

// ── mox-error 集成（可选 feature） ──────────────────────────────

#[cfg(feature = "mox-error")]
impl From<RbacError> for mox_error::MoxError {
    fn from(err: RbacError) -> Self {
        use mox_error::{ErrorDomain, ErrorLevel, MoxError};

        let (module, seq, msg, level, http) = match &err {
            RbacError::RoleNotFound(r) => {
                (03u8, 001u16, format!("角色不存在: {r}"), ErrorLevel::Warning, 404u16)
            }
            RbacError::CyclicInheritance(r) => (
                03,
                002,
                format!("角色循环继承: {r}"),
                ErrorLevel::Error,
                500,
            ),
            RbacError::PolicyLoadFailed(msg) => {
                (03, 003, format!("策略加载失败: {msg}"), ErrorLevel::Error, 500)
            }
            RbacError::AuditWriteFailed(msg) => (
                03,
                004,
                format!("审计写入失败: {msg}"),
                ErrorLevel::Warning,
                500,
            ),
            RbacError::PolicyInitFailed(msg) => (
                03,
                005,
                format!("策略初始化失败: {msg}"),
                ErrorLevel::Error,
                500,
            ),
        };

        MoxError::new(ErrorDomain::User, module, seq, msg, level, http)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_unique() {
        let errors = [
            RbacError::RoleNotFound("test".into()),
            RbacError::CyclicInheritance("test".into()),
            RbacError::PolicyLoadFailed("test".into()),
            RbacError::AuditWriteFailed("test".into()),
            RbacError::PolicyInitFailed("test".into()),
        ];
        let mut codes = Vec::new();
        for e in &errors {
            let code = e.code();
            assert!(!codes.contains(&code), "duplicate code: {code}");
            codes.push(code);
        }
    }

    #[test]
    fn error_code_format() {
        let err = RbacError::RoleNotFound("admin".into());
        assert_eq!(err.code(), "US03001");
        assert_eq!(err.http_status(), 404);
    }

    #[test]
    fn display_contains_code() {
        let err = RbacError::RoleNotFound("editor".into());
        let display = format!("{err}");
        assert!(display.contains("US03001"));
        assert!(display.contains("editor"));
    }

    #[cfg(feature = "mox-error")]
    #[test]
    fn into_mox_error_preserves_code() {
        let err = RbacError::RoleNotFound("test".into());
        let mox_err: mox_error::MoxError = err.into();
        assert_eq!(mox_err.code, "US03001");
        assert_eq!(mox_err.http_status, 404);
    }
}
