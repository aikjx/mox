//! 图谱存储错误类型

use thiserror::Error;

/// 存储错误
#[derive(Debug, Error)]
pub enum StorageError {
    /// 连接错误
    #[error("存储连接错误: {0}")]
    ConnectionError(String),

    /// 查询错误
    #[error("查询执行错误: {0}")]
    QueryError(String),

    /// 事务错误
    #[error("事务错误: {0}")]
    TransactionError(String),

    /// 资源不存在
    #[error("资源不存在: {0}")]
    NotFound(String),

    /// 约束冲突
    #[error("约束冲突: {0}")]
    ConstraintViolation(String),

    /// 超时
    #[error("操作超时")]
    Timeout,

    /// 内部错误
    #[error("内部存储错误: {0}")]
    Internal(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
