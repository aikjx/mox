// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::{
    backpressure::{BackpressureConfig, BackpressureMonitor},
    chunk_rebuild::{InMemoryPeerFetcher, PeerChunkFetcher, RebuildCoordinator},
    error::{VolumeError, VolumeResult},
};
use bytes::Bytes;
use mox_cloud_foundation::ChunkManagerProvider;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

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
    /// CAS 背压信号量：控制写入路径的最大并发数
    backpressure: Arc<BackpressureMonitor>,
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
            backpressure: Arc::new(BackpressureMonitor::with_default()),
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

    /// 使用自定义背压配置构建 VolumeServer（Builder 模式）
    pub fn with_backpressure_config(mut self, config: BackpressureConfig) -> Self {
        self.backpressure = Arc::new(BackpressureMonitor::new(config));
        self
    }

    /// 获取背压监视器引用（用于指标观测和测试）
    pub fn backpressure(&self) -> &Arc<BackpressureMonitor> {
        &self.backpressure
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

    /// 写入主入口：CAS 背压准入控制
    ///
    /// 在方法入口调用 `backpressure.try_acquire()` 获取并发槽；
    /// 达到 `max_concurrent` 时返回 `VolumeError::BackpressureRejected`。
    /// permit 在方法返回时自动 drop，释放并发槽。
    ///
    /// 注意：`rebuild_from_peers` 和 `restore_from_manifest` 内部调用
    /// `write_chunk`，因此自动继承背压控制，无需重复添加。
    pub fn write_chunk(&self, chunk_id: &str, data: Bytes) -> VolumeResult<ChunkAck> {
        // ---- CAS 背压准入 ----
        let _permit = match self.backpressure.try_acquire() {
            Ok(permit) => permit,
            Err(e) => {
                return Err(VolumeError::BackpressureRejected(format!("{e}")));
            },
        };
        // _permit 在作用域结束时自动 drop，释放并发槽

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
        self.inner.chunk_checksums.lock().insert(chunk_id.to_string(), crc);

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
        let expected_crc = self.inner.chunk_checksums.lock().get(chunk_id).copied().unwrap_or(0);
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
        self.inner.snapshot_store.lock().insert(snapshot_id.to_string(), manifest);
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
                crc = if crc & 1 == 1 { (crc >> 1) ^ poly } else { crc >> 1 };
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

// ---------------------------------------------------------------------------
// 单元测试：背压接入验证
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backpressure::BackpressureConfig;
    use std::time::Duration;

    fn custom_config(max: usize) -> BackpressureConfig {
        BackpressureConfig {
            max_concurrent: max,
            high_water: 0.8,
            low_water: 0.5,
            cooldown: Duration::ZERO,
        }
    }

    /// 测试 1：写入时背压 permit 被正确获取和释放
    #[test]
    fn test_backpressure_acquired_on_write() {
        let vs = VolumeServer::new("vol-bp-1".into(), 1024 * 1024)
            .with_backpressure_config(custom_config(4));

        // 写入前并发数为 0
        assert_eq!(vs.backpressure().current_concurrent(), 0);

        // 执行写入
        let ack = vs
            .write_chunk("chunk-1", Bytes::from_static(b"hello"))
            .expect("write should succeed");
        assert_eq!(ack.chunk_id, "chunk-1");

        // 写入完成后 permit 已释放，并发数归零
        assert_eq!(vs.backpressure().current_concurrent(), 0);

        // 指标显示有一次准入
        let metrics = vs.backpressure().metrics();
        assert_eq!(metrics.total_admissions, 1);
        assert_eq!(metrics.total_rejections, 0);
    }

    /// 测试 2：背压满时写入被拒绝
    #[test]
    fn test_backpressure_rejects_when_full() {
        let vs = VolumeServer::new("vol-bp-2".into(), 1024 * 1024)
            .with_backpressure_config(custom_config(1));

        // 手动持有一个 permit，占满唯一槽位
        let _held = vs.backpressure().try_acquire().expect("should acquire the only slot");
        assert_eq!(vs.backpressure().current_concurrent(), 1);

        // 此时写入应被背压拒绝
        let result = vs.write_chunk("chunk-rejected", Bytes::from_static(b"data"));
        match result {
            Err(VolumeError::BackpressureRejected(msg)) => {
                assert!(msg.contains("backpressure rejected"), "msg: {msg}");
            },
            other => panic!("expected BackpressureRejected, got {other:?}"),
        }

        // 释放 permit 后写入应成功
        drop(_held);
        assert_eq!(vs.backpressure().current_concurrent(), 0);

        let ack = vs
            .write_chunk("chunk-ok", Bytes::from_static(b"ok"))
            .expect("write should succeed after release");
        assert_eq!(ack.chunk_id, "chunk-ok");
        assert_eq!(vs.backpressure().current_concurrent(), 0);
    }

    /// 测试 3：自定义背压配置被正确应用
    #[test]
    fn test_backpressure_config_custom() {
        let config = BackpressureConfig {
            max_concurrent: 7,
            high_water: 0.75,
            low_water: 0.25,
            cooldown: Duration::from_millis(50),
        };
        let vs = VolumeServer::new("vol-bp-3".into(), 1024 * 1024)
            .with_backpressure_config(config.clone());

        let applied = vs.backpressure().config();
        assert_eq!(applied.max_concurrent, 7);
        assert!((applied.high_water - 0.75).abs() < 1e-6);
        assert!((applied.low_water - 0.25).abs() < 1e-6);
        assert_eq!(applied.cooldown, Duration::from_millis(50));

        // 高水位阈值 = 7 * 0.75 = 5
        assert_eq!(applied.high_threshold(), 5);
        // 低水位阈值 = 7 * 0.25 = 1
        assert_eq!(applied.low_threshold(), 1);

        // 验证 max_concurrent=7 实际生效：持有 7 个 permit 后第 8 个被拒绝
        let mut permits = Vec::new();
        for i in 0..7 {
            permits.push(
                vs.backpressure()
                    .try_acquire()
                    .unwrap_or_else(|_| panic!("permit {i} should succeed")),
            );
        }
        assert_eq!(vs.backpressure().current_concurrent(), 7);
        assert!(vs.backpressure().try_acquire().is_err());

        // 释放所有
        drop(permits);
        assert_eq!(vs.backpressure().current_concurrent(), 0);
    }

    /// 测试 4：默认构造的 VolumeServer 也有背压（默认配置，不影响现有行为）
    #[test]
    fn test_backpressure_default_constructor() {
        let vs = VolumeServer::new("vol-default".into(), 1024 * 1024);

        // 默认 max_concurrent=32，写入应正常
        let ack = vs
            .write_chunk("c1", Bytes::from_static(b"test"))
            .expect("default write should succeed");
        assert_eq!(ack.size, 4);
        assert_eq!(vs.backpressure().current_concurrent(), 0);

        // 默认配置值
        let cfg = vs.backpressure().config();
        assert_eq!(cfg.max_concurrent, 32);
    }

    /// 测试 5：连续多次写入，背压计数正确（permit 不泄漏）
    #[test]
    fn test_backpressure_consecutive_writes() {
        let vs = VolumeServer::new("vol-consec".into(), 1024 * 1024)
            .with_backpressure_config(custom_config(2));

        for i in 0..10 {
            let data = Bytes::from(format!("data-{i}"));
            vs.write_chunk(&format!("chunk-{i}"), data)
                .unwrap_or_else(|e| panic!("write {i} failed: {e:?}"));
            // 每次写入后并发数必须归零
            assert_eq!(vs.backpressure().current_concurrent(), 0, "after write {i}");
        }

        let metrics = vs.backpressure().metrics();
        assert_eq!(metrics.total_admissions, 10);
        assert_eq!(metrics.total_rejections, 0);
    }
}
