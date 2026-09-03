// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Reed-Solomon 纠删码装饰器（阶段3，feature `erasure`）。
//!
//! [`ErasureStore`] 是一个**后端不可知**的 [`ObjectStore`] 装饰器：包装任意底层
//! 存储（FS/S3），对达到阈值的大对象做 RS(n+k) 分片编码，小对象直接透传。
//!
//! 复用 `mox-cloud-volume-svc` 的 [`ReedSolomonEngine`]（GF(2^8) + SIMD）与
//! [`EcProfile`]，不重写引擎；本模块只负责"分片布局 + manifest + 容错重建"。
//!
//! ## 磁盘布局（逻辑 key，底层存储寻址）
//! ```text
//! __ec/<path>/m.json      # manifest（提交点：存在即 EC 模式）
//! __ec/<path>/s0 .. sN    # 数据 + 校验分片
//! ```
//! 普通对象仍以原始 `path` 存储；`get/head/exists` 以 manifest 存在性路由。
//!
//! ## 容错语义
//! 读时分片缺失（NotFound）或腐坏（crc32c 不匹配）→ 置 None →
//! `decode_reconstruct` 用剩余分片重建，丢 ≤ parity 片逐字节一致恢复。
//! 腐坏分片由 [`crate::heal`] 协调器写回底层完成自愈。

use crate::{sha256_hex, StoreError, StoreResult};
use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{BlobObject, ObjectStore};
use mox_cloud_volume_svc::{EcProfile, ReedSolomonEngine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// EC 分片逻辑目录前缀（与普通对象隔离）。
pub const EC_PREFIX: &str = "__ec";

/// ErasureStore 配置。
#[derive(Debug, Clone)]
pub struct ErasureConfig {
    /// 是否启用 EC（false → 纯透传装饰器，零开销）。
    pub enabled: bool,
    /// RS 参数（data/parity/min_size）。
    pub profile: EcProfile,
}

impl Default for ErasureConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: EcProfile::default(),
        }
    }
}

/// 存于底层的 EC manifest（提交点）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcManifestStore {
    /// 数据分片数
    pub data_shards: usize,
    /// 校验分片数
    pub parity_shards: usize,
    /// 原始字节数
    pub original_len: usize,
    /// 内容类型
    pub content_type: String,
    /// 每个分片的字节数
    pub shard_sizes: Vec<usize>,
    /// 每个分片的 crc32c（bitrot 检测依据）
    pub shard_crcs: Vec<u32>,
    /// 创建时间戳 ms
    pub created_ms: u64,
}

/// RS 纠删码对象存储装饰器。
pub struct ErasureStore {
    inner: Arc<dyn ObjectStore>,
    cfg: ErasureConfig,
    engine: ReedSolomonEngine,
}

impl ErasureStore {
    /// 包装底层存储。
    pub fn new(inner: Arc<dyn ObjectStore>, cfg: ErasureConfig) -> Self {
        Self {
            inner,
            cfg,
            engine: ReedSolomonEngine::new(),
        }
    }

    /// 底层存储引用（供 heal/管理面直连）。
    pub fn inner(&self) -> &Arc<dyn ObjectStore> {
        &self.inner
    }

    /// 当前 RS 参数。
    pub fn profile(&self) -> EcProfile {
        self.cfg.profile
    }

    /// EC 是否启用。
    pub fn enabled(&self) -> bool {
        self.cfg.enabled
    }

    /// 分片目录（逻辑 key）。
    pub fn ec_dir(path: &str) -> String {
        format!("{EC_PREFIX}/{path}")
    }

    /// manifest 逻辑 key。
    pub fn manifest_path(path: &str) -> String {
        format!("{}/m.json", Self::ec_dir(path))
    }

    /// 第 i 个分片的逻辑 key。
    pub fn shard_path(path: &str, i: usize) -> String {
        format!("{}/s{i}", Self::ec_dir(path))
    }

    /// 读取 EC manifest；非 EC 对象返回 Ok(None)。
    pub async fn read_manifest(&self, path: &str) -> StoreResult<Option<EcManifestStore>> {
        let mp = Self::manifest_path(path);
        match self.inner.get(&mp).await {
            Ok(b) => {
                let m = serde_json::from_slice::<EcManifestStore>(&b)
                    .map_err(|e| StoreError::Checksum(format!("manifest 解析失败 {mp}: {e}")))?;
                Ok(Some(m))
            }
            Err(StoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 读全部分片为 Option 槽位；缺失/腐坏 → None（交给重建）。
    async fn read_shards(&self, path: &str, total: usize, crcs: &[u32]) -> Vec<Option<Vec<u8>>> {
        let mut slots: Vec<Option<Vec<u8>>> = Vec::with_capacity(total);
        for i in 0..total {
            match self.inner.get(&Self::shard_path(path, i)).await {
                Ok(b) => {
                    let v = b.to_vec();
                    if crcs.get(i).copied() == Some(crc32c(&v)) {
                        slots.push(Some(v));
                    } else {
                        // 腐坏分片：置 None 走重建
                        slots.push(None);
                    }
                }
                Err(_) => slots.push(None),
            }
        }
        slots
    }

    /// 删除 EC 分片 + manifest（尽力而为）。
    pub async fn delete_ec(&self, path: &str, total: usize) -> StoreResult<()> {
        for i in 0..total {
            let _ = self.inner.delete(&Self::shard_path(path, i)).await;
        }
        let _ = self.inner.delete(&Self::manifest_path(path)).await;
        Ok(())
    }
}

#[async_trait]
impl ObjectStore for ErasureStore {
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        let size = data.len();
        let is_ec = self.cfg.enabled && !self.cfg.profile.is_replica(size as u64);
        if !is_ec {
            return self.inner.put(path, content_type, data).await;
        }
        let profile = self.cfg.profile;
        let total = profile.total_shards();
        let shards = self
            .engine
            .encode(&profile, &data)
            .map_err(|e| StoreError::Other(format!("EC encode 失败: {e}")))?;
        debug_assert_eq!(shards.len(), total);
        let shard_crcs: Vec<u32> = shards.iter().map(|s| crc32c(s)).collect();
        let shard_sizes: Vec<usize> = shards.iter().map(|s| s.len()).collect();

        // 先写分片，manifest 最后写（提交点）
        for (i, s) in shards.iter().enumerate() {
            self.inner
                .put(
                    &Self::shard_path(path, i),
                    "application/octet-stream",
                    Bytes::copy_from_slice(s),
                )
                .await?;
        }
        let manifest = EcManifestStore {
            data_shards: profile.data_shards as usize,
            parity_shards: profile.parity_shards as usize,
            original_len: size,
            content_type: content_type.to_string(),
            shard_sizes,
            shard_crcs,
            created_ms: now_ms(),
        };
        let raw = serde_json::to_vec(&manifest)
            .map_err(|e| StoreError::Other(format!("manifest 序列化失败: {e}")))?;
        self.inner
            .put(
                &Self::manifest_path(path),
                "application/json",
                Bytes::from(raw),
            )
            .await?;
        Ok(BlobObject {
            path: path.to_string(),
            content_type: content_type.to_string(),
            size_bytes: size as u64,
            sha256: Some(sha256_hex(&data)),
        })
    }

    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        if let Some(m) = self.read_manifest(path).await? {
            let profile = EcProfile::new(
                m.data_shards as u16,
                m.parity_shards as u16,
                self.cfg.profile.min_obj_size,
            )
            .map_err(|e| StoreError::Other(format!("manifest 参数非法: {e}")))?;
            let total = profile.total_shards();
            let slots = self.read_shards(path, total, &m.shard_crcs).await;
            let data = self
                .engine
                .decode_reconstruct(&profile, &slots, m.original_len)
                .map_err(|e| StoreError::Checksum(format!("EC decode 失败: {e}")))?;
            Ok(Bytes::from(data))
        } else {
            self.inner.get(path).await
        }
    }

    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        let full = self.get(path).await?;
        let start = offset as usize;
        let end = std::cmp::min(start + length as usize, full.len());
        if start >= full.len() {
            return Ok(Bytes::new());
        }
        Ok(full.slice(start..end))
    }

    async fn delete(&self, path: &str) -> StoreResult<()> {
        if let Some(m) = self.read_manifest(path).await? {
            let total = m.data_shards + m.parity_shards;
            self.delete_ec(path, total).await?;
            return Ok(());
        }
        self.inner.delete(path).await
    }

    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        if let Some(m) = self.read_manifest(path).await? {
            return Ok(BlobObject {
                path: path.to_string(),
                content_type: m.content_type.clone(),
                size_bytes: m.original_len as u64,
                sha256: None,
            });
        }
        self.inner.head(path).await
    }

    async fn exists(&self, path: &str) -> StoreResult<bool> {
        if self.read_manifest(path).await?.is_some() {
            return Ok(true);
        }
        self.inner.exists(path).await
    }
}

/// CRC32C（与 S3 checksum / volume 引擎一致）。
pub fn crc32c(data: &[u8]) -> u32 {
    crc32c::crc32c(data)
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
    use crate::{create_backend, BackendKind, StoreConfig};
    use std::path::Path;

    fn base(dir: &Path) -> Arc<dyn ObjectStore> {
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.to_path_buf(),
            ..Default::default()
        };
        create_backend(&cfg).unwrap().object.clone()
    }

    fn ec_store(dir: &Path) -> ErasureStore {
        let profile = EcProfile::new(4, 2, 64).unwrap();
        ErasureStore::new(base(dir), ErasureConfig { enabled: true, profile })
    }

    #[tokio::test]
    async fn ec_roundtrip_4_plus_2() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        // 超过 min_size=64 → EC 路径
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        store
            .put("big.bin", "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();
        let got = store.get("big.bin").await.unwrap();
        assert_eq!(&got[..], &data[..]);
        // manifest 存在（确认走了 EC）
        let m = store.read_manifest("big.bin").await.unwrap().unwrap();
        assert_eq!(m.data_shards, 4);
        assert_eq!(m.parity_shards, 2);
        assert_eq!(m.original_len, data.len());
        // head/exists
        let h = store.head("big.bin").await.unwrap();
        assert_eq!(h.size_bytes, data.len() as u64);
        assert!(store.exists("big.bin").await.unwrap());
    }

    #[tokio::test]
    async fn small_object_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        // 小于 min_size=64 → 普通透传
        let data = b"tiny".to_vec();
        store
            .put("small.txt", "text/plain", Bytes::from(data.clone()))
            .await
            .unwrap();
        assert!(store.read_manifest("small.txt").await.unwrap().is_none());
        assert_eq!(&store.get("small.txt").await.unwrap()[..], &data[..]);
    }

    #[tokio::test]
    async fn lost_shards_matrix_reconstruct() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data: Vec<u8> = (0..1500u32).map(|i| (i % 197) as u8).collect();
        store
            .put("obj.bin", "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();

        // 底层读回分片真实删除：验证 ErasureStore 用剩余分片重建
        for lost in 1..=2u32 {
            let dir2 = tempfile::tempdir().unwrap();
            let inner = base(dir2.path());
            let es = ErasureStore::new(
                inner.clone(),
                ErasureConfig {
                    enabled: true,
                    profile: EcProfile::new(4, 2, 64).unwrap(),
                },
            );
            es.put("obj.bin", "application/octet-stream", Bytes::from(data.clone()))
                .await
                .unwrap();
            // 删除 lost 个分片（底层直接删）
            for i in 0..lost {
                inner
                    .delete(&ErasureStore::shard_path("obj.bin", i as usize))
                    .await
                    .unwrap();
            }
            let got = es.get("obj.bin").await.unwrap();
            assert_eq!(&got[..], &data[..], "丢 {lost} 片必须逐字节一致");
        }
    }

    #[tokio::test]
    async fn corrupted_shard_detected_and_reconstructed() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data: Vec<u8> = (0..800u32).map(|i| (i % 233) as u8).collect();
        store
            .put("obj.bin", "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();

        // 腐坏 1 个分片（翻转字节）
        let inner = store.inner().clone();
        let sp = ErasureStore::shard_path("obj.bin", 0);
        let shard = inner.get(&sp).await.unwrap().to_vec();
        let mut corrupt = shard.clone();
        let mid = corrupt.len() / 2;
        corrupt[mid] ^= 0xFF;
        assert_ne!(corrupt, shard);
        inner
            .put(&sp, "application/octet-stream", Bytes::from(corrupt))
            .await
            .unwrap();

        // read_shards 应把腐坏分片识别为 None（走重建），仍能恢复
        let got = store.get("obj.bin").await.unwrap();
        assert_eq!(&got[..], &data[..], "腐坏 1 片必须可重建");
    }

    #[tokio::test]
    async fn delete_ec_object_removes_all_shards() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data: Vec<u8> = vec![7u8; 300];
        store
            .put("obj.bin", "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();
        assert!(store.exists("obj.bin").await.unwrap());
        store.delete("obj.bin").await.unwrap();
        assert!(!store.exists("obj.bin").await.unwrap());
        // 底层分片也清理
        let inner = store.inner().clone();
        for i in 0..6 {
            assert!(!inner.exists(&ErasureStore::shard_path("obj.bin", i)).await.unwrap());
        }
        assert!(!inner.exists(&ErasureStore::manifest_path("obj.bin")).await.unwrap());
    }
}
