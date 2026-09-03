//! L6 存储后端抽象 —— 统一的数据块（chunk）存取契约。
//!
//! 所有底层存储实现（本地 FS、S3 兼容、RustFs ECStore、内存等）均需实现
//! [`StorageBackend`]，向上层提供一致的 chunk 级读写接口。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 标识与数据结构
// ---------------------------------------------------------------------------

/// 数据块唯一标识（String 新类型）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(String);

impl ChunkId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ChunkId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChunkId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ChunkId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChunkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 数据块元信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub chunk_id: ChunkId,
    pub size_bytes: u64,
    pub created_at_ms: u64,
    pub checksum: String,
}

/// chunk 列表分页结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkListPage {
    pub items: Vec<ChunkInfo>,
    pub next_marker: Option<String>,
    pub is_truncated: bool,
}

// ---------------------------------------------------------------------------
// 后端类型与能力
// ---------------------------------------------------------------------------

/// 存储后端类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    LocalFs,
    S3Compatible,
    RustFsEcstore,
    InMemory,
    Other,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::LocalFs => write!(f, "local-fs"),
            BackendType::S3Compatible => write!(f, "s3-compatible"),
            BackendType::RustFsEcstore => write!(f, "rustfs-ecstore"),
            BackendType::InMemory => write!(f, "in-memory"),
            BackendType::Other => write!(f, "other"),
        }
    }
}

/// 一致性模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsistencyModel {
    Strong,
    ReadAfterWrite,
    Eventual,
}

impl std::fmt::Display for ConsistencyModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsistencyModel::Strong => write!(f, "strong"),
            ConsistencyModel::ReadAfterWrite => write!(f, "read-after-write"),
            ConsistencyModel::Eventual => write!(f, "eventual"),
        }
    }
}

/// 后端能力描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapabilities {
    pub supports_range_read: bool,
    pub supports_atomic_write: bool,
    pub supports_conditional_put: bool,
    pub consistency_model: ConsistencyModel,
    pub max_chunk_size: u64,
    pub preferred_chunk_size: u64,
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self {
            supports_range_read: false,
            supports_atomic_write: false,
            supports_conditional_put: false,
            consistency_model: ConsistencyModel::Eventual,
            max_chunk_size: 64 * 1024 * 1024,
            preferred_chunk_size: 4 * 1024 * 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 存储后端操作错误。
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("chunk not found")]
    NotFound,
    #[error("chunk already exists")]
    AlreadyExists,
    #[error("backend unavailable")]
    BackendUnavailable,
    #[error("invalid input")]
    InvalidInput,
    #[error("unsupported operation")]
    Unsupported,
    #[error("io error: {0}")]
    IoError(String),
}

// ---------------------------------------------------------------------------
// 核心 trait
// ---------------------------------------------------------------------------

/// L6 存储后端抽象。
///
/// 所有方法均为 `&self`，trait 是 object-safe 的，可通过 `Box<dyn StorageBackend>`
/// 进行动态分发。
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// 写入一个数据块。
    async fn put_chunk(&self, chunk_id: &ChunkId, data: &[u8]) -> Result<ChunkInfo, StorageError>;

    /// 读取一个数据块的完整内容。
    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError>;

    /// 删除一个数据块，返回是否存在并被删除。
    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<bool, StorageError>;

    /// 检查数据块是否存在。
    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool, StorageError>;

    /// 按前缀分页列出数据块。
    async fn list_chunks(
        &self,
        prefix: &str,
        marker: Option<&str>,
        limit: u32,
    ) -> Result<ChunkListPage, StorageError>;

    /// 后端类型。
    fn backend_type(&self) -> BackendType;

    /// 后端能力集。
    fn capabilities(&self) -> BackendCapabilities;

    /// 后端静态名称。
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyBackend;

    #[async_trait]
    impl StorageBackend for DummyBackend {
        async fn put_chunk(
            &self,
            chunk_id: &ChunkId,
            data: &[u8],
        ) -> Result<ChunkInfo, StorageError> {
            Ok(ChunkInfo {
                chunk_id: chunk_id.clone(),
                size_bytes: data.len() as u64,
                created_at_ms: 0,
                checksum: String::new(),
            })
        }

        async fn get_chunk(&self, _chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
            Ok(vec![])
        }

        async fn delete_chunk(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
            Ok(false)
        }

        async fn chunk_exists(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
            Ok(false)
        }

        async fn list_chunks(
            &self,
            _prefix: &str,
            _marker: Option<&str>,
            _limit: u32,
        ) -> Result<ChunkListPage, StorageError> {
            Ok(ChunkListPage { items: vec![], next_marker: None, is_truncated: false })
        }

        fn backend_type(&self) -> BackendType {
            BackendType::InMemory
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        fn name(&self) -> &'static str {
            "dummy-backend"
        }
    }

    #[test]
    fn test_types_construct() {
        let id = ChunkId::new("chunk-001");
        assert_eq!(id.as_str(), "chunk-001");
        assert_eq!(id.to_string(), "chunk-001");

        let info = ChunkInfo {
            chunk_id: id.clone(),
            size_bytes: 1024,
            created_at_ms: 1_700_000_000_000,
            checksum: "sha256:abc".into(),
        };
        assert_eq!(info.size_bytes, 1024);
        assert_eq!(info.chunk_id, id);

        let page = ChunkListPage {
            items: vec![info],
            next_marker: Some("marker-1".into()),
            is_truncated: true,
        };
        assert_eq!(page.items.len(), 1);
        assert!(page.is_truncated);

        let caps = BackendCapabilities::default();
        assert_eq!(caps.preferred_chunk_size, 4 * 1024 * 1024);
        assert_eq!(caps.consistency_model, ConsistencyModel::Eventual);

        assert_eq!(BackendType::LocalFs.to_string(), "local-fs");
        assert_eq!(ConsistencyModel::Strong.to_string(), "strong");
    }

    #[tokio::test]
    async fn test_trait_object_safe() {
        let backend: Box<dyn StorageBackend> = Box::new(DummyBackend);
        assert_eq!(backend.name(), "dummy-backend");
        assert_eq!(backend.backend_type(), BackendType::InMemory);
        assert_eq!(backend.capabilities().max_chunk_size, 64 * 1024 * 1024);

        let id = ChunkId::new("obj-1");
        let info = backend.put_chunk(&id, b"hello").await.unwrap();
        assert_eq!(info.size_bytes, 5);
        assert_eq!(info.chunk_id, id);

        assert!(!backend.chunk_exists(&id).await.unwrap());
    }
}
