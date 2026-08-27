// mox-dsql-core 错误类型定义
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DsqlError {
    #[error("SQL not found: {0}")]
    SqlNotFound(String),

    #[error("SQL not active: {0}, status: {1}")]
    SqlNotActive(String, String),

    #[error("Datasource not found: {0}")]
    DatasourceNotFound(String),

    #[error("Datasource disabled: {0}")]
    DatasourceDisabled(String),

    #[error("Invalid param: {0}")]
    InvalidParam(String),

    #[error("Missing required param: {0}")]
    MissingParam(String),

    #[error("Template render error: {0}")]
    TemplateError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Version conflict: {0}")]
    VersionConflict(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type DsqlResult<T> = Result<T, DsqlError>;
