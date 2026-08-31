//! 云存储错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("对象不存在: {0}")]
    NotFound(String),

    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("权限不足")]
    PermissionDenied,

    #[error("超出配额")]
    QuotaExceeded,
}
