//! 图谱融合错误类型

use thiserror::Error;

/// 融合服务错误
#[derive(Debug, Error)]
pub enum FusionError {
    /// 对齐错误
    #[error("实体对齐错误: {0}")]
    AlignmentError(String),

    /// 融合错误
    #[error("知识融合错误: {0}")]
    FusionError(String),

    /// 匹配错误
    #[error("实体匹配错误: {0}")]
    MatchingError(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 数据源错误
    #[error("数据源错误: {0}")]
    DataSourceError(String),

    /// 冲突解决错误
    #[error("冲突解决错误: {0}")]
    ConflictResolutionError(String),

    /// 超时
    #[error("操作超时")]
    Timeout,
}

pub type FusionResult<T> = Result<T, FusionError>;
