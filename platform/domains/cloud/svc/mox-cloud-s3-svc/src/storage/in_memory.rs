// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 内存存储后端：基于 `parking_lot::RwLock<HashMap>` 的 [`StorageBackend`] 实现。
//!
//! 用于 S3 服务的默认内存模式和单元测试。所有操作均为强一致（内存读写原子），
//! 支持按前缀分页列出 chunk。

use async_trait::async_trait;
use mox_cloud_domain_traits::{
    BackendCapabilities, BackendType, ChunkId, ChunkInfo, ChunkListPage, ConsistencyModel,
    StorageBackend, StorageError,
};
use parking_lot::RwLock;
use std::collections::HashMap;

/// 纯内存存储后端。
///
/// 数据存储在 `RwLock<HashMap<ChunkId, Vec<u8>>>` 中，元信息（size/checksum/created_at）
/// 在 `put_chunk` 时计算并随数据一并存储。
#[derive(Default)]
pub struct InMemoryStorageBackend {
    chunks: RwLock<HashMap<ChunkId, StoredChunk>>,
}

struct StoredChunk {
    data: Vec<u8>,
    info: ChunkInfo,
}

impl InMemoryStorageBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn now_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn md5_hex(data: &[u8]) -> String {
        use md5::{Digest, Md5};
        let mut h = Md5::new();
        h.update(data);
        hex::encode(h.finalize())
    }
}

#[async_trait]
impl StorageBackend for InMemoryStorageBackend {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: &[u8]) -> Result<ChunkInfo, StorageError> {
        let info = ChunkInfo {
            chunk_id: chunk_id.clone(),
            size_bytes: data.len() as u64,
            created_at_ms: Self::now_ms(),
            checksum: Self::md5_hex(data),
        };
        let mut guard = self.chunks.write();
        guard.insert(chunk_id.clone(), StoredChunk { data: data.to_vec(), info: info.clone() });
        Ok(info)
    }

    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
        let guard = self.chunks.read();
        guard.get(chunk_id).map(|c| c.data.clone()).ok_or(StorageError::NotFound)
    }

    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<bool, StorageError> {
        let mut guard = self.chunks.write();
        Ok(guard.remove(chunk_id).is_some())
    }

    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool, StorageError> {
        let guard = self.chunks.read();
        Ok(guard.contains_key(chunk_id))
    }

    async fn list_chunks(
        &self,
        prefix: &str,
        marker: Option<&str>,
        limit: u32,
    ) -> Result<ChunkListPage, StorageError> {
        let guard = self.chunks.read();
        // 收集匹配前缀的 chunk，按 chunk_id 字典序排序
        let mut items: Vec<&StoredChunk> = guard
            .iter()
            .filter(|(id, _)| id.as_str().starts_with(prefix))
            .map(|(_, c)| c)
            .collect();
        items.sort_by(|a, b| a.info.chunk_id.as_str().cmp(b.info.chunk_id.as_str()));

        // marker 分页：跳过 marker 之前的项
        let start_idx = if let Some(m) = marker {
            items.partition_point(|c| c.info.chunk_id.as_str() <= m)
        } else {
            0
        };

        let limit = limit as usize;
        let sliced: Vec<ChunkInfo> =
            items.iter().skip(start_idx).take(limit).map(|c| c.info.clone()).collect();

        let next_idx = start_idx + sliced.len();
        let is_truncated = next_idx < items.len();
        let next_marker = if is_truncated {
            Some(items[next_idx - 1].info.chunk_id.as_str().to_string())
        } else {
            None
        };

        Ok(ChunkListPage { items: sliced, next_marker, is_truncated })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::InMemory
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_range_read: true,
            supports_atomic_write: true,
            supports_conditional_put: false,
            consistency_model: ConsistencyModel::Strong,
            max_chunk_size: u64::MAX,
            preferred_chunk_size: 4 * 1024 * 1024,
        }
    }

    fn name(&self) -> &'static str {
        "in-memory-storage-backend"
    }
}

impl std::fmt::Debug for InMemoryStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryStorageBackend")
            .field("chunks", &self.chunks.read().len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_put_and_get_chunk() {
        let backend = InMemoryStorageBackend::new();
        let id = ChunkId::new("chunk-001");
        let data = b"hello world";

        let info = backend.put_chunk(&id, data).await.unwrap();
        assert_eq!(info.size_bytes, 11);
        assert_eq!(info.chunk_id, id);
        assert!(!info.checksum.is_empty());

        let got = backend.get_chunk(&id).await.unwrap();
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn test_get_nonexistent_returns_not_found() {
        let backend = InMemoryStorageBackend::new();
        let id = ChunkId::new("missing");
        let result = backend.get_chunk(&id).await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }

    #[tokio::test]
    async fn test_delete_chunk() {
        let backend = InMemoryStorageBackend::new();
        let id = ChunkId::new("del-me");
        backend.put_chunk(&id, b"data").await.unwrap();
        assert!(backend.chunk_exists(&id).await.unwrap());

        let deleted = backend.delete_chunk(&id).await.unwrap();
        assert!(deleted);
        assert!(!backend.chunk_exists(&id).await.unwrap());

        // 删除不存在的返回 false
        let deleted_again = backend.delete_chunk(&id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_chunk_exists() {
        let backend = InMemoryStorageBackend::new();
        let id = ChunkId::new("exists-check");
        assert!(!backend.chunk_exists(&id).await.unwrap());
        backend.put_chunk(&id, b"x").await.unwrap();
        assert!(backend.chunk_exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_list_chunks_with_prefix_and_pagination() {
        let backend = InMemoryStorageBackend::new();
        // 插入 5 个带前缀的 chunk + 1 个不带前缀的
        for i in 0..5 {
            let id = ChunkId::new(format!("obj:bucket:key:v{}", i));
            backend.put_chunk(&id, &[i as u8]).await.unwrap();
        }
        backend.put_chunk(&ChunkId::new("other:thing"), b"x").await.unwrap();

        // 前缀过滤
        let page = backend.list_chunks("obj:bucket:key:", None, 100).await.unwrap();
        assert_eq!(page.items.len(), 5);
        assert!(!page.is_truncated);

        // 分页：limit=2
        let page1 = backend.list_chunks("obj:bucket:key:", None, 2).await.unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.is_truncated);
        assert!(page1.next_marker.is_some());

        let page2 = backend
            .list_chunks("obj:bucket:key:", page1.next_marker.as_deref(), 2)
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.is_truncated);

        let page3 = backend
            .list_chunks("obj:bucket:key:", page2.next_marker.as_deref(), 2)
            .await
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.is_truncated);
        assert!(page3.next_marker.is_none());
    }

    #[tokio::test]
    async fn test_overwrite_chunk() {
        let backend = InMemoryStorageBackend::new();
        let id = ChunkId::new("overwrite");
        backend.put_chunk(&id, b"v1").await.unwrap();
        backend.put_chunk(&id, b"v2-longer").await.unwrap();
        let got = backend.get_chunk(&id).await.unwrap();
        assert_eq!(got, b"v2-longer");
    }

    #[test]
    fn test_backend_metadata() {
        let backend = InMemoryStorageBackend::new();
        assert_eq!(backend.backend_type(), BackendType::InMemory);
        assert_eq!(backend.name(), "in-memory-storage-backend");
        let caps = backend.capabilities();
        assert_eq!(caps.consistency_model, ConsistencyModel::Strong);
        assert!(caps.supports_atomic_write);
    }

    #[tokio::test]
    async fn test_trait_object_safe() {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let id = ChunkId::new("dyn-test");
        backend.put_chunk(&id, b"dyn").await.unwrap();
        assert_eq!(backend.get_chunk(&id).await.unwrap(), b"dyn");
        assert_eq!(backend.backend_type(), BackendType::InMemory);
    }
}
