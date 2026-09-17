// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! C2: mox-cloud-rebalance-svc 公共 API 黑盒集成测试 —— 均衡控制器端到端。

use std::collections::HashMap;

use mox_cloud_rebalance_svc::{
    DataTemperature, PlacementNode, RebalanceConfig, RebalanceController, RebalanceState,
    RebalanceStatusSummary,
};

fn node(id: &str, capacity: u64, used: u64) -> PlacementNode {
    PlacementNode {
        node_id: id.into(),
        addr: format!("10.0.0.{id}"),
        capacity,
        used,
        is_healthy: true,
        data_center: "dc1".into(),
        zone: format!("zone-{id}"),
        rack: format!("rack-{id}"),
        cpu_pct: 30,
        network_latency: 2,
        active_migrations: 0,
        preferred_temperature: DataTemperature::Hot,
    }
}

fn controller_with_unbalanced_nodes() -> RebalanceController {
    let config = RebalanceConfig {
        balance_threshold_pct: 10.0,
        max_concurrent_migrations: 4,
        ..RebalanceConfig::default()
    };
    let ctl = RebalanceController::new(config);
    // 强不均衡：n1 使用率 90%，n2 使用率 10%。
    ctl.update_nodes(vec![node("n1", 1000, 900), node("n2", 1000, 100)]);
    ctl
}

#[test]
fn controller_state_and_nodes() {
    let ctl = RebalanceController::new(RebalanceConfig::default());
    assert_eq!(ctl.get_state(), RebalanceState::Idle);
    assert!(ctl.get_nodes().is_empty());

    ctl.update_nodes(vec![node("n1", 1000, 500), node("n2", 1000, 500)]);
    assert_eq!(ctl.get_nodes().len(), 2);
}

#[test]
fn needs_rebalance_detects_imbalance() {
    let ctl = controller_with_unbalanced_nodes();
    assert!(ctl.needs_rebalance(), "90%/10% split must trigger rebalance");

    let balanced = RebalanceController::new(RebalanceConfig::default());
    balanced.update_nodes(vec![node("n1", 1000, 500), node("n2", 1000, 500)]);
    assert!(!balanced.needs_rebalance(), "even split must not trigger");

    // 关闭自动均衡 → 不触发。
    let disabled = RebalanceController::new(RebalanceConfig {
        auto_rebalance_enabled: false,
        ..RebalanceConfig::default()
    });
    disabled.update_nodes(vec![node("n1", 1000, 900), node("n2", 1000, 100)]);
    assert!(!disabled.needs_rebalance());
}

#[test]
fn generate_plan_builds_migrations() {
    let ctl = controller_with_unbalanced_nodes();
    let plan = ctl.generate_plan().expect("imbalanced cluster must yield a plan");
    assert!(!plan.plan_id.is_empty());
    assert!(!plan.migrations.is_empty(), "plan must schedule migrations");
    assert!(plan.total_bytes > 0);
    assert!(!plan.trigger_reason.is_empty());
}

#[test]
fn generate_plan_returns_none_when_balanced() {
    let ctl = RebalanceController::new(RebalanceConfig::default());
    ctl.update_nodes(vec![node("n1", 1000, 500), node("n2", 1000, 500)]);
    assert!(ctl.generate_plan().is_none(), "balanced cluster must not plan migrations");

    // 单节点无法均衡。
    let single = RebalanceController::new(RebalanceConfig::default());
    single.update_nodes(vec![node("n1", 1000, 900)]);
    assert!(single.generate_plan().is_none());
}

#[test]
fn run_rebalance_submits_migrations() {
    let ctl = controller_with_unbalanced_nodes();
    let result = ctl.run_rebalance();
    assert!(result.plan_id.is_some(), "plan must be scheduled");
    assert!(result.migrations_scheduled >= 1);
    assert!(result.total_bytes > 0);

    // 任务进入 manager 待执行队列。
    assert!(!ctl.task_manager().list_pending().is_empty());
    let stats = ctl.stats().snapshot();
    assert_eq!(stats["rebalance_rounds_total"], 1);
}

#[test]
fn tick_processes_pending_migrations() {
    let ctl = controller_with_unbalanced_nodes();
    let result = ctl.run_rebalance();
    let scheduled = result.migrations_scheduled as usize;
    assert!(scheduled >= 1);

    let processed = ctl.tick();
    assert_eq!(processed, scheduled, "tick must drain all scheduled migrations");
    assert_eq!(ctl.get_state(), RebalanceState::Idle, "after drain, controller returns Idle");
    assert!(ctl.task_manager().list_pending().is_empty());
    assert!(ctl.task_manager().list_running().is_empty());
}

#[test]
fn pause_resume_stop_state_machine() {
    let ctl = controller_with_unbalanced_nodes();
    let _ = ctl.run_rebalance();
    assert_eq!(ctl.get_state(), RebalanceState::Running);

    ctl.pause();
    assert_eq!(ctl.get_state(), RebalanceState::Paused);
    ctl.resume();
    assert_eq!(ctl.get_state(), RebalanceState::Running);

    ctl.stop();
    assert_eq!(ctl.get_state(), RebalanceState::Stopped);
    assert!(ctl.get_current_plan().is_none());
}

#[test]
fn status_summary_aggregates_state() {
    let ctl = controller_with_unbalanced_nodes();
    let _ = ctl.run_rebalance();

    let summary: RebalanceStatusSummary = ctl.get_status_summary();
    assert_eq!(summary.state, RebalanceState::Running);
    assert!(summary.balance_score >= 0.0 && summary.balance_score <= 100.0);
    assert!(summary.pending_migrations >= 1);
    assert_eq!(summary.running_migrations, 0);
    assert_eq!(summary.total_migrated_bytes, 0);
    assert!(summary.current_plan_id.is_some());
    assert!(summary.effective_bandwidth_bps > 0);
}

#[test]
fn recovery_plan_rebuilds_lost_replicas() {
    let ctl = controller_with_unbalanced_nodes();
    ctl.update_nodes(vec![
        node("n1", 1000, 500),
        node("n2", 1000, 500),
        node("n3", 1000, 500),
    ]);
    let replica_map = HashMap::from([("set-1".into(), vec!["n1".into(), "n2".into()])]);
    let tasks = ctl.generate_recovery_plan(&["n1".into()], &replica_map);
    assert_eq!(tasks.len(), 1, "one lost replica must yield one rebuild task");
    assert_ne!(tasks[0].target_node_id, "n1");
    assert!(tasks[0].target_node_id == "n2" || tasks[0].target_node_id == "n3");

    // 无故障 → 无任务。
    let noop = ctl.generate_recovery_plan(&[], &replica_map);
    assert!(noop.is_empty());
}
