// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 回源装饰器（feature `s3`）：实现「目标空读自动回源」铁律。
//!
//! 场景：迁移期目标后端（如 S3）尚未回填数据，读目标为空 → 自动读源后端
//! （如 FS）→ 命中后**回填到目标**（self-heal），下次读直达目标。
//!
//! 语义（三层铁律）：
//! 1. **读**：目标 `get`/`head` 命中即返回；`NotFound` → 读源 → 命中回填；
//! 2. **写**：写穿（write-through）到目标（源为只读基准，不回写）；
//! 3. **删**：目标 + 源双删，保证后续读不再回源脏数据。
//!
//! `FallbackObjectStore` 是纯装饰器：目标/源均为任意 [`ObjectStore`] 实现，
//! 后端不可知，可任意叠加。

use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{BlobObject, ObjectStore, StoreError, StoreResult};
use std::sync::Arc;

/// 回源装饰器（目标为主，源为基准）
#[derive(Clone)]
pub struct FallbackObjectStore {
    /// 目标后端（主读/写）；迁移目标
    primary: Arc<dyn ObjectStore>,
    /// 源后端（回源基准）；只读语义
    fallback: Arc<dyn ObjectStore>,
}

impl FallbackObjectStore {
    pub fn new(primary: Arc<dyn ObjectStore>, fallback: Arc<dyn ObjectStore>) -> Self {
        Self { primary, fallback }
    }

    /// 目标引用（供上层直连）
    pub fn primary(&self) -> &Arc<dyn ObjectStore> {
        &self.primary
    }

    /// 源引用
    pub fn fallback(&self) -> &Arc<dyn ObjectStore> {
        &self.fallback
    }
}

#[async_trait]
impl ObjectStore for FallbackObjectStore {
    /// 读：目标命中即返回；目标空 → 回源，命中后回填目标（self-heal）。
    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        match self.primary.get(path).await {
            Ok(data) => Ok(data),
            Err(StoreError::NotFound { .. }) => {
                let data = self.fallback.get(path).await?;
                // 回填目标（尽力而为，失败不影响本次读取）
                let _ = self
                    .primary
                    .put(path, "application/octet-stream", data.clone())
                    .await;
                Ok(data)
            }
            Err(e) => Err(e),
        }
    }

    /// 按区间读：目标命中即返回；目标空 → 回源区间 + 回填。
    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        match self.primary.get_range(path, offset, length).await {
            Ok(data) => Ok(data),
            Err(StoreError::NotFound { .. }) => {
                let data = self.fallback.get_range(path, offset, length).await?;
                let _ = self
                    .primary
                    .put(path, "application/octet-stream", data.clone())
                    .await;
                Ok(data)
            }
            Err(e) => Err(e),
        }
    }

    /// 写：写穿到目标（源为只读基准，不回写）。
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        self.primary.put(path, content_type, data).await
    }

    /// 删：目标 + 源双删（防止后续读回源脏数据）。
    async fn delete(&self, path: &str) -> StoreResult<()> {
        let (r1, r2) = tokio::join!(self.primary.delete(path), self.fallback.delete(path));
        r1?;
        let _ = r2; // 源不存在视为成功（幂等）
        Ok(())
    }

    /// 头信息：目标命中即返回；目标空 → 回源（不触发回填，HEAD 无数据体）。
    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        match self.primary.head(path).await {
            Ok(h) => Ok(h),
            Err(StoreError::NotFound { .. }) => self.fallback.head(path).await,
            Err(e) => Err(e),
        }
    }

    /// 存在性：目标或源任一存在即存在。
    async fn exists(&self, path: &str) -> StoreResult<bool> {
        Ok(self.primary.exists(path).await? || self.fallback.exists(path).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FsObjectStore;

    fn pair() -> (tempfile::TempDir, FallbackObjectStore) {
        let dir = tempfile::tempdir().unwrap();
        let primary = Arc::new(FsObjectStore::new(dir.path().join("primary")).unwrap());
        let fallback = Arc::new(FsObjectStore::new(dir.path().join("fallback")).unwrap());
        (dir, FallbackObjectStore::new(primary, fallback))
    }

    #[tokio::test]
    async fn fallback_reads_through_when_primary_empty() {
        let (_d, store) = pair();
        // 仅源有数据
        store
            .fallback()
            .put("doc.md", "text/markdown", Bytes::from_static(b"legacy content"))
            .await
            .unwrap();

        // 目标空 → 回源命中
        let data = store.get("doc.md").await.unwrap();
        assert_eq!(&data[..], b"legacy content");

        // 回填完成 → 目标已存在
        assert!(store.primary().exists("doc.md").await.unwrap());
        let direct = store.primary().get("doc.md").await.unwrap();
        assert_eq!(&direct[..], b"legacy content");
    }

    #[tokio::test]
    async fn fallback_primary_hit_short_circuits() {
        let (_d, store) = pair();
        store
            .primary()
            .put("a.txt", "text/plain", Bytes::from_static(b"newer"))
            .await
            .unwrap();
        store
            .fallback()
            .put("a.txt", "text/plain", Bytes::from_static(b"older"))
            .await
            .unwrap();
        // 目标命中优先，不读源
        let data = store.get("a.txt").await.unwrap();
        assert_eq!(&data[..], b"newer");
    }

    #[tokio::test]
    async fn fallback_delete_purges_both() {
        let (_d, store) = pair();
        for side in [store.primary(), store.fallback()] {
            side.put("x", "text/plain", Bytes::from_static(b"x")).await.unwrap();
        }
        store.delete("x").await.unwrap();
        assert!(!store.exists("x").await.unwrap());
        assert!(!store.primary().exists("x").await.unwrap());
        assert!(!store.fallback().exists("x").await.unwrap());
    }

    #[tokio::test]
    async fn fallback_missing_everywhere_is_not_found() {
        let (_d, store) = pair();
        assert!(matches!(store.get("nope").await, Err(StoreError::NotFound { .. })));
        assert!(matches!(store.head("nope").await, Err(StoreError::NotFound { .. })));
    }
}
