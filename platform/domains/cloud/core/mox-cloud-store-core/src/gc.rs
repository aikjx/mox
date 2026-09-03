// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 引用计数 GC（规范 §3.5 / §5.4）。
//!
//! GC 以**对象元数据扫描为准**（source of truth）：
//! 1. 扫描 `objects/` 得到被引用 chunk 集合；
//! 2. 重建 `refs/` 索引修正漂移；
//! 3. 扫描 `chunks/`，未被引用且超过宽限期（grace）的物理删除；
//!    未超宽限期的计为 soft-purged（保留）。
//!
//! 支持 `dry_run`（只报告不动数据），供运维预览。

use crate::dedup::{list_chunks, list_object_refs, ChunkRefManager};
use crate::fs_backend::KeyPathCodec;
use mox_base_store_core::StoreResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// GC 报告
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GCReport {
    pub chunks_scanned: u64,
    pub soft_purged: u64,
    pub hard_deleted: u64,
    pub bytes_freed: u64,
    pub warnings: Vec<String>,
}

/// 引用计数 GC
#[derive(Clone)]
pub struct GarbageCollector {
    data_dir: PathBuf,
    /// 宽限期（秒）：引用为 0 后保留时长，默认 30 天
    pub grace_secs: u64,
}

impl GarbageCollector {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            grace_secs: 30 * 24 * 3600,
        }
    }

    /// 指定宽限期（秒）构造，供测试 / 运维快速回收
    pub fn with_grace(data_dir: impl Into<PathBuf>, grace_secs: u64) -> Self {
        Self {
            data_dir: data_dir.into(),
            grace_secs,
        }
    }

    /// 执行 GC；`dry_run=true` 仅报告
    pub async fn collect(&self, dry_run: bool) -> StoreResult<GCReport> {
        let mut report = GCReport::default();

        // 1. 以对象元数据为准，收集被引用 chunk
        let refs = list_object_refs(&self.data_dir).await?;
        let mut referenced: HashSet<String> = HashSet::with_capacity(refs.len());
        for (_path, sha) in &refs {
            referenced.insert(sha.clone());
        }

        // 2. 重建索引（修正漂移）
        let chunks = list_chunks(&self.data_dir).await?;
        let chunk_refs = ChunkRefManager::new(self.data_dir.clone());
        let rebuild = chunk_refs
            .rebuild_from_objects(&referenced, chunks.len() as u64)
            .await?;
        if rebuild.rebuilt_entries > 0 {
            report.warnings.push(format!("索引漂移已修复 {} 项", rebuild.rebuilt_entries));
        }
        if rebuild.removed_stale_entries > 0 {
            report.warnings.push(format!("清除失效索引 {} 项", rebuild.removed_stale_entries));
        }

        // 3. 扫描 chunk，回收未引用项
        report.chunks_scanned = chunks.len() as u64;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        for sha in &chunks {
            let rc = chunk_refs.refcount(sha).await?;
            if rc > 0 {
                continue;
            }
            let cp = KeyPathCodec::chunk_path(&self.data_dir, sha);
            let size = tokio::fs::metadata(&cp).await.map(|m| m.len()).unwrap_or(0);
            let age = tokio::fs::metadata(&cp)
                .await
                .and_then(|m| m.modified().map_err(std::io::Error::other))
                .map(|t| {
                    t.duration_since(UNIX_EPOCH)
                        .map(|d| now.saturating_sub(d.as_secs()))
                        .unwrap_or(0)
                })
                .unwrap_or(0);

            if age >= self.grace_secs {
                if !dry_run {
                    // 先删 chunk，再删 ref 文件（chunk 无引用，删 ref 失败不影响正确性）
                    let _ = tokio::fs::remove_file(&cp).await;
                    let rp = KeyPathCodec::ref_path(&self.data_dir, sha);
                    let _ = tokio::fs::remove_file(&rp).await;
                }
                report.hard_deleted += 1;
                report.bytes_freed += size;
            } else {
                report.soft_purged += 1;
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FsObjectStore;
    use bytes::Bytes;
    use mox_base_store_core::ObjectStore;

    #[tokio::test]
    async fn gc_removes_unreferenced_chunks_after_grace() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();

        // 写两个对象：一个删除（引用为0），一个保留
        store.put("keep.txt", "text/plain", Bytes::from_static(b"keep me")).await.unwrap();
        store.put("gone.txt", "text/plain", Bytes::from_static(b"delete me")).await.unwrap();
        store.delete("gone.txt").await.unwrap();

        // 宽限期为 0 以便立即回收
        let gc = GarbageCollector { data_dir: dir.path().to_path_buf(), grace_secs: 0 };

        // dry-run：只报告
        let report = gc.collect(true).await.unwrap();
        assert_eq!(report.hard_deleted, 1, "dry-run 不应删除，但应识别 1 个候选：{:?}", report);
        assert_eq!(report.soft_purged, 0);

        // 实际执行
        let report = gc.collect(false).await.unwrap();
        assert_eq!(report.hard_deleted, 1);
        assert!(report.bytes_freed >= 9);

        // 保留对象仍可读
        let got = store.get("keep.txt").await.unwrap();
        assert_eq!(&got[..], b"keep me");
        // 已删除的 chunk 不再存在
        let sha = crate::sha256_hex(b"delete me");
        assert!(!tokio::fs::try_exists(store.chunk_path(&sha)).await.unwrap());
    }

    #[tokio::test]
    async fn gc_keeps_shared_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        store.put("a.txt", "text/plain", Bytes::from_static(b"shared")).await.unwrap();
        store.put("b.txt", "text/plain", Bytes::from_static(b"shared")).await.unwrap();
        store.delete("a.txt").await.unwrap();

        let gc = GarbageCollector { data_dir: dir.path().to_path_buf(), grace_secs: 0 };
        let report = gc.collect(false).await.unwrap();
        assert_eq!(report.hard_deleted, 0, "共享 chunk 引用仍为 1，不应删除：{:?}", report);
        // b.txt 仍可读
        assert_eq!(&store.get("b.txt").await.unwrap()[..], b"shared");
    }

    #[tokio::test]
    async fn gc_dry_run_is_non_destructive() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        store.put("x.txt", "text/plain", Bytes::from_static(b"x")).await.unwrap();
        store.delete("x.txt").await.unwrap();
        let gc = GarbageCollector { data_dir: dir.path().to_path_buf(), grace_secs: 0 };
        let report = gc.collect(true).await.unwrap();
        assert_eq!(report.hard_deleted, 1);
        // dry-run 后 chunk 仍存在
        let sha = crate::sha256_hex(b"x");
        assert!(tokio::fs::try_exists(store.chunk_path(&sha)).await.unwrap());
    }
}
