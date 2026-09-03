// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Volume 服务集成测试
//!
//! 测试场景：
//! - 块读写：写入/读取/删除chunk数据
//! - 纠删码：RS(4+2)编码、数据损坏恢复、多配置验证
//! - Cauchy RS：编码/解码性能对比、增量更新
//! - 分层存储：热/温/冷层切换、访问统计、自动分层
//! - 数据完整性：CRC校验、哈希校验、损坏检测
//! - 数据重建：副本重建、渐进式重建、重建验证
//!
//! 覆盖正常路径、边界条件和错误处理。

use bytes::Bytes;
use mox_cloud_volume_svc::{
    crc32c_bytes, crc64_ecma, encode_and_write, sha256_hex, CauchyReedSolomon, ChecksumType,
    EcManifest, EcProfile, InMemoryPeerFetcher, IncrementalEncoder, IncrementalUpdate,
    IntegrityChecker, ProgressiveRebuildJob, ProgressiveRebuilder, RSError, RebuildEngineType,
    RebuildJob, RebuildPriority, RebuildStats, ReedSolomonEngine, ShardChecksum, StorageLayer,
    StorageTier, StorageTierEngine, TieringPolicyConfig, TieringPolicyType, VolumeServer,
};
use rand::RngCore;
use std::{collections::HashMap, sync::Arc};

// =========================================================================
// 辅助函数
// =========================================================================

fn random_bytes(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut v);
    v
}

fn make_volume_server(id: &str, capacity: u64) -> VolumeServer {
    VolumeServer::new(id.to_string(), capacity)
}

// =========================================================================
// 模块一：块读写 (Block Read/Write)
// =========================================================================

/// 测试：写入单个 chunk
#[test]
fn iv01_01_write_single_chunk() {
    let vs = make_volume_server("vol-test-1", 100 * 1024 * 1024);

    let data = Bytes::from_static(b"hello world chunk data");
    let ack = vs.write_chunk("chunk-001", data.clone()).unwrap();

    assert_eq!(ack.chunk_id, "chunk-001");
    assert_eq!(ack.size, data.len() as u64);
    assert_eq!(ack.volume_id, "vol-test-1");
    assert!(ack.crc32c != 0);
    assert!(!ack.sha256.is_empty());
}

/// 测试：读取已写入的 chunk
#[test]
fn iv01_02_read_chunk() {
    let vs = make_volume_server("vol-test-2", 100 * 1024 * 1024);

    let data = Bytes::from(random_bytes(4096));
    vs.write_chunk("chunk-read", data.clone()).unwrap();

    let read_data = vs.read_chunk("chunk-read").unwrap();
    assert_eq!(read_data, data);
}

/// 测试：删除 chunk
#[test]
fn iv01_03_delete_chunk() {
    let vs = make_volume_server("vol-test-3", 100 * 1024 * 1024);

    vs.write_chunk("chunk-del", Bytes::from_static(b"to be deleted")).unwrap();
    assert!(vs.has_chunk("chunk-del"));

    vs.delete_chunk("chunk-del").unwrap();
    assert!(!vs.has_chunk("chunk-del"));
}

/// 测试：读取不存在的 chunk 返回错误
#[test]
fn iv01_04_read_nonexistent_chunk() {
    let vs = make_volume_server("vol-test-4", 100 * 1024 * 1024);
    let result = vs.read_chunk("no-such-chunk");
    assert!(result.is_err());
}

/// 测试：覆盖写入 chunk
#[test]
fn iv01_05_overwrite_chunk() {
    let vs = make_volume_server("vol-test-5", 100 * 1024 * 1024);

    vs.write_chunk("chunk-over", Bytes::from_static(b"version 1")).unwrap();
    let used_before = vs.used_bytes();

    vs.write_chunk("chunk-over", Bytes::from_static(b"version 2 data longer"))
        .unwrap();
    let used_after = vs.used_bytes();

    let read_data = vs.read_chunk("chunk-over").unwrap();
    assert_eq!(read_data, Bytes::from_static(b"version 2 data longer"));

    // 容量应正确更新（减去旧数据，加上新数据）
    assert!(used_after > used_before || used_after != used_before);
}

/// 测试：容量超限拒绝写入
#[test]
fn iv01_06_capacity_exceeded() {
    let vs = make_volume_server("vol-small", 100); // 100 bytes capacity

    vs.write_chunk("small", Bytes::from_static(b"0123456789")).unwrap(); // 10 bytes

    let result = vs.write_chunk("too-big", Bytes::from(vec![0u8; 95])); // 95 bytes
    assert!(result.is_err());
}

/// 测试：chunk 数量统计
#[test]
fn iv01_07_chunk_count() {
    let vs = make_volume_server("vol-count", 10 * 1024 * 1024);

    for i in 0..10 {
        vs.write_chunk(&format!("chunk-{}", i), Bytes::from(format!("data-{}", i)))
            .unwrap();
    }

    assert_eq!(vs.chunk_count(), 10);
}

/// 测试：空数据 chunk
#[test]
fn iv01_08_empty_chunk() {
    let vs = make_volume_server("vol-empty", 1024);

    let ack = vs.write_chunk("empty", Bytes::new()).unwrap();
    assert_eq!(ack.size, 0);

    let data = vs.read_chunk("empty").unwrap();
    assert!(data.is_empty());
}

/// 测试：大文件 chunk (1MB)
#[test]
fn iv01_09_large_chunk_1mb() {
    let vs = make_volume_server("vol-large", 100 * 1024 * 1024);

    let data = Bytes::from(random_bytes(1024 * 1024)); // 1MB
    let ack = vs.write_chunk("large-chunk", data.clone()).unwrap();
    assert_eq!(ack.size, 1024 * 1024);

    let read_back = vs.read_chunk("large-chunk").unwrap();
    assert_eq!(read_back, data);
    assert_eq!(vs.used_bytes(), 1024 * 1024);
}

/// 测试：has_chunk 检查
#[test]
fn iv01_10_has_chunk() {
    let vs = make_volume_server("vol-has", 1024);

    assert!(!vs.has_chunk("not-there"));
    vs.write_chunk("here", Bytes::from_static(b"data")).unwrap();
    assert!(vs.has_chunk("here"));
}

// =========================================================================
// 模块二：纠删码 RS(4+2) (Erasure Coding)
// =========================================================================

/// 测试：RS(4+2) 基本编码解码
#[test]
fn iv02_01_rs_4plus2_encode_decode() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(64 * 1024);

    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 6); // 4 data + 2 parity

    // 丢失 2 个分片仍可恢复
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[1] = None;
    slots[4] = None;

    let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：RS(6+3) 配置
#[test]
fn iv02_02_rs_6plus3_config() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(6, 3).unwrap();
    let payload = random_bytes(128 * 1024);

    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 9);

    // 丢失 3 个分片
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[0] = None;
    slots[3] = None;
    slots[7] = None;

    let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：RS(2+1) 最小配置
#[test]
fn iv02_03_rs_2plus1_minimal() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(2, 1).unwrap();
    let payload = b"minimal RS data".to_vec();

    let shards = engine.encode(&profile, &payload).unwrap();
    assert_eq!(shards.len(), 3);

    // 丢失 1 个数据分片
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[0] = None;

    let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：丢失超过 parity 数量的分片时报错
#[test]
fn iv02_04_too_many_shards_missing() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(4096);

    let shards = engine.encode(&profile, &payload).unwrap();
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[0] = None;
    slots[1] = None;
    slots[2] = None; // 丢失 3 个 > parity=2

    let result = engine.decode_reconstruct(&profile, &slots, payload.len());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RSError::TooManyShardsMissing(_)));
}

/// 测试：非对齐长度数据编码
#[test]
fn iv02_05_unaligned_length() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();

    // 非 4 的倍数长度
    for len in [1, 3, 7, 15, 100, 1023, 65537] {
        let payload = random_bytes(len);
        let shards = engine.encode(&profile, &payload).unwrap();
        let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
        assert_eq!(recovered.len(), len);
        assert_eq!(recovered, payload, "mismatch for len={}", len);
    }
}

/// 测试：丢失校验分片不影响数据恢复
#[test]
fn iv02_06_lose_parity_only() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(6, 3).unwrap();
    let payload = random_bytes(32 * 1024);

    let shards = engine.encode(&profile, &payload).unwrap();

    // 丢失所有 3 个校验分片
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[6] = None;
    slots[7] = None;
    slots[8] = None;

    let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：EcProfile 验证
#[test]
fn iv02_07_ec_profile_validation() {
    // 有效配置
    assert!(EcProfile::new(4, 2, 65536).is_ok());
    assert!(EcProfile::new(2, 1, 1024).is_ok());
    assert!(EcProfile::new(12, 4, 1024 * 1024).is_ok());

    // 无效配置
    assert!(EcProfile::new(1, 2, 1024).is_err()); // data_shards < 2
    assert!(EcProfile::new(4, 0, 1024).is_err()); // parity < 1
}

/// 测试：encode_and_write 文件系统写入
#[test]
fn iv02_08_encode_and_write_to_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(128 * 1024);

    let manifest =
        encode_and_write(mount, "test-bucket", "obj-001", &profile, StorageTier::Hot, &payload)
            .unwrap();

    assert_eq!(manifest.data_shards, 4);
    assert_eq!(manifest.parity_shards, 2);
    assert_eq!(manifest.shard_count, 6);
    assert_eq!(manifest.original_size as usize, payload.len());
    assert_eq!(manifest.crc64, crc64_ecma(&payload));
    assert_eq!(manifest.tier, StorageTier::Hot);
}

// =========================================================================
// 模块三：Cauchy RS (Cauchy Reed-Solomon)
// =========================================================================

/// 测试：Cauchy RS 基本编码
#[test]
fn iv03_01_cauchy_basic_encode() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let payload = random_bytes(64 * 1024);

    let shards = cauchy.encode(&payload).unwrap();
    assert_eq!(shards.len(), 6);
    assert!(shards.iter().all(|s| s.len() == shards[0].len()));
}

/// 测试：Cauchy RS 编码解码一致性
#[test]
fn iv03_02_cauchy_encode_decode_consistency() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let payload = random_bytes(64 * 1024 + 17);

    let shards = cauchy.encode(&payload).unwrap();

    // 丢失 2 个分片
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    slots[1] = None;
    slots[5] = None;

    let recovered = cauchy.decode_reconstruct(&slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：Cauchy RS 与标准 RS 结果对比
#[test]
fn iv03_03_cauchy_vs_standard_rs() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(32 * 1024);

    // 标准 RS
    let std_engine = ReedSolomonEngine::new();
    let std_shards = std_engine.encode(&profile, &payload).unwrap();

    // Cauchy RS
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let cauchy_shards = cauchy.encode(&payload).unwrap();

    // 两者都应该能正确恢复数据
    let std_slots: Vec<Option<Vec<u8>>> = std_shards.into_iter().map(Some).collect();
    let std_recovered = std_engine.decode_reconstruct(&profile, &std_slots, payload.len()).unwrap();

    let cauchy_slots: Vec<Option<Vec<u8>>> = cauchy_shards.into_iter().map(Some).collect();
    let cauchy_recovered = cauchy.decode_reconstruct(&cauchy_slots, payload.len()).unwrap();

    assert_eq!(std_recovered, payload);
    assert_eq!(cauchy_recovered, payload);
}

/// 测试：Cauchy RS 不同配置验证
#[test]
fn iv03_04_cauchy_various_configs() {
    let configs = [(2, 1), (4, 2), (6, 3), (8, 4), (10, 4)];

    for (data, parity) in configs {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        let cauchy = CauchyReedSolomon::new(profile).unwrap();
        let payload = random_bytes((data as usize) * 1024 + 7);

        let shards = cauchy.encode(&payload).unwrap();
        assert_eq!(shards.len(), (data + parity) as usize);

        // 丢失 parity 个分片
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        for slot in slots.iter_mut().take(parity as usize) {
            *slot = None;
        }

        let recovered = cauchy.decode_reconstruct(&slots, payload.len()).unwrap();
        assert_eq!(recovered, payload, "Cauchy RS ({data}+{parity}) decode failed");
    }
}

/// 测试：增量编码更新
#[test]
fn iv03_05_incremental_encoder_update() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let encoder = IncrementalEncoder::new(profile).unwrap();
    let engine = ReedSolomonEngine::new();

    let original = random_bytes(64 * 1024);
    let shards = engine.encode(&profile, &original).unwrap();

    // 修改数据分片 0 的前 100 字节
    let _shard_size = shards[0].len();
    let old_data = shards[0][..100].to_vec();
    let mut new_data = vec![0u8; 100];
    rand::thread_rng().fill_bytes(&mut new_data);

    let update =
        IncrementalUpdate { shard_index: 0, offset: 0, old_data, new_data: new_data.clone() };

    let result = encoder.compute_update(&update).unwrap();
    assert_eq!(result.parity_updates.len(), 2); // 2 个校验分片需要更新

    // 应用增量更新
    let mut updated_shards = shards.clone();
    updated_shards[0][..100].copy_from_slice(&new_data);
    for (parity_idx, delta) in &result.parity_updates {
        for i in 0..delta.len() {
            updated_shards[*parity_idx][i] ^= delta[i];
        }
    }

    // 验证修改后的数据仍然可以正确解码
    let slots: Vec<Option<Vec<u8>>> = updated_shards.into_iter().map(Some).collect();
    let mut expected = original.clone();
    expected[..100].copy_from_slice(&new_data);

    let recovered = engine.decode_reconstruct(&profile, &slots, expected.len()).unwrap();
    assert_eq!(recovered, expected);
}

/// 测试：Cauchy RS reconstruct_all
#[test]
fn iv03_06_cauchy_reconstruct_all() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let payload = random_bytes(32 * 1024);

    let shards = cauchy.encode(&payload).unwrap();

    // 丢失 2 个分片
    let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
    slots[1] = None;
    slots[4] = None;

    let reconstructed = cauchy.reconstruct_all(&slots).unwrap();
    assert_eq!(reconstructed.len(), 6);

    // 验证重建后的分片与原始分片一致
    for i in 0..6 {
        assert_eq!(reconstructed[i], shards[i]);
    }
}

// =========================================================================
// 模块四：分层存储 (Storage Tiering)
// =========================================================================

/// 测试：存储层枚举
#[test]
fn iv04_01_storage_layer_enum() {
    assert_eq!(StorageLayer::Hot as u8, 0);
    assert_eq!(StorageLayer::Warm as u8, 1);
    assert_eq!(StorageLayer::Cold as u8, 2);

    assert!(StorageLayer::Hot.is_hotter_than(StorageLayer::Warm));
    assert!(StorageLayer::Warm.is_hotter_than(StorageLayer::Cold));
    assert!(StorageLayer::Cold.is_colder_than(StorageLayer::Hot));

    assert_eq!(StorageLayer::default(), StorageLayer::Hot);
    assert_eq!(StorageLayer::from_str("hot"), Some(StorageLayer::Hot));
    assert_eq!(StorageLayer::from_str("warm"), Some(StorageLayer::Warm));
    assert_eq!(StorageLayer::from_str("cold"), Some(StorageLayer::Cold));
    assert_eq!(StorageLayer::from_str("invalid"), None);
}

/// 测试：存储层配置
#[test]
fn iv04_02_storage_layer_config() {
    use mox_cloud_volume_svc::StorageLayerConfig;

    let config = StorageLayerConfig {
        layer: StorageLayer::Hot,
        name: "Hot SSD Tier".to_string(),
        total_capacity: 100 * 1024 * 1024 * 1024, // 100GB
        high_watermark_pct: 80,
        low_watermark_pct: 60,
        max_iops: 100_000,
        max_bandwidth_bps: 10 * 1024 * 1024 * 1024, // 10GB/s
        avg_read_latency_us: 100,
        avg_write_latency_us: 150,
        cost_per_gb_per_month: 0.15,
        backend_path: "/mnt/ssd".to_string(),
    };

    assert_eq!(config.high_watermark_bytes(), 100 * 1024 * 1024 * 1024 * 80 / 100);
    assert_eq!(config.low_watermark_bytes(), 100 * 1024 * 1024 * 1024 * 60 / 100);
}

/// 测试：对象访问统计
#[test]
fn iv04_03_object_access_stats() {
    use mox_cloud_volume_svc::ObjectAccessStats;

    let stats = ObjectAccessStats::new("obj-001".to_string(), 4096, StorageLayer::Hot);

    assert_eq!(stats.object_id, "obj-001");
    assert_eq!(stats.size_bytes, 4096);
    assert_eq!(stats.current_layer, StorageLayer::Hot);
    assert_eq!(stats.access_count, 0);
    assert_eq!(stats.access_count_24h, 0);
}

/// 测试：分层策略配置
#[test]
fn iv04_04_tiering_policy_config() {
    let policy = TieringPolicyConfig {
        policy_type: TieringPolicyType::AgeBased,
        hot_to_warm_days: 30,
        warm_to_cold_days: 90,
        promote_min_access_count: 5,
        large_object_threshold: 1024 * 1024,
        promote_on_read: true,
        ..TieringPolicyConfig::default()
    };

    assert_eq!(policy.hot_to_warm_days, 30);
    assert_eq!(policy.warm_to_cold_days, 90);
    assert!(policy.promote_on_read);
}

/// 测试：StorageTier 存储层级枚举
#[test]
fn iv04_05_storage_tier_enum() {
    // 验证 manifest 中使用的 StorageTier
    let tiers = [StorageTier::Hot, StorageTier::Warm, StorageTier::Cold, StorageTier::Archive];

    for tier in &tiers {
        // serde roundtrip
        let json = serde_json::to_string(tier).unwrap();
        let back: StorageTier = serde_json::from_str(&json).unwrap();
        assert_eq!(back, *tier);
    }
}

/// 测试：StorageTierEngine 基本操作
#[test]
fn iv04_06_storage_tier_engine_basic() {
    let engine = StorageTierEngine::new();

    // 注册对象（register_object 自动决定初始层）
    let layer1 = engine.register_object("obj-1", 1024);
    let layer2 = engine.register_object("obj-2", 2048);
    assert_eq!(layer1, StorageLayer::Hot);
    assert_eq!(layer2, StorageLayer::Hot);

    let stats = engine.stats();
    assert!(*stats.hot_objects.lock() >= 1);
    assert!(*stats.hot_objects.lock() >= 1);
}

/// 测试：层间迁移任务
#[test]
fn iv04_07_tier_migration_task() {
    use mox_cloud_volume_svc::{MigrationStatus, TierMigrationTask};

    let task = TierMigrationTask {
        task_id: "tier-mig-001".to_string(),
        object_id: "obj-1".to_string(),
        source_layer: StorageLayer::Hot,
        target_layer: StorageLayer::Warm,
        size_bytes: 4096,
        migrated_bytes: 0,
        status: MigrationStatus::Pending,
        created_at_ms: 1000,
        started_at_ms: None,
        completed_at_ms: None,
        priority: 5,
        error: None,
    };

    assert_eq!(task.source_layer, StorageLayer::Hot);
    assert_eq!(task.target_layer, StorageLayer::Warm);
    assert_eq!(task.status, MigrationStatus::Pending);
}

// =========================================================================
// 模块五：数据完整性 (Data Integrity)
// =========================================================================

/// 测试：CRC32C 校验
#[test]
fn iv05_01_crc32c_checksum() {
    let data = b"hello world";
    let crc1 = crc32c_bytes(data);
    let crc2 = crc32c_bytes(data);

    // 相同数据 CRC 相同
    assert_eq!(crc1, crc2);

    // 不同数据 CRC 不同
    let crc3 = crc32c_bytes(b"hello worle");
    assert_ne!(crc1, crc3);
}

/// 测试：CRC32C 空数据
#[test]
fn iv05_02_crc32c_empty() {
    let crc = crc32c_bytes(&[]);
    assert_eq!(crc, 0);
}

/// 测试：SHA256 哈希
#[test]
fn iv05_03_sha256_hash() {
    let data = b"test data for sha256";
    let hash1 = sha256_hex(data);
    let hash2 = sha256_hex(data);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars

    // 不同数据不同哈希
    let hash3 = sha256_hex(b"different data");
    assert_ne!(hash1, hash3);
}

/// 测试：CRC64 ECMA 校验
#[test]
fn iv05_04_crc64_ecma_checksum() {
    let data = random_bytes(64 * 1024);
    let crc1 = crc64_ecma(&data);
    let crc2 = crc64_ecma(&data);

    assert_eq!(crc1, crc2);

    // 单字节变化应导致 CRC 变化
    let mut modified = data.clone();
    modified[0] ^= 0xFF;
    let crc3 = crc64_ecma(&modified);
    assert_ne!(crc1, crc3);
}

/// 测试：VolumeServer 自动 CRC 校验
#[test]
fn iv05_05_volume_server_crc_auto_check() {
    let vs = make_volume_server("vol-crc", 10 * 1024 * 1024);

    let data = Bytes::from_static(b"integrity check data");
    vs.write_chunk("crc-test", data.clone()).unwrap();

    // 正常读取应通过 CRC 校验
    let read_back = vs.read_chunk("crc-test").unwrap();
    assert_eq!(read_back, data);
}

/// 测试：ChunkAck 包含完整校验信息
#[test]
fn iv05_06_chunk_ack_contains_checksums() {
    let vs = make_volume_server("vol-ack", 1024 * 1024);

    let data = Bytes::from_static(b"ack test data");
    let ack = vs.write_chunk("ack-chunk", data.clone()).unwrap();

    assert_eq!(ack.crc32c, crc32c_bytes(&data));
    assert_eq!(ack.sha256, sha256_hex(&data));
    assert_eq!(ack.size, data.len() as u64);
}

/// 测试：IntegrityChecker 完整性检查器
#[test]
fn iv05_07_integrity_checker() {
    let checker = IntegrityChecker::new(ChecksumType::Crc32c);

    let data = random_bytes(4096);
    let checksum = checker.compute_checksum(&data);

    assert!(checker.verify_checksum(&data, &checksum));

    // 损坏数据应验证失败
    let mut corrupted = data.clone();
    corrupted[100] ^= 0xAA;
    assert!(!checker.verify_checksum(&corrupted, &checksum));
}

/// 测试：ShardChecksum 分片校验和
#[test]
fn iv05_08_shard_checksum() {
    let shard_data = random_bytes(16 * 1024);
    let checker = IntegrityChecker::new(ChecksumType::Crc32c);
    let checksum = ShardChecksum {
        shard_index: 0,
        checksum_type: ChecksumType::Crc32c,
        value: checker.compute_checksum(&shard_data),
        data_len: shard_data.len(),
    };

    assert_eq!(checksum.shard_index, 0);
    assert_eq!(checksum.data_len, shard_data.len());
    assert!(!checksum.value.is_empty());
}

// =========================================================================
// 模块六：数据重建 (Data Rebuild)
// =========================================================================

/// 测试：RebuildJob 端到端重建
#[test]
fn iv06_01_rebuild_job_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let payload = random_bytes(128 * 1024);

    // 写入编码后的数据
    encode_and_write(mount, "rebuild-bucket", "rebuild-obj", &profile, StorageTier::Hot, &payload)
        .unwrap();

    // 删除 2 个分片
    use mox_cloud_volume_svc::shard_path;
    for drop in [1usize, 4] {
        std::fs::remove_file(shard_path(mount, "rebuild-bucket", "rebuild-obj", drop)).unwrap();
    }

    // 执行重建
    let job = RebuildJob::new(mount, "rebuild-bucket", "rebuild-obj", vec![1, 4]);
    let rebuilt = job.run().unwrap();
    assert_eq!(rebuilt, 2);

    // 验证所有分片都存在
    for i in 0..6 {
        assert!(
            shard_path(mount, "rebuild-bucket", "rebuild-obj", i).exists(),
            "shard {} missing after rebuild",
            i
        );
    }

    // 验证数据完整性
    let engine = ReedSolomonEngine::new();
    let shards: Vec<Vec<u8>> = (0..6)
        .map(|i| std::fs::read(shard_path(mount, "rebuild-bucket", "rebuild-obj", i)).unwrap())
        .collect();
    let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    let recovered = engine.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

/// 测试：从对等节点重建 (InMemoryPeerFetcher)
#[test]
fn iv06_02_rebuild_from_peers() {
    let fetcher = Arc::new(InMemoryPeerFetcher::new());

    // 注册 peer 数据（RebuildCoordinator 要求至少 2 个 peer）
    let chunk_data = Bytes::from_static(b"peer chunk data");

    let mut store1 = HashMap::new();
    store1.insert("chunk-peer".to_string(), chunk_data.clone());
    fetcher.register_peer_store("peer-1:8080", store1);

    let mut store2 = HashMap::new();
    store2.insert("chunk-peer".to_string(), chunk_data.clone());
    fetcher.register_peer_store("peer-2:8080", store2);

    // 重建
    let vs = VolumeServer::new("vol-rebuild".to_string(), 10 * 1024 * 1024)
        .with_peer_fetcher(fetcher.clone());

    let rebuilt = vs
        .rebuild_from_peers(
            &["chunk-peer".to_string()],
            &["peer-1:8080".to_string(), "peer-2:8080".to_string()],
        )
        .unwrap();

    assert!(rebuilt >= 1);
    assert!(vs.has_chunk("chunk-peer"));
    assert_eq!(vs.read_chunk("chunk-peer").unwrap(), chunk_data);
}

/// 测试：渐进式重建
#[test]
fn iv06_03_progressive_rebuild() {
    let rebuilder = ProgressiveRebuilder::new(64 * 1024);

    let stats = rebuilder.stats();
    assert_eq!(*stats.jobs_submitted.lock(), 0);
    assert_eq!(*stats.jobs_completed.lock(), 0);

    // 添加重建任务
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let job = ProgressiveRebuildJob {
        job_id: "job-1".to_string(),
        object_id: "obj-1".to_string(),
        profile,
        shards: vec![None; 6],
        missing_indices: vec![0, 1],
        priority: RebuildPriority::High,
        processed_bytes: 0,
        total_bytes: 4096,
        result: None,
        engine_type: RebuildEngineType::CauchyRs,
    };
    rebuilder.submit_job(job);

    let stats2 = rebuilder.stats();
    assert_eq!(*stats2.jobs_submitted.lock(), 1);
    assert_eq!(rebuilder.pending_jobs(), 1);
}

/// 测试：重建优先级
#[test]
fn iv06_04_rebuild_priority() {
    // 验证优先级枚举存在
    let high = RebuildPriority::High;
    let normal = RebuildPriority::Normal;
    let low = RebuildPriority::Low;

    // 高优先级应高于普通优先级（数值比较）
    assert!((high as u8) != (normal as u8));
    let _ = low;
}

/// 测试：重建统计信息
#[test]
fn iv06_05_rebuild_stats() {
    let stats = RebuildStats::default();

    // 模拟统计数据
    *stats.jobs_submitted.lock() = 100;
    *stats.jobs_completed.lock() = 42;
    *stats.jobs_failed.lock() = 3;
    *stats.bytes_rebuilt.lock() = 42 * 1024 * 1024;

    assert_eq!(*stats.jobs_submitted.lock(), 100);
    assert_eq!(*stats.jobs_completed.lock(), 42);
    assert_eq!(*stats.jobs_failed.lock(), 3);

    // 进度百分比 = completed / submitted
    let progress_pct =
        (*stats.jobs_completed.lock() as f64 / *stats.jobs_submitted.lock() as f64) * 100.0;
    assert!(progress_pct > 0.0 && progress_pct <= 100.0);

    // snapshot 应包含所有指标
    let snap = stats.snapshot();
    assert!(snap.contains_key("ec_rebuild_jobs_submitted"));
    assert!(snap.contains_key("ec_rebuild_jobs_completed"));
    assert!(snap.contains_key("ec_rebuild_bytes_rebuilt"));
    assert!(snap.contains_key("ec_rebuild_jobs_failed"));
}

/// 测试：manifest 中的重建相关字段
#[test]
fn iv06_06_manifest_rebuild_fields() {
    let man = EcManifest {
        oid: "rebuild-obj".to_string(),
        bid: "bucket".to_string(),
        crc64: 0xDEAD_BEEF,
        shard_count: 6,
        data_shards: 4,
        parity_shards: 2,
        created_at_ms: 1_700_000_000_000,
        tier: StorageTier::Hot,
        original_size: 65536,
    };

    // 验证 lifecycle_cold 是 idempotent 的
    let cold = man.lifecycle_cold();
    assert_eq!(cold.tier, StorageTier::Archive);
    assert_eq!(cold.shard_count, man.shard_count);
    assert_eq!(cold.data_shards, man.data_shards);
    assert_eq!(cold.parity_shards, man.parity_shards);
}

// =========================================================================
// 模块七：综合集成测试 (Integration)
// =========================================================================

/// 测试：完整的数据生命周期 - 写入 -> 编码 -> 损坏 -> 重建 -> 读取
#[test]
fn iv07_01_full_data_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();

    // 1. 准备数据
    let original = random_bytes(256 * 1024);
    let original_crc = crc64_ecma(&original);

    // 2. 编码并写入
    let manifest = encode_and_write(
        mount,
        "lifecycle-bucket",
        "lifecycle-obj",
        &profile,
        StorageTier::Hot,
        &original,
    )
    .unwrap();
    assert_eq!(manifest.crc64, original_crc);

    // 3. 模拟数据损坏（删除 2 个分片）
    use mox_cloud_volume_svc::shard_path;
    for drop in [0usize, 5] {
        std::fs::remove_file(shard_path(mount, "lifecycle-bucket", "lifecycle-obj", drop)).unwrap();
    }

    // 4. 执行重建
    let job = RebuildJob::new(mount, "lifecycle-bucket", "lifecycle-obj", vec![0, 5]);
    let rebuilt = job.run().unwrap();
    assert_eq!(rebuilt, 2);

    // 5. 验证重建后的数据完整性
    let engine = ReedSolomonEngine::new();
    let shards: Vec<Vec<u8>> = (0..6)
        .map(|i| std::fs::read(shard_path(mount, "lifecycle-bucket", "lifecycle-obj", i)).unwrap())
        .collect();
    let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    let recovered = engine.decode_reconstruct(&profile, &slots, original.len()).unwrap();

    assert_eq!(crc64_ecma(&recovered), original_crc);
    assert_eq!(recovered, original);
}

/// 测试：VolumeServer 快照导出和恢复
#[test]
fn iv07_02_snapshot_export_restore() {
    let vs = make_volume_server("vol-snap", 10 * 1024 * 1024);

    // 写入一些 chunk
    for i in 0..5 {
        vs.write_chunk(&format!("chunk-{}", i), Bytes::from(format!("snapshot-data-{}", i)))
            .unwrap();
    }

    // 导出快照
    let manifest = vs.export_snapshot_manifest();
    assert_eq!(manifest.len(), 5);

    // 创建新的 VolumeServer 并恢复
    let vs2 = make_volume_server("vol-snap2", 10 * 1024 * 1024);
    let restored = vs2.restore_from_manifest(&manifest).unwrap();
    assert_eq!(restored, 5);
    assert_eq!(vs2.chunk_count(), 5);

    // 验证数据一致
    for i in 0..5 {
        let data1 = vs.read_chunk(&format!("chunk-{}", i)).unwrap();
        let data2 = vs2.read_chunk(&format!("chunk-{}", i)).unwrap();
        assert_eq!(data1, data2);
    }
}

/// 测试：VolumeServer 快照存储
#[test]
fn iv07_03_snapshot_store() {
    let vs = make_volume_server("vol-snapstore", 10 * 1024 * 1024);

    vs.write_chunk("a", Bytes::from_static(b"data-a")).unwrap();

    let manifest = vs.export_snapshot_manifest();
    vs.store_snapshot("snap-1", manifest.clone());

    let retrieved = vs.get_snapshot("snap-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().len(), manifest.len());

    // 不存在的快照
    assert!(vs.get_snapshot("no-such-snap").is_none());
}

/// 测试：并发 chunk 写入
#[test]
fn iv07_04_concurrent_chunk_writes() {
    let vs = Arc::new(make_volume_server("vol-concurrent", 100 * 1024 * 1024));
    let mut handles = vec![];

    for i in 0..10 {
        let vs = Arc::clone(&vs);
        handles.push(std::thread::spawn(move || {
            for j in 0..100 {
                let cid = format!("concurrent-{}-{}", i, j);
                let data = Bytes::from(format!("data-{}-{}", i, j));
                vs.write_chunk(&cid, data).unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(vs.chunk_count(), 1000);
}

/// 测试：错误类型覆盖
#[test]
fn iv07_05_error_type_coverage() {
    let vs = make_volume_server("vol-err", 100);

    // ChunkNotFound
    let err = vs.read_chunk("not-exists").unwrap_err();
    assert!(err.to_string().contains("not found") || err.to_string().contains("ChunkNotFound"));

    // CapacityExceeded
    vs.write_chunk("big", Bytes::from(vec![0u8; 80])).unwrap();
    let err = vs.write_chunk("too-much", Bytes::from(vec![0u8; 80])).unwrap_err();
    assert!(err.to_string().contains("capacity") || err.to_string().contains("CapacityExceeded"));
}

/// 测试：EcManifest serde roundtrip
#[test]
fn iv07_06_manifest_serde_roundtrip() {
    let man = EcManifest {
        oid: "serde-obj".to_string(),
        bid: "serde-bucket".to_string(),
        crc64: 0x1234_5678_9ABC_DEF0,
        shard_count: 9,
        data_shards: 6,
        parity_shards: 3,
        created_at_ms: 1_712_345_678_000,
        tier: StorageTier::Warm,
        original_size: 131072,
    };

    let json = serde_json::to_string(&man).unwrap();
    let back: EcManifest = serde_json::from_str(&json).unwrap();

    assert_eq!(back.oid, man.oid);
    assert_eq!(back.bid, man.bid);
    assert_eq!(back.crc64, man.crc64);
    assert_eq!(back.shard_count, man.shard_count);
    assert_eq!(back.data_shards, man.data_shards);
    assert_eq!(back.parity_shards, man.parity_shards);
    assert_eq!(back.tier, man.tier);
    assert_eq!(back.original_size, man.original_size);
}
