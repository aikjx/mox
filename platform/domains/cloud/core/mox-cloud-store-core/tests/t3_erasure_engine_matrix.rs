// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段3 集成测试：EC 纠删码引擎矩阵 + bitrot→自愈流水线（feature `erasure`）。
//!
//! 覆盖计划 §阶段3 验证项：
//! - **载荷矩阵**：0/1/部分分片/整块/大块 → EC 往返逐字节一致。
//! - **丢片矩阵**：多载荷 × 丢 1..=parity 片 → 重建逐字节一致。
//! - **腐坏检测**：翻转分片字节 → crc32c 识别为 corrupt。
//! - **自愈流水线**：腐坏+缺失 → bitrot 扫描 → heal 重建 → 复扫干净。
//! - **装饰器叠加**：EC + Cache + Snapshot 组合装配的端到端正确性。

#![cfg(feature = "erasure")]

use bytes::Bytes;
use mox_base_store_core::ObjectStore;
use mox_cloud_store_core::bitrot::BitrotDetector;
use mox_cloud_store_core::cache::{CacheConfig, ObjectCache};
use mox_cloud_store_core::erasure::{ErasureConfig, ErasureStore};
use mox_cloud_store_core::fs_backend::FsObjectStore;
use mox_cloud_store_core::heal::{HealAction, HealCoordinator};
use mox_cloud_store_core::snapshot::SnapshotManager;
use mox_cloud_volume_svc::EcProfile;
use std::path::Path;
use std::sync::Arc;

/// 确定性载荷生成（非平凡混合字节，含 0x00/0xFF 边界）。
fn payload(seed: u32, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut acc: u32 = seed.wrapping_mul(2654435761).wrapping_add(0x9E37_79B9);
    for _ in 0..len {
        acc = acc.wrapping_mul(1664525).wrapping_add(1013904223);
        out.push((acc >> 24) as u8);
    }
    out
}

/// 以 min_obj_size=0 装配 EC 存储（所有载荷强制走 RS-EC 路径）。
fn ec_store(dir: &Path) -> Arc<ErasureStore> {
    let base = Arc::new(FsObjectStore::new(dir.to_path_buf()).unwrap());
    let profile = EcProfile::new(4, 2, 0).unwrap();
    Arc::new(ErasureStore::new(
        base,
        ErasureConfig {
            enabled: true,
            profile,
        },
    ))
}

// ---------------------------------------------------------------------------
// 载荷矩阵：0/1/部分/整块/大块 → EC 往返逐字节一致
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ec_roundtrip_payload_matrix() {
    // 0=空载荷；1..3=垫零部分分片；63/64=min 边界；100/251=奇偶不对齐；
    // 1024/4096=整块；65536/65537=大块边界；100000=大块
    for len in [0usize, 1, 2, 3, 5, 63, 64, 65, 100, 251, 1024, 4096, 65536, 65537, 100000] {
        let dir = tempfile::tempdir().unwrap();
        let store = ec_store(dir.path());
        let data = payload(len as u32 ^ 0xA5, len);
        let key = format!("o{len}.bin");
        store
            .put(&key, "application/octet-stream", Bytes::from(data.clone()))
            .await
            .unwrap();
        // min_obj_size=0 → 所有载荷（含空）均走 EC，确认 manifest 存在
        assert!(store.read_manifest(&key).await.unwrap().is_some(), "len={len} 应走 EC");
        let got = store.get(&key).await.unwrap();
        assert_eq!(&got[..], &data[..], "len={len} 往返不一致");
        // head/exists 一致
        let h = store.head(&key).await.unwrap();
        assert_eq!(h.size_bytes, len as u64, "head size len={len}");
        assert!(store.exists(&key).await.unwrap());
    }
}

// ---------------------------------------------------------------------------
// 丢片矩阵：多载荷 × 丢 1..=parity 片 → 重建逐字节一致
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lost_shard_matrix_reconstructs_byte_identical() {
    for len in [64usize, 100, 251, 1024, 4096, 65536] {
        for lost in 1..=2usize {
            let dir = tempfile::tempdir().unwrap();
            let store = ec_store(dir.path());
            let data = payload(len as u32 ^ (lost as u32) * 31, len);
            store
                .put("m.bin", "application/octet-stream", Bytes::from(data.clone()))
                .await
                .unwrap();
            // 底层直接删除 lost 个分片
            let inner = store.inner().clone();
            for i in 0..lost {
                inner
                    .delete(&ErasureStore::shard_path("m.bin", i))
                    .await
                    .unwrap();
            }
            let got = store.get("m.bin").await.unwrap();
            assert_eq!(&got[..], &data[..], "len={len} lost={lost} 重建不一致");
        }
    }
}

// ---------------------------------------------------------------------------
// 腐坏检测 + bitrot→heal 自愈流水线
// ---------------------------------------------------------------------------

#[tokio::test]
async fn corrupted_shard_detected_via_crc() {
    let dir = tempfile::tempdir().unwrap();
    let store = ec_store(dir.path());
    let data = payload(0x1234, 2048);
    store
        .put("c.bin", "application/octet-stream", Bytes::from(data.clone()))
        .await
        .unwrap();

    // 翻转分片 3 的一个字节
    let sp = ErasureStore::shard_path("c.bin", 3);
    let shard = store.inner().get(&sp).await.unwrap().to_vec();
    let mut corrupt = shard.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0x80;
    store
        .inner()
        .put(&sp, "application/octet-stream", Bytes::from(corrupt))
        .await
        .unwrap();

    let det = BitrotDetector::new(store.clone());
    let scan = det.scan_object("c.bin").await.unwrap();
    assert_eq!(scan.corruptions.len(), 1);
    assert_eq!(scan.corruptions[0].shard_index, 3);
    assert_eq!(scan.corruptions[0].kind, "corrupt");

    // 腐坏不影响读（走重建）
    let got = store.get("c.bin").await.unwrap();
    assert_eq!(&got[..], &data[..]);
}

#[tokio::test]
async fn bitrot_heal_pipeline_repairs_and_cleans() {
    let dir = tempfile::tempdir().unwrap();
    let store = ec_store(dir.path());
    let data = payload(0xDEAD, 4096);
    store
        .put("doc.bin", "application/octet-stream", Bytes::from(data.clone()))
        .await
        .unwrap();

    // 腐坏分片 0 + 物理删除分片 1（2 问题片 ≤ parity=2，容错内）
    for (i, mode) in [(0usize, "corrupt"), (1, "missing")] {
        let sp = ErasureStore::shard_path("doc.bin", i);
        match mode {
            "corrupt" => {
                let shard = store.inner().get(&sp).await.unwrap().to_vec();
                let mut corrupt = shard.clone();
                corrupt[0] ^= 0x3C;
                store
                    .inner()
                    .put(&sp, "application/octet-stream", Bytes::from(corrupt))
                    .await
                    .unwrap();
            }
            "missing" => {
                store.inner().delete(&sp).await.unwrap();
            }
            _ => unreachable!(),
        }
    }

    // bitrot 扫描 → 2 条问题
    let det = BitrotDetector::new(store.clone());
    let scan = det.scan_object("doc.bin").await.unwrap();
    assert_eq!(scan.corruptions.len(), 2);
    assert_eq!(
        scan.corruptions.iter().filter(|c| c.kind == "corrupt").count(),
        1
    );
    assert_eq!(
        scan.corruptions.iter().filter(|c| c.kind == "missing").count(),
        1
    );

    // heal → 全部重建写回
    let heal = HealCoordinator::new(store.clone());
    let r = heal.heal_object("doc.bin").await.unwrap();
    assert_eq!(r.action, HealAction::Rebuild);
    assert_eq!(r.rebuilt_shards.len(), 2);
    assert!(r.errors.is_empty());

    // 复扫干净 + 数据可读一致
    let scan2 = det.scan_object("doc.bin").await.unwrap();
    assert!(scan2.corruptions.is_empty(), "自愈后扫描应干净: {:?}", scan2.corruptions);
    let got = store.get("doc.bin").await.unwrap();
    assert_eq!(&got[..], &data[..]);

    // 全量自愈计数
    assert!(heal.total_rebuilt() >= 2);
}

// ---------------------------------------------------------------------------
// 装饰器叠加：EC + Cache + Snapshot 组合装配
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ec_cache_snapshot_decorator_stack() {
    let dir = tempfile::tempdir().unwrap();
    let inner: Arc<dyn ObjectStore> =
        Arc::new(FsObjectStore::new(dir.path().to_path_buf()).unwrap());
    // EC 装饰
    let profile = EcProfile::new(4, 2, 0).unwrap();
    let ec = Arc::new(ErasureStore::new(
        inner.clone(),
        ErasureConfig {
            enabled: true,
            profile,
        },
    ));
    // Cache 装饰在 EC 之上
    let cache = Arc::new(ObjectCache::new(
        ec.clone(),
        CacheConfig {
            capacity_bytes: 64 * 1024 * 1024,
            max_entry_bytes: 8 * 1024 * 1024,
        },
    ));

    let data = payload(0x777, 8192);
    cache
        .put("big.bin", "application/octet-stream", Bytes::from(data.clone()))
        .await
        .unwrap();

    // 快照（基于 EC 底层）
    let sm = SnapshotManager::new(ec.clone(), Some(dir.path().to_path_buf()));
    let info = sm
        .create_snapshot("snap-ec", "baseline", &["big.bin".into()])
        .await
        .unwrap();
    assert_eq!(info.paths, vec!["big.bin".to_string()]);

    // 覆盖生产对象
    cache
        .put("big.bin", "application/octet-stream", Bytes::from(payload(1, 8192)))
        .await
        .unwrap();

    // 快照恢复 → 得到原始数据（COW 隔离）
    let restored = sm.restore_snapshot("snap-ec").await.unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(&restored[0].1[..], &data[..], "快照应保留写入时数据");

    // 缓存层仍可读（生产数据 = 新值）
    let current = cache.get("big.bin").await.unwrap();
    assert_ne!(&current[..], &data[..], "生产对象应已被覆盖");

    // 删除快照清理
    sm.delete_snapshot("snap-ec").await.unwrap();
    assert!(sm.list_snapshots().await.unwrap().is_empty());
}
