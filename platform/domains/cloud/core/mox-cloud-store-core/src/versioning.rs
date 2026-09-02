// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 对象版本管理（`versions/<fileIdHash>/vN.json`，零拷贝恢复）。
//!
//! 设计要点：
//! - **内容寻址复用去重**：每个版本通过内部对象路径
//!   `__versions__/<fileId>/vN` 引用内容块（引用计数 +1），因此被 GC
//!   视为已引用而免于回收；相同内容的多个版本共享同一 chunk。
//! - **零拷贝恢复**：`restore` 仅新增一条指向同一 SHA-256 的对象元数据
//!   （`put_ref`），不复制数据。
//! - **防穿越**：版本目录名使用 `sha256(fileId)`，免疫 `../` 路径穿越。

use crate::fs_backend::{atomic_write, FsObjectStore, KeyPathCodec};
use crate::sha256_hex;
use bytes::Bytes;
use mox_base_store_core::{BlobObject, ObjectStore, StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 版本元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub file_id: String,
    pub version: u64,
    /// 内容寻址哈希（与对象共享，零拷贝）
    pub sha256: String,
    pub size_bytes: u64,
    pub content_type: String,
    pub created_ms: u64,
    /// 业务附加元数据（如版本说明）
    #[serde(default)]
    pub meta: serde_json::Value,
}

/// 版本管理器
#[derive(Clone)]
pub struct VersionManager {
    versions_dir: PathBuf,
}

impl VersionManager {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            versions_dir: data_dir.into().join("versions"),
        }
    }

    /// 版本文件路径（fileId 哈希化防穿越）
    fn version_path(&self, file_id: &str, version: u64) -> PathBuf {
        self.versions_dir
            .join(sha256_hex(file_id.as_bytes()).get(..16).unwrap_or("0000000000000000"))
            .join(format!("v{version}.json"))
    }

    /// 下一版本号（max + 1，从 1 开始）
    async fn next_version(&self, file_id: &str) -> StoreResult<u64> {
        let dir = self.versions_dir.join(
            sha256_hex(file_id.as_bytes()).get(..16).unwrap_or("0000000000000000"),
        );
        let mut max = 0u64;
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => return Ok(1),
        };
        while let Ok(Some(ent)) = rd.next_entry().await {
            let fname = ent.file_name().to_string_lossy().into_owned();
            if let Some(rest) = fname.strip_prefix('v').and_then(|s| s.strip_suffix(".json")) {
                if let Ok(n) = rest.parse::<u64>() {
                    max = max.max(n);
                }
            }
        }
        Ok(max + 1)
    }

    /// 保存新版本：内容入存储（引用计数 +1），再落版本元数据。
    pub async fn save_version(
        &self,
        store: &FsObjectStore,
        file_id: &str,
        content_type: &str,
        data: Bytes,
        meta: serde_json::Value,
    ) -> StoreResult<VersionInfo> {
        let version = self.next_version(file_id).await?;
        // 内容寻址写入内部版本路径（独立对象引用 → GC 安全）
        let internal = format!("__versions__/{file_id}/v{version}");
        let obj = store.put(&internal, content_type, data).await?;
        let sha256 = obj.sha256.ok_or_else(|| StoreError::Other("缺少 sha256".into()))?;
        let info = VersionInfo {
            file_id: file_id.to_string(),
            version,
            sha256,
            size_bytes: obj.size_bytes,
            content_type: content_type.to_string(),
            created_ms: now_ms(),
            meta,
        };
        let p = self.version_path(file_id, version);
        atomic_write(&p, &serde_json::to_vec(&info).map_err(|e| StoreError::Other(e.to_string()))?).await?;
        Ok(info)
    }

    /// 列出某文件全部版本（按版本号升序）
    pub async fn list_versions(&self, file_id: &str) -> StoreResult<Vec<VersionInfo>> {
        let dir = self.versions_dir.join(
            sha256_hex(file_id.as_bytes()).get(..16).unwrap_or("0000000000000000"),
        );
        let mut out = Vec::new();
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => return Ok(out),
        };
        while let Ok(Some(ent)) = rd.next_entry().await {
            let p = ent.path();
            let raw = match tokio::fs::read(&p).await {
                Ok(r) => r,
                Err(_) => continue,
            };
            if let Ok(info) = serde_json::from_slice::<VersionInfo>(&raw) {
                out.push(info);
            }
        }
        out.sort_by_key(|v| v.version);
        Ok(out)
    }

    /// 读取指定版本元数据
    pub async fn get_version(&self, file_id: &str, version: u64) -> StoreResult<VersionInfo> {
        let p = self.version_path(file_id, version);
        let raw = tokio::fs::read(&p)
            .await
            .map_err(|_| StoreError::NotFound { path: p.to_string_lossy().into_owned() })?;
        serde_json::from_slice(&raw).map_err(|e| StoreError::Checksum(format!("版本元数据解析失败: {e}")))
    }

    /// 零拷贝恢复：把指定版本恢复到目标路径（仅新增引用，不复制数据）。
    pub async fn restore(
        &self,
        store: &FsObjectStore,
        file_id: &str,
        version: u64,
        target_path: &str,
    ) -> StoreResult<BlobObject> {
        let info = self.get_version(file_id, version).await?;
        store.put_ref(target_path, &info.content_type, &info.sha256, info.size_bytes).await
    }

    /// 删除指定版本：移除内部对象引用（引用计数 -1）+ 删除版本元数据。
    pub async fn delete_version(
        &self,
        store: &FsObjectStore,
        file_id: &str,
        version: u64,
    ) -> StoreResult<()> {
        let p = self.version_path(file_id, version);
        if tokio::fs::try_exists(&p).await.unwrap_or(false) {
            let internal = format!("__versions__/{file_id}/v{version}");
            if store.exists(&internal).await? {
                store.delete(&internal).await?;
            }
            let _ = tokio::fs::remove_file(&p).await;
        }
        Ok(())
    }

    /// 删除某文件的全部版本（含内部对象引用）
    pub async fn delete_all_versions(
        &self,
        store: &FsObjectStore,
        file_id: &str,
    ) -> StoreResult<()> {
        let versions = self.list_versions(file_id).await?;
        for v in versions {
            self.delete_version(store, file_id, v.version).await?;
        }
        let dir = self.versions_dir.join(
            sha256_hex(file_id.as_bytes()).get(..16).unwrap_or("0000000000000000"),
        );
        let _ = tokio::fs::remove_dir_all(&dir).await;
        Ok(())
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 内部版本对象路径（供 GC/审计复用）
pub fn internal_version_path(file_id: &str, version: u64) -> String {
    format!("__versions__/{file_id}/v{version}")
}

/// 版本文件所在目录（供运维工具使用）
pub fn versions_root(data_dir: &Path) -> PathBuf {
    data_dir.join("versions")
}

/// chunk 数据文件路径（复用 KeyPathCodec，供版本审计直接定位数据）
pub fn version_chunk_path(data_dir: &Path, sha: &str) -> PathBuf {
    KeyPathCodec::chunk_path(data_dir, sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn save_list_restore_delete_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        let vm = VersionManager::new(dir.path());

        let v1 = vm
            .save_version(&store, "doc-1", "text/markdown", Bytes::from_static(b"version one"), serde_json::json!({"note":"初始"}))
            .await
            .unwrap();
        assert_eq!(v1.version, 1);
        let v2 = vm
            .save_version(&store, "doc-1", "text/markdown", Bytes::from_static(b"version two"), serde_json::json!({"note":"更新"}))
            .await
            .unwrap();
        assert_eq!(v2.version, 2);

        let list = vm.list_versions("doc-1").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].version, 1);
        assert_eq!(list[1].version, 2);

        // 零拷贝恢复 v1 到工作路径
        let restored = vm.restore(&store, "doc-1", 1, "kb/doc-1.md").await.unwrap();
        assert_eq!(restored.sha256.as_deref(), Some(v1.sha256.as_str()));
        let got = store.get("kb/doc-1.md").await.unwrap();
        assert_eq!(&got[..], b"version one");
        // 恢复后 chunk 引用计数 +1（仍共享）
        let rc = store.refcount(&v1.sha256).await.unwrap();
        assert!(rc >= 2, "v1 内部引用 + 恢复对象引用，应 ≥2，实际 {rc}");

        // 删除 v1：内部引用 -1，数据仍在（v2/恢复对象仍引用）
        vm.delete_version(&store, "doc-1", 1).await.unwrap();
        assert!(vm.get_version("doc-1", 1).await.is_err());
        assert_eq!(&store.get("kb/doc-1.md").await.unwrap()[..], b"version one");
    }

    #[tokio::test]
    async fn identical_versions_dedup_share_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        let vm = VersionManager::new(dir.path());
        let a = Bytes::from_static(b"same bytes");
        vm.save_version(&store, "f", "text/plain", a.clone(), serde_json::json!({})).await.unwrap();
        vm.save_version(&store, "f", "text/plain", a, serde_json::json!({})).await.unwrap();
        let rc = store.refcount(&sha256_hex(b"same bytes")).await.unwrap();
        assert_eq!(rc, 2, "相同内容版本共享 chunk，引用计数应 = 2");
    }

    #[tokio::test]
    async fn delete_all_versions_cleans_refs() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        let vm = VersionManager::new(dir.path());
        let a = Bytes::from_static(b"v-data");
        vm.save_version(&store, "g", "text/plain", a.clone(), serde_json::json!({})).await.unwrap();
        vm.save_version(&store, "g", "text/plain", Bytes::from_static(b"v2-data"), serde_json::json!({})).await.unwrap();
        vm.delete_all_versions(&store, "g").await.unwrap();
        let rc1 = store.refcount(&sha256_hex(b"v-data")).await.unwrap();
        let rc2 = store.refcount(&sha256_hex(b"v2-data")).await.unwrap();
        assert_eq!(rc1, 0);
        assert_eq!(rc2, 0);
        assert!(vm.list_versions("g").await.unwrap().is_empty());
    }
}
