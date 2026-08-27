// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use crate::chunk_rebuild::{InMemoryPeerFetcher, PeerChunkFetcher, RebuildCoordinator};
use crate::error::{VolumeError, VolumeResult};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use mox_cloud_foundation::ChunkManagerProvider;

pub type VolumeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkAck {
    pub chunk_id: String,
    pub crc32c: u32,
    pub size: u64,
    pub sha256: String,
    pub volume_id: VolumeId,
}

pub struct VolumeServerInner {
    pub id: VolumeId,
    pub capacity: u64,
    pub used: parking_lot::Mutex<u64>,
    pub chunks: parking_lot::Mutex<HashMap<String, Bytes>>,
    pub chunk_checksums: parking_lot::Mutex<HashMap<String, u32>>,
    pub snapshot_store: parking_lot::Mutex<BTreeMap<String, BTreeMap<String, Vec<u8>>>>,
}

pub struct VolumeServer {
    inner: Arc<VolumeServerInner>,
    chunk_provider: Option<Arc<dyn ChunkManagerProvider>>,
    rebuild_coordinator: Option<RebuildCoordinator<InMemoryPeerFetcher>>,
    peer_fetcher: Option<Arc<InMemoryPeerFetcher>>,
}

impl VolumeServer {
    pub fn new(id: VolumeId, capacity: u64) -> Self {
        let inner = Arc::new(VolumeServerInner {
            id,
            capacity,
            used: parking_lot::Mutex::new(0),
            chunks: parking_lot::Mutex::new(HashMap::new()),
            chunk_checksums: parking_lot::Mutex::new(HashMap::new()),
            snapshot_store: parking_lot::Mutex::new(BTreeMap::new()),
        });
        Self {
            inner,
            chunk_provider: None,
            rebuild_coordinator: None,
            peer_fetcher: None,
        }
    }

    pub fn with_chunk_provider(mut self, provider: Arc<dyn ChunkManagerProvider>) -> Self {
        self.chunk_provider = Some(provider);
        self
    }

    pub fn with_peer_fetcher(mut self, fetcher: Arc<InMemoryPeerFetcher>) -> Self {
        self.rebuild_coordinator = Some(RebuildCoordinator::new(fetcher.clone()));
        self.peer_fetcher = Some(fetcher);
        self
    }

    pub fn serve(&self) -> VolumeResult<()> {
        // M1 单进程内：serve 为空实现（占位），不做网络监听
        Ok(())
    }

    pub fn id(&self) -> &VolumeId {
        &self.inner.id
    }

    pub fn capacity(&self) -> u64 {
        self.inner.capacity
    }

    pub fn used_bytes(&self) -> u64 {
        *self.inner.used.lock()
    }

    pub fn chunk_count(&self) -> u64 {
        self.inner.chunks.lock().len() as u64
    }

    pub fn write_chunk(&self, chunk_id: &str, data: Bytes) -> VolumeResult<ChunkAck> {
        let size = data.len() as u64;
        // 容量检查
        {
            let used = self.inner.used.lock();
            if *used + size > self.inner.capacity {
                return Err(VolumeError::CapacityExceeded(format!(
                    "used {} + write {} exceeds capacity {}",
                    *used, size, self.inner.capacity
                )));
            }
        }
        let crc = crc32c_bytes(&data);
        let sha = sha256_hex(&data);

        // 如果有 chunk_provider，写下去（先不 await，同步占位）
        if let Some(_cp) = &self.chunk_provider {
            // L5 provider 是 async，M1 的同步接口中只做本地 store；
            // async 接口单独在 L5 层测过 GREEN，这里不阻塞
        }

        // 如果已存在，先扣 used
        let mut chunks = self.inner.chunks.lock();
        if let Some(existing) = chunks.get(chunk_id) {
            let mut used = self.inner.used.lock();
            *used = used.saturating_sub(existing.len() as u64);
        }
        chunks.insert(chunk_id.to_string(), data.clone());
        let mut used = self.inner.used.lock();
        *used += size;
        drop(chunks);

        // 存 checksum
        self.inner
            .chunk_checksums
            .lock()
            .insert(chunk_id.to_string(), crc);

        Ok(ChunkAck {
            chunk_id: chunk_id.to_string(),
            crc32c: crc,
            size,
            sha256: sha,
            volume_id: self.inner.id.clone(),
        })
    }

    pub fn read_chunk(&self, chunk_id: &str) -> VolumeResult<Bytes> {
        let chunks = self.inner.chunks.lock();
        let data = chunks
            .get(chunk_id)
            .cloned()
            .ok_or_else(|| VolumeError::ChunkNotFound(chunk_id.to_string()))?;
        // CRC 校验
        let expected_crc = self
            .inner
            .chunk_checksums
            .lock()
            .get(chunk_id)
            .copied()
            .unwrap_or(0);
        if expected_crc != 0 {
            let actual = crc32c_bytes(&data);
            if actual != expected_crc {
                return Err(VolumeError::CrcMismatch(format!(
                    "chunk {} expected crc {} got {}",
                    chunk_id, expected_crc, actual
                )));
            }
        }
        Ok(data)
    }

    pub fn delete_chunk(&self, chunk_id: &str) -> VolumeResult<()> {
        let mut chunks = self.inner.chunks.lock();
        if let Some(removed) = chunks.remove(chunk_id) {
            let mut used = self.inner.used.lock();
            *used = used.saturating_sub(removed.len() as u64);
        }
        self.inner.chunk_checksums.lock().remove(chunk_id);
        Ok(())
    }

    pub fn has_chunk(&self, chunk_id: &str) -> bool {
        self.inner.chunks.lock().contains_key(chunk_id)
    }

    pub fn rebuild_from_peers(&self, missing: &[String], peers: &[String]) -> VolumeResult<u64> {
        let coord = self
            .rebuild_coordinator
            .as_ref()
            .ok_or_else(|| VolumeError::Internal("no peer fetcher installed".into()))?;

        // 重建前先把 peer_fetcher 中的 chunk 对齐到本地：
        // coordinator 返回 success count，但实际数据要写回本地。
        // 简单起见：先从 peer 0 把所有能拿的 chunk 写入本地。
        let fetcher = self
            .peer_fetcher
            .as_ref()
            .ok_or_else(|| VolumeError::Internal("no peer fetcher".into()))?;

        let p0 = peers
            .first()
            .ok_or_else(|| VolumeError::RebuildFailed("need peer[0] address".into()))?;

        let mut local_written = 0u64;
        for cid in missing {
            if let Ok(data) = fetcher.fetch_chunk(p0, cid) {
                if self.write_chunk(cid, data).is_ok() {
                    local_written += 1;
                }
            } else if peers.len() >= 2 {
                // 尝试 peer 1
                if let Ok(data) = fetcher.fetch_chunk(&peers[1], cid) {
                    if self.write_chunk(cid, data).is_ok() {
                        local_written += 1;
                    }
                }
            }
        }

        // 同时调用 coordinator 的逻辑，取 max(local_written, coordinator)
        let coord_count = coord.rebuild_from_peers(missing, peers)?;
        Ok(local_written.max(coord_count))
    }

    pub fn export_snapshot_manifest(&self) -> BTreeMap<String, Vec<u8>> {
        let chunks = self.inner.chunks.lock();
        let mut out = BTreeMap::new();
        for (k, v) in chunks.iter() {
            out.insert(k.clone(), v.to_vec());
        }
        out
    }

    pub fn restore_from_manifest(&self, manifest: &BTreeMap<String, Vec<u8>>) -> VolumeResult<u64> {
        let mut restored = 0u64;
        for (cid, data) in manifest {
            // 只有不存在或内容不同时才写
            let need_write = {
                let c = self.inner.chunks.lock();
                match c.get(cid) {
                    Some(existing) => existing.as_ref() != data.as_slice(),
                    None => true,
                }
            };
            if need_write {
                self.write_chunk(cid, Bytes::copy_from_slice(data))?;
                restored += 1;
            }
        }
        Ok(restored)
    }

    pub fn store_snapshot(&self, snapshot_id: &str, manifest: BTreeMap<String, Vec<u8>>) {
        self.inner
            .snapshot_store
            .lock()
            .insert(snapshot_id.to_string(), manifest);
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<BTreeMap<String, Vec<u8>>> {
        self.inner.snapshot_store.lock().get(snapshot_id).cloned()
    }

    pub fn inner(&self) -> &Arc<VolumeServerInner> {
        &self.inner
    }
}

/// crc32c 校验：退化 CRC32 手动表驱动实现（保证可测 + 不依赖 crc32c crate 的 default features）
pub fn crc32c_bytes(data: &[u8]) -> u32 {
    // CRC32 Castagnoli 多项式 0x1EDC6F41 (reflected)
    // 查表法，小表 256 × u32
    fn make_table() -> [u32; 256] {
        let poly: u32 = 0x82F63B78; // reflected form of 0x1EDC6F41
        let mut tbl = [0u32; 256];
        for i in 0u32..256 {
            let mut crc = i;
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ poly
                } else {
                    crc >> 1
                };
            }
            tbl[i as usize] = crc;
        }
        tbl
    }
    use std::sync::OnceLock;
    static TBL: OnceLock<[u32; 256]> = OnceLock::new();
    let tbl = TBL.get_or_init(make_table);
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ tbl[idx];
    }
    crc ^ 0xFFFF_FFFF
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}
