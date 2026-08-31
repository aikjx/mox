// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RBAC 引擎错误类型
//!
//! 错误码段：PL（Platform 域）· 模块 05（RBAC 权限引擎）
//! - PL05001 ~ PL05099 预留为 RBAC 引擎使用
//!
//! > 注：PL04 已分配给 mox-pipeline-framework（管线框架），
//! > RBAC 引擎作为独立的平台基础模块，使用 PL05 模块号。

use std::fmt;

/// RBAC 引擎错误
#[derive(Debug, Clone)]
pub enum RbacError {
    /// 角色不存在
    RoleNotFound(String),
    /// 策略不存在
    PolicyNotFound(String),
    /// 循环继承检测
    CyclicInheritance(String),
    /// 策略加载失败
    PolicyLoadFailed(String),
    /// 策略存储操作失败
    StoreError(String),
    /// 缓存操作失败
    CacheError(String),
    /// ABAC 条件表达式解析失败
    ConditionParseError {
        /// 表达式
        expression: String,
        /// 错误详情
        detail: String,
    },
    /// ABAC 条件评估失败
    ConditionEvalError {
        /// 表达式
        expression: String,
        /// 错误详情
        detail: String,
    },
    /// 审计写入失败（不阻断权限检查，仅记录）
    AuditWriteFailed(String),
    /// 策略初始化失败
    PolicyInitFailed(String),
    /// 无效的资源路径
    InvalidResourcePath(String),
    /// 无效的角色名
    InvalidRoleName(String),
    /// 权限检查上下文不完整
    IncompleteContext(String),
    /// 引擎未初始化
    EngineNotInitialized,
    /// 配置错误
    ConfigError(String),
}

impl RbacError {
    /// 错误码
    ///
    /// 格式：PL05XXX（Platform 域，模块 05 = RBAC 引擎）
    pub fn code(&self) -> &'static str {
        match self {
            Self::RoleNotFound(_) => "PL05001",
            Self::PolicyNotFound(_) => "PL05002",
            Self::CyclicInheritance(_) => "PL05003",
            Self::PolicyLoadFailed(_) => "PL05004",
            Self::StoreError(_) => "PL05005",
            Self::CacheError(_) => "PL05006",
            Self::ConditionParseError { .. } => "PL05007",
            Self::ConditionEvalError { .. } => "PL05008",
            Self::AuditWriteFailed(_) => "PL05009",
            Self::PolicyInitFailed(_) => "PL05010",
            Self::InvalidResourcePath(_) => "PL05011",
            Self::InvalidRoleName(_) => "PL05012",
            Self::IncompleteContext(_) => "PL05013",
            Self::EngineNotInitialized => "PL05014",
            Self::ConfigError(_) => "PL05015",
        }
    }

    /// HTTP 状态码映射
    pub fn http_status(&self) -> u16 {
        match self {
            Self::RoleNotFound(_) => 404,
            Self::PolicyNotFound(_) => 404,
            Self::CyclicInheritance(_) => 500,
            Self::PolicyLoadFailed(_) => 500,
            Self::StoreError(_) => 500,
            Self::CacheError(_) => 500,
            Self::ConditionParseError { .. } => 400,
            Self::ConditionEvalError { .. } => 500,
            Self::AuditWriteFailed(_) => 500,
            Self::PolicyInitFailed(_) => 500,
            Self::InvalidResourcePath(_) => 400,
            Self::InvalidRoleName(_) => 400,
            Self::IncompleteContext(_) => 400,
            Self::EngineNotInitialized => 500,
            Self::ConfigError(_) => 500,
        }
    }

    /// 错误严重等级
    pub fn level(&self) -> ErrorLevel {
        match self {
            Self::RoleNotFound(_) => ErrorLevel::Warning,
            Self::PolicyNotFound(_) => ErrorLevel::Warning,
            Self::CyclicInheritance(_) => ErrorLevel::Error,
            Self::PolicyLoadFailed(_) => ErrorLevel::Error,
            Self::StoreError(_) => ErrorLevel::Error,
            Self::CacheError(_) => ErrorLevel::Warning,
            Self::ConditionParseError { .. } => ErrorLevel::Warning,
            Self::ConditionEvalError { .. } => ErrorLevel::Error,
            Self::AuditWriteFailed(_) => ErrorLevel::Warning,
            Self::PolicyInitFailed(_) => ErrorLevel::Critical,
            Self::InvalidResourcePath(_) => ErrorLevel::Warning,
            Self::InvalidRoleName(_) => ErrorLevel::Warning,
            Self::IncompleteContext(_) => ErrorLevel::Warning,
            Self::EngineNotInitialized => ErrorLevel::Error,
            Self::ConfigError(_) => ErrorLevel::Error,
        }
    }
}

/// 错误等级（轻量定义，mox-error feature 启用后会桥接到 mox_error::ErrorLevel）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    /// 信息级
    Info,
    /// 警告级
    Warning,
    /// 错误级
    Error,
    /// 严重级
    Critical,
}

impl fmt::Display for ErrorLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => f.write_str("INFO"),
            Self::Warning => f.write_str("WARN"),
            Self::Error => f.write_str("ERROR"),
            Self::Critical => f.write_str("CRITICAL"),
        }
    }
}

impl fmt::Display for RbacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoleNotFound(role) => {
                write!(f, "[{}] RBAC: role not found '{}'", self.code(), role)
            }
            Self::PolicyNotFound(policy) => {
                write!(f, "[{}] RBAC: policy not found '{}'", self.code(), policy)
            }
            Self::CyclicInheritance(role) => write!(
                f,
                "[{}] RBAC: cyclic inheritance detected in role '{}'",
                self.code(),
                role
            ),
            Self::PolicyLoadFailed(msg) => {
                write!(f, "[{}] RBAC: policy load failed — {}", self.code(), msg)
            }
            Self::StoreError(msg) => {
                write!(f, "[{}] RBAC: store error — {}", self.code(), msg)
            }
            Self::CacheError(msg) => {
                write!(f, "[{}] RBAC: cache error — {}", self.code(), msg)
            }
            Self::ConditionParseError { expression, detail } => write!(
                f,
                "[{}] RBAC: condition parse error in '{}' — {}",
                self.code(),
                expression,
                detail
            ),
            Self::ConditionEvalError { expression, detail } => write!(
                f,
                "[{}] RBAC: condition eval error in '{}' — {}",
                self.code(),
                expression,
                detail
            ),
            Self::AuditWriteFailed(msg) => write!(
                f,
                "[{}] RBAC: audit write failed — {} (non-blocking)",
                self.code(),
                msg
            ),
            Self::PolicyInitFailed(msg) => {
                write!(f, "[{}] RBAC: policy init failed — {}", self.code(), msg)
            }
            Self::InvalidResourcePath(path) => {
                write!(f, "[{}] RBAC: invalid resource path '{}'", self.code(), path)
            }
            Self::InvalidRoleName(name) => {
                write!(f, "[{}] RBAC: invalid role name '{}'", self.code(), name)
            }
            Self::IncompleteContext(msg) => {
                write!(f, "[{}] RBAC: incomplete context — {}", self.code(), msg)
            }
            Self::EngineNotInitialized => {
                write!(f, "[{}] RBAC: engine not initialized", self.code())
            }
            Self::ConfigError(msg) => {
                write!(f, "[{}] RBAC: config error — {}", self.code(), msg)
            }
        }
    }
}

impl std::error::Error for RbacError {}

// ── mox-error 集成（可选 feature） ──────────────────────────────────────────

#[cfg(feature = "mox-error")]
impl From<RbacError> for mox_error::MoxError {
    fn from(err: RbacError) -> Self {
        use mox_error::{ErrorDomain, ErrorLevel as MoxLevel, MoxError};

        let to_level = |l: ErrorLevel| match l {
            ErrorLevel::Info => MoxLevel::Info,
            ErrorLevel::Warning => MoxLevel::Warning,
            ErrorLevel::Error => MoxLevel::Error,
            ErrorLevel::Critical => MoxLevel::Critical,
        };

        let (module, seq, msg) = match &err {
            RbacError::RoleNotFound(r) => (05u8, 001u16, format!("角色不存在: {r}")),
            RbacError::PolicyNotFound(p) => (05, 002, format!("策略不存在: {p}")),
            RbacError::CyclicInheritance(r) => (05, 003, format!("角色循环继承: {r}")),
            RbacError::PolicyLoadFailed(m) => (05, 004, format!("策略加载失败: {m}")),
            RbacError::StoreError(m) => (05, 005, format!("策略存储错误: {m}")),
            RbacError::CacheError(m) => (05, 006, format!("缓存错误: {m}")),
            RbacError::ConditionParseError { expression, detail } => (
                05,
                007,
                format!("条件表达式解析失败: {expression} — {detail}"),
            ),
            RbacError::ConditionEvalError { expression, detail } => (
                05,
                008,
                format!("条件表达式评估失败: {expression} — {detail}"),
            ),
            RbacError::AuditWriteFailed(m) => (05, 009, format!("审计写入失败: {m}")),
            RbacError::PolicyInitFailed(m) => (05, 010, format!("策略初始化失败: {m}")),
            RbacError::InvalidResourcePath(p) => (05, 011, format!("无效资源路径: {p}")),
            RbacError::InvalidRoleName(n) => (05, 012, format!("无效角色名: {n}")),
            RbacError::IncompleteContext(m) => (05, 013, format!("上下文不完整: {m}")),
            RbacError::EngineNotInitialized => (05, 014, "引擎未初始化".to_string()),
            RbacError::ConfigError(m) => (05, 015, format!("配置错误: {m}")),
        };

        MoxError::new(
            ErrorDomain::Platform,
            module,
            seq,
            msg,
            to_level(err.level()),
            err.http_status(),
        )
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_unique() {
        let errors: Vec<RbacError> = vec![
            RbacError::RoleNotFound("test".into()),
            RbacError::PolicyNotFound("test".into()),
            RbacError::CyclicInheritance("test".into()),
            RbacError::PolicyLoadFailed("test".into()),
            RbacError::StoreError("test".into()),
            RbacError::CacheError("test".into()),
            RbacError::ConditionParseError {
                expression: "x".into(),
                detail: "y".into(),
            },
            RbacError::ConditionEvalError {
                expression: "x".into(),
                detail: "y".into(),
            },
            RbacError::AuditWriteFailed("test".into()),
            RbacError::PolicyInitFailed("test".into()),
            RbacError::InvalidResourcePath("test".into()),
            RbacError::InvalidRoleName("test".into()),
            RbacError::IncompleteContext("test".into()),
            RbacError::EngineNotInitialized,
            RbacError::ConfigError("test".into()),
        ];

        let mut codes = Vec::new();
        for e in &errors {
            let code = e.code();
            assert!(!codes.contains(&code), "duplicate code: {code}");
            codes.push(code);
        }
        assert_eq!(codes.len(), 15);
    }

    #[test]
    fn error_code_format_pl05() {
        // 验证所有错误码都以 PL05 开头
        let errors: Vec<RbacError> = vec![
            RbacError::RoleNotFound("t".into()),
            RbacError::PolicyNotFound("t".into()),
            RbacError::CyclicInheritance("t".into()),
            RbacError::PolicyLoadFailed("t".into()),
            RbacError::StoreError("t".into()),
            RbacError::CacheError("t".into()),
            RbacError::ConditionParseError {
                expression: "e".into(),
                detail: "d".into(),
            },
            RbacError::ConditionEvalError {
                expression: "e".into(),
                detail: "d".into(),
            },
            RbacError::AuditWriteFailed("t".into()),
            RbacError::PolicyInitFailed("t".into()),
            RbacError::InvalidResourcePath("t".into()),
            RbacError::InvalidRoleName("t".into()),
            RbacError::IncompleteContext("t".into()),
            RbacError::EngineNotInitialized,
            RbacError::ConfigError("t".into()),
        ];

        for e in &errors {
            let code = e.code();
            assert!(
                code.starts_with("PL05"),
                "error code {} should start with PL05",
                code
            );
            // 格式：PL05 + 3位数字 = 7 字符
            assert_eq!(code.len(), 7, "error code {} should be 7 chars", code);
            assert!(
                code[4..].chars().all(|c| c.is_ascii_digit()),
                "error code {} should end with digits",
                code
            );
        }
    }

    #[test]
    fn error_display_contains_code() {
        let err = RbacError::RoleNotFound("admin".into());
        let display = format!("{err}");
        assert!(display.contains("PL05001"));
        assert!(display.contains("admin"));
    }

    #[test]
    fn error_http_status_mapping() {
        assert_eq!(RbacError::RoleNotFound("x".into()).http_status(), 404);
        assert_eq!(RbacError::PolicyNotFound("x".into()).http_status(), 404);
        assert_eq!(RbacError::CyclicInheritance("x".into()).http_status(), 500);
        assert_eq!(
            RbacError::ConditionParseError {
                expression: "x".into(),
                detail: "y".into(),
            }
            .http_status(),
            400
        );
    }

    #[test]
    fn error_level_mapping() {
        assert_eq!(RbacError::RoleNotFound("x".into()).level(), ErrorLevel::Warning);
        assert_eq!(
            RbacError::CyclicInheritance("x".into()).level(),
            ErrorLevel::Error
        );
        assert_eq!(
            RbacError::PolicyInitFailed("x".into()).level(),
            ErrorLevel::Critical
        );
        assert_eq!(RbacError::CacheError("x".into()).level(), ErrorLevel::Warning);
    }

    #[test]
    fn condition_error_display() {
        let err = RbacError::ConditionParseError {
            expression: "subject.age > 18".into(),
            detail: "unexpected token".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("PL05007"));
        assert!(display.contains("subject.age > 18"));
        assert!(display.contains("unexpected token"));
    }

    #[cfg(feature = "mox-error")]
    #[test]
    fn into_mox_error_preserves_code() {
        let err = RbacError::RoleNotFound("test".into());
        let mox_err: mox_error::MoxError = err.into();
        assert_eq!(mox_err.code, "PL05001");
        assert_eq!(mox_err.http_status, 404);
    }
}
