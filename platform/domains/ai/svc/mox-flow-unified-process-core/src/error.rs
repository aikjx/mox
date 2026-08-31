// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一流程错误类型
//!
//! 覆盖三套引擎的全部错误语义：
//! - flow_engine::FlowError (5 种)
//! - workflow_engine (OperatorError::Other + anyhow)
//! - flow_svc / expert_svc (无统一错误类型)

use thiserror::Error;

/// 统一流程错误枚举
#[derive(Debug, Error)]
pub enum UnifiedFlowError {
    // === 结构校验错误 ===
    /// 节点不存在
    #[error("节点不存在: {0}")]
    NodeNotFound(String),

    /// 边引用了不存在的节点
    #[error("边引用不存在的节点: edge={edge} node={node}")]
    EdgeRefNotFound { edge: String, node: String },

    /// 缺少 Start 节点
    #[error("流程图缺少 Start 节点")]
    MissingStartNode,

    /// 缺少 End 节点
    #[error("流程图缺少 End 节点")]
    MissingEndNode,

    /// 检测到循环
    #[error("检测到循环依赖: {0}")]
    CycleDetected(String),

    /// 配置无效
    #[error("配置无效: {0}")]
    InvalidConfig(String),

    /// 节点类型与配置不匹配
    #[error("节点 {node_id} 的类型与配置不匹配: expected={expected}")]
    ConfigMismatch { node_id: String, expected: String },

    // === 执行错误 ===
    /// 节点执行失败
    #[error("节点执行失败: node={node_id} reason={reason}")]
    NodeExecutionFailed { node_id: String, reason: String },

    /// 条件表达式求值错误
    #[error("条件表达式求值错误: {0}")]
    ConditionError(String),

    /// 执行步数超限（防无限循环）
    #[error("执行步数超限 (>{max_steps})，疑似无限循环")]
    ExecutionStepsExceeded { max_steps: usize },

    /// 子流程调用失败
    #[error("子流程调用失败: flow={flow_id} reason={reason}")]
    SubFlowFailed { flow_id: String, reason: String },

    /// 节点被 Guard 阻断
    #[error("节点 {node_id} 被 Guard 阻断: {reason}")]
    GuardBlocked { node_id: String, reason: String },

    // === 扩展错误 ===
    /// 扩展处理器错误
    #[error("扩展处理器错误: handler={handler}")]
    ExtensionError {
        handler: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// 未注册的节点处理器
    #[error("未找到节点 {node_id} 的处理器 (kind={kind})")]
    HandlerNotFound { node_id: String, kind: String },

    // === 内部错误 ===
    /// 内部错误（兜底）
    #[error("内部错误: {0}")]
    Internal(String),
}

impl UnifiedFlowError {
    /// 错误码（用于 API 响应）
    pub fn code(&self) -> &'static str {
        match self {
            Self::NodeNotFound(_) => "NODE_NOT_FOUND",
            Self::EdgeRefNotFound { .. } => "EDGE_REF_NOT_FOUND",
            Self::MissingStartNode => "MISSING_START_NODE",
            Self::MissingEndNode => "MISSING_END_NODE",
            Self::CycleDetected(_) => "CYCLE_DETECTED",
            Self::InvalidConfig(_) => "INVALID_CONFIG",
            Self::ConfigMismatch { .. } => "CONFIG_MISMATCH",
            Self::NodeExecutionFailed { .. } => "NODE_EXECUTION_FAILED",
            Self::ConditionError(_) => "CONDITION_ERROR",
            Self::ExecutionStepsExceeded { .. } => "EXECUTION_STEPS_EXCEEDED",
            Self::SubFlowFailed { .. } => "SUBFLOW_FAILED",
            Self::GuardBlocked { .. } => "GUARD_BLOCKED",
            Self::ExtensionError { .. } => "EXTENSION_ERROR",
            Self::HandlerNotFound { .. } => "HANDLER_NOT_FOUND",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// 是否为可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NodeExecutionFailed { .. } | Self::ExtensionError { .. } | Self::Internal(_)
        )
    }
}

/// 统一 Result 类型别名
pub type FlowResult<T> = Result<T, UnifiedFlowError>;
