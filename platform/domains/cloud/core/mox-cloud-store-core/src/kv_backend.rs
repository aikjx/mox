// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KV 存储持久化后端：原子 JSON 落盘（`kv/<keyhash>.json`）。
//!
//! 实现 [`mox_base_store_core::KvStore`] 物理口，供桶元数据/索引使用。
//! 全部写操作走 [`atomic_write`]，杜绝崩溃截断。

use crate::fs_backend::atomic_write;
use crate::sha256_hex;
use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{KvStore, StoreError, StoreResult};
use std::path::PathBuf;

/// 基于原子 JSON 的 KV 存储
#[derive(Clone)]
pub struct FsKvStore {
    root: PathBuf,
}

impl FsKvStore {
    pub fn new(root: impl Into<PathBuf>) -> StoreResult<Self> {
        let root: PathBuf = root.into();
        std::fs::create_dir_all(&root)
            .map_err(|e| StoreError::Io(format!("初始化 kv 目录失败 {}: {e}", root.display())))?;
        Ok(Self { root })
    }

    /// 数据根目录（供上层创建附属目录）
    pub fn data_dir(&self) -> &PathBuf {
        &self.root
    }

    fn value_path(&self, key: &str) -> PathBuf {
        let h = sha256_hex(key.as_bytes());
        self.root.join(h)
    }

    /// 列出全部 key（用于迁移/审计）
    pub async fn list_keys(&self) -> StoreResult<Vec<String>> {
        let mut out = Vec::new();
        let mut rd = tokio::fs::read_dir(&self.root)
            .await
            .map_err(|e| StoreError::Io(format!("读 kv 目录失败: {e}")))?;
        while let Ok(Some(ent)) = rd.next_entry().await {
            let fname = ent.file_name().to_string_lossy().into_owned();
            if !fname.starts_with(".tmp-") {
                out.push(fname);
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl KvStore for FsKvStore {
    async fn put(&self, key: &str, value: Bytes) -> StoreResult<()> {
        atomic_write(&self.value_path(key), &value).await
    }

    async fn get(&self, key: &str) -> StoreResult<Option<Bytes>> {
        let p = self.value_path(key);
        match tokio::fs::read(&p).await {
            Ok(raw) => Ok(Some(Bytes::from(raw))),
            Err(_) => Ok(None),
        }
    }

    async fn delete(&self, key: &str) -> StoreResult<()> {
        let p = self.value_path(key);
        match tokio::fs::remove_file(&p).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StoreError::Io(format!("删除 kv {} 失败: {e}", p.display()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kv_roundtrip_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        {
            let kv = FsKvStore::new(dir.path().join("kv")).unwrap();
            kv.put("bucket:docs", Bytes::from_static(b"{}")).await.unwrap();
            kv.put("bucket:files", Bytes::from_static(b"[]")).await.unwrap();
        }
        let kv = FsKvStore::new(dir.path().join("kv")).unwrap();
        assert_eq!(&kv.get("bucket:docs").await.unwrap().unwrap()[..], b"{}");
        assert_eq!(&kv.get("bucket:files").await.unwrap().unwrap()[..], b"[]");
        assert_eq!(kv.get("nope").await.unwrap(), None);
        kv.delete("bucket:docs").await.unwrap();
        assert_eq!(kv.get("bucket:docs").await.unwrap(), None);
    }
}
