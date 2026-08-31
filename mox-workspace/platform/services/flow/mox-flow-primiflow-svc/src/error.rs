//! 流程编排错误类型

use thiserror::Error;

/// 流程服务错误
#[derive(Debug, Error)]
pub enum FlowError {
    /// DAG 构建错误
    #[error("DAG 构建错误: {0}")]
    DagBuildError(String),

    /// 检测到环
    #[error("DAG 中检测到环: {0}")]
    CycleDetected(String),

    /// 调度错误
    #[error("调度错误: {0}")]
    SchedulerError(String),

    /// 执行错误
    #[error("执行错误: {0}")]
    ExecutionError(String),

    /// 节点未找到
    #[error("节点未找到: {0}")]
    NodeNotFound(String),

    /// 算子未找到
    #[error("算子未找到: {0}")]
    OperatorNotFound(String),

    /// 参数错误
    #[error("参数错误: {0}")]
    InvalidParameter(String),

    /// 超时
    #[error("执行超时")]
    Timeout,

    /// 流程已取消
    #[error("流程已取消")]
    Cancelled,
}

pub type FlowResult<T> = Result<T, FlowError>;
