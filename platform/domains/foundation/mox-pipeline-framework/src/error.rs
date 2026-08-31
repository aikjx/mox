// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 管线框架错误类型
//!
//! 错误码段：PL（Platform 域）· 模块 05（管线框架）
//! - PL05001 ~ PL05099 预留为管线框架使用
//!
//! 错误码分配：
//! - PL05001: 阶段执行失败
//! - PL05002: 钩子执行失败
//! - PL05003: 配置错误
//! - PL05004: 上下文无效
//! - PL05005: 管线已终止
//! - PL05006: 阶段处理器未找到
//! - PL05007: 审计写入失败（不阻断管线）
//! - PL05008: 插件加载失败
//! - PL05009: 插件卸载失败
//! - PL05010: 扩展点未找到

use std::fmt;

use crate::phase::PhaseId;

/// 管线执行错误
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Debug, Clone)]
pub enum PipelineError<P: PhaseId> {
    /// 阶段执行失败
    PhaseFailed { phase: P, message: String },
    /// 钩子执行失败（通常不阻断，但可配置为阻断）
    HookFailed { hook: String, message: String },
    /// 配置错误
    ConfigError(String),
    /// 上下文无效
    InvalidContext(String),
    /// 管线已终止
    PipelineAborted(String),
    /// 阶段处理器未找到
    HandlerNotFound(P),
    /// 审计写入失败（不阻断管线）
    AuditWriteFailed(String),
    /// 插件加载失败
    PluginLoadFailed { plugin_id: String, message: String },
    /// 插件卸载失败
    PluginUnloadFailed { plugin_id: String, message: String },
    /// 扩展点未找到
    ExtensionPointNotFound(String),
}

impl<P: PhaseId> PipelineError<P> {
    /// 错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::PhaseFailed { .. } => "PL05001",
            Self::HookFailed { .. } => "PL05002",
            Self::ConfigError(_) => "PL05003",
            Self::InvalidContext(_) => "PL05004",
            Self::PipelineAborted(_) => "PL05005",
            Self::HandlerNotFound(_) => "PL05006",
            Self::AuditWriteFailed(_) => "PL05007",
            Self::PluginLoadFailed { .. } => "PL05008",
            Self::PluginUnloadFailed { .. } => "PL05009",
            Self::ExtensionPointNotFound(_) => "PL05010",
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
            Self::PluginLoadFailed { .. } => 500,
            Self::PluginUnloadFailed { .. } => 500,
            Self::ExtensionPointNotFound(_) => 404,
        }
    }
}

impl<P: PhaseId> fmt::Display for PipelineError<P> {
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
            Self::PluginLoadFailed { plugin_id, message } => {
                write!(
                    f,
                    "[{}] Pipeline: plugin '{}' load failed — {}",
                    self.code(),
                    plugin_id,
                    message
                )
            }
            Self::PluginUnloadFailed { plugin_id, message } => {
                write!(
                    f,
                    "[{}] Pipeline: plugin '{}' unload failed — {}",
                    self.code(),
                    plugin_id,
                    message
                )
            }
            Self::ExtensionPointNotFound(name) => {
                write!(
                    f,
                    "[{}] Pipeline: extension point '{}' not found",
                    self.code(),
                    name
                )
            }
        }
    }
}

impl<P: PhaseId> std::error::Error for PipelineError<P> {}

// ── mox-error 集成（可选 feature） ──────────────────────────────

#[cfg(feature = "mox-error")]
impl<P: PhaseId> From<PipelineError<P>> for mox_error::MoxError {
    fn from(err: PipelineError<P>) -> Self {
        use mox_error::{ErrorDomain, ErrorLevel, MoxError};

        let (module, seq, msg, level, http) = match &err {
            PipelineError::PhaseFailed { phase, message } => (
                05u8,
                001u16,
                format!("阶段 {} 执行失败: {}", phase.name(), message),
                ErrorLevel::Error,
                500u16,
            ),
            PipelineError::HookFailed { hook, message } => (
                05,
                002,
                format!("钩子 {} 执行失败: {}", hook, message),
                ErrorLevel::Error,
                500,
            ),
            PipelineError::ConfigError(msg) => {
                (05, 003, format!("配置错误: {msg}"), ErrorLevel::Error, 500)
            }
            PipelineError::InvalidContext(msg) => (
                05,
                004,
                format!("上下文无效: {msg}"),
                ErrorLevel::Warning,
                400,
            ),
            PipelineError::PipelineAborted(msg) => (
                05,
                005,
                format!("管线已终止: {msg}"),
                ErrorLevel::Warning,
                500,
            ),
            PipelineError::HandlerNotFound(phase) => (
                05,
                006,
                format!("阶段处理器未找到: {}", phase.name()),
                ErrorLevel::Error,
                500,
            ),
            PipelineError::AuditWriteFailed(msg) => (
                05,
                007,
                format!("审计写入失败: {msg}"),
                ErrorLevel::Warning,
                500,
            ),
            PipelineError::PluginLoadFailed { plugin_id, message } => (
                05,
                008,
                format!("插件 {} 加载失败: {}", plugin_id, message),
                ErrorLevel::Error,
                500,
            ),
            PipelineError::PluginUnloadFailed { plugin_id, message } => (
                05,
                009,
                format!("插件 {} 卸载失败: {}", plugin_id, message),
                ErrorLevel::Warning,
                500,
            ),
            PipelineError::ExtensionPointNotFound(name) => (
                05,
                010,
                format!("扩展点未找到: {}", name),
                ErrorLevel::Warning,
                404,
            ),
        };

        MoxError::new(ErrorDomain::Platform, module, seq, msg, level, http)
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::NamedPhase;

    #[test]
    fn error_codes_are_unique() {
        let errors: Vec<PipelineError<NamedPhase>> = vec![
            PipelineError::PhaseFailed {
                phase: NamedPhase::new("analyze"),
                message: "test".into(),
            },
            PipelineError::HookFailed {
                hook: "test".into(),
                message: "test".into(),
            },
            PipelineError::ConfigError("test".into()),
            PipelineError::InvalidContext("test".into()),
            PipelineError::PipelineAborted("test".into()),
            PipelineError::HandlerNotFound(NamedPhase::new("analyze")),
            PipelineError::AuditWriteFailed("test".into()),
            PipelineError::PluginLoadFailed {
                plugin_id: "test".into(),
                message: "test".into(),
            },
            PipelineError::PluginUnloadFailed {
                plugin_id: "test".into(),
                message: "test".into(),
            },
            PipelineError::ExtensionPointNotFound("test".into()),
        ];
        let mut codes = Vec::new();
        for e in &errors {
            let code = e.code();
            assert!(!codes.contains(&code), "duplicate code: {code}");
            codes.push(code);
        }
        assert_eq!(codes.len(), 10);
    }

    #[test]
    fn phase_failed_format() {
        let err = PipelineError::PhaseFailed {
            phase: NamedPhase::blocking("gate"),
            message: "timeout".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("PL05001"));
        assert!(display.contains("gate"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn config_error_format() {
        let err: PipelineError<NamedPhase> = PipelineError::ConfigError("missing phases".into());
        assert_eq!(err.code(), "PL05003");
        assert_eq!(err.http_status(), 500);
    }

    #[test]
    fn extension_point_not_found_format() {
        let err: PipelineError<NamedPhase> =
            PipelineError::ExtensionPointNotFound("pre_analyze".into());
        assert_eq!(err.code(), "PL05010");
        assert_eq!(err.http_status(), 404);
    }

    #[test]
    fn plugin_load_failed_format() {
        let err: PipelineError<NamedPhase> = PipelineError::PluginLoadFailed {
            plugin_id: "my_plugin".into(),
            message: "dependency missing".into(),
        };
        assert_eq!(err.code(), "PL05008");
        let display = format!("{err}");
        assert!(display.contains("my_plugin"));
        assert!(display.contains("dependency missing"));
    }

    #[test]
    fn all_error_codes_start_with_pl05() {
        let errors: Vec<PipelineError<NamedPhase>> = vec![
            PipelineError::PhaseFailed {
                phase: NamedPhase::new("a"),
                message: "".into(),
            },
            PipelineError::HookFailed {
                hook: "".into(),
                message: "".into(),
            },
            PipelineError::ConfigError("".into()),
            PipelineError::InvalidContext("".into()),
            PipelineError::PipelineAborted("".into()),
            PipelineError::HandlerNotFound(NamedPhase::new("b")),
            PipelineError::AuditWriteFailed("".into()),
            PipelineError::PluginLoadFailed {
                plugin_id: "".into(),
                message: "".into(),
            },
            PipelineError::PluginUnloadFailed {
                plugin_id: "".into(),
                message: "".into(),
            },
            PipelineError::ExtensionPointNotFound("".into()),
        ];
        for e in &errors {
            assert!(
                e.code().starts_with("PL05"),
                "error code {} should start with PL05",
                e.code()
            );
        }
    }
}
