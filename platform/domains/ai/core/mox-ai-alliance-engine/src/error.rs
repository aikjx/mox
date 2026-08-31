// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 联盟引擎统一错误枚举（thiserror，便于 ? 传播）
//!
//! 所有阶段错误统一收敛到本枚举，上层服务可直接匹配处理。

use thiserror::Error;

/// 专家联盟管线错误枚举（所有阶段错误的统一集合）
#[derive(Debug, Error)]
pub enum AllianceError {
    /// query 不能为空
    #[error("query 不能为空")]
    EmptyQuery,

    /// 意图分类失败
    #[error("意图分类失败：{0}")]
    IntentClassify(String),

    /// 组队失败
    #[error("组队失败：{0}")]
    TeamBuild(String),

    /// 专家咨询超时
    #[error("专家咨询超时（{secs}s 隔离）")]
    ExpertTimeout { secs: u64, expert: String },

    /// 质量门禁不通过
    #[error("质量门禁不通过（Gate={gate:?}，retried={retried}）")]
    GateBlocked { gate: String, retried: bool },

    /// RBAC 未授权
    #[error("RBAC 未授权：需要权限 {perm:?}")]
    Unauthorized { perm: String },

    /// 管线阶段执行错误
    #[error("管线阶段 [{phase}] 执行失败：{message}")]
    PhaseError { phase: String, message: String },

    /// 流式输出错误
    #[error("流式输出错误：{0}")]
    StreamError(String),

    /// KG 连接器错误
    #[error("KG 连接器错误：{0}")]
    KgConnector(String),

    /// 内部错误（anyhow 透传）
    #[error("内部错误：{0}")]
    Internal(#[from] anyhow::Error),
}

impl AllianceError {
    /// 返回错误码字符串（用于 SSE error 事件与审计）
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyQuery => "EMPTY_QUERY",
            Self::IntentClassify(_) => "INTENT_CLASSIFY_FAILED",
            Self::TeamBuild(_) => "TEAM_BUILD_FAILED",
            Self::ExpertTimeout { .. } => "EXPERT_TIMEOUT",
            Self::GateBlocked { .. } => "GATE_BLOCKED",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::PhaseError { .. } => "PHASE_ERROR",
            Self::StreamError(_) => "STREAM_ERROR",
            Self::KgConnector(_) => "KG_CONNECTOR_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// 是否为可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ExpertTimeout { .. } | Self::KgConnector(_) | Self::Internal(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_distinct() {
        let errors: Vec<AllianceError> = vec![
            AllianceError::EmptyQuery,
            AllianceError::IntentClassify("test".into()),
            AllianceError::TeamBuild("test".into()),
            AllianceError::ExpertTimeout { secs: 60, expert: "test".into() },
            AllianceError::GateBlocked { gate: "D".into(), retried: false },
            AllianceError::Unauthorized { perm: "admin".into() },
            AllianceError::PhaseError { phase: "intent".into(), message: "test".into() },
            AllianceError::StreamError("test".into()),
            AllianceError::KgConnector("test".into()),
            AllianceError::Internal(anyhow::anyhow!("test")),
        ];
        let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "错误码不应重复");
    }

    #[test]
    fn empty_query_display() {
        let e = AllianceError::EmptyQuery;
        assert_eq!(format!("{}", e), "query 不能为空");
        assert_eq!(e.code(), "EMPTY_QUERY");
        assert!(!e.is_retryable());
    }

    #[test]
    fn expert_timeout_is_retryable() {
        let e = AllianceError::ExpertTimeout { secs: 60, expert: "security".into() };
        assert!(e.is_retryable());
        assert_eq!(e.code(), "EXPERT_TIMEOUT");
    }
}
