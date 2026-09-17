// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! C2: mox-cloud-rebalance-svc 公共 API 黑盒集成测试 —— 迁移任务生命周期。

use mox_cloud_rebalance_svc::{
    MigrationCheckpoint, MigrationPhase, MigrationStatus, MigrationTask, MigrationTaskManager,
    MigrationType, VerificationMethod,
};

fn task(id: &str, mtype: MigrationType, total: u64) -> MigrationTask {
    MigrationTask {
        task_id: id.into(),
        migration_type: mtype,
        source_node_id: "src-1".into(),
        source_addr: "10.0.0.1:9000".into(),
        target_node_id: "dst-1".into(),
        target_addr: "10.0.0.2:9000".into(),
        object_id: format!("obj-{id}"),
        total_bytes: total,
        migrated_bytes: 0,
        verified_bytes: 0,
        priority: 0, // 触发默认优先级
        status: MigrationStatus::Pending,
        phase: MigrationPhase::Init,
        created_at_ms: 1_000_000,
        started_at_ms: None,
        completed_at_ms: None,
        bandwidth_limit_bps: 0,
        retry_count: 0,
        max_retries: 3,
        last_error: None,
        checkpoint: None,
        delete_source_after: true,
        verify_after_migration: true,
        verification_method: VerificationMethod::Crc32c,
    }
}

#[test]
fn submit_and_execute_full_lifecycle() {
    let mgr = MigrationTaskManager::new(4);
    let id = mgr.submit_task(task("t1", MigrationType::CapacityRebalance, 1000));
    assert_eq!(id, "t1");
    assert_eq!(mgr.get_max_concurrent(), 4);

    // 取任务 → Running + Init 阶段 + 开始时间。
    let t = mgr.get_next_task().expect("queued task must be returned");
    assert_eq!(t.status, MigrationStatus::Running);
    assert_eq!(t.phase, MigrationPhase::Init);
    assert!(t.started_at_ms.is_some());

    // 进度更新。
    mgr.update_progress("t1", 400);
    mgr.report_progress("t1", 700, MigrationPhase::IncrementalSync);
    let running = mgr.get_task("t1").unwrap();
    assert_eq!(running.migrated_bytes, 700);
    assert_eq!(running.phase, MigrationPhase::IncrementalSync);

    // 完成 → 终态 + 统计。
    mgr.complete_task("t1", 1000);
    let done = mgr.get_task("t1").expect("completed task remains queryable");
    assert_eq!(done.status, MigrationStatus::Completed);
    let stats = mgr.stats().snapshot();
    assert_eq!(stats["migration_tasks_completed"], 1);
    assert_eq!(stats["migration_bytes_total"], 1000);
    assert_eq!(stats["migration_bytes_verified"], 1000);
    let completed = mgr.list_completed(10);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].status, MigrationStatus::Completed);
}

#[test]
fn priority_ordering_pulls_high_priority_first() {
    let mgr = MigrationTaskManager::new(4);
    mgr.submit_task(task("low", MigrationType::TierDown, 100)); // 默认优先级 2
    mgr.submit_task(task("high", MigrationType::FailureRecovery, 100)); // 默认优先级 10
    mgr.submit_task(task("mid", MigrationType::CapacityRebalance, 100)); // 默认 4

    let first = mgr.get_next_task().unwrap();
    assert_eq!(first.task_id, "high", "failure recovery must run first");
    let second = mgr.get_next_task().unwrap();
    assert_eq!(second.task_id, "mid");
    let third = mgr.get_next_task().unwrap();
    assert_eq!(third.task_id, "low");
}

#[test]
fn max_concurrent_caps_running_tasks() {
    let mgr = MigrationTaskManager::new(1);
    mgr.submit_task(task("a", MigrationType::Manual, 100));
    mgr.submit_task(task("b", MigrationType::Manual, 100));

    let _a = mgr.get_next_task().unwrap();
    assert!(mgr.get_next_task().is_none(), "second task must wait for slot");
    assert_eq!(mgr.list_pending().len(), 1);

    // 完成后释放槽位。
    mgr.complete_task("a", 100);
    let b = mgr.get_next_task().unwrap();
    assert_eq!(b.task_id, "b");
}

#[test]
fn fail_and_cancel_paths() {
    let mgr = MigrationTaskManager::new(4);
    let mut fail_task = task("f", MigrationType::Manual, 500);
    fail_task.max_retries = 0; // 首次失败即终态 Failed
    let id = mgr.submit_task(fail_task);
    let _ = mgr.get_next_task().unwrap();
    mgr.fail_task(&id, "backend error".into());
    let stats = mgr.stats().snapshot();
    assert_eq!(stats["migration_tasks_failed"], 1);
    let failed = mgr.list_completed(10);
    assert!(failed.iter().any(|t| t.status == MigrationStatus::Failed));

    // 取消一个 pending 任务。
    let cid = mgr.submit_task(task("c", MigrationType::Manual, 500));
    assert!(mgr.cancel_task(&cid), "pending task must be cancellable");
    let cancelled = mgr.list_completed(10);
    assert!(cancelled.iter().any(|t| t.task_id == cid && t.status == MigrationStatus::Cancelled));
    // 已取消任务不可再次取消。
    assert!(!mgr.cancel_task(&cid));
}

#[test]
fn pause_resume_flow() {
    let mgr = MigrationTaskManager::new(4);
    let id = mgr.submit_task(task("p", MigrationType::Manual, 500));
    let _ = mgr.get_next_task().unwrap();

    assert!(mgr.pause_task(&id), "running task must be pausable");
    let paused = mgr.get_task(&id).unwrap();
    assert_eq!(paused.status, MigrationStatus::Paused);

    assert!(mgr.resume_task(&id));
    let resumed = mgr.get_task(&id).unwrap();
    assert_eq!(resumed.status, MigrationStatus::Running);

    // 再次暂停仍成功（实现允许幂等置 Paused），且状态确实切回 Paused。
    assert!(mgr.pause_task(&id));
    assert_eq!(mgr.get_task(&id).unwrap().status, MigrationStatus::Paused);
}

#[test]
fn checkpoint_roundtrip_enables_resume() {
    let mgr = MigrationTaskManager::new(4);
    let id = mgr.submit_task(task("ck", MigrationType::TierUp, 10_000));
    let _ = mgr.get_next_task().unwrap();

    let ck = MigrationCheckpoint {
        completed_offset: 6_000,
        last_block_index: 12,
        verified_offset: 6_000,
        current_phase: MigrationPhase::FullSync,
        saved_at_ms: 1_234_567,
    };
    mgr.save_checkpoint(&id, ck.clone());
    let got = mgr.get_checkpoint(&id).expect("checkpoint must persist");
    assert_eq!(got.completed_offset, 6_000);
    assert_eq!(got.last_block_index, 12);
    assert_eq!(got.current_phase, MigrationPhase::FullSync);

    // 未知任务无断点。
    assert!(mgr.get_checkpoint("nope").is_none());
}

#[test]
fn bandwidth_limits_reflected_in_throughput() {
    let mgr = MigrationTaskManager::new(4);
    mgr.set_global_bandwidth_limit(100 * 1024 * 1024);
    assert_eq!(mgr.get_global_bandwidth_limit(), 100 * 1024 * 1024);
    // 无运行中任务时按 1 个任务均分。
    assert_eq!(mgr.per_task_bandwidth_bps(), 100 * 1024 * 1024);
    assert_eq!(mgr.current_throughput_bps(), 0, "no traffic yet");

    // 4 个并发任务在跑 → 均分为 100MB/4。
    for i in 0..4 {
        mgr.submit_task(task(&format!("bw{i}"), MigrationType::Manual, 100));
        let _ = mgr.get_next_task().unwrap();
    }
    assert_eq!(mgr.per_task_bandwidth_bps(), 100 * 1024 * 1024 / 4);
    assert_eq!(mgr.current_throughput_bps(), 4 * 10 * 1024 * 1024);
}

#[test]
fn migration_type_metadata() {
    assert_eq!(MigrationType::FailureRecovery.default_priority(), 10);
    assert_eq!(MigrationType::TierDown.default_priority(), 2);
    assert_eq!(MigrationType::NodeDecommission.name(), "node_decommission");
    assert!(MigrationStatus::Completed.is_terminal());
    assert!(MigrationStatus::Failed.is_terminal());
    assert!(!MigrationStatus::Pending.is_terminal());
    assert!(MigrationStatus::Running.is_cancellable());
    assert!(!MigrationStatus::Completed.is_cancellable());
}
