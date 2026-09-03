// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 存储统计（管理面 status/stats 数据来源）。
//!
//! 扫描 data_dir 的 `objects/`、`chunks/`、`refs/`、`versions/`、`kv/`
//! 五个目录，计算对象数、唯一块数、物理占用、逻辑体积与**去重率**
//! （逻辑体积 / 物理占用，≥1）。

use crate::dedup::{entry_is_dir, list_chunks, list_object_refs};
use crate::fs_backend::KeyPathCodec;
use mox_base_store_core::StoreResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 存储统计快照
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreStats {
    /// 对象元数据数（去重前逻辑对象数）
    pub object_count: u64,
    /// 内容寻址唯一块数（去重后）
    pub chunk_count: u64,
    /// 版本元数据数
    pub version_count: u64,
    /// KV 条目数
    pub kv_count: u64,
    /// 数据块物理占用（去重后字节）
    pub chunks_bytes: u64,
    /// 逻辑体积（∑ refcount × 块大小；去重前近似）
    pub logical_bytes: u64,
    /// 引用计数总量
    pub ref_total: u64,
}

impl StoreStats {
    /// 去重率 = 逻辑体积 / 物理占用（≥1）
    pub fn dedup_ratio(&self) -> f64 {
        if self.chunks_bytes == 0 {
            return 1.0;
        }
        (self.logical_bytes as f64 / self.chunks_bytes as f64).max(1.0)
    }
}

/// 扫描 data_dir 统计存储规模。
///
/// 数据源均来自磁盘布局（`objects/chunks/refs/versions/kv`），
/// 与 [`crate::GarbageCollector`] 共享同一套扫描语义，保证指标一致。
pub async fn collect_store_stats(data_dir: &Path) -> StoreResult<StoreStats> {
    let mut st = StoreStats::default();

    // 1. 对象元数据
    let objs = list_object_refs(data_dir).await?;
    st.object_count = objs.len() as u64;

    // 2. 唯一块 + 引用计数（逻辑体积）
    let chunks = list_chunks(data_dir).await?;
    st.chunk_count = chunks.len() as u64;
    for sha in &chunks {
        let cp = KeyPathCodec::chunk_path(data_dir, sha);
        let size = tokio::fs::metadata(&cp).await.map(|m| m.len()).unwrap_or(0);
        st.chunks_bytes += size;
        let rp = KeyPathCodec::ref_path(data_dir, sha);
        if let Ok(raw) = tokio::fs::read(&rp).await {
            if let Ok(e) = serde_json::from_slice::<crate::dedup::RefEntry>(&raw) {
                st.ref_total += e.refcount;
                st.logical_bytes += size * e.refcount;
            }
        }
    }

    // 3. 版本 / KV
    st.version_count = count_files(&data_dir.join("versions")).await;
    st.kv_count = count_files(&data_dir.join("kv")).await;

    Ok(st)
}

/// 递归统计目录下文件数
async fn count_files(root: &Path) -> u64 {
    let mut n = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        while let Ok(Some(ent)) = rd.next_entry().await {
            if entry_is_dir(&ent).await {
                stack.push(ent.path());
            } else {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FsObjectStore;
    use bytes::Bytes;
    use mox_base_store_core::ObjectStore;

    #[tokio::test]
    async fn stats_reflects_dedup_and_versions() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();

        // 两个对象共享同一内容 → 1 个唯一块，引用计数 2
        store
            .put("a.txt", "text/plain", Bytes::from_static(b"same-content"))
            .await
            .unwrap();
        store
            .put("b.txt", "text/plain", Bytes::from_static(b"same-content"))
            .await
            .unwrap();
        store
            .put("c.bin", "application/octet-stream", Bytes::from_static(b"0123456789"))
            .await
            .unwrap();

        let st = collect_store_stats(dir.path()).await.unwrap();
        assert_eq!(st.object_count, 3);
        assert_eq!(st.chunk_count, 2, "去重后唯一块应为 2");
        assert_eq!(st.ref_total, 3);
        assert!(st.dedup_ratio() > 1.0, "共享内容应产生 >1 的去重率");
        assert!(st.chunks_bytes > 0);
    }

    #[tokio::test]
    async fn stats_empty_dir_is_zero() {
        let dir = tempfile::tempdir().unwrap();
        let st = collect_store_stats(dir.path()).await.unwrap();
        assert_eq!(st.object_count, 0);
        assert_eq!(st.chunk_count, 0);
        assert_eq!(st.dedup_ratio(), 1.0);
    }
}
