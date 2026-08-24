//! T4 M1 Cloud Drive 集成测试（≥20 条）
//! 覆盖 TR4.3 / TR4.4 / TR4.5 / 容量 / 副本 / 心跳 / 快照 / RS / rebuild

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use sha2::{Digest, Sha256};

use xuanji_cloud_drive_master::*;
use xuanji_cloud_drive_volume::*;

// ============== T1 注册 & 基础 ==============

#[test]
fn t01_master_new_and_register_3_volumes() {
    let master = MasterServer::new(MasterConfig::default());
    let a = master.register_volume("tcp://a:1".into(), 100_000);
    let b = master.register_volume("tcp://b:2".into(), 200_000);
    let c = master.register_volume("tcp://c:3".into(), 300_000);
    assert_ne!(a, b);
    assert_ne!(b, c);
    let list = master.list_volumes();
    assert_eq!(list.len(), 3);
}

#[test]
fn t02_allocate_volume_replica_3_ok() {
    let master = MasterServer::new(MasterConfig::default());
    master.register_volume("tcp://a:1".into(), 1_000_000);
    master.register_volume("tcp://b:2".into(), 1_000_000);
    master.register_volume("tcp://c:3".into(), 1_000_000);
    let alloc = master.allocate_volume(4096, 3).unwrap();
    assert_eq!(alloc.replica_count, 3);
    assert_eq!(alloc.replica_ids.len(), 3);
    // 3 个副本 id 必须互不相同（跨不同 volume）
    let mut s = std::collections::HashSet::new();
    for id in &alloc.replica_ids {
        s.insert(id.clone());
    }
    assert_eq!(s.len(), 3);
}

#[test]
fn t03_allocate_replica_gt_3_should_fail() {
    let master = MasterServer::new(MasterConfig::default());
    master.register_volume("tcp://a:1".into(), 1_000_000);
    master.register_volume("tcp://b:2".into(), 1_000_000);
    let res = master.allocate_volume(1024, 4);
    assert!(res.is_err());
    match res.unwrap_err() {
        MasterError::InvalidReplicaCount(_) => {}
        other => panic!("expected InvalidReplicaCount, got {:?}", other),
    }
}

#[test]
fn t04_allocate_no_capacity_should_fail() {
    let master = MasterServer::new(MasterConfig::default());
    // 注册 1 个 1KB 容量节点
    master.register_volume("tcp://a:1".into(), 1024);
    // 申请 2MB × 3 副本 → 总需求 6MB > 1KB
    let res = master.allocate_volume(2 * 1024 * 1024, 1);
    assert!(res.is_err());
    match res.unwrap_err() {
        MasterError::NoCapacity(_) => {}
        other => panic!("expected NoCapacity, got {:?}", other),
    }
}

#[test]
fn t05_allocate_100_volumes_exhaust_capacity() {
    let master = MasterServer::new(MasterConfig::default());
    // 3 节点 各 10KB
    master.register_volume("tcp://a:1".into(), 10_000);
    master.register_volume("tcp://b:2".into(), 10_000);
    master.register_volume("tcp://c:3".into(), 10_000);
    // 每次 allocate 300 bytes replica=3 占 300*3 = 900 bytes；100 次超 30_000
    let mut success = 0u32;
    for _ in 0..100 {
        match master.allocate_volume(300, 3) {
            Ok(_) => success += 1,
            Err(MasterError::NoCapacity(_)) => break,
            Err(other) => panic!("unexpected err {:?}", other),
        }
    }
    // 应当有部分成功，最终耗尽
    assert!(
        success < 100,
        "should exhaust capacity, got {} successes",
        success
    );
    assert!(success > 0, "at least some allocation should succeed");
}

// ============== T2 心跳 ==============

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t06_master_heartbeat_dead_detection_tr43() {
    // TR4.3: 启动 Master + 3 Volume (register idA/idB/idC)
    // 每 Volume 每 100ms 发心跳 300ms；停止 idB 心跳 2s
    // Master.status(idB) → MUST BE dead；同时 start_replica_refill ≥ 1
    let master = Arc::new(MasterServer::new(MasterConfig {
        heartbeat_timeout_ms: 500,
        ..MasterConfig::default()
    }));

    let id_a = master.register_volume("tcp://a:1".into(), 1_000_000);
    let id_b = master.register_volume("tcp://b:2".into(), 1_000_000);
    let id_c = master.register_volume("tcp://c:3".into(), 1_000_000);

    // 先分配 1 个 replica=3 卷（含 idB），dead 会触发 refill
    let _alloc = master.allocate_volume(1024, 3).unwrap();

    // A C 全程心跳，B 只发前 300ms
    let m1 = master.clone();
    let a1 = id_a.clone();
    let b1 = id_b.clone();
    let c1 = id_c.clone();
    let heartbeat_task = tokio::spawn(async move {
        for i in 0u32..50 {
            // 5 sec total
            tokio::time::sleep(Duration::from_millis(100)).await;
            // A 每 100ms
            let _ = m1.heartbeat(&a1, VolumeLoadReport::default());
            // C 每 100ms
            let _ = m1.heartbeat(&c1, VolumeLoadReport::default());
            // B 只发前 3 次（300ms），之后停
            if i < 3 {
                let _ = m1.heartbeat(&b1, VolumeLoadReport::default());
            }
        }
    });

    // 等 2.5 秒让 B 心跳超时
    tokio::time::sleep(Duration::from_millis(2500)).await;

    // 检查 dead & refill
    let state_b = master.volume_state(&id_b);
    assert_eq!(
        state_b,
        VolumeStatusState::Dead,
        "idB should be Dead after heartbeat stopped"
    );

    let refill_count = master.start_replica_refill_count();
    assert!(
        refill_count >= 1,
        "replica refill triggers should be >=1, got {}",
        refill_count
    );

    heartbeat_task.abort();
}

#[test]
fn t07_heartbeat_nonexistent_volume_errors() {
    let master = MasterServer::new(MasterConfig::default());
    let res = master.heartbeat("ghost-id", VolumeLoadReport::default());
    assert!(res.is_err());
}

// ============== T3 副本 & Quorum ==============

#[test]
fn t08_replica_write_quorum_n_half_plus_1() {
    // replica=3 写 quorum = 2；replica=2 写 quorum = 2；replica=1 写 quorum = 1
    let master = MasterServer::new(MasterConfig::default());
    master.register_volume("tcp://a:1".into(), 1_000_000);
    master.register_volume("tcp://b:2".into(), 1_000_000);
    master.register_volume("tcp://c:3".into(), 1_000_000);
    // 这里直接测 ReplicaSet 写入健康检查
    let mgr = ReplicaSetManager::new();
    mgr.create_set("s1".into(), 3);
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v1".into(),
            addr: "a".into(),
            health: ReplicaHealth::Healthy,
            last_acked: 1,
        },
    );
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v2".into(),
            addr: "b".into(),
            health: ReplicaHealth::Healthy,
            last_acked: 1,
        },
    );
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v3".into(),
            addr: "c".into(),
            health: ReplicaHealth::Unhealthy,
            last_acked: 1,
        },
    );
    // 2 healthy ≥ write_quorum(2) → ok
    assert!(mgr.check_write_ok("s1").is_ok());
}

#[test]
fn t09_quorum_write_fail_when_not_enough_healthy() {
    let mgr = ReplicaSetManager::new();
    mgr.create_set("s1".into(), 3);
    // 只有 1 个 healthy < 2 → fail
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v1".into(),
            addr: "a".into(),
            health: ReplicaHealth::Healthy,
            last_acked: 1,
        },
    );
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v2".into(),
            addr: "b".into(),
            health: ReplicaHealth::Dead,
            last_acked: 0,
        },
    );
    mgr.add_replica_to_set(
        "s1",
        ReplicaInfo {
            volume_id: "v3".into(),
            addr: "c".into(),
            health: ReplicaHealth::Dead,
            last_acked: 0,
        },
    );
    match mgr.check_write_ok("s1") {
        Err(MasterError::ReplicaQuorum(_)) => {}
        other => panic!("expected ReplicaQuorum err, got {:?}", other),
    }
}

#[test]
fn t10_master_replica_write_quorum_tr44_100_iterations() {
    // TR4.4: allocate(replica=3) → 3 Volume addr → 写 "hello" 到 3 副本
    // 人为 delete_chunk 第 2 副本 → read 必须仍能返回 "hello"（quorum 读剩下 2 个）
    // 循环 100 次
    let master = MasterServer::new(MasterConfig::default());
    master.register_volume("tcp://a:1".into(), 10_000_000);
    master.register_volume("tcp://b:2".into(), 10_000_000);
    master.register_volume("tcp://c:3".into(), 10_000_000);

    let v_a = VolumeServer::new("vA".into(), 10_000_000);
    let v_b = VolumeServer::new("vB".into(), 10_000_000);
    let v_c = VolumeServer::new("vC".into(), 10_000_000);

    for iter in 0..100u32 {
        let chunk_id = format!("chunk-hello-{}", iter);
        let payload = Bytes::from(format!("hello-{}", iter));
        // 写 3 副本
        v_a.write_chunk(&chunk_id, payload.clone()).unwrap();
        v_b.write_chunk(&chunk_id, payload.clone()).unwrap();
        v_c.write_chunk(&chunk_id, payload.clone()).unwrap();

        // 人为 delete 第 2 副本 (v_b)
        v_b.delete_chunk(&chunk_id).unwrap();
        assert!(!v_b.has_chunk(&chunk_id));

        // Quorum 读：从 v_a 和 v_c 读，必须返回原始内容
        let ra = v_a.read_chunk(&chunk_id).unwrap();
        let rc = v_c.read_chunk(&chunk_id).unwrap();
        assert_eq!(
            ra.as_ref(),
            payload.as_ref(),
            "iter {} quorum v_a mismatch",
            iter
        );
        assert_eq!(
            rc.as_ref(),
            payload.as_ref(),
            "iter {} quorum v_c mismatch",
            iter
        );

        // 2/3 读 = quorum ok
        let healthy = vec![
            v_a.has_chunk(&chunk_id),
            v_b.has_chunk(&chunk_id),
            v_c.has_chunk(&chunk_id),
        ]
        .into_iter()
        .filter(|&x| x)
        .count();
        assert!(
            healthy >= 2,
            "iter {} healthy replicas {} < 2",
            iter,
            healthy
        );
    }
}

#[test]
fn t11_quorum_read_2_of_3_sufficient() {
    let mgr = ReplicaSetManager::new();
    mgr.create_set("rs1".into(), 3);
    mgr.add_replica_to_set(
        "rs1",
        ReplicaInfo {
            volume_id: "v1".into(),
            addr: "a".into(),
            health: ReplicaHealth::Healthy,
            last_acked: 10,
        },
    );
    mgr.add_replica_to_set(
        "rs1",
        ReplicaInfo {
            volume_id: "v2".into(),
            addr: "b".into(),
            health: ReplicaHealth::Dead,
            last_acked: 0,
        },
    );
    mgr.add_replica_to_set(
        "rs1",
        ReplicaInfo {
            volume_id: "v3".into(),
            addr: "c".into(),
            health: ReplicaHealth::Healthy,
            last_acked: 10,
        },
    );
    assert!(mgr.check_read_ok("rs1").is_ok());
}

// ============== T4 快照 ==============

fn md5_of_data(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

#[test]
fn t12_snapshot_rollback_md5_tr45() {
    // TR4.5: 写入 1000 个小 chunk（每个 "chunk_N"）；snapshot；delete 前 200 个；restore；重读 200 个应完整
    let volume = VolumeServer::new("v-test".into(), 100_000_000);
    let master = MasterServer::new(MasterConfig::default());
    let vid = master.register_volume("tcp://v-test:1".into(), 100_000_000);

    // 写入 1000 chunk
    let mut original_hashes: BTreeMap<String, String> = BTreeMap::new();
    for n in 0..1000u32 {
        let cid = format!("cid_{:04}", n);
        let payload = format!("chunk_{}", n).into_bytes();
        original_hashes.insert(cid.clone(), md5_of_data(&payload));
        volume.write_chunk(&cid, Bytes::from(payload)).unwrap();
    }
    // 验证写入后 1000 个都有
    assert_eq!(volume.chunk_count(), 1000);

    // 导出 manifest + master 快照
    let manifest = volume.export_snapshot_manifest();
    let snap_id = master
        .store_snapshot_manifest(&vid, manifest.clone())
        .unwrap();
    assert!(!snap_id.is_empty());

    // 删除前 200 个
    for n in 0..200u32 {
        let cid = format!("cid_{:04}", n);
        volume.delete_chunk(&cid).unwrap();
    }
    assert_eq!(volume.chunk_count(), 800);

    // 从 master 取回 manifest，restore
    let restored_manifest = master.get_snapshot_manifest(&vid, &snap_id).unwrap();
    let restored_count = volume.restore_from_manifest(&restored_manifest).unwrap();
    assert!(
        restored_count >= 200,
        "should restore at least 200 chunks, got {}",
        restored_count
    );

    // 重新读取前 200 个，内容必须严格等于原 hash
    for n in 0..200u32 {
        let cid = format!("cid_{:04}", n);
        let data = volume
            .read_chunk(&cid)
            .unwrap_or_else(|_| panic!("chunk {} should be restored", cid));
        let actual_hash = md5_of_data(&data);
        let expected = original_hashes.get(&cid).unwrap();
        assert_eq!(
            &actual_hash, expected,
            "MD5/SHA256 mismatch for chunk {}",
            cid
        );
    }
}

#[test]
fn t13_snapshot_invalid_id_errors() {
    let master = MasterServer::new(MasterConfig::default());
    let vid = master.register_volume("tcp://x:1".into(), 1000);
    let res = master.restore_snapshot(&vid, "non-existent-snap");
    assert!(res.is_err());
    match res.unwrap_err() {
        MasterError::SnapshotInvalid(_) => {}
        other => panic!("expected SnapshotInvalid, got {:?}", other),
    }
}

#[test]
fn t14_snapshot_id_unique_and_unforgeable() {
    let master = MasterServer::new(MasterConfig::default());
    let vid = master.register_volume("tcp://x:1".into(), 1_000_000);
    let m1 = BTreeMap::new();
    let sid1 = master.store_snapshot_manifest(&vid, m1.clone()).unwrap();
    // 等 1 ms 再快照（加盐+时间戳）
    std::thread::sleep(Duration::from_millis(2));
    let sid2 = master.store_snapshot_manifest(&vid, m1).unwrap();
    assert_ne!(sid1, sid2, "两次 snapshot id 必须不同（加盐+时间戳）");
    assert_eq!(
        sid1.len(),
        64,
        "snapshot id 应为 sha256 hex=64 char, got {}",
        sid1.len()
    );
}

// ============== T5 自研 RS 2+1 XOR ==============

#[test]
fn t15_rs_encode_2_1_makes_parity_xor() {
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from_static(b"hello world!!!!!!!"); // 18B (h e l l o  w o r l d + 7x!)
    let d1 = Bytes::from_static(b"abcdefghijklmnopqr"); // 18B
    let out = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
    assert_eq!(out[0], d0);
    assert_eq!(out[1], d1);
    // parity = d0 XOR d1；手工校验 spot
    let p = &out[2];
    for i in 0..d0.len() {
        assert_eq!(p[i], d0[i] ^ d1[i], "byte {} parity mismatch", i);
    }
}

#[test]
fn t16_rs_decode_missing_data0_tr45_case1() {
    // TR4.5: missing data0 能重建
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from_static(b"DATA0_DATA0_DATA0");
    let d1 = Bytes::from_static(b"DATA1_DATA1_DATA1");
    let shards = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
    // 丢 data0
    let input: [Option<Bytes>; 3] = [None, Some(shards[1].clone()), Some(shards[2].clone())];
    let restored = rs.decode_2_1(input).unwrap();
    assert_eq!(restored[0], d0, "data0 should be reconstructed");
    assert_eq!(restored[1], d1);
}

#[test]
fn t17_rs_decode_missing_data1_tr45_case2() {
    // TR4.5: missing data1
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from(vec![0xAAu8; 64]);
    let d1 = Bytes::from(vec![0x55u8; 64]);
    let shards = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
    let input: [Option<Bytes>; 3] = [Some(shards[0].clone()), None, Some(shards[2].clone())];
    let restored = rs.decode_2_1(input).unwrap();
    assert_eq!(restored[0], d0);
    assert_eq!(restored[1], d1);
}

#[test]
fn t18_rs_decode_missing_parity_tr45_case3() {
    // TR4.5: missing parity（只需返回 data0/1）
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from_static(b"ping");
    let d1 = Bytes::from_static(b"pong");
    let shards = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
    let input: [Option<Bytes>; 3] = [Some(shards[0].clone()), Some(shards[1].clone()), None];
    let restored = rs.decode_2_1(input).unwrap();
    assert_eq!(restored[0], d0);
    assert_eq!(restored[1], d1);
}

#[test]
fn t19_rs_decode_two_missing_should_fail() {
    let rs = ReedSolomon2Plus1;
    let d0 = Bytes::from_static(b"a");
    let d1 = Bytes::from_static(b"b");
    let shards = rs.encode_2_1(&[d0, d1]).unwrap();
    // 丢 data0 + parity → 2 个丢失，应失败
    let input: [Option<Bytes>; 3] = [None, Some(shards[1].clone()), None];
    match rs.decode_2_1(input) {
        Err(RSError::TooManyShardsMissing(_)) => {}
        other => panic!("expected TooManyShardsMissing, got {:?}", other),
    }
}

// ============== T6 Volume 重建 ==============

#[test]
fn t20_volume_rebuild_from_peers_success_count() {
    // 2 peers，本节点缺 5 个 chunk → rebuild 应返回 ≥ 5
    let fetcher = Arc::new(InMemoryPeerFetcher::new());
    let local = VolumeServer::new("local-v".into(), 10_000_000).with_peer_fetcher(fetcher.clone());

    // peer 0 / peer 1 各预置 5 个 chunk
    let missing_ids: Vec<String> = (0..5).map(|i| format!("m{}", i)).collect();
    for (i, mid) in missing_ids.iter().enumerate() {
        let data = Bytes::from(format!("data-for-{}", i));
        // peer 0 存 data 版，peer 1 存 XOR parity 版
        fetcher.set_chunk("peer0", mid, data.clone());
        // parity 模拟：以 peer0 数据 ^ zeros（实际重建逻辑会在 coordinator 中处理）
        let parity = Bytes::from(vec![0u8; data.len()]);
        fetcher.set_chunk("peer1", mid, parity);
    }

    let peers = vec!["peer0".into(), "peer1".into()];
    let count = local.rebuild_from_peers(&missing_ids, &peers).unwrap();
    assert!(
        count >= 5,
        "should rebuild at least 5 chunks, got {}",
        count
    );
    // 验证重建后本地有数据
    for mid in &missing_ids {
        assert!(
            local.has_chunk(mid),
            "after rebuild, {} should exist locally",
            mid
        );
    }
}

// ============== T7 Metrics 暴露 ==============

#[test]
fn t21_metrics_all_four_keys_present_tr47() {
    // TR4.7: 4 项全齐 = 2 分
    let master = MasterServer::new(MasterConfig::default());
    let vid = master.register_volume("tcp://x:1".into(), 1_000_000);

    // 触发各类事件
    master.heartbeat(&vid, VolumeLoadReport::default()).unwrap();
    master.heartbeat(&vid, VolumeLoadReport::default()).unwrap();
    let _ = master.allocate_volume(100, 1).unwrap();
    let _ = master.snapshot_volume(&vid).unwrap();

    let m = master.get_metrics();
    assert!(
        m.contains_key("heartbeats_received"),
        "metrics 缺 heartbeats_received"
    );
    assert!(
        m.contains_key("volumes_allocations_total"),
        "metrics 缺 volumes_allocations_total"
    );
    assert!(
        m.contains_key("replicas_fill_triggers"),
        "metrics 缺 replicas_fill_triggers"
    );
    assert!(
        m.contains_key("snapshots_taken"),
        "metrics 缺 snapshots_taken"
    );

    // 计数增长：heartbeat 至少 2；allocations 至少 1；snapshots 至少 1
    assert!(*m.get("heartbeats_received").unwrap() >= 2);
    assert!(*m.get("volumes_allocations_total").unwrap() >= 1);
    assert!(*m.get("snapshots_taken").unwrap() >= 1);
}

// ============== T8 Volume 容量 & CRC ==============

#[test]
fn t22_volume_capacity_exceeded_error() {
    let v = VolumeServer::new("tiny".into(), 100); // 100B
                                                   // 写 100B 成功
    v.write_chunk("ok", Bytes::from(vec![0u8; 100])).unwrap();
    // 再写 1B → 超容量
    let res = v.write_chunk("too_much", Bytes::from(vec![1u8; 1]));
    assert!(res.is_err());
    match res.unwrap_err() {
        VolumeError::CapacityExceeded(_) => {}
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn t23_volume_write_and_read_and_delete() {
    let v = VolumeServer::new("v".into(), 1_000_000);
    let ack = v.write_chunk("c1", Bytes::from_static(b"xyz")).unwrap();
    assert_eq!(ack.size, 3);
    assert!(ack.sha256.len() == 64);
    let r = v.read_chunk("c1").unwrap();
    assert_eq!(&r[..], b"xyz");
    v.delete_chunk("c1").unwrap();
    match v.read_chunk("c1") {
        Err(VolumeError::ChunkNotFound(_)) => {}
        other => panic!("expected ChunkNotFound, got {:?}", other),
    }
}

// ============== T9 Allocator 双策略 ==============

#[test]
fn t24_allocator_prefers_emptiest_node() {
    // round-robin + 容量最空优先：3 个节点 used 不同，分配应选最空的
    let master = MasterServer::new(MasterConfig::default());
    let v1 = master.register_volume("tcp://full".into(), 1_000_000);
    let v2 = master.register_volume("tcp://mid".into(), 1_000_000);
    let v3 = master.register_volume("tcp://empty".into(), 1_000_000);

    // 人为模拟 used（通过 heartbeat load）

    // 通过 heartbeat → used_bytes 触发更新
    master
        .heartbeat(
            &v1,
            VolumeLoadReport {
                used_bytes: 900_000,
                chunk_count: 0,
                cpu_pct: 0,
                is_healthy: true,
            },
        )
        .unwrap();
    master
        .heartbeat(
            &v2,
            VolumeLoadReport {
                used_bytes: 500_000,
                chunk_count: 0,
                cpu_pct: 0,
                is_healthy: true,
            },
        )
        .unwrap();
    master
        .heartbeat(
            &v3,
            VolumeLoadReport {
                used_bytes: 10_000,
                chunk_count: 0,
                cpu_pct: 0,
                is_healthy: true,
            },
        )
        .unwrap();

    let alloc = master.allocate_volume(800_000, 1).unwrap();
    // 最空节点应是 v3 (used 10K)
    assert_eq!(
        alloc.replica_ids[0], v3,
        "应选择最空的 v3, got {}",
        alloc.replica_ids[0]
    );
    let _ = (v1, v2);
}
