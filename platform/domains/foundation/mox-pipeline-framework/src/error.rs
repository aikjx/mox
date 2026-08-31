// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 管线框架错误类型
//!
//! 错误码段：PL（Platform 域）· 模块 04（管线框架）
//! - PL04001 ~ PL04099 预留为管线框架使用

use std::fmt;

use crate::phase::Phase;

/// 管线执行错误
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 阶段执行失败
    PhaseFailed { phase: Phase, message: String },
    /// 钩子执行失败（通常不阻断，但可配置为阻断）
    HookFailed { hook: String, message: String },
    /// 配置错误
    ConfigError(String),
    /// 上下文无效
    InvalidContext(String),
    /// 管线已终止
    PipelineAborted(String),
    /// 阶段处理器未找到
    HandlerNotFound(Phase),
    /// 审计写入失败（不阻断管线）
    AuditWriteFailed(String),
}

impl PipelineError {
    /// 错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::PhaseFailed { .. } => "PL04001",
            Self::HookFailed { .. } => "PL04002",
            Self::ConfigError(_) => "PL04003",
            Self::InvalidContext(_) => "PL04004",
            Self::PipelineAborted(_) => "PL04005",
            Self::HandlerNotFound(_) => "PL04006",
            Self::AuditWriteFailed(_) => "PL04007",
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            Self::PhaseFailed { .. } => 500,
            Self::HookFailed { .. } => 500,
            Self::ConfigError(_) => 500,
            Self::InvalidContext(_) => 400,
            Self::PipelineAborted(_) => 500,
            Self::HandlerNotFound(_) => 500,
            Self::AuditWriteFailed(_) => 500,
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhaseFailed { phase, message } => {
                write!(
                    f,
                    "[{}] Pipeline: phase '{}' failed — {}",
                    self.code(),
                    phase.name(),
                    message
                )
            }
            Self::HookFailed { hook, message } => {
                write!(
                    f,
                    "[{}] Pipeline: hook '{}' failed — {}",
                    self.code(),
                    hook,
                    message
                )
            }
            Self::ConfigError(msg) => {
                write!(f, "[{}] Pipeline: config error — {}", self.code(), msg)
            }
            Self::InvalidContext(msg) => {
                write!(f, "[{}] Pipeline: invalid context — {}", self.code(), msg)
            }
            Self::PipelineAborted(msg) => {
                write!(f, "[{}] Pipeline: aborted — {}", self.code(), msg)
            }
            Self::HandlerNotFound(phase) => {
                write!(
                    f,
                    "[{}] Pipeline: handler not found for phase '{}'",
                    self.code(),
                    phase.name()
                )
            }
            Self::AuditWriteFailed(msg) => {
                write!(
                    f,
                    "[{}] Pipeline: audit write failed — {} (non-blocking)",
                    self.code(),
                    msg
                )
            }
        }
    }
}

impl std::error::Error for PipelineError {}

// ── mox-error 集成（可选 feature） ──────────────────────────────

#[cfg(feature = "mox-error")]
impl From<PipelineError> for mox_error::MoxError {
    fn from(err: PipelineError) -> Self {
        use mox_error::{ErrorDomain, ErrorLevel, MoxError};

        let (module, seq, msg, level, http) = match &err {
            PipelineError::PhaseFailed { phase, message } => (
                04u8,
                001u16,
                format!("阶段 {} 执行失败: {}", phase.name(), message),
                ErrorLevel::Error,
                500u16,
            ),
            PipelineError::HookFailed { hook, message } => (
                04,
                002,
                format!("钩子 {} 执行失败: {}", hook, message),
                ErrorLevel::Error,
                500,
            ),
            PipelineError::ConfigError(msg) => {
                (04, 003, format!("配置错误: {msg}"), ErrorLevel::Error, 500)
            }
            PipelineError::InvalidContext(msg) => (
                04,
                004,
                format!("上下文无效: {msg}"),
                ErrorLevel::Warning,
                400,
            ),
            PipelineError::PipelineAborted(msg) => (
                04,
                005,
                format!("管线已终止: {msg}"),
                ErrorLevel::Warning,
                500,
            ),
            PipelineError::HandlerNotFound(phase) => (
                04,
                006,
                format!("阶段处理器未找到: {}", phase.name()),
                ErrorLevel::Error,
                500,
            ),
            PipelineError::AuditWriteFailed(msg) => (
                04,
                007,
                format!("审计写入失败: {msg}"),
                ErrorLevel::Warning,
                500,
            ),
        };

        MoxError::new(ErrorDomain::Platform, module, seq, msg, level, http)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::Phase;

    #[test]
    fn error_codes_are_unique() {
        let errors = [
            PipelineError::PhaseFailed {
                phase: Phase::Analyze,
                message: "test".into(),
            },
            PipelineError::HookFailed {
                hook: "test".into(),
                message: "test".into(),
            },
            PipelineError::ConfigError("test".into()),
            PipelineError::InvalidContext("test".into()),
            PipelineError::PipelineAborted("test".into()),
            PipelineError::HandlerNotFound(Phase::Analyze),
            PipelineError::AuditWriteFailed("test".into()),
        ];
        let mut codes = Vec::new();
        for e in &errors {
            let code = e.code();
            assert!(!codes.contains(&code), "duplicate code: {code}");
            codes.push(code);
        }
    }

    #[test]
    fn phase_failed_format() {
        let err = PipelineError::PhaseFailed {
            phase: Phase::Gate,
            message: "timeout".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("PL04001"));
        assert!(display.contains("gate"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn config_error_format() {
        let err = PipelineError::ConfigError("missing phases".into());
        assert_eq!(err.code(), "PL04003");
        assert_eq!(err.http_status(), 500);
    }

    #[cfg(feature = "mox-error")]
    #[test]
    fn into_mox_error_preserves_code() {
        let err = PipelineError::ConfigError("test".into());
        let mox_err: mox_error::MoxError = err.into();
        assert_eq!(mox_err.code, "PL04003");
        assert_eq!(mox_err.http_status, 500);
    }
}
