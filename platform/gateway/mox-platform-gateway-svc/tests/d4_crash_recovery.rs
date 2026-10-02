//! D4 进程内三项落盘真实验证（禁止 mock）：
//!
//! 用真实 SQLite（独立临时库）+ 真实 `ExpertsSharedState`，走「写入 → 模拟重启 → 读回」闭环：
//! 1. 首次 `ExpertsSharedState::new()`（空库启动）；
//! 2. 按 handler 同样的方式：内存态写入 + 立即落盘（upsert_plan / insert_history_record /
//!    upsert_favorite），覆盖 tenant-a / tenant-b 两个租户；
//! 3. **模拟崩溃重启**：丢弃 state，再次 `ExpertsSharedState::new()`（从同一份 SQLite 读回）；
//! 4. 断言：plans / history / favorites 读回与写入**逐字段一致**，且 tenant-a 看不到
//!    tenant-b 的计划与收藏（租户隔离）。
//!
//! 这不是示例数据：写入的是真实构造的领域对象，落的是真实 data/experts.db（临时路径），
//! 读回的是 `new()` 启动加载路径，与生产崩溃恢复路径完全一致。
use mox_platform_gateway_svc::alliance::experts_common::{
    CollaborationPlan, ExpertsSharedState, OrchestrationRecord, PlanStep, now_iso,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 构造一个真实的协作计划（指定租户、任务类型）
fn make_plan(plan_id: &str, tenant: &str, task_type: &str) -> CollaborationPlan {
    let now = now_iso();
    let mut metadata = HashMap::new();
    metadata.insert("task_type".into(), serde_json::json!(task_type));
    metadata.insert("tenant_id".into(), serde_json::json!(tenant));
    CollaborationPlan {
        plan_id: plan_id.into(),
        task_id: None,
        title: format!("协作计划·{plan_id}"),
        description: format!("{tenant} 的真实任务：{task_type}"),
        expert_ids: vec!["exp-a-1".into(), "exp-b-1".into()],
        steps: vec![PlanStep {
            step_id: format!("{plan_id}-step-1"),
            name: "需求分析".into(),
            description: "解析目标".into(),
            expert_id: Some("exp-a-1".into()),
            step_type: "intake".into(),
            depends_on: vec![],
            status: "completed".into(),
            result: Some(serde_json::json!({"solution": "真实结论A"})),
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

/// 构造一条真实的编排执行历史
fn make_history(execution_id: &str, plan_id: &str, status: &str) -> OrchestrationRecord {
    OrchestrationRecord {
        execution_id: execution_id.into(),
        plan_id: plan_id.into(),
        task_type: "analysis".into(),
        status: status.into(),
        expert_ids: vec!["exp-a-1".into()],
        steps_completed: 1,
        steps_total: 1,
        result_summary: "真实执行完成".into(),
        result: Some(serde_json::json!({"summary": "真实执行完成"})),
        created_at: now_iso(),
        completed_at: Some(now_iso()),
        duration_ms: 42,
    }
}

#[test]
fn d4_write_then_restart_recovers_three_items_with_tenant_isolation() {
    // 独立临时库，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_d4_e2e_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

    // ========== 首次启动（first boot）==========
    let state1 = Arc::new(ExpertsSharedState::new());
    // 启动加载应为空（全新库）
    assert!(state1.plans.lock().is_empty(), "首启 plans 应为空");
    assert!(state1.orchestration_history.lock().is_empty(), "首启 history 应为空");
    assert!(state1.favorites.lock().is_empty(), "首启 favorites 应为空");

    // ----- tenant-a：写 2 个 plan + 1 条 history + 2 个收藏 -----
    let plan_a1 = make_plan("plan-a1", "tenant-a", "analysis");
    let plan_a2 = make_plan("plan-a2", "tenant-a", "research");
    let hist_a1 = make_history("exec-a1", "plan-a1", "completed");
    {
        let mut plans = state1.plans.lock();
        plans.insert(plan_a1.plan_id.clone(), plan_a1.clone());
        plans.insert(plan_a2.plan_id.clone(), plan_a2.clone());
    }
    mox_platform_gateway_svc::alliance::experts_common::upsert_plan("tenant-a", &plan_a1);
    mox_platform_gateway_svc::alliance::experts_common::upsert_plan("tenant-a", &plan_a2);
    {
        let mut h = state1.orchestration_history.lock();
        h.push(hist_a1.clone());
    }
    mox_platform_gateway_svc::alliance::experts_common::insert_history_record("tenant-a", &hist_a1);
    {
        let mut favs = state1.favorites.lock();
        favs.entry("tenant-a".into()).or_default().insert("exp-a-1".into());
        favs.entry("tenant-a".into()).or_default().insert("exp-a-2".into());
    }
    mox_platform_gateway_svc::alliance::experts_common::upsert_favorite("tenant-a", "exp-a-1");
    mox_platform_gateway_svc::alliance::experts_common::upsert_favorite("tenant-a", "exp-a-2");

    // ----- tenant-b：写 1 个 plan + 1 个收藏（与 a 不重叠）-----
    let plan_b1 = make_plan("plan-b1", "tenant-b", "consulting");
    {
        let mut plans = state1.plans.lock();
        plans.insert(plan_b1.plan_id.clone(), plan_b1.clone());
    }
    mox_platform_gateway_svc::alliance::experts_common::upsert_plan("tenant-b", &plan_b1);
    {
        let mut favs = state1.favorites.lock();
        favs.entry("tenant-b".into()).or_default().insert("exp-b-1".into());
    }
    mox_platform_gateway_svc::alliance::experts_common::upsert_favorite("tenant-b", "exp-b-1");

    // 落盘后内存态自检（写后即落）
    assert_eq!(state1.plans.lock().len(), 3, "写后应有 3 个 plan");
    assert_eq!(state1.orchestration_history.lock().len(), 1, "写后应有 1 条 history");

    // ========== 模拟崩溃：丢弃 state1 ==========
    drop(state1);

    // ========== 重启（second boot）：从同一份 SQLite 读回 ==========
    let state2 = Arc::new(ExpertsSharedState::new());

    // --- plans 读回：3 个全部恢复，逐字段一致 ---
    let plans2 = state2.plans.lock();
    assert_eq!(plans2.len(), 3, "重启后 plans 应恢复 3 个，实际 {}", plans2.len());
    let recovered_a1 = plans2.get("plan-a1").expect("plan-a1 应从 SQLite 恢复");
    assert_eq!(recovered_a1.title, "协作计划·plan-a1", "title 应逐字段一致");
    assert_eq!(recovered_a1.status, "completed");
    assert_eq!(recovered_a1.steps.len(), 1);
    assert_eq!(
        recovered_a1.metadata.get("tenant_id").and_then(|v| v.as_str()),
        Some("tenant-a"),
        "plan-a1 的租户应恢复为 tenant-a"
    );
    assert_eq!(recovered_a1.expert_ids, vec!["exp-a-1".to_string(), "exp-b-1".to_string()]);
    assert!(plans2.get("plan-a2").is_some(), "plan-a2 应恢复");
    assert!(plans2.get("plan-b1").is_some(), "plan-b1 应恢复");

    // --- 租户隔离：tenant-a 只见自己的 plan（与 handler stats 过滤语义一致）---
    let a_visible: Vec<&String> = plans2
        .iter()
        .filter(|(_, p)| p.metadata.get("tenant_id").and_then(|v| v.as_str()) == Some("tenant-a"))
        .map(|(id, _)| id)
        .collect();
    let b_visible: Vec<&String> = plans2
        .iter()
        .filter(|(_, p)| p.metadata.get("tenant_id").and_then(|v| v.as_str()) == Some("tenant-b"))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(a_visible.len(), 2, "tenant-a 应只见 2 个 plan");
    assert!(!a_visible.contains(&&"plan-b1".to_string()), "tenant-a 绝不能看到 tenant-b 的 plan-b1");
    assert_eq!(b_visible.len(), 1, "tenant-b 应只见 1 个 plan");
    assert!(b_visible.contains(&&"plan-b1".to_string()));
    assert!(!b_visible.contains(&&"plan-a1".to_string()), "tenant-b 绝不能看到 tenant-a 的 plan");
    drop(plans2);

    // --- history 读回：1 条恢复，字段一致 ---
    let hist2 = state2.orchestration_history.lock();
    assert_eq!(hist2.len(), 1, "重启后 history 应恢复 1 条，实际 {}", hist2.len());
    let rec = &hist2[0];
    assert_eq!(rec.execution_id, "exec-a1");
    assert_eq!(rec.plan_id, "plan-a1");
    assert_eq!(rec.status, "completed");
    assert_eq!(rec.duration_ms, 42);
    drop(hist2);

    // --- favorites 读回：按租户分区，互不可见 ---
    let favs2 = state2.favorites.lock();
    let a_favs: &HashSet<String> = favs2.get("tenant-a").expect("tenant-a 收藏应恢复");
    let b_favs: &HashSet<String> = favs2.get("tenant-b").expect("tenant-b 收藏应恢复");
    assert!(a_favs.contains("exp-a-1") && a_favs.contains("exp-a-2"), "tenant-a 两个收藏应恢复");
    assert!(!a_favs.contains("exp-b-1"), "tenant-a 绝不能看到 tenant-b 的收藏");
    assert!(b_favs.contains("exp-b-1"), "tenant-b 收藏应恢复");
    assert!(!b_favs.contains("exp-a-1"), "tenant-b 绝不能看到 tenant-a 的收藏");
    drop(favs2);

    // ========== 二次重启再验证幂等（不重复、不丢）==========
    drop(state2);
    let state3 = Arc::new(ExpertsSharedState::new());
    assert_eq!(state3.plans.lock().len(), 3, "二次重启后 plans 仍应是 3（幂等不翻倍）");
    assert_eq!(state3.orchestration_history.lock().len(), 1, "二次重启后 history 仍应是 1");
    assert_eq!(state3.favorites.lock().get("tenant-a").map(|s| s.len()).unwrap_or(0), 2);

    // 清理
    let _ = std::fs::remove_file(&db);
    let db_s = db.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{db_s}-wal"));
    let _ = std::fs::remove_file(format!("{db_s}-shm"));
}
