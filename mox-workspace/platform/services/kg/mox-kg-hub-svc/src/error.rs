//! 图谱集成服务错误类型

use thiserror::Error;

/// 集成服务错误
#[derive(Debug, Error)]
pub enum HubError {
    /// 数据源连接错误
    #[error("数据源连接错误: {0}")]
    ConnectionError(String),

    /// 数据抽取错误
    #[error("数据抽取错误: {0}")]
    ExtractionError(String),

    /// 数据转换错误
    #[error("数据转换错误: {0}")]
    TransformError(String),

    /// 数据加载错误
    #[error("数据加载错误: {0}")]
    LoadError(String),

    /// 连接器未找到
    #[error("连接器未找到: {0}")]
    ConnectorNotFound(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 任务执行错误
    #[error("任务执行错误: {0}")]
    TaskError(String),

    /// 超时
    #[error("操作超时")]
    Timeout,
}

pub type HubResult<T> = Result<T, HubError>;
