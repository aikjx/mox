// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! FS 后端：内容寻址（SHA-256 两级散列）去重存储，真实落盘。
//!
//! 实现 [`mox_base_store_core::ObjectStore`] / [`KvStore`] / [`ObjectStreamWriter`]
//! 三路物理口。所有写操作走原子写（tmp + rename），杜绝崩溃截断。
//! 对象数据以 `chunks/<xx>/<sha256>` 内容寻址存储（去重单元），对象元数据
//! 以 JSON 落盘于 `objects/<xx>/<keyhash>.json`，引用计数索引在 `refs/`。

use crate::dedup::ChunkRefManager;
use crate::kv_backend::FsKvStore;
use crate::{hash_prefix, key_file_name, sha256_hex};
use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{
    BlobObject, KvStore, ObjectStore, ObjectStreamWriter, StoreError, StoreResult, StreamHandle,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 对象元数据（落盘 JSON）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub path: String,
    pub content_type: String,
    pub size_bytes: u64,
    /// 内容寻址哈希（SHA-256 hex）
    pub sha256: String,
    /// 创建时间（epoch ms）
    pub created_ms: u64,
}

/// 逻辑 key ↔ 物理文件名的编解码器（防路径穿越）
///
/// 物理文件名 = `sha256(key).hex`，逻辑路径只存于元数据中。
pub struct KeyPathCodec;

impl KeyPathCodec {
    /// 对象元数据文件路径：`objects/<xx>/<keyhash>.obj`
    pub fn object_meta_path(data_dir: &Path, key: &str) -> PathBuf {
        let key_hash = key_file_name(key);
        data_dir
            .join("objects")
            .join(key_hash.get(..2).unwrap_or("00"))
            .join(key_hash)
    }

    /// chunk 数据文件路径：`chunks/<xx>/<sha256>`
    pub fn chunk_path(data_dir: &Path, sha: &str) -> PathBuf {
        data_dir
            .join("chunks")
            .join(hash_prefix(sha))
            .join(sha)
    }

    /// 引用计数文件路径：`refs/<xx>/<sha256>.json`
    pub fn ref_path(data_dir: &Path, sha: &str) -> PathBuf {
        data_dir
            .join("refs")
            .join(hash_prefix(sha))
            .join(format!("{sha}.json"))
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 原子写：先写临时文件再 rename，崩溃时不会留下半写文件。
pub async fn atomic_write(path: &Path, data: &[u8]) -> StoreResult<()> {
    let parent = path.parent().ok_or_else(|| StoreError::Other("路径无父目录".into()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| StoreError::Io(format!("创建目录失败 {}: {e}", parent.display())))?;
    let tmp = parent.join(format!(".tmp-{}-{}", std::process::id(), uuid::Uuid::new_v4()));
    tokio::fs::write(&tmp, data)
        .await
        .map_err(|e| StoreError::Io(format!("写临时文件失败 {}: {e}", tmp.display())))?;
    // 同目录 rename 在 Windows 与 POSIX 均为原子操作
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            StoreError::Io(format!("原子提交失败 {} → {}: {e}", tmp.display(), path.display()))
        })?;
    Ok(())
}

/// FS 对象存储（内容寻址 + 引用计数 + 原子写）
#[derive(Clone)]
pub struct FsObjectStore {
    data_dir: Arc<PathBuf>,
    chunk_refs: Arc<ChunkRefManager>,
    kv: Arc<FsKvStore>,
    /// 读时是否校验 SHA-256（默认 true）
    verify_checksum: bool,
}

impl FsObjectStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> StoreResult<Self> {
        Self::with_options(data_dir, true)
    }

    pub fn with_options(data_dir: impl Into<PathBuf>, verify_checksum: bool) -> StoreResult<Self> {
        let data_dir: PathBuf = data_dir.into();
        std::fs::create_dir_all(data_dir.join("objects"))
            .map_err(|e| StoreError::Io(format!("初始化 objects 目录失败: {e}")))?;
        std::fs::create_dir_all(data_dir.join("chunks"))
            .map_err(|e| StoreError::Io(format!("初始化 chunks 目录失败: {e}")))?;
        std::fs::create_dir_all(data_dir.join("refs"))
            .map_err(|e| StoreError::Io(format!("初始化 refs 目录失败: {e}")))?;
        std::fs::create_dir_all(data_dir.join("kv"))
            .map_err(|e| StoreError::Io(format!("初始化 kv 目录失败: {e}")))?;
        Ok(Self {
            chunk_refs: Arc::new(ChunkRefManager::new(data_dir.clone())),
            kv: Arc::new(FsKvStore::new(data_dir.join("kv"))?),
            data_dir: Arc::new(data_dir),
            verify_checksum,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// chunk 数据文件路径
    pub fn chunk_path(&self, sha: &str) -> PathBuf {
        KeyPathCodec::chunk_path(&self.data_dir, sha)
    }

    /// 查询某 chunk 的引用计数（供测试 / GC 预览 / 审计使用）
    pub async fn refcount(&self, sha: &str) -> StoreResult<u64> {
        self.chunk_refs.refcount(sha).await
    }

    async fn read_meta(&self, path: &str) -> StoreResult<ObjectMeta> {
        let mp = KeyPathCodec::object_meta_path(&self.data_dir, path);
        let raw = tokio::fs::read(&mp)
            .await
            .map_err(|_| StoreError::NotFound { path: path.to_string() })?;
        serde_json::from_slice(&raw)
            .map_err(|e| StoreError::Checksum(format!("元数据解析失败 {}: {e}", mp.display())))
    }

    async fn write_chunk_if_absent(&self, sha: &str, data: &[u8]) -> StoreResult<()> {
        let cp = self.chunk_path(sha);
        if tokio::fs::try_exists(&cp).await.unwrap_or(false) {
            return Ok(());
        }
        atomic_write(&cp, data).await
    }

    /// 将一段数据按内容寻址写入并返回对象元数据
    async fn store_bytes(&self, path: &str, content_type: &str, data: &[u8]) -> StoreResult<BlobObject> {
        let sha = sha256_hex(data);
        self.write_chunk_if_absent(&sha, data).await?;
        self.chunk_refs.add_ref(&sha).await?;
        let meta = ObjectMeta {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes: data.len() as u64,
            sha256: sha.clone(),
            created_ms: now_ms(),
        };
        let mp = KeyPathCodec::object_meta_path(&self.data_dir, path);
        atomic_write(&mp, &serde_json::to_vec(&meta).map_err(|e| StoreError::Other(e.to_string()))?).await?;
        Ok(BlobObject {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes: meta.size_bytes,
            sha256: Some(sha),
        })
    }

    /// 零拷贝写入：以已有 chunk 哈希新增对象引用（不复制数据）。
    ///
    /// 供版本恢复 / 去重场景复用已存在内容块。
    pub async fn put_ref(
        &self,
        path: &str,
        content_type: &str,
        sha: &str,
        size_bytes: u64,
    ) -> StoreResult<BlobObject> {
        let cp = self.chunk_path(sha);
        if !tokio::fs::try_exists(&cp).await.unwrap_or(false) {
            return Err(StoreError::NotFound {
                path: format!("chunk:{sha}"),
            });
        }
        self.chunk_refs.add_ref(sha).await?;
        let meta = ObjectMeta {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes,
            sha256: sha.to_string(),
            created_ms: now_ms(),
        };
        let mp = KeyPathCodec::object_meta_path(&self.data_dir, path);
        atomic_write(&mp, &serde_json::to_vec(&meta).map_err(|e| StoreError::Other(e.to_string()))?).await?;
        Ok(BlobObject {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes,
            sha256: Some(sha.to_string()),
        })
    }

    /// 校验数据与期望哈希一致
    fn verify(&self, data: &[u8], expected: &str) -> StoreResult<()> {
        if !self.verify_checksum {
            return Ok(());
        }
        let actual = sha256_hex(data);
        if actual != expected {
            return Err(StoreError::Checksum(format!(
                "数据损坏：期望 {expected}，实际 {actual}"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl ObjectStore for FsObjectStore {
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        self.store_bytes(path, content_type, &data).await
    }

    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        let meta = self.read_meta(path).await?;
        let cp = self.chunk_path(&meta.sha256);
        let data = tokio::fs::read(&cp)
            .await
            .map_err(|_| StoreError::NotFound { path: format!("chunk:{}", meta.sha256) })?;
        self.verify(&data, &meta.sha256)?;
        Ok(Bytes::from(data))
    }

    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        let data = ObjectStore::get(self, path).await?;
        let start = offset as usize;
        let end = start.saturating_add(length as usize).min(data.len());
        if start >= data.len() {
            return Ok(Bytes::new());
        }
        Ok(data.slice(start..end))
    }

    async fn delete(&self, path: &str) -> StoreResult<()> {
        let mp = KeyPathCodec::object_meta_path(&self.data_dir, path);
        if !tokio::fs::try_exists(&mp).await.unwrap_or(false) {
            return Ok(()); // 幂等删除，对齐 S3 语义
        }
        let meta = self.read_meta(path).await?;
        self.chunk_refs.remove_ref(&meta.sha256).await?;
        tokio::fs::remove_file(&mp)
            .await
            .map_err(|e| StoreError::Io(format!("删除元数据失败 {}: {e}", mp.display())))?;
        Ok(())
    }

    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        let meta = self.read_meta(path).await?;
        Ok(BlobObject {
            path: meta.path,
            content_type: meta.content_type,
            size_bytes: meta.size_bytes,
            sha256: Some(meta.sha256),
        })
    }

    async fn exists(&self, path: &str) -> StoreResult<bool> {
        let mp = KeyPathCodec::object_meta_path(&self.data_dir, path);
        Ok(tokio::fs::try_exists(&mp).await.unwrap_or(false))
    }
}

#[async_trait]
impl KvStore for FsObjectStore {
    async fn put(&self, key: &str, value: Bytes) -> StoreResult<()> {
        self.kv.put(key, value).await
    }

    async fn get(&self, key: &str) -> StoreResult<Option<Bytes>> {
        self.kv.get(key).await
    }

    async fn delete(&self, key: &str) -> StoreResult<()> {
        self.kv.delete(key).await
    }
}

#[async_trait]
impl ObjectStreamWriter for FsObjectStore {
    async fn open_writer(&self, path: &str, _content_type: &str) -> StoreResult<StreamHandle> {
        let dir = self.data_dir.join("mpu");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| StoreError::Io(format!("创建 mpu 目录失败: {e}")))?;
        let tmp_path = dir.join(format!(
            "stream-{}-{}.tmp",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        Ok(StreamHandle {
            path: path.to_string(),
            state: tmp_path.to_string_lossy().into_owned(),
        })
    }

    async fn write(&self, handle: &StreamHandle, chunk: Bytes) -> StoreResult<()> {
        let tmp = PathBuf::from(&handle.state);
        use tokio::io::AsyncWriteExt;
        let mut f = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&tmp)
            .await
            .map_err(|e| StoreError::Io(format!("打开流文件失败 {}: {e}", tmp.display())))?;
        f.write_all(&chunk)
            .await
            .map_err(|e| StoreError::Io(format!("追加流数据失败: {e}")))?;
        f.flush().await.map_err(|e| StoreError::Io(format!("flush 失败: {e}")))?;
        Ok(())
    }

    async fn close(&self, handle: StreamHandle) -> StoreResult<BlobObject> {
        let tmp = PathBuf::from(&handle.state);
        let data = tokio::fs::read(&tmp)
            .await
            .map_err(|e| StoreError::Io(format!("读取流文件失败 {}: {e}", tmp.display())))?;
        let _ = tokio::fs::remove_file(&tmp).await;
        self.store_bytes(&handle.path, "application/octet-stream", &data).await
    }
}

// =============== 工具 ===============

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (tempfile::TempDir, FsObjectStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FsObjectStore::new(dir.path()).unwrap();
        (dir, store)
    }

    #[tokio::test]
    async fn put_get_head_exists_delete_roundtrip() {
        let (_d, store) = tmp_store();
        let data = Bytes::from_static(b"hello world");
        let obj = ObjectStore::put(&store, "kb/doc/a.md", "text/markdown", data.clone()).await.unwrap();
        assert_eq!(obj.size_bytes, 11);
        assert!(obj.sha256.is_some());

        let got = ObjectStore::get(&store, "kb/doc/a.md").await.unwrap();
        assert_eq!(&got[..], b"hello world");
        assert!(store.exists("kb/doc/a.md").await.unwrap());

        let h = store.head("kb/doc/a.md").await.unwrap();
        assert_eq!(h.content_type, "text/markdown");
        assert_eq!(h.size_bytes, 11);

        ObjectStore::delete(&store, "kb/doc/a.md").await.unwrap();
        assert!(!store.exists("kb/doc/a.md").await.unwrap());
        assert!(matches!(ObjectStore::get(&store, "kb/doc/a.md").await, Err(StoreError::NotFound { .. })));
    }

    #[tokio::test]
    async fn range_read_works() {
        let (_d, store) = tmp_store();
        ObjectStore::put(&store, "b.bin", "application/octet-stream", Bytes::from_static(b"0123456789")).await.unwrap();
        let part = store.get_range("b.bin", 2, 4).await.unwrap();
        assert_eq!(&part[..], b"2345");
        let tail = store.get_range("b.bin", 8, 100).await.unwrap();
        assert_eq!(&tail[..], b"89");
        let oob = store.get_range("b.bin", 99, 4).await.unwrap();
        assert!(oob.is_empty());
    }

    #[tokio::test]
    async fn atomic_write_leaves_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nested/dir/x.json");
        atomic_write(&p, br#"{"k":1}"#).await.unwrap();
        let raw = tokio::fs::read(&p).await.unwrap();
        assert_eq!(&raw[..], br#"{"k":1}"#);
        // 覆盖写也是原子的
        atomic_write(&p, br#"{"k":2}"#).await.unwrap();
        let raw = tokio::fs::read(&p).await.unwrap();
        assert_eq!(&raw[..], br#"{"k":2}"#);
    }

    #[tokio::test]
    async fn content_addressing_dedups_identical_bytes() {
        let (_d, store) = tmp_store();
        ObjectStore::put(&store, "a.txt", "text/plain", Bytes::from_static(b"same content")).await.unwrap();
        ObjectStore::put(&store, "b.txt", "text/plain", Bytes::from_static(b"same content")).await.unwrap();
        // 相同内容 → 同一 chunk 文件
        let sha = sha256_hex(b"same content");
        let cp = store.chunk_path(&sha);
        assert!(tokio::fs::try_exists(&cp).await.unwrap());
        // refcount = 2
        let rc = store.refcount(&sha).await.unwrap();
        assert_eq!(rc, 2);
    }

    #[tokio::test]
    async fn kv_store_persistence_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = FsObjectStore::new(dir.path()).unwrap();
            KvStore::put(&store, "meta:1", Bytes::from_static(b"v1")).await.unwrap();
            KvStore::put(&store, "meta:2", Bytes::from_static(b"v2")).await.unwrap();
        }
        let store = FsObjectStore::new(dir.path()).unwrap();
        assert_eq!(&KvStore::get(&store, "meta:1").await.unwrap().unwrap()[..], b"v1");
        assert_eq!(&KvStore::get(&store, "meta:2").await.unwrap().unwrap()[..], b"v2");
        assert_eq!(KvStore::get(&store, "meta:3").await.unwrap(), None);
    }

    #[tokio::test]
    async fn stream_writer_roundtrip() {
        let (_d, store) = tmp_store();
        let h = store.open_writer("big.bin", "application/octet-stream").await.unwrap();
        for part in [b"part1".as_slice(), b"-part2", b"-part3"] {
            store.write(&h, Bytes::copy_from_slice(part)).await.unwrap();
        }
        let obj = store.close(h).await.unwrap();
        assert_eq!(obj.size_bytes, 17);
        let got = ObjectStore::get(&store, "big.bin").await.unwrap();
        assert_eq!(&got[..], b"part1-part2-part3");
    }
}
