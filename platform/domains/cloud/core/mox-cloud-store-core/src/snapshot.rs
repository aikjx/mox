// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 对象快照（阶段3，feature `erasure`）——COW 语义的隔离副本。
//!
//! [`SnapshotManager`] 对一组对象创建快照：将对象**完整复制**到
//! `__snap/<id><path>`（与生产对象隔离，不受后续覆盖/删除影响），
//! 支持按快照恢复。
//!
//! - manifest：`__snap/<id>/m.json`（提交点，含路径清单/标签/时间戳）。
//! - 索引：进程内 `Mutex<HashMap>` + 可选 `data_dir` 扫描（FS 后端跨重启恢复）。
//! - 恢复：读取快照副本逐对象返回，不触碰生产路径（调用方自行选择写回）。

use crate::{list_object_refs, StoreError, StoreResult};
use bytes::Bytes;
use mox_base_store_core::ObjectStore;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 快照逻辑目录前缀。
pub const SNAP_PREFIX: &str = "__snap";

/// 快照元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// 快照 ID
    pub id: String,
    /// 标签
    pub tag: String,
    /// 快照覆盖的对象路径
    pub paths: Vec<String>,
    /// 创建时间戳 ms
    pub created_ms: u64,
}

/// 对象快照管理器。
pub struct SnapshotManager {
    inner: Arc<dyn ObjectStore>,
    /// FS 数据目录（存在时支持跨重启扫描索引）
    data_dir: Option<PathBuf>,
    /// 进程内索引
    index: Mutex<HashMap<String, SnapshotInfo>>,
}

impl SnapshotManager {
    /// 构造快照管理器。
    pub fn new(inner: Arc<dyn ObjectStore>, data_dir: Option<PathBuf>) -> Self {
        Self {
            inner,
            data_dir,
            index: Mutex::new(HashMap::new()),
        }
    }

    /// 快照 manifest 逻辑 key。
    pub fn manifest_path(id: &str) -> String {
        format!("{SNAP_PREFIX}/{id}/m.json")
    }

    /// 快照内对象副本的逻辑 key。
    pub fn snapshot_object_path(id: &str, path: &str) -> String {
        format!("{SNAP_PREFIX}/{id}{path}")
    }

    /// 创建快照（跳过不存在对象）。
    pub async fn create_snapshot(
        &self,
        id: &str,
        tag: &str,
        paths: &[String],
    ) -> StoreResult<SnapshotInfo> {
        let mut copied = Vec::new();
        for p in paths {
            match self.inner.get(p).await {
                Ok(data) => {
                    let dst = Self::snapshot_object_path(id, p);
                    self.inner
                        .put(&dst, "application/octet-stream", data)
                        .await?;
                    copied.push(p.clone());
                }
                Err(StoreError::NotFound { .. }) => {
                    // 跳过不存在对象
                }
                Err(e) => return Err(e),
            }
        }
        let info = SnapshotInfo {
            id: id.to_string(),
            tag: tag.to_string(),
            paths: copied,
            created_ms: now_ms(),
        };
        let raw = serde_json::to_vec(&info)
            .map_err(|e| StoreError::Other(format!("快照 manifest 序列化失败: {e}")))?;
        self.inner
            .put(&Self::manifest_path(id), "application/json", Bytes::from(raw))
            .await?;
        self.index.lock().insert(id.to_string(), info.clone());
        Ok(info)
    }

    /// 读取快照元数据。
    pub async fn get_snapshot(&self, id: &str) -> StoreResult<SnapshotInfo> {
        if let Some(info) = self.index.lock().get(id) {
            return Ok(info.clone());
        }
        let mp = Self::manifest_path(id);
        let b = self
            .inner
            .get(&mp)
            .await
            .map_err(|_| StoreError::NotFound { path: mp })?;
        let info = serde_json::from_slice::<SnapshotInfo>(&b)
            .map_err(|e| StoreError::Checksum(format!("快照 manifest 解析失败: {e}")))?;
        Ok(info)
    }

    /// 列出全部快照（进程内索引 + FS 扫描补充）。
    pub async fn list_snapshots(&self) -> StoreResult<Vec<SnapshotInfo>> {
        let mut out: Vec<SnapshotInfo> = self.index.lock().values().cloned().collect();
        // FS 后端：扫描 __snap/ 下 manifest 补充跨重启恢复
        if let Some(dir) = &self.data_dir {
            if let Ok(refs) = list_object_refs(dir).await {
                let mut seen: std::collections::HashSet<String> =
                    out.iter().map(|s| s.id.clone()).collect();
                for (path, _) in refs {
                    if let Some(id) = Self::parse_manifest_id(&path) {
                        if seen.contains(&id) {
                            continue;
                        }
                        if let Ok(info) = self.get_snapshot(&id).await {
                            seen.insert(id);
                            out.push(info);
                        }
                    }
                }
            }
        }
        out.sort_by_key(|s| s.created_ms);
        Ok(out)
    }

    /// 恢复快照：读取全部对象副本，返回 (path, data) 列表。
    pub async fn restore_snapshot(&self, id: &str) -> StoreResult<Vec<(String, Bytes)>> {
        let info = self.get_snapshot(id).await?;
        let mut out = Vec::with_capacity(info.paths.len());
        for p in &info.paths {
            let src = Self::snapshot_object_path(id, p);
            let data = self
                .inner
                .get(&src)
                .await
                .map_err(|_| StoreError::NotFound { path: src })?;
            out.push((p.clone(), data));
        }
        Ok(out)
    }

    /// 删除快照（清理副本 + manifest + 索引）。
    pub async fn delete_snapshot(&self, id: &str) -> StoreResult<()> {
        let info = self.get_snapshot(id).await?;
        for p in &info.paths {
            let _ = self.inner.delete(&Self::snapshot_object_path(id, p)).await;
        }
        let _ = self.inner.delete(&Self::manifest_path(id)).await;
        self.index.lock().remove(id);
        Ok(())
    }

    /// 从 `__snap/<id>/m.json` 路径提取快照 id。
    fn parse_manifest_id(path: &str) -> Option<String> {
        let prefix = format!("{SNAP_PREFIX}/");
        let suffix = "/m.json";
        if !path.starts_with(&prefix) || !path.ends_with(suffix) {
            return None;
        }
        let mid = &path[prefix.len()..path.len() - suffix.len()];
        if mid.is_empty() || mid.contains('/') {
            return None;
        }
        Some(mid.to_string())
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_backend::FsObjectStore;
    use std::path::Path;

    fn base(dir: &Path) -> Arc<dyn ObjectStore> {
        Arc::new(FsObjectStore::new(dir.to_path_buf()).unwrap())
    }

    #[tokio::test]
    async fn create_restore_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let store = base(dir.path());
        store.put("docs/a.md", "text/markdown", Bytes::from_static(b"v1")).await.unwrap();
        store.put("docs/b.md", "text/markdown", Bytes::from_static(b"v1-b")).await.unwrap();

        let sm = SnapshotManager::new(store.clone(), Some(dir.path().to_path_buf()));
        let info = sm
            .create_snapshot(
                "snap1",
                "baseline",
                &["docs/a.md".into(), "docs/b.md".into(), "docs/missing.md".into()],
            )
            .await
            .unwrap();
        assert_eq!(info.paths.len(), 2, "缺失对象应被跳过");

        // 修改原对象
        store.put("docs/a.md", "text/markdown", Bytes::from_static(b"v2-changed")).await.unwrap();

        // 恢复快照 → 得到 v1 内容
        let restored = sm.restore_snapshot("snap1").await.unwrap();
        assert_eq!(restored.len(), 2);
        let a = restored.iter().find(|(p, _)| p == "docs/a.md").unwrap();
        assert_eq!(&a.1[..], b"v1");
        let b = restored.iter().find(|(p, _)| p == "docs/b.md").unwrap();
        assert_eq!(&b.1[..], b"v1-b");

        // list
        let list = sm.list_snapshots().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "snap1");

        // 生产对象未被快照污染
        assert_eq!(&store.get("docs/a.md").await.unwrap()[..], b"v2-changed");
    }

    #[tokio::test]
    async fn delete_snapshot_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let store = base(dir.path());
        store.put("x.bin", "x", Bytes::from_static(b"data")).await.unwrap();
        let sm = SnapshotManager::new(store.clone(), Some(dir.path().to_path_buf()));
        sm.create_snapshot("snap2", "t", &["x.bin".into()]).await.unwrap();
        assert!(store
            .exists(&SnapshotManager::snapshot_object_path("snap2", "x.bin"))
            .await
            .unwrap());
        sm.delete_snapshot("snap2").await.unwrap();
        assert!(!store
            .exists(&SnapshotManager::snapshot_object_path("snap2", "x.bin"))
            .await
            .unwrap());
        assert!(sm.list_snapshots().await.unwrap().is_empty());
    }
}
