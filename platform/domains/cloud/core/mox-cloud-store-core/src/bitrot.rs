// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Bitrot 检测（阶段3，feature `erasure`）——分片 crc32c 校验 + 限速后台扫描。
//!
//! 介质静默腐坏（bitrot）无法靠读写成功与否察觉：扇区/比特翻转后读取仍成功，
//! 但数据已与写入时不一致。本模块基于 EC manifest 中记录的每分片 crc32c，
//! 周期性重读分片比对，识别两类问题：
//!
//! - **腐坏（corrupt）**：分片可读但 crc32c 不匹配 → 介质翻转。
//! - **缺失（missing）**：分片读不到（NotFound）→ 物理丢失 / 人为删除。
//!
//! 检测结果交给 [`crate::heal::HealCoordinator`] 走 EC 重建自愈。
//!
//! ## 后台扫描
//! [`BitrotDetector::start_background`] 以独立线程 + 限速间隔（默认 24h）周期
//! 扫描对象集合，报告通过 `std::sync::mpsc` 通道交给管理面。

use crate::erasure::{crc32c, ErasureStore};
use mox_base_store_core::StoreResult;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::Duration;

/// 默认扫描间隔：24 小时。
pub const DEFAULT_SCAN_INTERVAL: Duration = Duration::from_secs(24 * 3600);

/// 单条腐坏/缺失记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corruption {
    /// 逻辑对象 key
    pub path: String,
    /// 分片序号
    pub shard_index: usize,
    /// 期望 crc32c（来自 manifest）
    pub expected_crc: u32,
    /// 实际 crc32c（缺失时为 0）
    pub actual_crc: u32,
    /// 类型："corrupt" | "missing"
    pub kind: String,
}

/// 单对象扫描报告。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanReport {
    /// 已扫描对象数
    pub objects_scanned: u64,
    /// 已校验分片数
    pub shards_checked: u64,
    /// 腐坏/缺失记录
    pub corruptions: Vec<Corruption>,
}

/// 一轮全量扫描汇总。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BitrotSummary {
    /// 扫描对象数
    pub objects_scanned: u64,
    /// 校验分片数
    pub shards_checked: u64,
    /// 受影响对象数（含 ≥1 条腐坏/缺失）
    pub affected_objects: u64,
    /// 腐坏记录数
    pub corruptions: u64,
    /// 全部记录明细
    pub details: Vec<Corruption>,
}

/// Bitrot 检测器。
pub struct BitrotDetector {
    store: Arc<ErasureStore>,
}

impl BitrotDetector {
    /// 包装已装配的 EC 存储。
    pub fn new(store: Arc<ErasureStore>) -> Self {
        Self { store }
    }

    /// 底层 EC 存储引用。
    pub fn store(&self) -> &Arc<ErasureStore> {
        &self.store
    }

    /// 扫描单个对象：非 EC 对象返回空报告，EC 对象逐分片 crc32c 校验。
    pub async fn scan_object(&self, path: &str) -> StoreResult<ScanReport> {
        let mut report = ScanReport::default();
        let Some(m) = self.store.read_manifest(path).await? else {
            return Ok(report);
        };
        report.objects_scanned = 1;
        let total = m.data_shards + m.parity_shards;
        for i in 0..total {
            let sp = ErasureStore::shard_path(path, i);
            report.shards_checked += 1;
            let expected = m.shard_crcs.get(i).copied().unwrap_or(0);
            match self.store.inner().get(&sp).await {
                Ok(b) => {
                    let actual = crc32c(&b);
                    if actual != expected {
                        report.corruptions.push(Corruption {
                            path: path.to_string(),
                            shard_index: i,
                            expected_crc: expected,
                            actual_crc: actual,
                            kind: "corrupt".into(),
                        });
                    }
                }
                Err(_) => {
                    report.corruptions.push(Corruption {
                        path: path.to_string(),
                        shard_index: i,
                        expected_crc: expected,
                        actual_crc: 0,
                        kind: "missing".into(),
                    });
                }
            }
        }
        Ok(report)
    }

    /// 扫描一组对象并汇总。
    pub async fn scan_paths(&self, paths: &[&str]) -> StoreResult<BitrotSummary> {
        let mut summary = BitrotSummary::default();
        for p in paths {
            let r = self.scan_object(p).await?;
            summary.objects_scanned += r.objects_scanned;
            summary.shards_checked += r.shards_checked;
            if !r.corruptions.is_empty() {
                summary.affected_objects += 1;
                summary.corruptions += r.corruptions.len() as u64;
                summary.details.extend(r.corruptions);
            }
        }
        Ok(summary)
    }

    /// 启动后台限速扫描：独立线程周期扫描，结果经通道上报。
    ///
    /// 返回 (JoinHandle, 报告接收端)。线程退出（stop 信号或存储故障）时关闭通道。
    pub fn start_background(
        store: Arc<ErasureStore>,
        paths: Vec<String>,
        interval: Duration,
        capacity: usize,
    ) -> (std::thread::JoinHandle<()>, Receiver<BitrotSummary>) {
        let (tx, rx): (SyncSender<BitrotSummary>, _) = sync_channel(capacity.max(1));
        let handle = std::thread::Builder::new()
            .name("bitrot-scan".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("bitrot runtime");
                let det = BitrotDetector::new(store);
                let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                loop {
                    match rt.block_on(det.scan_paths(&refs)) {
                        Ok(summary) => {
                            if tx.send(summary).is_err() {
                                // 管理面已关闭 → 退出
                                return;
                            }
                        }
                        Err(e) => {
                            tracing::error!("bitrot 扫描失败: {e}");
                        }
                    }
                    std::thread::sleep(interval);
                }
            })
            .expect("spawn bitrot thread");
        (handle, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure::{ErasureConfig, ErasureStore};
    use crate::fs_backend::FsObjectStore;
    use bytes::Bytes;
    use mox_base_store_core::ObjectStore;
    use mox_cloud_kernel::EcProfile;
    use std::path::Path;

    fn ec_store(dir: &Path) -> Arc<ErasureStore> {
        let base = Arc::new(FsObjectStore::new(dir.to_path_buf()).unwrap());
        let profile = EcProfile::new(4, 2, 64).unwrap();
        Arc::new(ErasureStore::new(
            base,
            ErasureConfig {
                enabled: true,
                profile,
            },
        ))
    }

    async fn put_big(store: &ErasureStore, path: &str, seed: u32, len: usize) {
        let data: Vec<u8> = (0..len as u32).map(|i| ((i + seed) % 251) as u8).collect();
        store
            .put(path, "application/octet-stream", Bytes::from(data))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn healthy_ec_object_scans_clean() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 7, 1024).await;
        let det = BitrotDetector::new(store.clone());
        let report = det.scan_object("a.bin").await.unwrap();
        assert_eq!(report.objects_scanned, 1);
        assert_eq!(report.shards_checked, 6);
        assert!(report.corruptions.is_empty());
    }

    #[tokio::test]
    async fn non_ec_object_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        store
            .put("tiny.txt", "text/plain", Bytes::from_static(b"tiny"))
            .await
            .unwrap();
        let det = BitrotDetector::new(store);
        let report = det.scan_object("tiny.txt").await.unwrap();
        assert_eq!(report.objects_scanned, 0, "非 EC 对象不应扫描");
        assert!(report.corruptions.is_empty());
    }

    #[tokio::test]
    async fn corrupted_shard_detected() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 3, 512).await;

        // 腐坏分片 2（翻转字节）
        let sp = ErasureStore::shard_path("a.bin", 2);
        let shard = store.inner().get(&sp).await.unwrap().to_vec();
        let mut corrupt = shard.clone();
        let mid = corrupt.len() / 2;
        corrupt[mid] ^= 0x55;
        store
            .inner()
            .put(&sp, "application/octet-stream", Bytes::from(corrupt))
            .await
            .unwrap();

        let det = BitrotDetector::new(store);
        let report = det.scan_object("a.bin").await.unwrap();
        assert_eq!(report.corruptions.len(), 1);
        assert_eq!(report.corruptions[0].shard_index, 2);
        assert_eq!(report.corruptions[0].kind, "corrupt");
        assert_ne!(report.corruptions[0].actual_crc, report.corruptions[0].expected_crc);
    }

    #[tokio::test]
    async fn missing_shard_detected() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 5, 768).await;

        // 底层直接删除分片 4
        store
            .inner()
            .delete(&ErasureStore::shard_path("a.bin", 4))
            .await
            .unwrap();

        let det = BitrotDetector::new(store);
        let report = det.scan_object("a.bin").await.unwrap();
        assert_eq!(report.corruptions.len(), 1);
        assert_eq!(report.corruptions[0].shard_index, 4);
        assert_eq!(report.corruptions[0].kind, "missing");
        assert_eq!(report.corruptions[0].actual_crc, 0);
    }

    #[tokio::test]
    async fn scan_paths_aggregates_summary() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 1, 1024).await;
        put_big(&store, "b.bin", 2, 1024).await;
        // 腐坏 b 分片 1
        let sp = ErasureStore::shard_path("b.bin", 1);
        let shard = store.inner().get(&sp).await.unwrap().to_vec();
        let mut corrupt = shard.clone();
        corrupt[0] ^= 0xFF;
        store
            .inner()
            .put(&sp, "application/octet-stream", Bytes::from(corrupt))
            .await
            .unwrap();

        let det = BitrotDetector::new(store);
        let summary = det
            .scan_paths(&["a.bin", "b.bin"])
            .await
            .unwrap();
        assert_eq!(summary.objects_scanned, 2);
        assert_eq!(summary.shards_checked, 12);
        assert_eq!(summary.affected_objects, 1);
        assert_eq!(summary.corruptions, 1);
        assert_eq!(summary.details[0].path, "b.bin");
    }
}
