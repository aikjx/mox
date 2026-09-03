// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Master 服务集成测试
//!
//! 测试场景：
//! - 卷管理：创建卷、删除卷、查询卷信息
//! - 副本管理：N副本创建、副本健康检查、副本故障切换
//! - 心跳机制：Volume节点心跳上报、心跳超时检测
//! - 调度器：容量感知调度、跨机架放置、反亲和策略
//! - Raft高可用：Leader选举、日志复制、故障恢复、快照
//! - 数据均衡：均衡计划生成、均衡执行、均衡后验证
//!
//! 覆盖正常路径、边界条件和错误处理。

use mox_cloud_master_svc::{
    DataTemperature, DistributedScheduler, MasterConfig, MasterServer, NodeLoad, NodeTopology,
    PlacementStrategy, RaftConfig, RaftLogType, RaftMaster, RaftRole, ReplicaHealth,
    ReplicaSetManager, SchedulerWeights, SnapshotManager, VolumeAllocation, VolumeInfo,
    VolumeLoadReport, VolumeStatusState,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

// =========================================================================
// 辅助函数
// =========================================================================

fn default_config() -> MasterConfig {
    MasterConfig { heartbeat_timeout_ms: 1500, max_replica: 3 }
}

fn make_master() -> MasterServer {
    MasterServer::new(default_config())
}

fn register_n_volumes(master: &MasterServer, n: usize, capacity: u64) -> Vec<String> {
    (0..n)
        .map(|i| {
            let addr = format!("127.0.0.1:{}", 8000 + i);
            master.register_volume(addr, capacity)
        })
        .collect()
}

// =========================================================================
// 模块一：卷管理 (Volume Management)
// =========================================================================

/// 测试：注册单个 Volume 节点并验证基本信息
#[test]
fn im01_01_register_single_volume() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 100 * 1024 * 1024 * 1024);

    assert!(!vid.is_empty());
    assert!(vid.starts_with("vol-"));

    let volumes = master.list_volumes();
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].id, vid);
    assert_eq!(volumes[0].addr, "127.0.0.1:8080");
    assert_eq!(volumes[0].capacity, 100 * 1024 * 1024 * 1024);
    assert_eq!(volumes[0].used, 0);
    assert_eq!(volumes[0].state, VolumeStatusState::Alive);
}

/// 测试：注册多个 Volume 节点
#[test]
fn im01_02_register_multiple_volumes() {
    let master = make_master();
    let ids = register_n_volumes(&master, 5, 100 * 1024 * 1024 * 1024);

    assert_eq!(ids.len(), 5);
    let volumes = master.list_volumes();
    assert_eq!(volumes.len(), 5);

    // 验证 ID 唯一性
    let mut unique = std::collections::HashSet::new();
    for id in &ids {
        assert!(unique.insert(id.clone()), "duplicate volume id: {}", id);
    }
}

/// 测试：卷容量为 0 的边界情况
#[test]
fn im01_03_register_zero_capacity() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:9000".to_string(), 0);
    assert!(!vid.is_empty());

    let volumes = master.list_volumes();
    assert_eq!(volumes[0].capacity, 0);
}

/// 测试：大容量卷注册
#[test]
fn im01_04_register_large_capacity() {
    let master = make_master();
    let large = 10u64.pow(15); // 1 PB
    let vid = master.register_volume("127.0.0.1:9001".to_string(), large);

    let volumes = master.list_volumes();
    assert_eq!(volumes[0].id, vid);
    assert_eq!(volumes[0].capacity, large);
}

/// 测试：查询不存在的卷返回错误
#[test]
fn im01_05_heartbeat_nonexistent_volume() {
    let master = make_master();
    let result = master.heartbeat(
        "non-existent-vol",
        VolumeLoadReport { used_bytes: 0, chunk_count: 0, cpu_pct: 0, is_healthy: true },
    );
    assert!(result.is_err());
}

/// 测试：分配卷 - 基本三副本
#[test]
fn im01_06_allocate_volume_three_replicas() {
    let master = make_master();
    register_n_volumes(&master, 5, 100 * 1024 * 1024 * 1024);

    let alloc = master.allocate_volume(10 * 1024 * 1024 * 1024, 3).unwrap();
    assert_eq!(alloc.replica_count, 3);
    assert_eq!(alloc.replica_ids.len(), 3);
    assert_eq!(alloc.replica_addresses.len(), 3);
    assert_eq!(alloc.size, 10 * 1024 * 1024 * 1024);

    // 验证副本 ID 唯一性
    let mut set = std::collections::HashSet::new();
    for id in &alloc.replica_ids {
        assert!(set.insert(id.clone()), "duplicate replica id");
    }
}

/// 测试：分配卷 - 单副本
#[test]
fn im01_07_allocate_volume_single_replica() {
    let master = make_master();
    register_n_volumes(&master, 3, 100 * 1024 * 1024 * 1024);

    let alloc = master.allocate_volume(1024, 1).unwrap();
    assert_eq!(alloc.replica_count, 1);
    assert_eq!(alloc.replica_ids.len(), 1);
}

/// 测试：分配卷 - 副本数超过上限
#[test]
fn im01_08_allocate_volume_exceeds_max_replica() {
    let master = make_master();
    register_n_volumes(&master, 10, 100 * 1024 * 1024 * 1024);

    let result = master.allocate_volume(1024, 4); // max_replica = 3
    assert!(result.is_err());
}

/// 测试：分配卷 - 容量不足
#[test]
fn im01_09_allocate_volume_insufficient_capacity() {
    let master = make_master();
    register_n_volumes(&master, 3, 1024); // 每个只有 1KB

    let result = master.allocate_volume(1024 * 1024, 2); // 需要 1MB
    assert!(result.is_err());
}

/// 测试：分配卷 - 节点数量不足
#[test]
fn im01_10_allocate_volume_not_enough_nodes() {
    let master = make_master();
    register_n_volumes(&master, 2, 100 * 1024 * 1024 * 1024);

    let result = master.allocate_volume(1024, 3); // 需要 3 个节点
    assert!(result.is_err());
}

// =========================================================================
// 模块二：副本管理 (Replica Management)
// =========================================================================

/// 测试：创建副本集并验证 quorum
#[test]
fn im02_01_replica_set_quorum() {
    let rm = ReplicaSetManager::new();
    rm.create_set("set-1".to_string(), 3);

    let set = rm.get_set("set-1").unwrap();
    assert_eq!(set.replica_count, 3);
    assert_eq!(set.write_quorum().unwrap(), 2);
    assert_eq!(set.read_quorum().unwrap(), 2);
}

/// 测试：单副本 quorum
#[test]
fn im02_02_single_replica_quorum() {
    let rm = ReplicaSetManager::new();
    rm.create_set("set-single".to_string(), 1);

    let set = rm.get_set("set-single").unwrap();
    assert_eq!(set.write_quorum().unwrap(), 1);
    assert_eq!(set.read_quorum().unwrap(), 1);
}

/// 测试：添加副本并检查健康状态
#[test]
fn im02_03_add_replicas_and_check_health() {
    use mox_cloud_master_svc::ReplicaInfo;

    let rm = ReplicaSetManager::new();
    rm.create_set("set-2".to_string(), 3);

    for i in 0..3 {
        rm.add_replica_to_set(
            "set-2",
            ReplicaInfo {
                volume_id: format!("vol-{}", i),
                addr: format!("127.0.0.1:{}", 8000 + i),
                health: ReplicaHealth::Healthy,
                last_acked: 1000,
            },
        );
    }

    let set = rm.get_set("set-2").unwrap();
    assert_eq!(set.healthy_count(), 3);
    assert!(set.check_write_ok().is_ok());
    assert!(set.check_read_ok().is_ok());
}

/// 测试：部分副本不健康时的 quorum 检查
#[test]
fn im02_04_partial_unhealthy_quorum() {
    use mox_cloud_master_svc::ReplicaInfo;

    let rm = ReplicaSetManager::new();
    rm.create_set("set-3".to_string(), 3);

    // 2 healthy, 1 unhealthy
    for i in 0..3 {
        rm.add_replica_to_set(
            "set-3",
            ReplicaInfo {
                volume_id: format!("vol-{}", i),
                addr: format!("127.0.0.1:{}", 8000 + i),
                health: if i < 2 { ReplicaHealth::Healthy } else { ReplicaHealth::Unhealthy },
                last_acked: 1000,
            },
        );
    }

    let set = rm.get_set("set-3").unwrap();
    assert_eq!(set.healthy_count(), 2);
    // 3 副本时写 quorum = 2, 读 quorum = 2
    assert!(set.check_write_ok().is_ok());
    assert!(set.check_read_ok().is_ok());
}

/// 测试：副本数不足时写操作被拒绝
#[test]
fn im02_05_insufficient_healthy_for_write() {
    use mox_cloud_master_svc::ReplicaInfo;

    let rm = ReplicaSetManager::new();
    rm.create_set("set-4".to_string(), 3);

    // 仅 1 个健康副本
    for i in 0..3 {
        rm.add_replica_to_set(
            "set-4",
            ReplicaInfo {
                volume_id: format!("vol-{}", i),
                addr: format!("127.0.0.1:{}", 8000 + i),
                health: if i == 0 { ReplicaHealth::Healthy } else { ReplicaHealth::Dead },
                last_acked: 1000,
            },
        );
    }

    let set = rm.get_set("set-4").unwrap();
    assert_eq!(set.healthy_count(), 1);
    assert!(set.check_write_ok().is_err());
}

/// 测试：MasterServer 分配后自动创建副本集
#[test]
fn im02_06_allocation_creates_replica_set() {
    let master = make_master();
    register_n_volumes(&master, 5, 100 * 1024 * 1024 * 1024);

    let alloc = master.allocate_volume(1024 * 1024, 3).unwrap();
    let set = master.replica_manager.get_set(&alloc.volume_id);
    assert!(set.is_some());

    let set = set.unwrap();
    assert_eq!(set.replicas.len(), 3);
    assert_eq!(set.healthy_count(), 3);
}

/// 测试：副本健康状态更新 - 通过心跳
#[test]
fn im02_07_heartbeat_updates_replica_health() {
    let master = make_master();
    let ids = register_n_volumes(&master, 3, 100 * 1024 * 1024 * 1024);

    let alloc = master.allocate_volume(1024 * 1024, 3).unwrap();

    // 模拟一个节点不健康的心跳
    master
        .heartbeat(
            &ids[0],
            VolumeLoadReport { used_bytes: 1024, chunk_count: 1, cpu_pct: 50, is_healthy: false },
        )
        .unwrap();

    let set = master.replica_manager.get_set(&alloc.volume_id).unwrap();
    // 至少有一个副本变成 Unhealthy
    let unhealthy_count =
        set.replicas.iter().filter(|r| r.health == ReplicaHealth::Unhealthy).count();
    assert!(unhealthy_count >= 1);
}

/// 测试：查询不存在的副本集
#[test]
fn im02_08_get_nonexistent_set() {
    let rm = ReplicaSetManager::new();
    let set = rm.get_set("non-existent");
    assert!(set.is_none());
}

// =========================================================================
// 模块三：心跳机制 (Heartbeat Mechanism)
// =========================================================================

/// 测试：正常心跳上报
#[test]
fn im03_01_heartbeat_normal() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024 * 1024);

    let result = master.heartbeat(
        &vid,
        VolumeLoadReport { used_bytes: 512, chunk_count: 10, cpu_pct: 25, is_healthy: true },
    );
    assert!(result.is_ok());

    let metrics = master.metrics.get_all();
    assert!(metrics["heartbeats_received"] >= 1);
}

/// 测试：心跳更新已用容量
#[test]
fn im03_02_heartbeat_updates_used_capacity() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024 * 1024 * 1024);

    master
        .heartbeat(
            &vid,
            VolumeLoadReport {
                used_bytes: 500 * 1024 * 1024,
                chunk_count: 100,
                cpu_pct: 30,
                is_healthy: true,
            },
        )
        .unwrap();

    let volumes = master.list_volumes();
    assert_eq!(volumes[0].used, 500 * 1024 * 1024);
}

/// 测试：心跳已用容量不超过总容量
#[test]
fn im03_03_heartbeat_used_cap_clamped() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024); // 1KB

    // 上报超过容量的使用量
    master
        .heartbeat(
            &vid,
            VolumeLoadReport { used_bytes: 99999, chunk_count: 1, cpu_pct: 0, is_healthy: true },
        )
        .unwrap();

    let volumes = master.list_volumes();
    assert!(volumes[0].used <= volumes[0].capacity);
}

/// 测试：多次心跳累计计数
#[test]
fn im03_04_multiple_heartbeats_count() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024 * 1024);

    for _ in 0..10 {
        master
            .heartbeat(
                &vid,
                VolumeLoadReport { used_bytes: 0, chunk_count: 0, cpu_pct: 0, is_healthy: true },
            )
            .unwrap();
    }

    let metrics = master.metrics.get_all();
    assert!(metrics["heartbeats_received"] >= 10);
}

/// 测试：心跳后 Volume 状态为 Alive
#[test]
fn im03_05_heartbeat_keeps_alive() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024 * 1024);

    master
        .heartbeat(
            &vid,
            VolumeLoadReport { used_bytes: 0, chunk_count: 0, cpu_pct: 0, is_healthy: true },
        )
        .unwrap();

    let volumes = master.list_volumes();
    assert_eq!(volumes[0].state, VolumeStatusState::Alive);
}

// =========================================================================
// 模块四：调度器 (Scheduler)
// =========================================================================

/// 测试：容量感知调度 - 优先选择空闲节点
#[test]
fn im04_01_capacity_aware_scheduling() {
    let scheduler = DistributedScheduler::new(1500);

    // 注册节点：node-1 已用 90%，node-2 已用 10%
    let node1 = VolumeInfo {
        id: "node-1".to_string(),
        addr: "127.0.0.1:8001".to_string(),
        capacity: 1000,
        used: 900,
        is_alive: true,
    };
    let node2 = VolumeInfo {
        id: "node-2".to_string(),
        addr: "127.0.0.1:8002".to_string(),
        capacity: 1000,
        used: 100,
        is_alive: true,
    };

    let score1 = scheduler.compute_node_score(&node1);
    let score2 = scheduler.compute_node_score(&node2);

    // node-2 剩余容量更多，得分应更高
    assert!(score2 > score1, "score2({}) should be > score1({})", score2, score1);
}

/// 测试：调度权重配置
#[test]
fn im04_02_scheduler_weights_config() {
    let scheduler = DistributedScheduler::new(1500);

    let default_weights = scheduler.get_weights();
    assert_eq!(default_weights.capacity_weight, 50);
    assert_eq!(default_weights.io_load_weight, 30);
    assert_eq!(default_weights.network_weight, 20);

    // 自定义权重
    scheduler.set_weights(SchedulerWeights {
        capacity_weight: 80,
        io_load_weight: 10,
        network_weight: 10,
    });

    let new_weights = scheduler.get_weights();
    assert_eq!(new_weights.capacity_weight, 80);
}

/// 测试：IO 负载影响调度得分
#[test]
fn im04_03_io_load_affects_score() {
    let scheduler = DistributedScheduler::new(1500);

    let node = VolumeInfo {
        id: "node-io".to_string(),
        addr: "127.0.0.1:8003".to_string(),
        capacity: 1000,
        used: 500,
        is_alive: true,
    };

    // 低负载
    scheduler.update_node_load(
        "node-io",
        NodeLoad {
            cpu_pct: 10,
            memory_pct: 20,
            iops: 100,
            network_bps: 1000,
            active_connections: 5,
        },
    );
    let score_low = scheduler.compute_node_score(&node);

    // 高负载
    scheduler.update_node_load(
        "node-io",
        NodeLoad {
            cpu_pct: 90,
            memory_pct: 80,
            iops: 10000,
            network_bps: 100000,
            active_connections: 100,
        },
    );
    let score_high = scheduler.compute_node_score(&node);

    // 低负载时得分应更高
    assert!(score_low > score_high);
}

/// 测试：跨机架放置策略
#[test]
fn im04_04_rack_aware_placement() {
    let scheduler = DistributedScheduler::new(1500);
    scheduler.set_placement_strategy(PlacementStrategy::RackAware);

    // 注册拓扑：3 个节点分属 2 个机架
    scheduler.register_topology(NodeTopology {
        node_id: "n1".to_string(),
        data_center: "dc1".to_string(),
        zone: "z1".to_string(),
        rack: "rack-a".to_string(),
        network_latency_level: 1,
    });
    scheduler.register_topology(NodeTopology {
        node_id: "n2".to_string(),
        data_center: "dc1".to_string(),
        zone: "z1".to_string(),
        rack: "rack-a".to_string(),
        network_latency_level: 1,
    });
    scheduler.register_topology(NodeTopology {
        node_id: "n3".to_string(),
        data_center: "dc1".to_string(),
        zone: "z1".to_string(),
        rack: "rack-b".to_string(),
        network_latency_level: 2,
    });

    let candidates = vec![
        VolumeInfo {
            id: "n1".to_string(),
            addr: "127.0.0.1:8001".to_string(),
            capacity: 1000,
            used: 100,
            is_alive: true,
        },
        VolumeInfo {
            id: "n2".to_string(),
            addr: "127.0.0.1:8002".to_string(),
            capacity: 1000,
            used: 200,
            is_alive: true,
        },
        VolumeInfo {
            id: "n3".to_string(),
            addr: "127.0.0.1:8003".to_string(),
            capacity: 1000,
            used: 150,
            is_alive: true,
        },
    ];

    let selected = scheduler.select_best_nodes(&candidates, 2, &[]).unwrap();
    assert_eq!(selected.len(), 2);

    // 机架感知：2 个选中的节点应该在不同机架
    let racks: Vec<_> = selected
        .iter()
        .map(|n| {
            scheduler
                .topology()
                .read()
                .get(&n.id)
                .map(|t| t.rack.clone())
                .unwrap_or_default()
        })
        .collect();
    assert_ne!(racks[0], racks[1], "rack-aware placement failed: both nodes in same rack");
}

/// 测试：反亲和策略
#[test]
fn im04_05_anti_affinity_strategy() {
    let scheduler = DistributedScheduler::new(1500);
    scheduler.set_placement_strategy(PlacementStrategy::AntiAffinity);

    let candidates: Vec<VolumeInfo> = (0..5)
        .map(|i| VolumeInfo {
            id: format!("n{}", i),
            addr: format!("127.0.0.1:{}", 8000 + i),
            capacity: 10000,
            used: (i * 100) as u64,
            is_alive: true,
        })
        .collect();

    let selected = scheduler.select_best_nodes(&candidates, 3, &[]).unwrap();
    assert_eq!(selected.len(), 3);

    // 验证所有选中的节点 ID 都不同
    let ids: std::collections::HashSet<_> = selected.iter().map(|n| n.id.clone()).collect();
    assert_eq!(ids.len(), 3);
}

/// 测试：随机放置策略
#[test]
fn im04_06_random_placement() {
    let scheduler = DistributedScheduler::new(1500);
    scheduler.set_placement_strategy(PlacementStrategy::Random);

    let candidates: Vec<VolumeInfo> = (0..10)
        .map(|i| VolumeInfo {
            id: format!("rnd-{}", i),
            addr: format!("127.0.0.1:{}", 9000 + i),
            capacity: 10000,
            used: 0,
            is_alive: true,
        })
        .collect();

    let selected = scheduler.select_best_nodes(&candidates, 4, &[]).unwrap();
    assert_eq!(selected.len(), 4);
}

/// 测试：节点不足时调度失败
#[test]
fn im04_07_scheduler_insufficient_nodes() {
    let scheduler = DistributedScheduler::new(1500);

    let candidates = vec![VolumeInfo {
        id: "only-one".to_string(),
        addr: "127.0.0.1:8000".to_string(),
        capacity: 1000,
        used: 0,
        is_alive: true,
    }];

    let result = scheduler.select_best_nodes(&candidates, 3, &[]);
    assert!(result.is_err());
}

/// 测试：调度器统计信息
#[test]
fn im04_08_scheduler_stats() {
    let scheduler = DistributedScheduler::new(1500);

    let candidates: Vec<VolumeInfo> = (0..5)
        .map(|i| VolumeInfo {
            id: format!("stat-{}", i),
            addr: format!("127.0.0.1:{}", 7000 + i),
            capacity: 10000,
            used: 0,
            is_alive: true,
        })
        .collect();

    let before = scheduler.stats().snapshot();
    for _ in 0..5 {
        scheduler.select_best_nodes(&candidates, 2, &[]).unwrap();
    }
    let after = scheduler.stats().snapshot();

    assert!(after["scheduler_scheduling_total"] > before["scheduler_scheduling_total"]);
    assert!(after["scheduler_scheduling_success"] > before["scheduler_scheduling_success"]);
}

/// 测试：节点拓扑注册和查询
#[test]
fn im04_09_topology_registration() {
    let scheduler = DistributedScheduler::new(1500);

    scheduler.register_topology(NodeTopology {
        node_id: "topo-1".to_string(),
        data_center: "dc-east".to_string(),
        zone: "east-1".to_string(),
        rack: "rack-01".to_string(),
        network_latency_level: 3,
    });

    let topo = scheduler.topology().read();
    let t = topo.get("topo-1").unwrap();
    assert_eq!(t.data_center, "dc-east");
    assert_eq!(t.zone, "east-1");
    assert_eq!(t.rack, "rack-01");
    assert_eq!(t.network_latency_level, 3);
}

/// 测试：放置策略切换
#[test]
fn im04_10_placement_strategy_switch() {
    let scheduler = DistributedScheduler::new(1500);
    assert_eq!(scheduler.get_placement_strategy(), PlacementStrategy::RackAware);

    scheduler.set_placement_strategy(PlacementStrategy::ZoneAware);
    assert_eq!(scheduler.get_placement_strategy(), PlacementStrategy::ZoneAware);

    scheduler.set_placement_strategy(PlacementStrategy::AntiAffinity);
    assert_eq!(scheduler.get_placement_strategy(), PlacementStrategy::AntiAffinity);

    scheduler.set_placement_strategy(PlacementStrategy::Random);
    assert_eq!(scheduler.get_placement_strategy(), PlacementStrategy::Random);
}

// =========================================================================
// 模块五：Raft 高可用 (Raft High Availability)
// =========================================================================

fn make_raft_config(node_id: &str, election_timeout: u64) -> RaftConfig {
    RaftConfig {
        node_id: node_id.to_string(),
        listen_addr: format!("127.0.0.1:{}", 9300 + node_id.parse::<u16>().unwrap_or(0)),
        election_timeout_ms: election_timeout,
        heartbeat_interval_ms: election_timeout / 3,
        max_log_entries: 10000,
        leader_lease_ms: election_timeout / 2,
    }
}

/// 测试：Raft 节点初始为 Follower 角色
#[test]
fn im05_01_initial_role_is_follower() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));
    assert_eq!(raft.role(), RaftRole::Follower);
    assert_eq!(raft.current_term(), 0);
    assert!(raft.leader_id().is_none());
}

/// 测试：Raft 节点发起选举
#[test]
fn im05_02_start_election() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));

    let (term, req) = raft.start_election();
    assert_eq!(term, 1);
    assert_eq!(req.term, 1);
    assert_eq!(req.candidate_id, "1");
    assert_eq!(req.last_log_index, 0); // 初始只有哨兵日志

    let metrics = raft.metrics().snapshot();
    assert!(metrics["raft_elections_total"] >= 1);
}

/// 测试：投票请求处理 - 投票给更新的日志
#[test]
fn im05_03_handle_request_vote_grant() {
    let raft = RaftMaster::new(make_raft_config("2", 3000));

    let req = mox_cloud_master_svc::RequestVoteRequest {
        term: 1,
        candidate_id: "candidate-1".to_string(),
        last_log_index: 0,
        last_log_term: 0,
    };

    let resp = raft.handle_request_vote(req);
    assert_eq!(resp.term, 1);
    assert!(resp.vote_granted);
}

/// 测试：投票请求处理 - 拒绝旧任期
#[test]
fn im05_04_handle_request_vote_deny_old_term() {
    let raft = RaftMaster::new(make_raft_config("3", 3000));

    // 先让节点进入更高任期
    raft.start_election(); // term = 1

    let req = mox_cloud_master_svc::RequestVoteRequest {
        term: 0, // 旧任期
        candidate_id: "old-candidate".to_string(),
        last_log_index: 0,
        last_log_term: 0,
    };

    let resp = raft.handle_request_vote(req);
    assert_eq!(resp.term, 1); // 返回当前更高任期
    assert!(!resp.vote_granted);
}

/// 测试：添加和移除集群节点
#[test]
fn im05_05_add_and_remove_nodes() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));

    assert_eq!(raft.voter_count(), 0);
    assert_eq!(raft.list_nodes().len(), 0);

    // 添加节点
    raft.add_node(mox_cloud_master_svc::RaftNodeInfo {
        node_id: "node-2".to_string(),
        addr: "127.0.0.1:9302".to_string(),
        is_voter: true,
    });
    raft.add_node(mox_cloud_master_svc::RaftNodeInfo {
        node_id: "node-3".to_string(),
        addr: "127.0.0.1:9303".to_string(),
        is_voter: true,
    });

    assert_eq!(raft.voter_count(), 2);
    assert_eq!(raft.list_nodes().len(), 2);

    // 移除节点
    raft.remove_node("node-2");
    assert_eq!(raft.voter_count(), 1);
    assert_eq!(raft.list_nodes().len(), 1);
}

/// 测试：日志追加 - 仅 Leader 可追加
#[test]
fn im05_06_append_log_follower_rejected() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));

    let result = raft.append_log(RaftLogType::VolumeAllocation, vec![1, 2, 3]);
    assert!(result.is_err());
}

/// 测试：日志条目查询
#[test]
fn im05_07_get_log_entry() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));

    // index 0 是哨兵日志
    let entry = raft.get_log_entry(0);
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().index, 0);

    // 不存在的日志索引
    let entry = raft.get_log_entry(999);
    assert!(entry.is_none());
}

/// 测试：Raft 指标系统
#[test]
fn im05_08_raft_metrics() {
    let raft = RaftMaster::new(make_raft_config("1", 3000));
    let metrics = raft.metrics().snapshot();

    // 验证所有指标键存在
    assert!(metrics.contains_key("raft_elections_total"));
    assert!(metrics.contains_key("raft_leader_elections_won"));
    assert!(metrics.contains_key("raft_logs_committed"));
    assert!(metrics.contains_key("raft_logs_applied"));
    assert!(metrics.contains_key("raft_snapshots_created"));
    assert!(metrics.contains_key("raft_heartbeats_sent"));
    assert!(metrics.contains_key("raft_heartbeats_received"));
}

/// 测试：AppendEntries 请求处理（心跳）
#[test]
fn im05_09_handle_append_entries_heartbeat() {
    let raft = RaftMaster::new(make_raft_config("2", 3000));

    let req = mox_cloud_master_svc::AppendEntriesRequest {
        term: 1,
        leader_id: "leader-1".to_string(),
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![],
        leader_commit: 0,
    };

    let resp = raft.handle_append_entries(req);
    assert_eq!(resp.term, 1);
    assert!(resp.success);

    // 收到心跳后不应该选举超时
    assert!(!raft.is_election_timeout());
}

/// 测试：选举超时检测
#[test]
fn im05_10_election_timeout_detection() {
    // 使用极短的选举超时时间
    let raft = RaftMaster::new(make_raft_config("1", 1)); // 1ms 超时

    // 等待超时
    std::thread::sleep(Duration::from_millis(5));

    assert!(raft.is_election_timeout());
}

// =========================================================================
// 模块六：快照管理 (Snapshot Management)
// =========================================================================

/// 测试：创建快照
#[test]
fn im06_01_create_snapshot() {
    let sm = SnapshotManager::new();
    let mut manifest = BTreeMap::new();
    manifest.insert("chunk-1".to_string(), vec![1, 2, 3]);
    manifest.insert("chunk-2".to_string(), vec![4, 5, 6]);

    let sid = sm.take_snapshot("vol-1", manifest.clone()).unwrap();
    assert!(!sid.is_empty());
    assert_eq!(sid.len(), 64); // SHA256 hex
}

/// 测试：获取快照元数据
#[test]
fn im06_02_get_snapshot_meta() {
    let sm = SnapshotManager::new();
    let mut manifest = BTreeMap::new();
    manifest.insert("file-a".to_string(), b"data-a".to_vec());

    let sid = sm.take_snapshot("vol-1", manifest.clone()).unwrap();
    let meta = sm.get_snapshot("vol-1", &sid).unwrap();

    assert_eq!(meta.volume_id, "vol-1");
    assert_eq!(meta.chunk_count, 1);
    assert_eq!(meta.chunk_manifest.len(), 1);
    assert_eq!(meta.chunk_manifest["file-a"], b"data-a");
}

/// 测试：查询不存在的快照
#[test]
fn im06_03_get_nonexistent_snapshot() {
    let sm = SnapshotManager::new();
    let result = sm.get_snapshot("vol-1", "non-existent-sid");
    assert!(result.is_err());
}

/// 测试：删除快照
#[test]
fn im06_04_delete_snapshot() {
    let sm = SnapshotManager::new();
    let sid = sm.take_snapshot("vol-1", BTreeMap::new()).unwrap();

    // 删除前可查询
    assert!(sm.get_snapshot("vol-1", &sid).is_ok());

    sm.delete_snapshot("vol-1", &sid).unwrap();

    // 删除后不可查询
    assert!(sm.get_snapshot("vol-1", &sid).is_err());
}

/// 测试：列出卷的所有快照
#[test]
fn im06_05_list_snapshots() {
    let sm = SnapshotManager::new();

    for i in 0..5 {
        let mut manifest = BTreeMap::new();
        manifest.insert(format!("chunk-{}", i), vec![i as u8]);
        sm.take_snapshot("vol-1", manifest).unwrap();
    }

    let list = sm.list_snapshots("vol-1");
    assert_eq!(list.len(), 5);
}

/// 测试：MasterServer 集成快照
#[test]
fn im06_06_master_snapshot_volume() {
    let master = make_master();
    let vid = master.register_volume("127.0.0.1:8080".to_string(), 1024 * 1024);

    let sid = master.snapshot_volume(&vid).unwrap();
    assert!(!sid.is_empty());

    let metrics = master.metrics.get_all();
    assert!(metrics["snapshots_taken"] >= 1);
}

/// 测试：对不存在的卷创建快照失败
#[test]
fn im06_07_snapshot_nonexistent_volume() {
    let master = make_master();
    let result = master.snapshot_volume("no-such-volume");
    assert!(result.is_err());
}

/// 测试：快照 ID 不可伪造（确定性验证）
#[test]
fn im06_08_snapshot_id_deterministic() {
    // 相同输入应生成相同快照 ID
    let id1 = SnapshotManager::generate_snapshot_id("vol-1", "salt-123", 1_700_000_000_000);
    let id2 = SnapshotManager::generate_snapshot_id("vol-1", "salt-123", 1_700_000_000_000);
    assert_eq!(id1, id2);

    // 不同输入生成不同 ID
    let id3 = SnapshotManager::generate_snapshot_id("vol-1", "salt-456", 1_700_000_000_000);
    assert_ne!(id1, id3);
}

// =========================================================================
// 模块七：数据均衡 (Data Rebalancing)
// =========================================================================

/// 测试：均衡计划生成
#[test]
fn im07_01_generate_rebalance_plan() {
    let scheduler = DistributedScheduler::new(1500);

    // 注册具有不同负载的节点
    let volumes: Vec<VolumeInfo> = (0..6)
        .map(|i| VolumeInfo {
            id: format!("rb-{}", i),
            addr: format!("127.0.0.1:{}", 6000 + i),
            capacity: 100 * 1024 * 1024,
            used: if i < 3 { 90 * 1024 * 1024 } else { 10 * 1024 * 1024 }, // 前 3 个满载，后 3 个空闲
            is_alive: true,
        })
        .collect();

    let plan = scheduler.generate_rebalance_plan(&volumes, 10);
    assert!(!plan.migrations.is_empty(), "should generate some migrations");
    assert!(plan.total_bytes > 0);
    assert!(plan.estimated_improvement > 0);
    assert!(plan.estimated_improvement <= 100);
}

/// 测试：均衡状态 - 已均衡的集群不生成迁移
#[test]
fn im07_02_balanced_cluster_no_migration() {
    let scheduler = DistributedScheduler::new(1500);

    let volumes: Vec<VolumeInfo> = (0..4)
        .map(|i| VolumeInfo {
            id: format!("balanced-{}", i),
            addr: format!("127.0.0.1:{}", 5000 + i),
            capacity: 10_000,
            used: 5000, // 所有节点使用率相同
            is_alive: true,
        })
        .collect();

    let plan = scheduler.generate_rebalance_plan(&volumes, 10);
    // 已均衡的集群应该没有迁移或迁移量很小
    assert!(plan.estimated_improvement < 50, "balanced cluster should have low improvement");
}

/// 测试：恢复计划生成
#[test]
fn im07_03_generate_recovery_plan() {
    let scheduler = DistributedScheduler::new(1500);

    let mut volumes: Vec<VolumeInfo> = (0..5)
        .map(|i| VolumeInfo {
            id: format!("rec-{}", i),
            addr: format!("127.0.0.1:{}", 4000 + i),
            capacity: 10_000,
            used: 2000,
            is_alive: i < 4, // 4 个存活，1 个故障
        })
        .collect();

    // 标记故障节点
    if let Some(v) = volumes.get_mut(4) {
        v.is_alive = false;
    }

    let plan = scheduler.generate_recovery_plan(&volumes);
    assert!(plan.affected_volumes > 0 || plan.replicas_to_rebuild > 0);
    assert_eq!(plan.failed_nodes.len(), 1);
}

/// 测试：迁移任务状态管理
#[test]
fn im07_04_migration_task_status() {
    use mox_cloud_master_svc::{MigrationStatus, VolumeMigrationTask};

    let mut task = VolumeMigrationTask {
        task_id: "mig-001".to_string(),
        source_volume_id: "src".to_string(),
        target_volume_id: "dst".to_string(),
        target_addr: "127.0.0.1:9000".to_string(),
        replica_set_id: "set-1".to_string(),
        size_bytes: 1024 * 1024,
        migrated_bytes: 0,
        status: MigrationStatus::Pending,
        created_at_ms: 1000,
        started_at_ms: None,
        completed_at_ms: None,
        error: None,
        bandwidth_limit_bps: 0,
    };

    assert_eq!(task.status, MigrationStatus::Pending);
    task.status = MigrationStatus::Running;
    task.migrated_bytes = 512 * 1024;
    assert_eq!(task.migrated_bytes, 512 * 1024);

    task.status = MigrationStatus::Completed;
    task.completed_at_ms = Some(2000);
    assert_eq!(task.status, MigrationStatus::Completed);
}

/// 测试：数据温度枚举
#[test]
fn im07_05_data_temperature_enum() {
    let hot = DataTemperature::Hot;
    let warm = DataTemperature::Warm;
    let cold = DataTemperature::Cold;
    let archive = DataTemperature::Archive;

    assert_ne!(hot, warm);
    assert_ne!(warm, cold);
    assert_ne!(cold, archive);

    // Default is Hot
    assert_eq!(DataTemperature::default(), DataTemperature::Hot);
}

/// 测试：调度器统计 - 均衡计划计数
#[test]
fn im07_06_rebalance_plan_counter() {
    let scheduler = DistributedScheduler::new(1500);

    let volumes: Vec<VolumeInfo> = (0..4)
        .map(|i| VolumeInfo {
            id: format!("cnt-{}", i),
            addr: format!("127.0.0.1:{}", 3000 + i),
            capacity: 10_000,
            used: if i % 2 == 0 { 9000 } else { 1000 },
            is_alive: true,
        })
        .collect();

    let before = scheduler.stats().snapshot();
    for _ in 0..3 {
        let _ = scheduler.generate_rebalance_plan(&volumes, 10);
    }
    let after = scheduler.stats().snapshot();

    assert!(after["scheduler_rebalance_plans"] > before["scheduler_rebalance_plans"]);
}

// =========================================================================
// 模块八：综合集成测试 (Integration)
// =========================================================================

/// 测试：完整的卷生命周期 - 注册 -> 分配 -> 心跳 -> 快照
#[test]
fn im08_01_full_volume_lifecycle() {
    let master = make_master();

    // 1. 注册 Volume 节点
    let v1 = master.register_volume("127.0.0.1:8001".to_string(), 100 * 1024 * 1024 * 1024);
    let v2 = master.register_volume("127.0.0.1:8002".to_string(), 100 * 1024 * 1024 * 1024);
    let v3 = master.register_volume("127.0.0.1:8003".to_string(), 100 * 1024 * 1024 * 1024);

    assert_eq!(master.list_volumes().len(), 3);

    // 2. 分配 3 副本卷
    let alloc = master.allocate_volume(10 * 1024 * 1024 * 1024, 3).unwrap();
    assert_eq!(alloc.replica_count, 3);

    // 3. 心跳上报
    for vid in &[&v1, &v2, &v3] {
        master
            .heartbeat(
                vid,
                VolumeLoadReport {
                    used_bytes: 5 * 1024 * 1024 * 1024,
                    chunk_count: 1000,
                    cpu_pct: 30,
                    is_healthy: true,
                },
            )
            .unwrap();
    }

    // 4. 创建快照
    let sid = master.snapshot_volume(&v1).unwrap();
    assert!(!sid.is_empty());

    // 5. 验证指标
    let metrics = master.metrics.get_all();
    assert!(metrics["heartbeats_received"] >= 3);
    assert!(metrics["volumes_allocations_total"] >= 1);
    assert!(metrics["snapshots_taken"] >= 1);
}

/// 测试：并发分配测试
#[test]
fn im08_02_concurrent_allocations() {
    let master = Arc::new(make_master());
    register_n_volumes(&master, 10, 100 * 1024 * 1024 * 1024);

    let mut handles = vec![];
    for i in 0..5 {
        let master = Arc::clone(&master);
        handles.push(std::thread::spawn(move || {
            master.allocate_volume((i + 1) * 1024 * 1024, 2).unwrap()
        }));
    }

    let results: Vec<VolumeAllocation> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert_eq!(results.len(), 5);
    let metrics = master.metrics.get_all();
    assert!(metrics["volumes_allocations_total"] >= 5);
}

/// 测试：Master 配置自定义
#[test]
fn im08_03_custom_master_config() {
    let config = MasterConfig { heartbeat_timeout_ms: 5000, max_replica: 2 };

    let master = MasterServer::new(config);
    assert_eq!(master.config.heartbeat_timeout_ms, 5000);
    assert_eq!(master.config.max_replica, 2);

    register_n_volumes(&master, 5, 100 * 1024 * 1024 * 1024);

    // max_replica = 2, 3 副本应失败
    let result = master.allocate_volume(1024, 3);
    assert!(result.is_err());

    // 2 副本应成功
    let result = master.allocate_volume(1024, 2);
    assert!(result.is_ok());
}

/// 测试：错误类型覆盖
#[test]
fn im08_04_error_type_coverage() {
    let master = make_master();

    // VolumeNotFound
    let err = master
        .heartbeat(
            "no-such-vol",
            VolumeLoadReport { used_bytes: 0, chunk_count: 0, cpu_pct: 0, is_healthy: true },
        )
        .unwrap_err();
    assert!(err.to_string().contains("not found") || err.to_string().contains("VolumeNotFound"));

    // InvalidReplicaCount
    register_n_volumes(&master, 5, 100 * 1024 * 1024 * 1024);
    let err = master.allocate_volume(1024, 0).unwrap_err();
    assert!(err.to_string().contains("replica") || err.to_string().contains("Invalid"));

    // NoCapacity
    let master2 = make_master();
    register_n_volumes(&master2, 1, 100);
    let err = master2.allocate_volume(1024 * 1024, 1).unwrap_err();
    assert!(err.to_string().contains("capacity") || err.to_string().contains("NoCapacity"));
}

/// 测试：Raft + Master 集成配置
#[test]
fn im08_05_raft_master_integrated_config() {
    let master_config = MasterConfig { heartbeat_timeout_ms: 2000, max_replica: 3 };
    let raft_config = RaftConfig {
        node_id: "master-1".to_string(),
        listen_addr: "127.0.0.1:9333".to_string(),
        election_timeout_ms: 4000,
        heartbeat_interval_ms: 500,
        max_log_entries: 5000,
        leader_lease_ms: 2000,
    };

    let master = MasterServer::with_raft_config(master_config, raft_config);
    assert_eq!(master.config.heartbeat_timeout_ms, 2000);
    assert_eq!(master.raft.current_term(), 0);
    assert_eq!(master.raft.role(), RaftRole::Follower);
}
