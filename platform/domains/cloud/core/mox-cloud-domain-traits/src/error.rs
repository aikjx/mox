//! 统一顶层错误类型 [`CloudError`] —— 跨 crate 错误聚合与上下文链。
//!
//! 各 svc crate（volume / s3 / filer / master / rebalance）的错误类型均可
//! 通过 `From` 转换为 [`CloudError`]，保留上下文链（thiserror 风格）。
//! domain-traits 内部的 [`StorageError`] / [`MetaError`] / [`ReadError`] /
//! [`WriteError`] 通过 `#[from]` 自动转换。

use crate::meta_storage::MetaError;
use crate::shard_reader::ReadError;
use crate::shard_writer::WriteError;
use crate::storage_backend::StorageError;

/// Mox Cloud 统一顶层错误枚举。
///
/// 覆盖存储、元数据、读、写、各 svc 服务层及通用错误场景。
/// 所有变体均实现 `Display`，可通过 `From` 从底层错误类型转换。
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    /// 存储后端错误（chunk 级存取）。
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// 元数据存储错误。
    #[error("Metadata error: {0}")]
    Meta(#[from] MetaError),

    /// 分片读取错误。
    #[error("Read error: {0}")]
    Read(#[from] ReadError),

    /// 分片写入错误。
    #[error("Write error: {0}")]
    Write(#[from] WriteError),

    /// volume-svc 错误。
    #[error("Volume error: {0}")]
    Volume(String),

    /// s3-svc 错误。
    #[error("S3 error: {0}")]
    S3(String),

    /// filer-svc 错误。
    #[error("Filer error: {0}")]
    Filer(String),

    /// master-svc 错误。
    #[error("Master error: {0}")]
    Master(String),

    /// rebalance-svc 错误。
    #[error("Rebalance error: {0}")]
    Rebalance(String),

    /// 背压机制拒绝请求。
    #[error("Backpressure rejected: {0}")]
    BackpressureRejected(String),

    /// 资源未找到。
    #[error("Not found: {0}")]
    NotFound(String),

    /// 资源已存在。
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// 无效输入。
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// 不支持的操作。
    #[error("Unsupported: {0}")]
    Unsupported(String),

    /// 内部错误。
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 统一结果类型别名。
pub type CloudResult<T> = Result<T, CloudError>;

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_error_construct() {
        let e1 = CloudError::NotFound("chunk abc".into());
        assert!(format!("{e1}").contains("Not found: chunk abc"));

        let e2 = CloudError::Internal("panic in worker".into());
        assert!(format!("{e2}").contains("Internal error: panic in worker"));

        let e3 = CloudError::BackpressureRejected("max writes=64".into());
        assert!(format!("{e3}").contains("Backpressure rejected: max writes=64"));
    }

    #[test]
    fn test_cloud_error_from_storage_error() {
        let se = StorageError::NotFound;
        let ce: CloudError = se.into();
        assert!(matches!(ce, CloudError::Storage(StorageError::NotFound)));
        assert!(format!("{ce}").contains("Storage error: chunk not found"));

        let se2 = StorageError::IoError("disk full".into());
        let ce2: CloudError = se2.into();
        assert!(matches!(ce2, CloudError::Storage(StorageError::IoError(_))));
        assert!(format!("{ce2}").contains("disk full"));
    }

    #[test]
    fn test_cloud_error_from_meta_error() {
        let me = MetaError::NotFound;
        let ce: CloudError = me.into();
        assert!(matches!(ce, CloudError::Meta(MetaError::NotFound)));
        assert!(format!("{ce}").contains("Metadata error: meta key not found"));

        let me2 = MetaError::TransactionConflict;
        let ce2: CloudError = me2.into();
        assert!(matches!(ce2, CloudError::Meta(MetaError::TransactionConflict)));
    }

    #[test]
    fn test_cloud_error_from_read_error() {
        let re = ReadError::Timeout(std::time::Duration::from_secs(5));
        let ce: CloudError = re.into();
        assert!(matches!(ce, CloudError::Read(ReadError::Timeout(_))));
        assert!(format!("{ce}").contains("Read error: read timeout after"));
    }

    #[test]
    fn test_cloud_error_from_write_error() {
        let we = WriteError::QuorumNotReached {
            succeeded: 1,
            required: 2,
        };
        let ce: CloudError = we.into();
        assert!(matches!(
            ce,
            CloudError::Write(WriteError::QuorumNotReached { .. })
        ));
        assert!(format!("{ce}").contains("Write error: quorum not reached"));
    }

    #[test]
    fn test_cloud_error_display_all_variants() {
        let variants: Vec<CloudError> = vec![
            CloudError::Volume("vol-1 offline".into()),
            CloudError::S3("signature mismatch".into()),
            CloudError::Filer("inode corrupted".into()),
            CloudError::Master("leader election failed".into()),
            CloudError::Rebalance("shard migration timeout".into()),
            CloudError::AlreadyExists("bucket my-bucket".into()),
            CloudError::InvalidInput("negative size".into()),
            CloudError::Unsupported("zero-copy write".into()),
        ];
        for v in &variants {
            let s = format!("{v}");
            assert!(!s.is_empty(), "Display must not be empty");
        }
    }

    #[test]
    fn test_cloud_result_type_alias() {
        let ok: CloudResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: CloudResult<i32> = Err(CloudError::NotFound("test".into()));
        assert!(err.is_err());
    }
}
