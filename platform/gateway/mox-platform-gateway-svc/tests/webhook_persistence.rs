//! v5 webhook 订阅落盘真实验证（禁止 mock，2026-10-03 全维终验）：
//!
//! T4 webhook 此前为进程内内存 HashMap、重启即失。本轮把订阅写穿 `alliance_webhooks`
//! 表并在启动 `restore_webhooks_from_db` 读回。本测试用真实 SQLite（独立临时库）+ 真实
//! `ExpertsSharedState::new()` 启动路径，走「登记 → 模拟崩溃重启 → 读回」闭环：
//!
//! 1. 首次 `new()`（空库）；经与 handler 完全相同的 `events.register_webhook` 登记订阅
//!    （该方法内部写穿 SQLite），覆盖 tenant-a / tenant-b；
//! 2. **模拟崩溃重启**：丢弃 state，再次 `new()`（从同一份 SQLite 读回 webhook）；
//! 3. 断言：订阅读回与登记逐字段一致（id/url/event_types/tenant/created_at），
//!    跨租户不可见；删除订阅后重启不再恢复；二次重启幂等不翻倍。
use mox_platform_gateway_svc::alliance::experts_common::ExpertsSharedState;
use std::sync::Arc;

#[test]
fn webhook_write_then_restart_recovers_with_tenant_isolation_and_delete_persists() {
    // 独立临时库，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_webhook_persist_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

    // ========== 首次启动（first boot）==========
    let state1 = Arc::new(ExpertsSharedState::new());
    // 全新库：内存注册表应为空
    assert!(state1.events.list_webhooks("tenant-a").is_empty(), "首启 tenant-a webhook 应为空");
    assert!(state1.events.list_webhooks("tenant-b").is_empty(), "首启 tenant-b webhook 应为空");

    // ----- tenant-a：登记 2 个订阅（一个全收、一个类型过滤）-----
    let wh_a1 = state1.events.register_webhook(
        "tenant-a",
        "http://127.0.0.1:9/hooks/a-all".into(),
        vec![],
    );
    let wh_a2 = state1.events.register_webhook(
        "tenant-a",
        "http://127.0.0.1:9/hooks/a-registered".into(),
        vec!["ExpertRegistered".into(), "ExpertDisabled".into()],
    );
    // ----- tenant-b：登记 1 个订阅 -----
    let wh_b1 = state1.events.register_webhook(
        "tenant-b",
        "http://127.0.0.1:9/hooks/b1".into(),
        vec!["PlanCreated".into()],
    );

    // 内存自检（写后即入热投影）
    assert_eq!(state1.events.list_webhooks("tenant-a").len(), 2, "写后 tenant-a 应 2 个订阅");
    assert_eq!(state1.events.list_webhooks("tenant-b").len(), 1, "写后 tenant-b 应 1 个订阅");

    // ========== 模拟崩溃：丢弃 state1 ==========
    drop(state1);

    // ========== 重启（second boot）：从同一份 SQLite 读回 webhook ==========
    let state2 = Arc::new(ExpertsSharedState::new());

    // --- tenant-a：2 个订阅全部恢复，逐字段一致 ---
    let a_list = state2.events.list_webhooks("tenant-a");
    assert_eq!(a_list.len(), 2, "重启后 tenant-a 应恢复 2 个订阅，实际 {}", a_list.len());
    let rec_a1 = a_list.iter().find(|w| w.id == wh_a1.id).expect("wh_a1 应从 SQLite 恢复");
    assert_eq!(rec_a1.url, "http://127.0.0.1:9/hooks/a-all", "url 应逐字段一致");
    assert_eq!(rec_a1.tenant, "tenant-a");
    assert!(rec_a1.event_types.is_empty(), "空过滤应恢复为空数组（=全收）");
    assert!(!rec_a1.created_at.is_empty(), "created_at 应恢复");
    let rec_a2 = a_list.iter().find(|w| w.id == wh_a2.id).expect("wh_a2 应恢复");
    assert_eq!(rec_a2.event_types, vec!["ExpertRegistered".to_string(), "ExpertDisabled".to_string()], "event_types 过滤应恢复");

    // --- 租户隔离：tenant-a 绝不能看到 tenant-b 的订阅 ---
    let b_list = state2.events.list_webhooks("tenant-b");
    assert_eq!(b_list.len(), 1, "重启后 tenant-b 应恢复 1 个订阅");
    assert!(b_list.iter().any(|w| w.id == wh_b1.id), "wh_b1 应恢复");
    assert!(a_list.iter().all(|w| w.id != wh_b1.id), "tenant-a 绝不能看到 tenant-b 的订阅");
    assert!(b_list.iter().all(|w| w.id != wh_a1.id), "tenant-b 绝不能看到 tenant-a 的订阅");

    // --- 派发匹配语义在重启后仍正确（同租户 + 类型过滤）---
    let matched = state2.events.matching_webhooks("tenant-a", "ExpertRegistered");
    assert!(matched.iter().any(|w| w.id == wh_a1.id), "全收订阅应命中 ExpertRegistered");
    assert!(matched.iter().any(|w| w.id == wh_a2.id), "类型过滤订阅应命中 ExpertRegistered");
    // PlanCreated 只应命中 wh_a1（空过滤=全收），不命中 wh_a2（过滤不含 PlanCreated）
    let matched_pc = state2.events.matching_webhooks("tenant-a", "PlanCreated");
    assert!(matched_pc.iter().any(|w| w.id == wh_a1.id));
    assert!(matched_pc.iter().all(|w| w.id != wh_a2.id), "wh_a2 不含 PlanCreated 不应命中");
    // tenant-b 的 wh_b1 不应在 tenant-a 的 ExpertRegistered 事件里被投到
    assert!(matched.iter().all(|w| w.tenant == "tenant-a"), "匹配结果必须同租户");

    // ========== 删除订阅后重启：删除须持久化 ==========
    // 在 state2 删除 wh_a2
    assert!(state2.events.delete_webhook("tenant-a", &wh_a2.id), "删除 wh_a2 应命中");
    assert!(!state2.events.delete_webhook("tenant-a", &wh_a2.id), "重复删除应返回 false");
    // 跨租户删除应失败（tenant-b 删不了 tenant-a 的 wh_a1）
    assert!(!state2.events.delete_webhook("tenant-b", &wh_a1.id), "跨租户删除必须失败");
    drop(state2);

    // 三次启动：wh_a2 已删不应恢复；wh_a1 / wh_b1 仍在
    let state3 = Arc::new(ExpertsSharedState::new());
    let a_list3 = state3.events.list_webhooks("tenant-a");
    assert_eq!(a_list3.len(), 1, "删除重启后 tenant-a 应只剩 1 个订阅");
    assert!(a_list3.iter().any(|w| w.id == wh_a1.id), "wh_a1 应仍在");
    assert!(a_list3.iter().all(|w| w.id != wh_a2.id), "wh_a2 删除后不应恢复");
    assert_eq!(state3.events.list_webhooks("tenant-b").len(), 1, "wh_b1 应仍在");

    // ========== 四次启动再验证幂等（不翻倍）==========
    drop(state3);
    let state4 = Arc::new(ExpertsSharedState::new());
    assert_eq!(state4.events.list_webhooks("tenant-a").len(), 1, "二次重启后 tenant-a 仍 1（幂等）");
    assert_eq!(state4.events.list_webhooks("tenant-b").len(), 1, "二次重启后 tenant-b 仍 1（幂等）");

    // 清理
    let _ = std::fs::remove_file(&db);
    let db_s = db.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{db_s}-wal"));
    let _ = std::fs::remove_file(format!("{db_s}-shm"));
}
