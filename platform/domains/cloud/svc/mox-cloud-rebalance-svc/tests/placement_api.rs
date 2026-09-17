// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! C2: mox-cloud-rebalance-svc 公共 API 黑盒集成测试 —— 放置策略。
//!
//! 仅通过 crate 对外公共 API（`mox_cloud_rebalance_svc::*`）驱动，
//! 覆盖 PlacementEngine 的目标选择 / 排名 / 约束过滤 / 副本分散 / 均衡度。

use std::collections::HashSet;

use mox_cloud_rebalance_svc::{
    DataTemperature, PlacementConstraints, PlacementEngine, PlacementNode,
    PlacementStrategyType, PlacementWeights,
};

fn node(id: &str, capacity: u64, used: u64, cpu: u8) -> PlacementNode {
    PlacementNode {
        node_id: id.into(),
        addr: format!("10.0.0.{id}"),
        capacity,
        used,
        is_healthy: true,
        data_center: "dc1".into(),
        zone: "zone-a".into(),
        rack: "rack-1".into(),
        cpu_pct: cpu,
        network_latency: 3,
        active_migrations: 0,
        preferred_temperature: DataTemperature::Hot,
    }
}

fn zone(node: &mut PlacementNode, z: &str, rack: &str) {
    node.zone = z.into();
    node.rack = rack.into();
}

#[test]
fn select_target_picks_least_loaded_capacity_first() {
    let engine = PlacementEngine::with_strategy(PlacementStrategyType::CapacityFirst);
    let nodes = vec![
        node("n1", 1000, 900, 90),
        node("n2", 1000, 400, 40),
        node("n3", 1000, 100, 20),
    ];
    let best = engine.select_target(&nodes, &PlacementConstraints::default()).unwrap();
    assert_eq!(best.node.node_id, "n3", "capacity-first must pick least used node");
    assert!(best.score > 0.0);
}

#[test]
fn select_target_load_first_prefers_low_cpu() {
    let engine = PlacementEngine::with_strategy(PlacementStrategyType::LoadFirst);
    let nodes = vec![
        node("n1", 1000, 900, 90),
        node("n2", 1000, 400, 10),
        node("n3", 1000, 100, 80),
    ];
    let best = engine.select_target(&nodes, &PlacementConstraints::default()).unwrap();
    assert_eq!(best.node.node_id, "n2", "load-first must pick lowest CPU node");
}

#[test]
fn rank_candidates_sorted_desc_and_breaks_down() {
    let engine = PlacementEngine::new();
    let nodes = vec![node("n1", 1000, 500, 50), node("n2", 1000, 500, 50)];
    let ranked = engine.rank_candidates(&nodes, &PlacementConstraints::default());
    assert_eq!(ranked.len(), 2);
    assert!(ranked[0].score >= ranked[1].score, "ranking must be descending");
    // ScoreBreakdown 可读（结构公开）。
    let _ = ranked[0].score_breakdown.capacity_score;
    let _ = ranked[0].score_breakdown.load_score;
    let _ = ranked[0].score_breakdown.topology_score;
}

#[test]
fn constraints_excluded_nodes_and_min_free_respected() {
    let engine = PlacementEngine::new();
    let nodes = vec![
        node("n1", 1000, 950, 10),
        node("n2", 1000, 400, 20),
        node("n3", 1000, 200, 30),
    ];
    let constraints = PlacementConstraints {
        excluded_nodes: HashSet::from(["n3".into()]),
        min_free_bytes: 500,
        ..PlacementConstraints::default()
    };
    // n3 被排除；n1 free=50 < 500 不满足；只剩 n2。
    let best = engine.select_target(&nodes, &constraints).unwrap();
    assert_eq!(best.node.node_id, "n2");

    // 全部不满足 → None。
    let strict = PlacementConstraints {
        excluded_nodes: HashSet::from(["n2".into(), "n3".into()]),
        min_free_bytes: 600,
        ..PlacementConstraints::default()
    };
    assert!(engine.select_target(&nodes, &strict).is_none());
}

#[test]
fn constraints_temperature_and_dc_respected() {
    let engine = PlacementEngine::new();
    let mut n_cold = node("cold", 1000, 100, 10);
    n_cold.preferred_temperature = DataTemperature::Cold;
    let mut n_hot = node("hot", 1000, 900, 90);
    n_hot.preferred_temperature = DataTemperature::Hot;

    let nodes = vec![n_cold, n_hot];
    let constraints = PlacementConstraints {
        preferred_temperature: Some(DataTemperature::Cold),
        same_data_center: Some("dc1".into()),
        ..PlacementConstraints::default()
    };
    let best = engine.select_target(&nodes, &constraints).unwrap();
    assert_eq!(best.node.node_id, "cold", "must respect temperature preference");
}

#[test]
fn select_replica_nodes_spreads_across_zones() {
    let engine = PlacementEngine::new();
    let mut nodes = vec![
        node("n1", 1000, 300, 30),
        node("n2", 1000, 300, 30),
        node("n3", 1000, 300, 30),
        node("n4", 1000, 300, 30),
        node("n5", 1000, 300, 30),
    ];
    // 三个不同可用区/机架。
    zone(&mut nodes[1], "zone-b", "rack-2");
    zone(&mut nodes[2], "zone-c", "rack-3");
    zone(&mut nodes[3], "zone-d", "rack-4");
    zone(&mut nodes[4], "zone-e", "rack-5");

    let selected = engine.select_replica_nodes(&nodes, 3, &[]);
    assert_eq!(selected.len(), 3, "must select exactly count nodes");

    // 已存在节点不能被再次选中。
    let selected2 = engine.select_replica_nodes(&nodes, 2, &["n1".into(), "n2".into()]);
    let ids: HashSet<&str> = selected2.iter().map(|c| c.node.node_id.as_str()).collect();
    assert_eq!(ids.len(), 2);
    assert!(!ids.contains("n1") && !ids.contains("n2"), "existing nodes excluded");
}

#[test]
fn compute_cluster_balance_scores() {
    let engine = PlacementEngine::new();
    // 均匀分布 → 均衡度接近 100。
    let even = vec![node("n1", 1000, 500, 50), node("n2", 1000, 500, 50)];
    let score = engine.compute_cluster_balance(&even);
    assert!((score - 100.0).abs() < 1e-6, "even usage should score ~100, got {score}");

    // 极端不均衡 → 均衡度显著低于 100。
    let skewed = vec![node("n1", 1000, 100, 10), node("n2", 1000, 900, 90)];
    let skewed_score = engine.compute_cluster_balance(&skewed);
    assert!(skewed_score < score, "skewed usage must score lower");

    // 空/单节点边界。
    assert_eq!(engine.compute_cluster_balance(&[]), 0.0);
    assert_eq!(engine.compute_cluster_balance(&[node("n1", 1000, 500, 50)]), 100.0);
}

#[test]
fn weights_strategy_roundtrip() {
    let engine = PlacementEngine::new();
    assert_eq!(engine.get_strategy(), PlacementStrategyType::Balanced);

    engine.set_strategy(PlacementStrategyType::TopologyAware);
    assert_eq!(engine.get_strategy(), PlacementStrategyType::TopologyAware);
    let weights: PlacementWeights = engine.get_weights();
    assert_eq!(weights.topology_weight, 45, "topology-aware must boost topology weight");

    let custom = PlacementWeights {
        capacity_weight: 10,
        load_weight: 20,
        topology_weight: 30,
        network_weight: 40,
        migration_weight: 50,
    };
    engine.set_weights(custom);
    assert_eq!(engine.get_weights().migration_weight, 50);
}
