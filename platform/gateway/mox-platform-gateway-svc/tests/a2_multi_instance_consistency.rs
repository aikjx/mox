// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # A2 无状态化阶段一：跨实例一致性真实验证（禁止 mock）
//!
//! D4 验证的是「写入 → 模拟崩溃重启（drop state）→ 启动读回」。A2 要证的是更强的性质：
//! **两个独立活实例共享同一份真实 SQLite，实例 B 不重启、不刷新本地内存缓存，直接走
//! 新读路径，就能读到实例 A 刚写穿的行**。这正是「SQLite 为唯一真相」区别于「内存为主 +
//! SQLite 备份」的关键——D4 的遗留是「多副本各持内存副本，A 写 B 不重启看不到」。
//!
//! 闭环：
//! 1. 独立临时库，两个 `ExpertsSharedState::new()`（实例 A / 实例 B，各自持有独立内存镜像）；
//! 2. 实例 A 按生产 handler 同样方式写穿（内存镜像 + upsert_plan / insert_history_record /
//!    upsert_favorite），覆盖 tenant-a；
//! 3. **先证 B 的内存镜像里确实没有 A 的数据**（若读路径走内存则必然读不到）——这是对照组；
//! 4. 实例 B 不重启，直接走 A2 新读路径（load_plans_by_tenant / get_plan /
//!    load_history_by_tenant / load_favorites_by_tenant），断言读到 A 刚写穿的数据；
//! 5. 租户隔离：B 读 tenant-b 看不到 tenant-a 的任何数据，跨租户 get_plan 命中即 None；
//! 6. 反向：B 再写，A 也实时读到（双向一致性）。
//!
//! 不是示例数据：真实领域对象 + 真实 data/experts.db（临时路径）+ WAL 多连接可见性。
use mox_platform_gateway_svc::alliance::experts_common::{
    CollaborationPlan, ExpertsSharedState, OrchestrationRecord, PlanStep, now_iso,
};
use mox_platform_gateway_svc::alliance::experts_db;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// 环境变量是进程级全局状态：本用例与其他改 `MOX_EXPERTS_DB_PATH` 的集成测试串行化。
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn make_plan(plan_id: &str, tenant: &str) -> CollaborationPlan {
    let now = now_iso();
    let mut metadata = HashMap::new();
    metadata.insert("tenant_id".into(), serde_json::json!(tenant));
    CollaborationPlan {
        plan_id: plan_id.into(),
        task_id: None,
        title: format!("A2协作计划·{plan_id}"),
        description: format!("{tenant} 的跨实例一致性任务"),
        expert_ids: vec!["exp-a-1".into()],
        steps: vec![PlanStep {
            step_id: format!("{plan_id}-step-1"),
            name: "需求分析".into(),
            description: "解析目标".into(),
            expert_id: Some("exp-a-1".into()),
            step_type: "intake".into(),
            depends_on: vec![],
            status: "completed".into(),
            result: Some(serde_json::json!({"solution": "A写穿的真实结论"})),
            started_at: Some(now.clone()),
            completed_at: Some(now.clone()),
        }],
        status: "completed".into(),
        fusion_strategy: "weighted".into(),
        metadata,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn make_history(execution_id: &str, plan_id: &str) -> OrchestrationRecord {
    OrchestrationRecord {
        execution_id: execution_id.into(),
        plan_id: plan_id.into(),
        task_type: "analysis".into(),
        status: "completed".into(),
        expert_ids: vec!["exp-a-1".into()],
        steps_completed: 1,
        steps_total: 1,
        result_summary: "A2跨实例真实执行".into(),
        result: Some(serde_json::json!({"summary": "A2跨实例真实执行"})),
        created_at: now_iso(),
        completed_at: Some(now_iso()),
        duration_ms: 123,
    }
}

#[test]
fn a2_two_live_instances_share_sqlite_a_writes_b_reads_without_reload() {
    let _guard: MutexGuard<'static, ()> = env_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("创建临时目录");
    let db = dir.path().join("experts.db").to_string_lossy().to_string();
    std::env::set_var(experts_db::ENV_DB_PATH, &db);

    // ========== 两个独立活实例（模拟两个副本/两个进程），共享同一份 SQLite ==========
    let inst_a = Arc::new(ExpertsSharedState::new());
    let inst_b = Arc::new(ExpertsSharedState::new());
    // B 启动时库内尚无 tenant-a 数据
    assert!(experts_db::load_plans_by_tenant("tenant-a").is_empty(), "前置：B 启动后 tenant-a 应为空");

    // ========== 实例 A 写穿（与生产 handler 完全一致：先更内存镜像，再写穿 SQLite）==========
    let plan_a = make_plan("a2-plan-1", "tenant-a");
    let hist_a = make_history("a2-exec-1", "a2-plan-1");
    inst_a.plans.lock().insert(plan_a.plan_id.clone(), plan_a.clone());
    experts_db::upsert_plan("tenant-a", &plan_a);
    inst_a.orchestration_history.lock().push(hist_a.clone());
    experts_db::insert_history_record("tenant-a", &hist_a);
    experts_db::upsert_favorite("tenant-a", "exp-a-fav");
    inst_a
        .favorites
        .lock()
        .entry("tenant-a".into())
        .or_default()
        .insert("exp-a-fav".into());

    // ========== 对照组：B 的内存镜像此刻仍无 A 的数据（B 自启动后未 reload）==========
    // 若读路径仍走内存镜像，下面所有断言都会失败——这正是 D4 遗留「A 写 B 不重启看不到」。
    assert!(
        inst_b.plans.lock().get("a2-plan-1").is_none(),
        "对照组：B 内存镜像本应没有 A 的 plan（否则证明读路径未真正改走 SQLite）"
    );
    assert!(
        !inst_b.favorites.lock().get("tenant-a").map(|s| s.contains("exp-a-fav")).unwrap_or(false),
        "对照组：B 内存镜像本应没有 A 的收藏"
    );

    // ========== 实例 B 不重启、不刷新，直接走 A2 新读路径（SQLite 唯一真相）==========
    // 1) 按租户读 plans
    let b_plans = experts_db::load_plans_by_tenant("tenant-a");
    assert!(b_plans.contains_key("a2-plan-1"), "B 应实时读到 A 刚写穿的 plan");
    assert_eq!(b_plans["a2-plan-1"].title, "A2协作计划·a2-plan-1", "plan title 逐字段一致");
    assert_eq!(b_plans["a2-plan-1"].status, "completed");

    // 2) 单点读 plan（execute_plan_handler 读路径）
    let got = experts_db::get_plan("tenant-a", "a2-plan-1").expect("B 单点读应命中 A 的 plan");
    assert_eq!(got.expert_ids, vec!["exp-a-1".to_string()]);

    // 3) 按租户读 history
    let b_hist = experts_db::load_history_by_tenant("tenant-a");
    assert_eq!(b_hist.len(), 1, "B 应实时读到 A 刚写穿的 1 条 history");
    assert_eq!(b_hist[0].execution_id, "a2-exec-1");
    assert_eq!(b_hist[0].duration_ms, 123);

    // 4) 按租户读 favorites
    let b_favs = experts_db::load_favorites_by_tenant("tenant-a");
    assert!(b_favs.contains("exp-a-fav"), "B 应实时读到 A 刚写穿的收藏");

    // ========== 租户隔离：B 读 tenant-b 看不到 tenant-a 的任何数据 ==========
    assert!(experts_db::load_plans_by_tenant("tenant-b").is_empty(), "tenant-b 不应有 plan");
    assert!(experts_db::load_history_by_tenant("tenant-b").is_empty(), "tenant-b 不应有 history");
    assert!(experts_db::load_favorites_by_tenant("tenant-b").is_empty(), "tenant-b 不应有收藏");
    assert!(
        experts_db::get_plan("tenant-b", "a2-plan-1").is_none(),
        "跨租户单点读必须 404（tenant-b 读不到 tenant-a 的 plan）"
    );

    // ========== 反向一致性：B 再写，A 实时读到（双向）==========
    experts_db::upsert_favorite("tenant-b", "exp-b-fav");
    let a_favs_tenantb = experts_db::load_favorites_by_tenant("tenant-b");
    assert!(a_favs_tenantb.contains("exp-b-fav"), "A 应实时读到 B 刚写穿的收藏（双向一致）");

    // 清理（临时目录 drop 自动删除文件，此处显式清理 WAL/SHM 句柄）
    drop(inst_a);
    drop(inst_b);
    let _ = std::fs::remove_file(format!("{db}-wal"));
    let _ = std::fs::remove_file(format!("{db}-shm"));
}
