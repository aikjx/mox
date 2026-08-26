//! # 算子商城：路径迁移与版本化 集成测试
//!
//! 覆盖：
//! - semver 解析 / 比较 / bump
//! - 版本快照 / 变更日志 / 回滚
//! - 历史保留数裁剪（OUS_MARKET_KEEP_VERSIONS）
//! - 旧路径迁移（自动备份 + 审计）
//! - 旧路径读取兼容（自动补迁）
//! - 导入冲突策略（overwrite / skip / rename）与签名校验
//! - 租户 / 创建人 / 权限过滤
//! - ZIP 与签名往返
//!
//! 说明：涉及 OUS_HOME 环境变量的测试用全局锁串行执行，
//! 每个测试使用独立临时目录，避免并行污染。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as TokioMutex;

use mox_platform_orchestrator_svc::market::{
    list_packages_filtered, load_package, package_path, save_package, FlowEdge, FlowNode,
    MarketState, OperatorPackage,
};
use mox_platform_orchestrator_svc::market_dsl;
use mox_platform_orchestrator_svc::market_migration::{
    audit_log_path, backups_dir, ensure_migrated, packages_dir, sign_doc, verify_doc, zip_read,
    zip_write,
};
use mox_platform_orchestrator_svc::market_version::{
    bump_patch_version, diff_packages, get_version, is_valid_version, list_versions,
    read_changelog, rollback, snapshot_package, version_cmp, SemVer,
};

/// 环境变量测试串行锁
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 唯一的临时 OUS_HOME
fn temp_home(tag: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4().to_string();
    std::env::temp_dir().join(format!("ous-test-{}-{}", tag, unique))
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

/// 构造测试包
fn make_pkg(id: &str, version: &str, requirement: &str) -> OperatorPackage {
    OperatorPackage {
        id: id.to_string(),
        name: format!("包-{}", id),
        category: "测试".to_string(),
        author: "tester".to_string(),
        version: version.to_string(),
        summary: "测试包".to_string(),
        requirement: requirement.to_string(),
        nodes: vec![FlowNode {
            id: "n1".into(),
            label: "开始".into(),
            node_type: "start".into(),
            x: 0.0,
            y: 0.0,
            note: "".into(),
        }],
        edges: vec![],
        features: vec![],
        tags: vec![],
        created_at: mox_platform_orchestrator_svc::market_migration::now_rfc3339(),
        updated_at: mox_platform_orchestrator_svc::market_migration::now_rfc3339(),
        clone_count: 0,
        forked_from: None,
        tenant: "default".to_string(),
        tenant_id: "default".to_string(),
        created_by: "tester".to_string(),
        permissions: vec![],
        ..Default::default()
    }
}

// ========== 1) semver ==========

#[test]
fn semver_parse_compare_and_bump() {
    assert!(is_valid_version("1.2.3"));
    assert!(is_valid_version("1.2.3-alpha.1"));
    assert!(!is_valid_version("v1.2.3"));
    assert!(!is_valid_version("1.2.3.4"));

    assert_eq!(version_cmp("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
    assert_eq!(version_cmp("1.9.0", "1.10.0"), std::cmp::Ordering::Less);
    assert_eq!(
        version_cmp("2.0.0-alpha", "2.0.0"),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        version_cmp("2.0.0-alpha.10", "2.0.0-alpha.9"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        version_cmp("1.0.0+build5", "1.0.0"),
        std::cmp::Ordering::Equal
    );

    assert_eq!(bump_patch_version("1.2.9"), "1.2.10");
    assert_eq!(
        SemVer::parse("1.2.3").unwrap().bump_minor().to_string(),
        "1.3.0"
    );
    assert_eq!(
        SemVer::parse("1.2.3").unwrap().bump_major().to_string(),
        "2.0.0"
    );
}

// ========== 2) 快照 / 回滚 ==========

#[test]
fn snapshot_rollback_and_changelog() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("snapshot");
    std::env::set_var("OUS_HOME", &home);

    let mut p = make_pkg("p-snap", "1.0.0", "需求 v1");
    save_package(&p).unwrap();
    snapshot_package(&p, "alice", "首次发布").unwrap();
    // 更新
    p.version = "1.0.1".to_string();
    p.requirement = "需求 v1.1".to_string();
    save_package(&p).unwrap();
    snapshot_package(&p, "alice", "补丁更新").unwrap();

    // 版本列表：2 个快照
    let versions = list_versions("p-snap");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, "1.0.1"); // 降序

    // 读取历史版本
    let v1 = get_version("p-snap", "1.0.0").expect("应读到 1.0.0 快照");
    assert_eq!(v1.requirement, "需求 v1");

    // 差异对比
    let diff = diff_packages(&v1, &load_package("p-snap").unwrap());
    assert!(diff.changed);
    assert!(diff.fields_changed.contains(&"requirement".to_string()));

    // 变更日志已追加
    let cl = read_changelog("p-snap");
    assert!(cl.contains("首次发布"));
    assert!(cl.contains("补丁更新"));

    // 回滚到 1.0.0
    let rolled = rollback("p-snap", "1.0.0", "bob").expect("回滚应成功");
    assert_eq!(rolled.version, "1.0.0");
    let cur = load_package("p-snap").unwrap();
    assert_eq!(cur.version, "1.0.0");
    assert_eq!(cur.requirement, "需求 v1");
    assert!(read_changelog("p-snap").contains("回滚到 v1.0.0"));

    cleanup(&home);
    std::env::remove_var("OUS_HOME");
}

// ========== 3) 历史保留裁剪 ==========

#[test]
fn prune_keeps_configured_limit() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("prune");
    std::env::set_var("OUS_HOME", &home);
    std::env::set_var("OUS_MARKET_KEEP_VERSIONS", "3");

    for i in 0..6 {
        let p = make_pkg("p-prune", &format!("1.0.{}", i), "需求");
        snapshot_package(&p, "tester", "迭代").unwrap();
    }
    let versions = list_versions("p-prune");
    assert_eq!(versions.len(), 3, "应只保留最近 3 个快照");
    // 保留的是最新 3 个版本
    assert_eq!(versions[0].version, "1.0.5");
    assert_eq!(versions[2].version, "1.0.3");

    cleanup(&home);
    std::env::remove_var("OUS_HOME");
    std::env::remove_var("OUS_MARKET_KEEP_VERSIONS");
}

// ========== 4) 旧路径迁移（自动备份 + 审计）==========

#[test]
fn migrate_legacy_dir_with_backup_and_audit() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("migrate");
    let legacy = temp_home("legacy");
    std::env::set_var("OUS_HOME", &home);
    std::env::set_var("OUS_LEGACY_MARKET_DIR", &legacy);

    // 造旧数据
    std::fs::create_dir_all(&legacy).unwrap();
    let p1 = make_pkg("legacy-1", "1.0.0", "旧包1");
    let p2 = make_pkg("legacy-2", "2.0.0", "旧包2");
    std::fs::write(
        legacy.join("legacy-1.json"),
        serde_json::to_string_pretty(&p1).unwrap(),
    )
    .unwrap();
    std::fs::write(
        legacy.join("legacy-2.json"),
        serde_json::to_string_pretty(&p2).unwrap(),
    )
    .unwrap();
    // 非 json 文件不迁移
    std::fs::write(legacy.join("notes.txt"), "x").unwrap();

    let report = ensure_migrated();
    assert_eq!(report.migrated_from_legacy, 2);
    assert_eq!(report.backed_up, 2);
    assert!(report.backup_dir.is_some());

    // 目标目录有文件
    assert!(packages_dir().join("legacy-1.json").exists());
    assert!(packages_dir().join("legacy-2.json").exists());
    // 备份目录有副本
    let backup = backups_dir();
    assert!(backup.exists());
    let mut backup_files = 0;
    for e in std::fs::read_dir(&backup).unwrap().flatten() {
        if e.path().is_dir() {
            backup_files += std::fs::read_dir(e.path()).unwrap().count();
        }
    }
    assert_eq!(backup_files, 2, "备份应包含被迁移的 2 个包");

    // 审计日志存在且含 migration 记录
    let audit = std::fs::read_to_string(audit_log_path()).unwrap_or_default();
    assert!(audit.contains("migration"));

    // 二次启动幂等：不再重复迁移
    let report2 = ensure_migrated();
    assert_eq!(report2.migrated_from_legacy, 0);

    cleanup(&home);
    cleanup(&legacy);
    std::env::remove_var("OUS_HOME");
    std::env::remove_var("OUS_LEGACY_MARKET_DIR");
}

// ========== 5) 旧路径读取兼容（自动补迁）==========

#[test]
fn read_falls_back_to_legacy_and_auto_migrates() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("readfb");
    let legacy = temp_home("legacy2");
    std::env::set_var("OUS_HOME", &home);
    std::env::set_var("OUS_LEGACY_MARKET_DIR", &legacy);

    std::fs::create_dir_all(&legacy).unwrap();
    let p = make_pkg("fb-1", "1.0.0", "遗留读取");
    std::fs::write(
        legacy.join("fb-1.json"),
        serde_json::to_string_pretty(&p).unwrap(),
    )
    .unwrap();

    // 未迁移时也能读到（向后兼容）
    let loaded = load_package("fb-1").expect("应能从遗留路径读到");
    assert_eq!(loaded.id, "fb-1");
    // 读操作触发自动补迁到归一化路径
    assert!(
        package_path("fb-1").exists(),
        "读取后应自动迁移到 packages/"
    );
    assert!(!legacy.join("fb-1.json").exists(), "遗留副本应被清理");

    cleanup(&home);
    cleanup(&legacy);
    std::env::remove_var("OUS_HOME");
    std::env::remove_var("OUS_LEGACY_MARKET_DIR");
}

// ========== 6) ZIP 与签名 ==========

#[test]
fn zip_and_signature_roundtrip() {
    let entries = vec![
        ("manifest.json".to_string(), br#"{"count":1}"#.to_vec()),
        ("packages/a.json".to_string(), br#"{"id":"a"}"#.to_vec()),
    ];
    let bytes = zip_write(&entries);
    let read = zip_read(&bytes).unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(read[1].0, "packages/a.json");

    let mut doc = serde_json::json!({ "kind": "ous-market-export", "package": { "id": "a" } });
    let sig = sign_doc(&mut doc);
    assert!(!sig.is_empty());
    assert!(verify_doc(&doc));
    doc["package"]["id"] = serde_json::Value::String("b".into());
    assert!(!verify_doc(&doc), "篡改后签名应校验失败");
}

// ========== 7) 导入冲突策略 ==========

#[test]
fn import_conflict_strategies() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("import");
    std::env::set_var("OUS_HOME", &home);

    // 预置一个已存在包
    let existing = make_pkg("imp-1", "1.0.0", "已有");
    save_package(&existing).unwrap();

    // 构造带签名的导出文档
    let incoming = make_pkg("imp-1", "2.0.0", "导入的新版本");
    let mut doc = serde_json::json!({ "kind": "ous-market-export", "package": incoming });
    sign_doc(&mut doc);

    // skip（默认）：不覆盖
    let r = mox_platform_orchestrator_svc::routes::market::import_one(
        doc.clone(),
        mox_platform_orchestrator_svc::routes::market::ConflictStrategy::Skip,
        true,
        "tester",
    );
    assert_eq!(r.status, "skipped");
    assert_eq!(load_package("imp-1").unwrap().version, "1.0.0");

    // overwrite：覆盖，且旧版本被快照
    let r = mox_platform_orchestrator_svc::routes::market::import_one(
        doc.clone(),
        mox_platform_orchestrator_svc::routes::market::ConflictStrategy::Overwrite,
        true,
        "tester",
    );
    assert_eq!(r.status, "overwritten");
    assert_eq!(load_package("imp-1").unwrap().version, "2.0.0");
    let versions = list_versions("imp-1");
    assert!(
        versions.iter().any(|v| v.version == "1.0.0"),
        "覆盖前旧版本应已快照"
    );

    // rename：新 id 导入
    let r = mox_platform_orchestrator_svc::routes::market::import_one(
        doc.clone(),
        mox_platform_orchestrator_svc::routes::market::ConflictStrategy::Rename,
        true,
        "tester",
    );
    assert_eq!(r.status, "renamed");
    let new_id = r.id;
    assert_ne!(new_id, "imp-1");
    assert!(load_package(&new_id).is_ok());

    // 签名被篡改：verify=true 应拒绝
    let mut tampered = doc.clone();
    tampered["package"]["requirement"] = serde_json::Value::String("被篡改".into());
    let r = mox_platform_orchestrator_svc::routes::market::import_one(
        tampered,
        mox_platform_orchestrator_svc::routes::market::ConflictStrategy::Overwrite,
        true,
        "tester",
    );
    assert_eq!(r.status, "rejected");
    assert!(r.reason.unwrap_or_default().contains("签名"));

    // verify=false 时无签名也允许（裸包）
    let bare = make_pkg("imp-2", "1.0.0", "裸包");
    let r = mox_platform_orchestrator_svc::routes::market::import_one(
        serde_json::to_value(&bare).unwrap(),
        mox_platform_orchestrator_svc::routes::market::ConflictStrategy::Skip,
        false,
        "tester",
    );
    assert_eq!(r.status, "imported");
    assert!(load_package("imp-2").is_ok());

    cleanup(&home);
    std::env::remove_var("OUS_HOME");
}

// ========== 8) 租户 / 创建人 / 权限过滤 ==========

#[test]
fn tenant_owner_permission_filters() {
    // 中毒容错：若此前某测试 panic 导致锁中毒，仍可回收内部守卫继续运行，避免级联 PoisonError。
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = temp_home("filters");
    std::env::set_var("OUS_HOME", &home);

    let mut a = make_pkg("f-a", "1.0.0", "租户A");
    a.tenant_id = "tenant-a".to_string();
    a.created_by = "alice".to_string();
    a.permissions = vec!["read".to_string(), "deploy".to_string()];
    save_package(&a).unwrap();

    let mut b = make_pkg("f-b", "1.0.0", "租户B");
    b.tenant_id = "tenant-b".to_string();
    b.created_by = "bob".to_string();
    b.permissions = vec!["read".to_string()];
    save_package(&b).unwrap();

    let state = MarketState {
        index: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };
    mox_platform_orchestrator_svc::market::reload_index_sync(&state);

    // 按租户
    let ta = list_packages_filtered(&state, None, None, None, Some("tenant-a"), None, None);
    assert_eq!(ta.len(), 1);
    assert_eq!(ta[0].id, "f-a");

    // 按创建人
    let bob = list_packages_filtered(&state, None, None, None, None, Some("bob"), None);
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].id, "f-b");

    // 按权限
    let deploy = list_packages_filtered(&state, None, None, None, None, None, Some("deploy"));
    assert_eq!(deploy.len(), 1);
    assert_eq!(deploy[0].id, "f-a");
    let read = list_packages_filtered(&state, None, None, None, None, None, Some("read"));
    assert_eq!(read.len(), 2);

    cleanup(&home);
    std::env::remove_var("OUS_HOME");
}

// ========== 9) DSL 转换链路 ==========

#[test]
fn dsl_conversion_chain() {
    let mut p = make_pkg("dsl-1", "1.0.0", "订单处理需求");
    p.nodes = vec![
        FlowNode {
            id: "s".into(),
            label: "开始".into(),
            node_type: "start".into(),
            x: 0.0,
            y: 0.0,
            note: "".into(),
        },
        FlowNode {
            id: "c".into(),
            label: "校验库存".into(),
            node_type: "decision".into(),
            x: 0.0,
            y: 1.0,
            note: "查库存".into(),
        },
        FlowNode {
            id: "o".into(),
            label: "发货".into(),
            node_type: "operator".into(),
            x: 0.0,
            y: 2.0,
            note: "".into(),
        },
        FlowNode {
            id: "e".into(),
            label: "结束".into(),
            node_type: "end".into(),
            x: 0.0,
            y: 3.0,
            note: "".into(),
        },
    ];
    p.edges = vec![
        FlowEdge {
            id: "e1".into(),
            source: "s".into(),
            target: "c".into(),
            label: "".into(),
        },
        FlowEdge {
            id: "e2".into(),
            source: "c".into(),
            target: "o".into(),
            label: "有货".into(),
        },
        FlowEdge {
            id: "e3".into(),
            source: "c".into(),
            target: "e".into(),
            label: "缺货".into(),
        },
        FlowEdge {
            id: "e4".into(),
            source: "o".into(),
            target: "e".into(),
            label: "".into(),
        },
    ];

    let dsl = market_dsl::package_to_flow_definition(&p);
    assert_eq!(dsl.nodes.len(), 4);
    assert_eq!(dsl.variables["requirement"], "订单处理需求");

    let wf = market_dsl::flow_definition_to_business_workflow(&dsl);
    assert_eq!(wf.start_node_id, "s");
    assert_eq!(wf.nodes.len(), 4);
    let cond = wf
        .nodes
        .iter()
        .find(|n| serde_json::to_value(&n.node_type).unwrap() == serde_json::json!("condition"))
        .expect("应有条件节点");
    let cfg = serde_json::to_value(&cond.config).unwrap();
    assert_eq!(cfg["expression"], "有货");
    assert_eq!(cfg["true_path"], "o");
    assert_eq!(cfg["false_path"], "e");
    let code = market_dsl::generate_workflow_code(&wf);
    assert!(code.contains("transitions"));
}

// ========== 10) 异步状态初始化（种子 + 索引）==========

static ASYNC_LOCK: OnceLock<TokioMutex<()>> = OnceLock::new();

#[tokio::test]
#[allow(clippy::await_holding_lock)] // 故意：TokioMutex guard 为 Send，跨 await 持有以串行化各异步测试对 OUS_HOME 的初始化
async fn init_state_creates_seed_and_index() {
    // 与所有同步 env 测试共用 ENV_LOCK：保证 OUS_HOME 在本异步 init 期间不被并发改写/清理。
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = ASYNC_LOCK.get_or_init(|| TokioMutex::new(())).lock().await;
    let home = temp_home("init");
    std::env::set_var("OUS_HOME", &home);

    let state = mox_platform_orchestrator_svc::market::init_market_state().await;
    let idx = state.index.lock().await;
    assert!(idx.contains_key("seed-ous-full-flow"), "应生成种子包");
    drop(idx);
    assert!(package_path("seed-ous-full-flow").exists());

    cleanup(&home);
    std::env::remove_var("OUS_HOME");
}
