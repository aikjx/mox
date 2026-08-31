// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 核心引擎错误类型（基于 proto::ExpertError 扩展）
//!
//! P2 架构解耦 · 阶段 4：
//! - 领域协议错误在 `mox-ai-expert-proto` 中定义（SSOT）
//! - CoreError 增加引擎内部特有的错误类别（注册表、裁决、治理等）
//! - 通过 `From` 转换与 proto 错误互通

use mox_ai_expert_proto::ExpertError;
use thiserror::Error;

/// 核心引擎错误
#[derive(Debug, Error)]
pub enum CoreError {
    /// 协议层错误（透传）
    #[error(transparent)]
    Proto(#[from] ExpertError),

    /// 专家注册表错误
    #[error("Registry error: {0}")]
    Registry(String),

    /// 专家未找到
    #[error("Expert not found: {0}")]
    ExpertNotFound(String),

    /// 维度未注册
    #[error("Dimension not registered: {0:?}")]
    DimensionNotRegistered(mox_ai_expert_proto::Dimension),

    /// 裁决冲突（无法仲裁的同级冲突）
    #[error("Reconciliation conflict: {0}")]
    ReconcileConflict(String),

    /// 治理闸门拦截
    #[error("Governance gate blocked: {0}")]
    GovernanceBlocked(String),

    /// 璇玑验证否决（最高权限，不可覆盖）
    #[error("Algorithm verification vetoed: {0}")]
    AlgorithmVeto(String),

    /// 归一化失败
    #[error("Normalization failed: {0}")]
    NormalizationFailed(String),

    /// 无效输入
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// 内部错误（非预期）
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 核心引擎 Result 类型别名
pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// 是否为阻断级错误（需要升级为 Blocking）
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            CoreError::GovernanceBlocked(_) | CoreError::AlgorithmVeto(_)
        )
    }

    /// 是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(self, CoreError::Internal(_))
    }

    /// 错误码（用于日志/审计）
    pub fn code(&self) -> &str {
        match self {
            CoreError::Proto(e) => &e.code,
            CoreError::Registry(_) => "CORE_REGISTRY",
            CoreError::ExpertNotFound(_) => "CORE_EXPERT_NOT_FOUND",
            CoreError::DimensionNotRegistered(_) => "CORE_DIM_NOT_REGISTERED",
            CoreError::ReconcileConflict(_) => "CORE_RECONCILE_CONFLICT",
            CoreError::GovernanceBlocked(_) => "CORE_GOVERN_BLOCKED",
            CoreError::AlgorithmVeto(_) => "CORE_ALGO_VETO",
            CoreError::NormalizationFailed(_) => "CORE_NORMALIZE_FAILED",
            CoreError::InvalidInput(_) => "CORE_INVALID_INPUT",
            CoreError::Internal(_) => "CORE_INTERNAL",
        }
    }
}

// ─── 与第三方错误的转换 ────────────────────────────────────────────────────

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Internal(format!("IO error: {}", e))
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::InvalidInput(format!("JSON error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_codes_are_distinct() {
        let errors = vec![
            CoreError::Registry("test".into()),
            CoreError::ExpertNotFound("e1".into()),
            CoreError::ReconcileConflict("c1".into()),
            CoreError::GovernanceBlocked("g1".into()),
            CoreError::AlgorithmVeto("v1".into()),
            CoreError::NormalizationFailed("n1".into()),
            CoreError::InvalidInput("i1".into()),
            CoreError::Internal("x1".into()),
        ];
        let codes: std::collections::HashSet<&str> =
            errors.iter().map(|e| e.code()).collect();
        assert_eq!(codes.len(), errors.len(), "每个错误应有唯一 code");
    }

    #[test]
    fn blocking_errors_identified() {
        assert!(CoreError::GovernanceBlocked("x".into()).is_blocking());
        assert!(CoreError::AlgorithmVeto("x".into()).is_blocking());
        assert!(!CoreError::Registry("x".into()).is_blocking());
        assert!(!CoreError::InvalidInput("x".into()).is_blocking());
    }

    #[test]
    fn internal_errors_are_retryable() {
        assert!(CoreError::Internal("x".into()).is_retryable());
        assert!(!CoreError::ExpertNotFound("x".into()).is_retryable());
    }

    #[test]
    fn display_format_works() {
        let e = CoreError::ExpertNotFound("algo".into());
        let s = format!("{}", e);
        assert!(s.contains("algo"));
        assert!(s.contains("not found"));
    }
}
