// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟 SQLite 持久化层集成测试
//!
//! 位于 `tests/`，与单元测试分属两个独立进程——单元测试走默认路径
//! `data/experts.db`，本进程通过 `MOX_EXPERTS_DB_PATH` 指向每用例独立临时库，
//! 互不干扰、可断言精确行数。进程内用例经 `ENV_LOCK` 串行化（环境变量是
//! 进程级全局状态）。
//!
//! 覆盖：注册表/会话（含消息投影）/图谱/预约 往返一致性、WAL、完整性检查、
//! JSON→SQLite 迁移（导入+归档+幂等+DB已有跳过）、并发写安全。

use mox_platform_gateway_svc::experts_common::{ExpertDescriptor, ExpertGraph, ExpertSession};
use mox_platform_gateway_svc::experts_db;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TempDb {
    _dir: tempfile::TempDir,
    #[allow(dead_code)]
    path: String,
}

impl TempDb {
    fn dir(&self) -> &std::path::Path {
        self._dir.path()
    }
}

/// 为当前用例设置独立临时库（返回锁守卫 + 临时库句柄）
fn setup_env() -> (MutexGuard<'static, ()>, TempDb) {
    let guard = env_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.path().join("experts.db").to_string_lossy().to_string();
    std::env::set_var(experts_db::ENV_DB_PATH, &path);
    (guard, TempDb { _dir: dir, path })
}

/// 结构体无 PartialEq，统一以 JSON 值对比（往返一致性）
fn as_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("序列化")
}

// =====================================================================
// 1. 专家注册表：往返一致性 + 列投影
// =====================================================================

#[test]
fn test_registry_roundtrip_with_projection() {
    let (_g, _db) = setup_env();

    let mut e1 = ExpertDescriptor::minimal("exp-it-001".into(), "持久化测试专家·甲".into());
    e1.domains = vec!["backend".into(), "sqlite".into()];
    e1.skills = vec!["Rust".into(), "rusqlite".into()];
    e1.capabilities = vec![mox_platform_gateway_svc::experts_common::ExpertCapability {
        id: "cap-rust".into(),
        name: "Rust".into(),
        domain: "backend".into(),
        proficiency: 95,
        description: "系统编程".into(),
    }];
    e1.metadata.insert("tier".into(), json!("gold"));
    e1.tags = vec!["it-test".into()];
    let mut e2 = ExpertDescriptor::minimal("exp-it-002".into(), "持久化测试专家·乙".into());
    e2.enabled = false;

    let mut map = HashMap::new();
    map.insert(e1.id.clone(), e1.clone());
    map.insert(e2.id.clone(), e2.clone());
    experts_db::save_registry(&map);

    let loaded = experts_db::load_registry();
    assert_eq!(loaded.len(), 2, "独立临时库应精确 2 行");
    assert_eq!(as_json(loaded.get("exp-it-001").unwrap()), as_json(&e1));
    assert_eq!(as_json(loaded.get("exp-it-002").unwrap()), as_json(&e2));

    // 列投影：热查询字段建列且与文档一致
    let conn = experts_db::open_experts_db().unwrap();
    let disabled: i64 = conn
        .query_row("SELECT COUNT(*) FROM experts WHERE enabled = 0", [], |r| r.get(0))
        .unwrap();
    assert_eq!(disabled, 1, "e2.enabled=false 应投影到列");
    let name: String = conn
        .query_row("SELECT name FROM experts WHERE id = 'exp-it-001'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "持久化测试专家·甲");
}

// =====================================================================
// 2. 会话：往返一致性 + 消息规范化投影 + Option 列
// =====================================================================

#[test]
fn test_sessions_roundtrip_with_message_projection() {
    let (_g, _db) = setup_env();

    let s1: ExpertSession = serde_json::from_value(json!({
        "id": "sess-it-001",
        "title": "SQLite 持久化会话",
        "expert_ids": ["exp-it-001"],
        "user_id": "u-1001",
        "session_type": "multi",
        "status": "active",
        "topic": "JSON→SQLite 迁移验证",
        "messages": [
            { "id": "m1", "role": "user", "sender_id": "u-1001", "sender_name": "用户甲",
              "content": "如何设计专家域持久化？", "created_at": "2026-09-04T00:00:00Z" },
            { "id": "m2", "role": "expert", "sender_id": "exp-it-001", "sender_name": "专家甲",
              "content": "使用 WAL + 事务全量同步。", "rating": 5,
              "attachments": [{ "kind": "doc", "ref": "experts_db.md" }],
              "created_at": "2026-09-04T00:01:00Z" },
            { "id": "m3", "role": "system", "content": "会话已创建",
              "created_at": "2026-09-04T00:00:30Z" }
        ],
        "tags": ["it"],
        "created_at": "2026-09-04T00:00:00Z",
        "last_active_at": "2026-09-04T00:01:00Z"
    }))
    .unwrap();

    let s2: ExpertSession = serde_json::from_value(json!({
        "id": "sess-it-002",
        "status": "archived",
        "archived_at": "2026-09-04T01:00:00Z",
        "created_at": "2026-09-04T00:00:00Z"
    }))
    .unwrap();

    let mut map = HashMap::new();
    map.insert(s1.id.clone(), s1.clone());
    map.insert(s2.id.clone(), s2.clone());
    experts_db::save_sessions(&map);

    let loaded = experts_db::load_sessions();
    assert_eq!(loaded.len(), 2);
    assert_eq!(as_json(loaded.get("sess-it-001").unwrap()), as_json(&s1));
    assert_eq!(as_json(loaded.get("sess-it-002").unwrap()), as_json(&s2));

    // 消息规范化投影：3 条、按 seq 保序
    let conn = experts_db::open_experts_db().unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_messages WHERE session_id = 'sess-it-001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 3);
    let role: String = conn
        .query_row(
            "SELECT role FROM session_messages WHERE session_id = 'sess-it-001' AND seq = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(role, "expert");
    // Option 列：archived_at
    let archived: Option<String> = conn
        .query_row(
            "SELECT archived_at FROM sessions WHERE id = 'sess-it-002'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(archived.as_deref(), Some("2026-09-04T01:00:00Z"));
}

// =====================================================================
// 3. 能力图谱：节点/边/元信息往返 + 保序
// =====================================================================

#[test]
fn test_graph_roundtrip() {
    let (_g, _db) = setup_env();

    let g: ExpertGraph = serde_json::from_value(json!({
        "nodes": [
            { "id": "exp-it-001", "label": "专家甲", "node_type": "expert",
              "properties": { "title": "架构师" } },
            { "id": "domain-backend", "label": "backend", "node_type": "domain" }
        ],
        "edges": [
            { "source": "exp-it-001", "target": "domain-backend",
              "edge_type": "has_domain", "weight": 1.0 },
            { "source": "exp-it-001", "target": "domain-backend",
              "edge_type": "similar_to", "weight": 0.5 }
        ],
        "built_at": "2026-09-04T00:00:00Z",
        "version": 7
    }))
    .unwrap();
    experts_db::save_graph(&g);

    let loaded = experts_db::load_graph();
    assert_eq!(loaded.nodes.len(), 2);
    assert_eq!(loaded.edges.len(), 2);
    assert_eq!(loaded.version, 7);
    assert_eq!(loaded.built_at, "2026-09-04T00:00:00Z");
    assert!(loaded.nodes.iter().any(|n| n.id == "domain-backend"));
    assert!(loaded.nodes.iter().any(|n| n.id == "exp-it-001"));
    // 边按写入顺序保序（seq）
    assert_eq!(loaded.edges[0].edge_type, "has_domain");
    assert_eq!(loaded.edges[1].edge_type, "similar_to");
}

// =====================================================================
// 4. 预约：JSON 文档行存储往返 + 写入顺序保持
// =====================================================================

#[test]
fn test_bookings_roundtrip() {
    let (_g, _db) = setup_env();

    let rows = vec![
        json!({
            "id": "bk-it-001", "expert_id": "exp-it-001", "expert_name": "专家甲",
            "user_id": "u-1", "topic": "架构评审", "scheduled_at": "2026-09-05T10:00:00Z",
            "duration_minutes": 60, "status": "confirmed", "created_at": "2026-09-04T00:00:00Z"
        }),
        json!({
            "id": "bk-it-002", "expert_id": "exp-it-002", "expert_name": "专家乙",
            "user_id": "u-2", "topic": "性能诊断", "scheduled_at": "2026-09-06T14:00:00Z",
            "duration_minutes": 90, "status": "pending", "created_at": "2026-09-04T00:10:00Z"
        }),
    ];
    experts_db::save_bookings(&rows);

    let loaded = experts_db::load_bookings();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0]["id"], "bk-it-001", "按写入顺序（rowid）返回");
    assert_eq!(loaded[1]["id"], "bk-it-002");
    assert_eq!(loaded[0]["topic"], "架构评审");
}

// =====================================================================
// 5. WAL 模式 + 完整性检查
// =====================================================================

#[test]
fn test_wal_mode_and_integrity() {
    let (_g, _db) = setup_env();

    let mut map = HashMap::new();
    let e = ExpertDescriptor::minimal("exp-it-wal".into(), "WAL检查专家".into());
    map.insert(e.id.clone(), e);
    experts_db::save_registry(&map);

    let conn = experts_db::open_experts_db().unwrap();
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode.to_lowercase(), "wal", "应启用 WAL（读写并发）");
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok.to_lowercase(), "ok");
    assert_eq!(experts_db::integrity_check().unwrap().to_lowercase(), "ok");
}

// =====================================================================
// 6. 迁移：导入 + 归档 + 读回一致 + 幂等
// =====================================================================

#[test]
fn test_migration_imports_and_archives() {
    let (_g, db) = setup_env();
    let dir = db.dir();

    std::fs::write(
        dir.join("experts_registry.json"),
        json!({ "exp-mig-001": { "id": "exp-mig-001", "name": "迁移专家",
                                 "created_at": "2026-01-01T00:00:00Z" } })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("experts_sessions.json"),
        json!({ "sess-mig-001": {
                    "id": "sess-mig-001", "created_at": "2026-01-01T00:00:00Z",
                    "messages": [ { "id": "m1", "role": "user", "content": "你好",
                                    "created_at": "2026-01-01T00:00:00Z" } ] } })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("experts_graph.json"),
        json!({ "nodes": [ { "id": "exp-mig-001", "label": "迁移专家",
                             "node_type": "expert" } ],
                "edges": [], "version": 1, "built_at": "2026-01-01T00:00:00Z" })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("experts_bookings.json"),
        json!([ { "id": "bk-mig-001", "expert_id": "exp-mig-001", "status": "pending",
                  "created_at": "2026-01-01T00:00:00Z" } ])
        .to_string(),
    )
    .unwrap();

    let report = experts_db::migrate_json_to_sqlite();
    assert_eq!(report.registry, 1);
    assert_eq!(report.sessions, 1);
    assert_eq!(report.graph_nodes, 1);
    assert_eq!(report.bookings, 1);
    assert_eq!(report.archived.len(), 4, "四类 JSON 均应归档");
    for a in &report.archived {
        assert!(std::path::Path::new(a).exists(), "归档文件应存在: {}", a);
        assert!(a.contains(".migrated-"), "归档名应带后缀: {}", a);
    }

    // 原文件已改名归档
    for f in [
        "experts_registry.json",
        "experts_sessions.json",
        "experts_graph.json",
        "experts_bookings.json",
    ] {
        assert!(!dir.join(f).exists(), "{} 应已归档", f);
    }

    // 数据可从 SQLite 读回
    let reg = experts_db::load_registry();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg["exp-mig-001"].name, "迁移专家");
    let sess = experts_db::load_sessions();
    assert_eq!(sess["sess-mig-001"].messages.len(), 1);
    let g = experts_db::load_graph();
    assert_eq!(g.nodes.len(), 1);
    let bk = experts_db::load_bookings();
    assert_eq!(bk.len(), 1);

    // 幂等：JSON 已归档，再次迁移为 noop
    let report2 = experts_db::migrate_json_to_sqlite();
    assert!(report2.is_noop(), "二次迁移应为 noop: {:?}", report2);
}

// =====================================================================
// 7. 迁移：SQLite 已有数据则跳过导入且不归档（SQLite 为权威）
// =====================================================================

#[test]
fn test_migration_skips_when_db_populated() {
    let (_g, db) = setup_env();

    // 先直接写入 SQLite
    let e = ExpertDescriptor::minimal("exp-db-001".into(), "DB内专家".into());
    let mut map = HashMap::new();
    map.insert(e.id.clone(), e);
    experts_db::save_registry(&map);

    // 同目录放一个内容不同的"历史"JSON
    let json_path = db.dir().join("experts_registry.json");
    std::fs::write(
        &json_path,
        json!({ "exp-mig-x": { "id": "exp-mig-x", "name": "不该导入的专家",
                               "created_at": "2026-01-01T00:00:00Z" } })
        .to_string(),
    )
    .unwrap();

    let report = experts_db::migrate_json_to_sqlite();
    assert_eq!(report.registry, 0, "DB 已有数据应跳过导入");
    assert!(
        !report.archived.iter().any(|a| a.contains("experts_registry.json")),
        "跳过导入时不应归档 JSON"
    );
    assert!(json_path.exists(), "JSON 应保留");

    let reg = experts_db::load_registry();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg["exp-db-001"].name, "DB内专家");
}

// =====================================================================
// 8. 并发写安全：8 线程 × 10 轮全量替换，最终库完整且确定
// =====================================================================

#[test]
fn test_concurrent_writers_integrity() {
    let (_g, _db) = setup_env();

    let mut handles = Vec::new();
    for t in 0..8usize {
        handles.push(std::thread::spawn(move || {
            for i in 0..10usize {
                let mut map = HashMap::new();
                for j in 0..5usize {
                    let id = format!("exp-t{}-r{}-{}", t, i, j);
                    map.insert(
                        id.clone(),
                        ExpertDescriptor::minimal(id, format!("并发专家{}-{}-{}", t, i, j)),
                    );
                }
                experts_db::save_registry(&map);
                let _ = experts_db::load_registry();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let conn = experts_db::open_experts_db().unwrap();
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok.to_lowercase(), "ok", "并发写后库应完整无损");
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM experts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 5, "全量替换语义下最终行数 = 单次写入行数（最后提交者胜出）");
}
