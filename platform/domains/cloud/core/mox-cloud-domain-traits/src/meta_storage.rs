//! L4 元数据存储抽象 —— 统一的文件/对象元数据存取契约。
//!
//! 所有元数据后端（本地 KV、Redis、RDBMS 等）均需实现 [`MetaStorage`]，
//! 向上层提供一致的属性读写、目录列举、创建/删除/重命名接口。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 标识与数据结构
// ---------------------------------------------------------------------------

/// 统一的元数据键（String 新类型）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaKey(String);

impl MetaKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for MetaKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MetaKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for MetaKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 元数据值，包含属性集合与版本信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaValue {
    pub attributes: HashMap<String, String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub version: u64,
}

impl Default for MetaValue {
    fn default() -> Self {
        Self {
            attributes: HashMap::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
            version: 0,
        }
    }
}

/// 目录条目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    File,
    Directory,
    Symlink,
    Other,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::File => write!(f, "file"),
            EntryType::Directory => write!(f, "directory"),
            EntryType::Symlink => write!(f, "symlink"),
            EntryType::Other => write!(f, "other"),
        }
    }
}

/// 目录列表中的单个条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub key: MetaKey,
    pub entry_type: EntryType,
    pub size_bytes: u64,
}

/// 目录列表分页结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirListPage {
    pub entries: Vec<DirEntry>,
    pub next_marker: Option<String>,
    pub is_truncated: bool,
}

// ---------------------------------------------------------------------------
// 并发模型
// ---------------------------------------------------------------------------

/// 元数据存储的并发控制模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConcurrencyModel {
    InternalMutex,
    Mvcc,
    EventualConsistency,
}

impl std::fmt::Display for ConcurrencyModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcurrencyModel::InternalMutex => write!(f, "internal-mutex"),
            ConcurrencyModel::Mvcc => write!(f, "mvcc"),
            ConcurrencyModel::EventualConsistency => write!(f, "eventual-consistency"),
        }
    }
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 元数据存储操作错误。
#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("meta key not found")]
    NotFound,
    #[error("meta key already exists")]
    AlreadyExists,
    #[error("invalid input")]
    InvalidInput,
    #[error("transaction conflict")]
    TransactionConflict,
    #[error("backend unavailable")]
    BackendUnavailable,
    #[error("unsupported operation")]
    Unsupported,
}

// ---------------------------------------------------------------------------
// 核心 trait
// ---------------------------------------------------------------------------

/// L4 元数据存储抽象。
///
/// 所有方法均为 `&self`，trait 是 object-safe 的。
#[async_trait]
pub trait MetaStorage: Send + Sync {
    /// 获取指定键的属性值，不存在时返回 `Ok(None)`。
    async fn get_attr(&self, key: &MetaKey) -> Result<Option<MetaValue>, MetaError>;

    /// 设置（覆盖）指定键的属性值。
    async fn set_attr(&self, key: &MetaKey, value: &MetaValue) -> Result<(), MetaError>;

    /// 分页列举父键下的直接子条目。
    async fn list_dir(
        &self,
        parent: &MetaKey,
        marker: Option<&str>,
        limit: u32,
    ) -> Result<DirListPage, MetaError>;

    /// 创建一个新键，若已存在应返回 [`MetaError::AlreadyExists`]。
    async fn create(&self, key: &MetaKey, value: &MetaValue) -> Result<(), MetaError>;

    /// 删除指定键，返回是否存在并被删除。
    async fn remove(&self, key: &MetaKey) -> Result<bool, MetaError>;

    /// 重命名（移动）一个键。
    async fn rename(&self, old_key: &MetaKey, new_key: &MetaKey) -> Result<(), MetaError>;

    /// 该后端使用的并发控制模型。
    fn concurrency_model(&self) -> ConcurrencyModel;
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyMeta;

    #[async_trait]
    impl MetaStorage for DummyMeta {
        async fn get_attr(&self, _key: &MetaKey) -> Result<Option<MetaValue>, MetaError> {
            Ok(None)
        }

        async fn set_attr(&self, _key: &MetaKey, _value: &MetaValue) -> Result<(), MetaError> {
            Ok(())
        }

        async fn list_dir(
            &self,
            _parent: &MetaKey,
            _marker: Option<&str>,
            _limit: u32,
        ) -> Result<DirListPage, MetaError> {
            Ok(DirListPage {
                entries: vec![],
                next_marker: None,
                is_truncated: false,
            })
        }

        async fn create(&self, _key: &MetaKey, _value: &MetaValue) -> Result<(), MetaError> {
            Ok(())
        }

        async fn remove(&self, _key: &MetaKey) -> Result<bool, MetaError> {
            Ok(false)
        }

        async fn rename(&self, _old_key: &MetaKey, _new_key: &MetaKey) -> Result<(), MetaError> {
            Ok(())
        }

        fn concurrency_model(&self) -> ConcurrencyModel {
            ConcurrencyModel::Mvcc
        }
    }

    #[test]
    fn test_types_construct() {
        let key = MetaKey::new("/bucket/dir/file.txt");
        assert_eq!(key.as_str(), "/bucket/dir/file.txt");
        assert_eq!(key.to_string(), "/bucket/dir/file.txt");

        let mut attrs = HashMap::new();
        attrs.insert("content-type".into(), "text/plain".into());
        let value = MetaValue {
            attributes: attrs,
            created_at_ms: 100,
            updated_at_ms: 200,
            version: 3,
        };
        assert_eq!(value.version, 3);
        assert_eq!(value.attributes.get("content-type").unwrap(), "text/plain");

        let default_value = MetaValue::default();
        assert!(default_value.attributes.is_empty());
        assert_eq!(default_value.version, 0);

        let entry = DirEntry {
            name: "file.txt".into(),
            key: MetaKey::new("/bucket/dir/file.txt"),
            entry_type: EntryType::File,
            size_bytes: 4096,
        };
        assert_eq!(entry.entry_type, EntryType::File);

        let page = DirListPage {
            entries: vec![entry],
            next_marker: None,
            is_truncated: false,
        };
        assert_eq!(page.entries.len(), 1);

        assert_eq!(EntryType::Directory.to_string(), "directory");
        assert_eq!(ConcurrencyModel::Mvcc.to_string(), "mvcc");
    }

    #[tokio::test]
    async fn test_trait_object_safe() {
        let meta: Box<dyn MetaStorage> = Box::new(DummyMeta);
        assert_eq!(meta.concurrency_model(), ConcurrencyModel::Mvcc);

        let key = MetaKey::new("/test/key");
        assert!(meta.get_attr(&key).await.unwrap().is_none());

        let value = MetaValue::default();
        meta.set_attr(&key, &value).await.unwrap();
        meta.create(&key, &value).await.unwrap();
        assert!(!meta.remove(&key).await.unwrap());

        let page = meta.list_dir(&MetaKey::new("/test"), None, 10).await.unwrap();
        assert!(page.entries.is_empty());
    }
}
