// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 自愈协调器（阶段3，feature `erasure`）——基于 bitrot 扫描结果调度 EC 重建。
//!
//! [`HealCoordinator`] 是 [`crate::bitrot::BitrotDetector`] 的对偶：检测器
//! 只"发现问题"，协调器负责"恢复数据"。流程：
//!
//! 1. 读取对象 EC manifest（data/parity/crc 清单）。
//! 2. 底层逐分片读取，crc32c 比对 → 缺失/腐坏分片置 `None`。
//! 3. 调用 `mox-cloud-kernel` 的 [`ReedSolomonEngine::reconstruct_shards`]
//!    重建全部分片（确定性：重建分片与原分片逐字节一致）。
//! 4. 将重建分片写回底层，重校 crc32c 与 manifest 一致（无需改 manifest）。
//!
//! 超过容错（缺失 > parity）→ `Unrecoverable`，上报并保留现场。
//!
//! ## 后台周期自愈
//! [`HealCoordinator::start_background`] 独立线程周期自愈一组对象，结果经通道上报。

use crate::bitrot::{Corruption, ScanReport};
use crate::erasure::{crc32c, ErasureStore};
use crate::{StoreError, StoreResult};
use bytes::Bytes;
use mox_cloud_kernel::{EcProfile, ReedSolomonEngine};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::Duration;

/// 自愈动作分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealAction {
    /// 分片缺失 → EC 重建写回
    Rebuild,
    /// 分片腐坏 → EC 重建写回
    Repair,
    /// 缺失超过容错 → 无法恢复
    Unrecoverable,
    /// 无需处理（健康对象 / 非 EC 对象）
    Ok,
}

/// 单对象自愈结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealResult {
    /// 逻辑对象 key
    pub path: String,
    /// 自愈动作
    pub action: HealAction,
    /// 已重建写回的分片序号
    pub rebuilt_shards: Vec<usize>,
    /// 重建失败原因（仅 Unrecoverable/出错时非空）
    pub errors: Vec<String>,
}

/// 一轮自愈汇总。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealReport {
    /// 成功自愈对象数
    pub objects_healed: u64,
    /// 重建写回分片数
    pub shards_rebuilt: u64,
    /// 无法恢复对象数
    pub unrecoverable: u64,
    /// 健康跳过对象数
    pub objects_ok: u64,
    /// 明细
    pub results: Vec<HealResult>,
}

/// 自愈协调器。
pub struct HealCoordinator {
    store: Arc<ErasureStore>,
    engine: ReedSolomonEngine,
    /// 累计重建分片数（监控）
    total_rebuilt: AtomicU64,
}

impl HealCoordinator {
    /// 包装已装配的 EC 存储。
    pub fn new(store: Arc<ErasureStore>) -> Self {
        Self {
            store,
            engine: ReedSolomonEngine::new(),
            total_rebuilt: AtomicU64::new(0),
        }
    }

    /// 底层 EC 存储引用。
    pub fn store(&self) -> &Arc<ErasureStore> {
        &self.store
    }

    /// 累计重建分片数。
    pub fn total_rebuilt(&self) -> u64 {
        self.total_rebuilt.load(Ordering::Relaxed)
    }

    /// 自愈单个对象：读取分片槽位 → EC 重建 → 写回缺失/腐坏分片。
    pub async fn heal_object(&self, path: &str) -> StoreResult<HealResult> {
        let mut result = HealResult {
            path: path.to_string(),
            action: HealAction::Ok,
            rebuilt_shards: Vec::new(),
            errors: Vec::new(),
        };
        let Some(m) = self.store.read_manifest(path).await? else {
            // 非 EC 对象：无需处理
            return Ok(result);
        };

        let profile = EcProfile::new(
            m.data_shards as u16,
            m.parity_shards as u16,
            self.store.profile().min_obj_size,
        )
        .map_err(|e| StoreError::Other(format!("manifest 参数非法: {e}")))?;
        let total = profile.total_shards();

        // 1. 读分片槽位：crc32c 比对，缺失/腐坏 → None；
        //    `physically_missing` 区分"物理缺失(NotFound)"与"腐坏(可读但 crc 不符)"
        let mut slots: Vec<Option<Vec<u8>>> = Vec::with_capacity(total);
        let mut physically_missing: Vec<bool> = Vec::with_capacity(total);
        let mut problem_indices: Vec<usize> = Vec::new();
        let mut has_problem = false;
        for i in 0..total {
            match self.store.inner().get(&ErasureStore::shard_path(path, i)).await {
                Ok(b) => {
                    let v = b.to_vec();
                    let expected = m.shard_crcs.get(i).copied().unwrap_or(0);
                    if crc32c(&v) == expected {
                        slots.push(Some(v));
                        physically_missing.push(false);
                    } else {
                        slots.push(None);
                        physically_missing.push(false); // 腐坏：可读但坏
                        problem_indices.push(i);
                        has_problem = true;
                    }
                }
                Err(_) => {
                    slots.push(None);
                    physically_missing.push(true); // 物理缺失
                    problem_indices.push(i);
                    has_problem = true;
                }
            }
        }

        if !has_problem {
            return Ok(result); // 健康
        }

        // 2. EC 重建全部分片（确定性输出）
        let rebuilt = match self.engine.reconstruct_shards(&profile, &slots) {
            Ok(r) => r,
            Err(e) => {
                result.action = HealAction::Unrecoverable;
                result.errors.push(format!("EC 重建失败: {e}"));
                return Ok(result);
            }
        };
        debug_assert_eq!(rebuilt.len(), total);

        // 3. 写回缺失/腐坏分片 + 重校 crc
        let mut all_ok = true;
        for &i in &problem_indices {
            let sp = ErasureStore::shard_path(path, i);
            if let Some(rd) = rebuilt.get(i) {
                if self
                    .store
                    .inner()
                    .put(&sp, "application/octet-stream", Bytes::copy_from_slice(rd))
                    .await
                    .is_err()
                {
                    all_ok = false;
                    result.errors.push(format!("写回分片 s{i} 失败"));
                    continue;
                }
                let expected = m.shard_crcs.get(i).copied().unwrap_or(0);
                if crc32c(rd) != expected {
                    all_ok = false;
                    result.errors.push(format!(
                        "分片 s{i} 重建后 crc 不匹配 ({} != {expected})",
                        crc32c(rd)
                    ));
                    continue;
                }
                result.rebuilt_shards.push(i);
            } else {
                all_ok = false;
                result.errors.push(format!("分片 s{i} 重建输出缺失"));
            }
        }

        result.action = if all_ok {
            // 有物理缺失（NotFound）→ Rebuild；否则（仅腐坏）→ Repair
            let has_missing = problem_indices
                .iter()
                .any(|&i| physically_missing.get(i).copied().unwrap_or(false));
            if has_missing {
                HealAction::Rebuild
            } else {
                HealAction::Repair
            }
        } else {
            HealAction::Unrecoverable
        };
        if !result.rebuilt_shards.is_empty() {
            self.total_rebuilt
                .fetch_add(result.rebuilt_shards.len() as u64, Ordering::Relaxed);
        }
        Ok(result)
    }

    /// 基于 bitrot 扫描报告批量自愈（按对象去重）。
    pub async fn heal_report(&self, report: &ScanReport) -> StoreResult<HealReport> {
        // 按对象分组
        let mut by_path: BTreeMap<String, Vec<&Corruption>> = BTreeMap::new();
        for c in &report.corruptions {
            by_path.entry(c.path.clone()).or_default().push(c);
        }
        let mut out = HealReport::default();
        for (path, _corruptions) in by_path {
            match self.heal_object(&path).await {
                Ok(r) => {
                    match r.action {
                        HealAction::Ok => out.objects_ok += 1,
                        HealAction::Unrecoverable => out.unrecoverable += 1,
                        _ => {
                            out.objects_healed += 1;
                            out.shards_rebuilt += r.rebuilt_shards.len() as u64;
                        }
                    }
                    out.results.push(r);
                }
                Err(e) => {
                    out.unrecoverable += 1;
                    out.results.push(HealResult {
                        path,
                        action: HealAction::Unrecoverable,
                        rebuilt_shards: Vec::new(),
                        errors: vec![format!("自愈执行失败: {e}")],
                    });
                }
            }
        }
        Ok(out)
    }

    /// 启动后台周期自愈：独立线程对固定对象集合周期重建，结果经通道上报。
    ///
    /// 返回 (JoinHandle, 报告接收端)。线程退出（通道关闭 / 存储故障）时结束。
    pub fn start_background(
        store: Arc<ErasureStore>,
        paths: Vec<String>,
        interval: Duration,
        capacity: usize,
    ) -> (std::thread::JoinHandle<()>, Receiver<HealReport>) {
        let (tx, rx): (SyncSender<HealReport>, _) = sync_channel(capacity.max(1));
        let handle = std::thread::Builder::new()
            .name("ec-heal".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("heal runtime");
                let heal = HealCoordinator::new(store);
                let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                loop {
                    let mut report = HealReport::default();
                    for p in &refs {
                        match rt.block_on(heal.heal_object(p)) {
                            Ok(r) => report.results.push(r),
                            Err(e) => {
                                report.unrecoverable += 1;
                                report.results.push(HealResult {
                                    path: (*p).to_string(),
                                    action: HealAction::Unrecoverable,
                                    rebuilt_shards: Vec::new(),
                                    errors: vec![format!("自愈执行失败: {e}")],
                                });
                            }
                        }
                    }
                    for r in &report.results {
                        match r.action {
                            HealAction::Ok => report.objects_ok += 1,
                            HealAction::Unrecoverable => report.unrecoverable += 1,
                            _ => {
                                report.objects_healed += 1;
                                report.shards_rebuilt += r.rebuilt_shards.len() as u64;
                            }
                        }
                    }
                    if tx.send(report).is_err() {
                        return;
                    }
                    std::thread::sleep(interval);
                }
            })
            .expect("spawn heal thread");
        (handle, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitrot::BitrotDetector;
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

    async fn put_big(store: &ErasureStore, path: &str, seed: u32, len: usize) -> Vec<u8> {
        let data: Vec<u8> = (0..len as u32).map(|i| ((i + seed) % 251) as u8).collect();
        store
            .put(path, "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();
        data
    }

    #[tokio::test]
    async fn healthy_object_noop() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 1, 512).await;
        let heal = HealCoordinator::new(store.clone());
        let r = heal.heal_object("a.bin").await.unwrap();
        assert_eq!(r.action, HealAction::Ok);
        assert!(r.rebuilt_shards.is_empty());
    }

    #[tokio::test]
    async fn non_ec_object_noop() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        store
            .put("tiny.txt", "text/plain", Bytes::from_static(b"tiny"))
            .await
            .unwrap();
        let heal = HealCoordinator::new(store);
        let r = heal.heal_object("tiny.txt").await.unwrap();
        assert_eq!(r.action, HealAction::Ok);
    }

    #[tokio::test]
    async fn heal_corrupted_shard_repairs_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data = put_big(&store, "a.bin", 3, 2048).await;

        // 腐坏分片 0
        let sp = ErasureStore::shard_path("a.bin", 0);
        let shard = store.inner().get(&sp).await.unwrap().to_vec();
        let mut corrupt = shard.clone();
        let mid = corrupt.len() / 2;
        corrupt[mid] ^= 0xAA;
        store
            .inner()
            .put(&sp, "application/octet-stream", Bytes::from(corrupt))
            .await
            .unwrap();

        let heal = HealCoordinator::new(store.clone());
        let r = heal.heal_object("a.bin").await.unwrap();
        assert_eq!(r.action, HealAction::Repair);
        assert_eq!(r.rebuilt_shards, vec![0]);
        assert!(r.errors.is_empty());

        // 自愈后：底层分片 crc 已恢复，且对象可完整读出
        let det = BitrotDetector::new(store.clone());
        let scan = det.scan_object("a.bin").await.unwrap();
        assert!(scan.corruptions.is_empty(), "自愈后扫描应干净");
        let got = store.get("a.bin").await.unwrap();
        assert_eq!(&got[..], &data[..]);
    }

    #[tokio::test]
    async fn heal_missing_shard_rebuilds_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data = put_big(&store, "b.bin", 7, 1500).await;

        // 删除分片 3、4（2 片缺失 = parity 上限）
        for i in [3usize, 4] {
            store
                .inner()
                .delete(&ErasureStore::shard_path("b.bin", i))
                .await
                .unwrap();
        }

        let heal = HealCoordinator::new(store.clone());
        let r = heal.heal_object("b.bin").await.unwrap();
        assert_eq!(r.action, HealAction::Rebuild);
        assert_eq!(r.rebuilt_shards.len(), 2);
        assert!(r.rebuilt_shards.contains(&3) && r.rebuilt_shards.contains(&4));

        // 重建后读取一致 + 分片存在
        let got = store.get("b.bin").await.unwrap();
        assert_eq!(&got[..], &data[..]);
        assert!(store
            .inner()
            .exists(&ErasureStore::shard_path("b.bin", 3))
            .await
            .unwrap());
        assert!(store
            .inner()
            .exists(&ErasureStore::shard_path("b.bin", 4))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn beyond_tolerance_unrecoverable() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "c.bin", 11, 900).await;

        // 删除 3 片 > parity=2 → 无法重建
        for i in 0..3usize {
            store
                .inner()
                .delete(&ErasureStore::shard_path("c.bin", i))
                .await
                .unwrap();
        }

        let heal = HealCoordinator::new(store.clone());
        let r = heal.heal_object("c.bin").await.unwrap();
        assert_eq!(r.action, HealAction::Unrecoverable);
        assert!(!r.errors.is_empty());
        // 读也应失败（TooManyShardsMissing）
        assert!(store.get("c.bin").await.is_err());
    }

    #[tokio::test]
    async fn heal_report_groups_by_object() {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        put_big(&store, "a.bin", 2, 512).await;
        put_big(&store, "b.bin", 4, 512).await;

        // 各腐坏 1 片
        for p in ["a.bin", "b.bin"] {
            let sp = ErasureStore::shard_path(p, 0);
            let shard = store.inner().get(&sp).await.unwrap().to_vec();
            let mut corrupt = shard.clone();
            corrupt[0] ^= 0x0F;
            store
                .inner()
                .put(&sp, "application/octet-stream", Bytes::from(corrupt))
                .await
                .unwrap();
        }

        let det = BitrotDetector::new(store.clone());
        let scan = det.scan_paths(&["a.bin", "b.bin"]).await.unwrap();
        assert_eq!(scan.affected_objects, 2);

        let heal = HealCoordinator::new(store.clone());
        let report = heal
            .heal_report(&scan_to_report(&scan.details))
            .await
            .unwrap();
        assert_eq!(report.objects_healed, 2);
        assert_eq!(report.shards_rebuilt, 2);
        assert_eq!(report.unrecoverable, 0);
        assert_eq!(report.objects_ok, 0);

        // 自愈后全部干净
        let scan2 = det.scan_paths(&["a.bin", "b.bin"]).await.unwrap();
        assert_eq!(scan2.corruptions, 0);
    }

    fn scan_to_report(details: &[Corruption]) -> ScanReport {
        ScanReport {
            objects_scanned: 0,
            shards_checked: 0,
            corruptions: details.to_vec(),
        }
    }
}
