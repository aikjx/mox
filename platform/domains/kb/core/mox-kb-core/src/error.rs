// =============================================================================
// KB 错误类型
// =============================================================================

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KbError {
    #[error("文档不存在: {0}")]
    DocumentNotFound(String),
    #[error("文档已存在: {0}")]
    DocumentExists(String),
    #[error("存储错误: {0}")]
    StorageError(String),
    #[error("搜索错误: {0}")]
    SearchError(String),
    #[error("版本错误: {0}")]
    VersionError(String),
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    #[error("参数错误: {0}")]
    InvalidParam(String),
    #[error("内部错误: {0}")]
    InternalError(String),
}

pub type KbResult<T> = Result<T, KbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KbError::DocumentNotFound("doc123".to_string());
        assert!(format!("{err}").contains("文档不存在"));
        assert!(format!("{err}").contains("doc123"));
    }
}
