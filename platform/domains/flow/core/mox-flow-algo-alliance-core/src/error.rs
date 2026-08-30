// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法联盟错误类型

use thiserror::Error;

/// 算法联盟结果类型
pub type AlgoResult<T> = Result<T, AlgoError>;

/// 算法联盟错误
#[derive(Debug, Error)]
pub enum AlgoError {
    /// 算法未找到
    #[error("algorithm not found: {0}")]
    AlgorithmNotFound(String),

    /// 算法版本不兼容
    #[error("algorithm version incompatible: {0}")]
    VersionIncompatible(String),

    /// 参数错误
    #[error("invalid parameter '{param}': {reason}")]
    InvalidParameter { param: String, reason: String },

    /// 缺少必需参数
    #[error("missing required parameter: {0}")]
    MissingParameter(String),

    /// 参数类型不匹配
    #[error("parameter type mismatch: expected {expected}, got {got}")]
    ParameterTypeMismatch { expected: String, got: String },

    /// 输入数据形状不匹配
    #[error("input shape mismatch: expected {expected}, got {got}")]
    InputShapeMismatch { expected: String, got: String },

    /// 输入数据类型不匹配
    #[error("input type mismatch: expected {expected}, got {got}")]
    InputTypeMismatch { expected: String, got: String },

    /// 执行错误
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// 超时
    #[error("algorithm execution timeout after {0}ms")]
    Timeout(u64),

    /// 资源不足
    #[error("insufficient resources: {0}")]
    InsufficientResources(String),

    /// 流水线错误
    #[error("pipeline error: {0}")]
    PipelineError(String),

    /// 流水线状态错误
    #[error("pipeline state error: {0}")]
    PipelineStateError(String),

    /// 调优错误
    #[error("auto-tuner error: {0}")]
    TuningError(String),

    /// 计算引擎错误
    #[error("compute engine error: {0}")]
    ComputeEngineError(String),

    /// 适配器错误
    #[error("adapter error: {0}")]
    AdapterError(String),

    /// 内部错误
    #[error("internal error: {0}")]
    InternalError(String),
}

impl AlgoError {
    /// 是否是可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AlgoError::Timeout(_)
                | AlgoError::InsufficientResources(_)
                | AlgoError::ComputeEngineError(_)
        )
    }

    /// 错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            AlgoError::AlgorithmNotFound(_) => "ALGO_NOT_FOUND",
            AlgoError::VersionIncompatible(_) => "VERSION_INCOMPATIBLE",
            AlgoError::InvalidParameter { .. } => "INVALID_PARAM",
            AlgoError::MissingParameter(_) => "MISSING_PARAM",
            AlgoError::ParameterTypeMismatch { .. } => "PARAM_TYPE_MISMATCH",
            AlgoError::InputShapeMismatch { .. } => "INPUT_SHAPE_MISMATCH",
            AlgoError::InputTypeMismatch { .. } => "INPUT_TYPE_MISMATCH",
            AlgoError::ExecutionError(_) => "EXECUTION_ERROR",
            AlgoError::Timeout(_) => "TIMEOUT",
            AlgoError::InsufficientResources(_) => "INSUFFICIENT_RESOURCES",
            AlgoError::PipelineError(_) => "PIPELINE_ERROR",
            AlgoError::PipelineStateError(_) => "PIPELINE_STATE_ERROR",
            AlgoError::TuningError(_) => "TUNING_ERROR",
            AlgoError::ComputeEngineError(_) => "COMPUTE_ENGINE_ERROR",
            AlgoError::AdapterError(_) => "ADAPTER_ERROR",
            AlgoError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}
