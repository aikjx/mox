//! MOX 统一基座 · 统一存储抽象层
//!
//! 定义物理存储的统一 trait（BlobStore / ObjectStore / KVStore 三路物理口），
//! 供 cloud 域各 svc（mox-cloud-s3/volume/filer/master-svc）作为基座后端实现。
//!
//! ## 设计原则
//! - **物理口 trait，不内置后端**：本 crate 只定义契约，由 cloud 域实现。
//! - 大对象（Blob）支持 RANGE 直达流式通道（读/写指定字节区间），
//!   不因图谱化牺牲吞吐。
//! - 依赖方向：域 → mox-base-store-core ← mox-cloud-*（实现方）。

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

/// 统一存储错误
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("对象不存在: {path}")]
    NotFound { path: String },
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("校验失败: {0}")]
    Checksum(String),
    #[error("其他错误: {0}")]
    Other(String),
}

/// 统一结果类型
pub type StoreResult<T> = Result<T, StoreError>;

/// 统一 Blob 对象（物理二进制）
#[derive(Debug, Clone)]
pub struct BlobObject {
    /// 逻辑路径
    pub path: String,
    /// 内容类型
    pub content_type: String,
    /// 字节数
    pub size_bytes: u64,
    /// 内容寻址哈希（SHA-256 hex，用于去重）
    pub sha256: Option<String>,
}

/// 对象存储 trait（大对象物理口）
///
/// cloud 域 mox-cloud-s3-svc / mox-cloud-filer-svc 等实现此 trait。
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// 写入完整对象
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject>;

    /// 读取完整对象
    async fn get(&self, path: &str) -> StoreResult<Bytes>;

    /// 按字节区间读取（RANGE 直达流式通道）
    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes>;

    /// 删除对象
    async fn delete(&self, path: &str) -> StoreResult<()>;

    /// 对象元数据
    async fn head(&self, path: &str) -> StoreResult<BlobObject>;

    /// 判断对象是否存在
    async fn exists(&self, path: &str) -> StoreResult<bool>;
}

/// 对象存储操作（逐字节流写）——供超大对象流式上传
#[async_trait]
pub trait ObjectStreamWriter: Send + Sync {
    /// 打开流式写入句柄
    async fn open_writer(&self, path: &str, content_type: &str) -> StoreResult<StreamHandle>;

    /// 追加一段数据
    async fn write(&self, handle: &StreamHandle, chunk: Bytes) -> StoreResult<()>;

    /// 关闭并提交（计算哈希）
    async fn close(&self, handle: StreamHandle) -> StoreResult<BlobObject>;
}

/// 流式写入句柄（实现方可持有内部状态）
#[derive(Debug, Clone)]
pub struct StreamHandle {
    /// 逻辑路径
    pub path: String,
    /// 实现方状态（如上传会话 ID）
    pub state: String,
}

/// KV 存储 trait（元数据 / 索引物理口）
#[async_trait]
pub trait KvStore: Send + Sync {
    /// 写 key-value
    async fn put(&self, key: &str, value: Bytes) -> StoreResult<()>;

    /// 读 key-value
    async fn get(&self, key: &str) -> StoreResult<Option<Bytes>>;

    /// 删除 key
    async fn delete(&self, key: &str) -> StoreResult<()>;
}

/// 内存版 BlobStore（默认实现 / 测试用；生产由 cloud 域替换为 S3/Volume 实现）
pub struct InMemoryObjectStore {
    objects: std::sync::Mutex<std::collections::HashMap<String, (String, Vec<u8>)>>,
}

impl Default for InMemoryObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryObjectStore {
    /// 新建内存存储
    pub fn new() -> Self {
        Self {
            objects: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl ObjectStore for InMemoryObjectStore {
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        let sha = format!("sha256-{}", simple_hash(path));
        let size = data.len() as u64;
        let obj = BlobObject {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes: size,
            sha256: Some(sha),
        };
        self.objects
            .lock()
            .map_err(|e| StoreError::Other(e.to_string()))?
            .insert(path.to_string(), (content_type.to_string(), data.to_vec()));
        Ok(obj)
    }

    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        let map = self
            .objects
            .lock()
            .map_err(|e| StoreError::Other(e.to_string()))?;
        map.get(path)
            .map(|(_, data)| Bytes::copy_from_slice(data))
            .ok_or_else(|| StoreError::NotFound { path: path.to_string() })
    }

    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        let data = self.get(path).await?;
        let start = offset as usize;
        let end = std::cmp::min(start + length as usize, data.len());
        if start >= data.len() {
            return Ok(Bytes::new());
        }
        Ok(data.slice(start..end))
    }

    async fn delete(&self, path: &str) -> StoreResult<()> {
        self.objects
            .lock()
            .map_err(|e| StoreError::Other(e.to_string()))?
            .remove(path);
        Ok(())
    }

    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        let map = self
            .objects
            .lock()
            .map_err(|e| StoreError::Other(e.to_string()))?;
        map.get(path)
            .map(|(ct, data)| BlobObject {
                path: path.to_string(),
                content_type: ct.clone(),
                size_bytes: data.len() as u64,
                sha256: Some(format!("sha256-{}", simple_hash(path))),
            })
            .ok_or_else(|| StoreError::NotFound { path: path.to_string() })
    }

    async fn exists(&self, path: &str) -> StoreResult<bool> {
        Ok(self
            .objects
            .lock()
            .map_err(|e| StoreError::Other(e.to_string()))?
            .contains_key(path))
    }
}

/// 简单确定性哈希（测试/示例用；生产使用真正的 SHA-256）
fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_put_get_works() {
        let store = InMemoryObjectStore::new();
        let obj = store
            .put("kg/expert/a.png", "image/png", Bytes::from_static(b"hello world"))
            .await
            .unwrap();
        assert_eq!(obj.size_bytes, 11);
        assert!(obj.sha256.is_some());

        let data = store.get("kg/expert/a.png").await.unwrap();
        assert_eq!(&data[..], b"hello world");
    }

    #[tokio::test]
    async fn range_read_works() {
        let store = InMemoryObjectStore::new();
        store
            .put("b.bin", "application/octet-stream", Bytes::from_static(b"0123456789"))
            .await
            .unwrap();
        let part = store.get_range("b.bin", 2, 4).await.unwrap();
        assert_eq!(&part[..], b"2345");
    }

    #[tokio::test]
    async fn missing_returns_not_found() {
        let store = InMemoryObjectStore::new();
        let r = store.get("nope").await;
        assert!(matches!(r, Err(StoreError::NotFound { .. })));
    }

    #[tokio::test]
    async fn exists_and_delete_work() {
        let store = InMemoryObjectStore::new();
        store.put("x", "text/plain", Bytes::from_static(b"x")).await.unwrap();
        assert!(store.exists("x").await.unwrap());
        store.delete("x").await.unwrap();
        assert!(!store.exists("x").await.unwrap());
    }
}
