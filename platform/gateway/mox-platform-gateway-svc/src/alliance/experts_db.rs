// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟 SQLite 持久化层（Experts DB）
//!
//! 将专家联盟核心数据（专家注册表 / 会话 / 能力图谱 / 预约）从 JSON 文件
//! 持久化迁移到 SQLite（`data/experts.db`），提供企业级的原子性、事务与
//! 并发安全：
//!
//! - **WAL 模式**：读写并发不互斥（`PRAGMA journal_mode=WAL`）
//! - **busy_timeout**：跨连接写竞争自动等待（默认 5s）
//! - **事务化全量同步**：每次 save 在单事务内完成（崩溃时要么全写入要么
//!   全不写，不会出现 JSON 半截文件式的损坏）
//! - **列投影 + JSON 文档混合建模**：热查询字段建列（可索引），完整领域
//!   对象存 `data_json`（结构演进零 DDL 迁移成本）
//! - **自动迁移**：启动时检测历史 JSON 文件（experts_registry/sessions/
//!   graph/bookings），一次性导入 SQLite 后改名归档（`*.json.migrated-<ts>`）
//!
//! 数据库路径可通过环境变量 [`ENV_DB_PATH`] 覆盖（测试隔离用）；历史 JSON
//! 文件与数据库同目录（默认 `data/`，与历史路径完全兼容）。
//!
//! 容错策略：与原 JSON 持久化一致——持久化失败仅记录 stderr 不阻断业务
//! （内存态 `ExpertsSharedState` 仍是权威数据源，SQLite 为持久投影）。

use crate::alliance::experts_common::{
    CollaborationPlan, ExpertDescriptor, ExpertGraph, ExpertSession, GraphEdge, GraphNode,
    OrchestrationRecord,
};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 数据库路径环境变量（测试/部署隔离用）
pub const ENV_DB_PATH: &str = "MOX_EXPERTS_DB_PATH";
/// 默认数据库路径（相对 cwd，与 data/ 约定一致）
pub const DEFAULT_DB_PATH: &str = "data/experts.db";
/// 跨连接写锁等待超时（毫秒）
const BUSY_TIMEOUT_MS: u64 = 5000;

/// 历史 JSON 文件名（与数据库同目录；默认 data/ 下，与历史路径兼容）
const JSON_REGISTRY_FILE: &str = "experts_registry.json";
const JSON_SESSIONS_FILE: &str = "experts_sessions.json";
const JSON_GRAPH_FILE: &str = "experts_graph.json";
const JSON_BOOKINGS_FILE: &str = "experts_bookings.json";

/// 解析当前数据库路径（每次调用读取，保证测试/运行时可覆盖）
pub fn db_path() -> String {
    std::env::var(ENV_DB_PATH).unwrap_or_else(|_| DEFAULT_DB_PATH.to_string())
}

/// 历史 JSON 文件完整路径（与数据库同目录）
fn json_path(file: &str) -> String {
    match Path::new(&db_path()).parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            dir.join(file).to_string_lossy().to_string()
        }
        _ => file.to_string(),
    }
}

/// 打开数据库连接并初始化 schema（幂等）
///
/// 每次调用新建短连接（与 crate 内 IAM 层的常驻连接风格不同，此处写入
/// 已由内存态 Mutex 串行化，短连接 + WAL + busy_timeout 更利于测试隔离
/// 与多进程部署），并确保：
/// - `journal_mode=WAL`：读写并发
/// - `synchronous=NORMAL`：WAL 下的安全与性能平衡点
/// - `busy_timeout=5s`：跨连接写竞争自动等待
pub fn open_experts_db() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录 {} 失败: {}", parent.display(), e))?;
        }
    }
    let conn = Connection::open(&path).map_err(|e| format!("打开 {} 失败: {}", path, e))?;
    conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("设置 WAL 失败: {}", e))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| format!("设置 synchronous 失败: {}", e))?;
    init_schema(&conn)?;
    migrate_schema_version(&conn)?;
    Ok(conn)
}

/// 当前 schema 版本号。加列/加表/改主键时递增此值，并在 migrate_schema_version 中
/// 加对应版本间的迁移步骤。
///
/// v1：单租户初始 schema（experts.id 为全局主键，graph_edges.seq 为全局行号）。
/// v2：A1 多租户——experts/graph_nodes/graph_edges/graph_meta 全部引入
/// `tenant_id TEXT NOT NULL DEFAULT 'default'`，主键升为复合键
/// (tenant_id, id|seq|k)。存量 v1 库在迁移时把既有行统一归到 `default` 租户。
/// v3：D4 进程内三项落盘——新增 collaboration_plans / orchestration_history /
/// favorites 三张表（均带 `tenant_id` 复合主键）。三项此前为纯内存态、无历史数据，
/// 故 v2→v3 仅由 `init_schema` 的 `CREATE TABLE IF NOT EXISTS` 建表 + bump
/// `user_version`，无需数据搬迁。
/// v4：T4 事件驱动——新增 alliance_event_log 事件轨迹表（事件总线消费者落库，带
/// `tenant_id` 行级隔离）。全新表、无历史数据，故 v3→v4 同样仅由 `init_schema` 的
/// `CREATE TABLE IF NOT EXISTS` 建表 + bump `user_version`，无需数据搬迁。
/// v5：全维终验（2026-10-03）——webhook 外部订阅表落盘（alliance_webhooks）。此前 webhook
/// 注册为进程内内存 HashMap、重启即失；现新增本表，CRUD 写穿 + 启动读回，重启恢复订阅。
/// 全新表、无历史数据，故 v4→v5 同样仅由 `init_schema` 的 `CREATE TABLE IF NOT EXISTS`
/// 建表 + bump `user_version`，无需数据搬迁。
const SCHEMA_VERSION: i32 = 5;

/// 启动时按 `PRAGMA user_version` 做 schema 版本迁移。
///
/// - 新库（user_version=0 且无表）：`init_schema` 已直接建成 v2 形状，此处仅 bump 到 2；
/// - 存量 v1 库（user_version=1，表无 tenant_id 列）：重建四张表为复合主键形状，
///   既有行归 `default` 租户；
/// - 已迁移库（user_version>=2）：直接跳过。
/// 保持 WAL / busy_timeout 等 PRAGMA 不变，不破坏单写者约定。
fn migrate_schema_version(conn: &Connection) -> Result<(), String> {
    let current: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(0);
    if current >= SCHEMA_VERSION as i64 {
        return Ok(());
    }
    // 存量 v1 库：experts 表存在但缺 tenant_id 列 → 重建为 v2 复合主键形状。
    // 新库由 init_schema 直接建成 v2 形状（含 tenant_id），无需重建。
    if table_exists(conn, "experts") && !column_exists(conn, "experts", "tenant_id") {
        migrate_v1_to_v2(conn)?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|e| format!("设置 PRAGMA user_version={} 失败: {}", SCHEMA_VERSION, e))?;
    tracing::info!("[experts_db] schema 迁移完成: user_version {} -> {}", current, SCHEMA_VERSION);
    Ok(())
}

/// 判断表是否存在
fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |_| Ok(()),
    ).is_ok()
}

/// 判断某表是否已含某列（PRAGMA table_info）
fn column_exists(conn: &Connection, table: &str, col: &str) -> bool {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({})", table)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rows = stmt.query_map([], |r| r.get::<_, String>(1));
    if let Ok(iter) = rows {
        for name in iter.flatten() {
            if name == col {
                return true;
            }
        }
    }
    false
}

/// v1 → v2 迁移：把单租户表重建为按 `tenant_id` 复合主键的形状。
///
/// 既有行统一归入 `default` 租户（与「无头请求=default」的零回归语义一致）。
/// SQLite 不支持 ALTER 改主键，故采用「建新表 → 拷贝 → 删旧表 → 改名」标准流程。
fn migrate_v1_to_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        -- experts：(tenant_id, id) 复合主键
        CREATE TABLE experts_new (
            tenant_id    TEXT NOT NULL DEFAULT 'default',
            id           TEXT NOT NULL,
            name         TEXT NOT NULL DEFAULT '',
            title        TEXT NOT NULL DEFAULT '',
            organization TEXT NOT NULL DEFAULT '',
            expert_type  TEXT NOT NULL DEFAULT 'ai',
            status       TEXT NOT NULL DEFAULT 'online',
            enabled      INTEGER NOT NULL DEFAULT 1,
            avg_rating   REAL NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT '',
            updated_at   TEXT NOT NULL DEFAULT '',
            data_json    TEXT NOT NULL,
            PRIMARY KEY (tenant_id, id)
        );
        INSERT INTO experts_new
            (tenant_id, id, name, title, organization, expert_type, status, enabled, avg_rating, created_at, updated_at, data_json)
        SELECT 'default', id, name, title, organization, expert_type, status, enabled, avg_rating, created_at, updated_at, data_json FROM experts;
        DROP TABLE experts;
        ALTER TABLE experts_new RENAME TO experts;
        CREATE INDEX idx_experts_tenant ON experts(tenant_id);
        CREATE INDEX idx_experts_name ON experts(name);
        CREATE INDEX idx_experts_enabled ON experts(enabled);

        -- graph_nodes：(tenant_id, id) 复合主键
        CREATE TABLE graph_nodes_new (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            id        TEXT NOT NULL,
            label     TEXT NOT NULL DEFAULT '',
            node_type TEXT NOT NULL DEFAULT '',
            data_json TEXT NOT NULL,
            PRIMARY KEY (tenant_id, id)
        );
        INSERT INTO graph_nodes_new (tenant_id, id, label, node_type, data_json)
        SELECT 'default', id, label, node_type, data_json FROM graph_nodes;
        DROP TABLE graph_nodes;
        ALTER TABLE graph_nodes_new RENAME TO graph_nodes;
        CREATE INDEX idx_graph_nodes_tenant ON graph_nodes(tenant_id);

        -- graph_edges：(tenant_id, seq) 复合主键（seq 语义在租户内重新从 0 计数）
        CREATE TABLE graph_edges_new (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            seq       INTEGER NOT NULL,
            source    TEXT NOT NULL DEFAULT '',
            target    TEXT NOT NULL DEFAULT '',
            edge_type TEXT NOT NULL DEFAULT '',
            weight    REAL NOT NULL DEFAULT 0,
            data_json TEXT NOT NULL,
            PRIMARY KEY (tenant_id, seq)
        );
        INSERT INTO graph_edges_new (tenant_id, seq, source, target, edge_type, weight, data_json)
        SELECT 'default', seq, source, target, edge_type, weight, data_json FROM graph_edges;
        DROP TABLE graph_edges;
        ALTER TABLE graph_edges_new RENAME TO graph_edges;
        CREATE INDEX idx_graph_edges_tenant ON graph_edges(tenant_id);

        -- graph_meta：(tenant_id, k) 复合主键
        CREATE TABLE graph_meta_new (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            k TEXT NOT NULL,
            v TEXT NOT NULL,
            PRIMARY KEY (tenant_id, k)
        );
        INSERT INTO graph_meta_new (tenant_id, k, v)
        SELECT 'default', k, v FROM graph_meta;
        DROP TABLE graph_meta;
        ALTER TABLE graph_meta_new RENAME TO graph_meta;
        "#,
    )
    .map_err(|e| format!("v1→v2 重建多租户表失败: {}", e))?;
    Ok(())
}

/// 幂等建表（列投影 + JSON 文档混合建模）
///
/// v2（A1 多租户）：experts / graph_nodes / graph_edges / graph_meta 均带
/// `tenant_id` 并以 `(tenant_id, …)` 为复合主键。新库由此直接建成 v2 形状；
/// 存量 v1 库由 [`migrate_v1_to_v2`] 重建。
fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        -- 专家注册表：热查询字段建列，完整描述符存 data_json；按租户复合主键
        CREATE TABLE IF NOT EXISTS experts (
            tenant_id    TEXT NOT NULL DEFAULT 'default',
            id           TEXT NOT NULL,
            name         TEXT NOT NULL DEFAULT '',
            title        TEXT NOT NULL DEFAULT '',
            organization TEXT NOT NULL DEFAULT '',
            expert_type  TEXT NOT NULL DEFAULT 'ai',
            status       TEXT NOT NULL DEFAULT 'online',
            enabled      INTEGER NOT NULL DEFAULT 1,
            avg_rating   REAL NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT '',
            updated_at   TEXT NOT NULL DEFAULT '',
            data_json    TEXT NOT NULL,
            PRIMARY KEY (tenant_id, id)
        );
        CREATE INDEX IF NOT EXISTS idx_experts_tenant ON experts(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_experts_name ON experts(name);
        CREATE INDEX IF NOT EXISTS idx_experts_enabled ON experts(enabled);

        -- 会话：完整会话（含 messages）存 data_json
        CREATE TABLE IF NOT EXISTS sessions (
            id             TEXT PRIMARY KEY,
            title          TEXT NOT NULL DEFAULT '',
            user_id        TEXT NOT NULL DEFAULT '',
            session_type   TEXT NOT NULL DEFAULT 'single',
            status         TEXT NOT NULL DEFAULT 'active',
            created_at     TEXT NOT NULL DEFAULT '',
            last_active_at TEXT NOT NULL DEFAULT '',
            archived_at    TEXT,
            data_json      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
        CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);

        -- 会话消息规范化投影（权威数据在 sessions.data_json，本表供
        -- 消息级查询/统计/审计使用，随 save_sessions 同事务重建）
        CREATE TABLE IF NOT EXISTS session_messages (
            session_id TEXT NOT NULL,
            seq        INTEGER NOT NULL,
            msg_id     TEXT NOT NULL DEFAULT '',
            role       TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT '',
            content    TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (session_id, seq)
        );
        CREATE INDEX IF NOT EXISTS idx_messages_session ON session_messages(session_id);

        -- 能力图谱：节点/边投影 + 元信息（均按租户隔离，复合主键）
        CREATE TABLE IF NOT EXISTS graph_nodes (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            id        TEXT NOT NULL,
            label     TEXT NOT NULL DEFAULT '',
            node_type TEXT NOT NULL DEFAULT '',
            data_json TEXT NOT NULL,
            PRIMARY KEY (tenant_id, id)
        );
        CREATE INDEX IF NOT EXISTS idx_graph_nodes_tenant ON graph_nodes(tenant_id);

        CREATE TABLE IF NOT EXISTS graph_edges (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            seq       INTEGER NOT NULL,
            source    TEXT NOT NULL DEFAULT '',
            target    TEXT NOT NULL DEFAULT '',
            edge_type TEXT NOT NULL DEFAULT '',
            weight    REAL NOT NULL DEFAULT 0,
            data_json TEXT NOT NULL,
            PRIMARY KEY (tenant_id, seq)
        );
        CREATE INDEX IF NOT EXISTS idx_graph_edges_tenant ON graph_edges(tenant_id);

        CREATE TABLE IF NOT EXISTS graph_meta (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            k TEXT NOT NULL,
            v TEXT NOT NULL,
            PRIMARY KEY (tenant_id, k)
        );

        -- 专家广场预约（experts_ext）
        CREATE TABLE IF NOT EXISTS bookings (
            id         TEXT PRIMARY KEY,
            expert_id  TEXT NOT NULL DEFAULT '',
            user_id    TEXT NOT NULL DEFAULT '',
            status     TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT '',
            data_json  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_bookings_expert ON bookings(expert_id);

        -- D4（v3）：协作计划落盘（按租户复合主键；完整计划含 steps/metadata 存 data_json）
        CREATE TABLE IF NOT EXISTS collaboration_plans (
            tenant_id  TEXT NOT NULL DEFAULT 'default',
            plan_id    TEXT NOT NULL,
            title      TEXT NOT NULL DEFAULT '',
            status     TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT '',
            data_json  TEXT NOT NULL,
            PRIMARY KEY (tenant_id, plan_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plans_tenant ON collaboration_plans(tenant_id);

        -- D4（v3）：编排执行历史落盘（按租户 + execution_id 复合主键）
        CREATE TABLE IF NOT EXISTS orchestration_history (
            tenant_id    TEXT NOT NULL DEFAULT 'default',
            execution_id TEXT NOT NULL,
            plan_id      TEXT NOT NULL DEFAULT '',
            status       TEXT NOT NULL DEFAULT '',
            created_at   TEXT NOT NULL DEFAULT '',
            data_json    TEXT NOT NULL,
            PRIMARY KEY (tenant_id, execution_id)
        );
        CREATE INDEX IF NOT EXISTS idx_history_tenant ON orchestration_history(tenant_id);

        -- D4（v3）：专家收藏落盘（按租户隔离；租户内 expert_id 唯一）
        CREATE TABLE IF NOT EXISTS favorites (
            tenant_id TEXT NOT NULL,
            expert_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (tenant_id, expert_id)
        );
        CREATE INDEX IF NOT EXISTS idx_favorites_tenant ON favorites(tenant_id);

        -- T4（v4）：事件轨迹日志（进程内事件总线消费者落库）。
        -- 每行一个真实业务事件（计划创建/状态推进/专家注册·禁用），带租户行级隔离；
        -- 结构化载荷存 payload（权威），热查询字段（event_type/plan_id）建列建索引。
        CREATE TABLE IF NOT EXISTS alliance_event_log (
            tenant_id   TEXT NOT NULL DEFAULT 'default',
            event_id    TEXT NOT NULL,
            event_type  TEXT NOT NULL,
            source      TEXT NOT NULL DEFAULT '',
            occurred_at TEXT NOT NULL DEFAULT '',
            plan_id     TEXT NOT NULL DEFAULT '',
            payload     TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (tenant_id, event_id)
        );
        CREATE INDEX IF NOT EXISTS idx_event_log_tenant ON alliance_event_log(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_event_log_type ON alliance_event_log(event_type);
        CREATE INDEX IF NOT EXISTS idx_event_log_plan ON alliance_event_log(plan_id);

        -- v5（全维终验）：webhook 外部订阅落盘（重启恢复；此前为进程内内存 HashMap）。
        -- id 全局主键；tenant_id 行级隔离；event_types 为 JSON 数组字符串（空=全收）。
        CREATE TABLE IF NOT EXISTS alliance_webhooks (
            id          TEXT PRIMARY KEY,
            tenant_id   TEXT NOT NULL,
            url         TEXT NOT NULL,
            event_types TEXT NOT NULL DEFAULT '[]',
            created_at  TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_webhooks_tenant ON alliance_webhooks(tenant_id);
        "#,
    )
    .map_err(|e| format!("初始化 schema 失败: {}", e))
}

fn log_err(op: &str, err: &str) {
    eprintln!("[experts_db] {} 失败: {}", op, err);
}

/// A2（无状态化阶段一）：写穿 busy/locked 应用层重试循环。
///
/// 打开连接时已设 `busy_timeout=5s`（见 [`open_experts_db`]），WAL 下读写本不互斥，
/// 仅多写者并发才会争锁。即便如此，跨进程写竞争在 busy_timeout 耗尽的极端情形下仍可能
/// 返回 `SQLITE_BUSY` / `SQLITE_LOCKED`（错误串含 `locked`）。这里在应用层对「锁类瞬时错误」
/// 再做有限次（≤5）退避重试，进一步降低多副本共享同一 SQLite 时写穿的瞬时失败率；
/// 非锁类错误（如序列化失败）不重试，直接按既有约定 log_err 不阻断业务。
fn retry_write<F: Fn() -> Result<(), String>>(op: &str, f: F) {
    const MAX_ATTEMPTS: u32 = 5;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match f() {
            Ok(()) => return,
            Err(e) if (e.contains("busy") || e.contains("locked")) && attempt < MAX_ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_millis(40 * attempt as u64));
            }
            Err(e) => {
                log_err(op, &e);
                return;
            }
        }
    }
}

fn table_count(conn: &Connection, table: &str) -> Result<i64, String> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
        .map_err(|e| format!("COUNT({}) 失败: {}", table, e))
}

// =====================================================================
// 专家注册表（experts）
// =====================================================================

fn save_registry_conn(
    conn: &Connection,
    tenant: &str,
    registry: &HashMap<String, ExpertDescriptor>,
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    // 仅清空本租户行——多租户下绝不能 DELETE FROM experts（会跨租户清库）
    tx.execute("DELETE FROM experts WHERE tenant_id = ?1", params![tenant])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO experts (tenant_id, id, name, title, organization, expert_type, status, enabled, avg_rating, created_at, updated_at, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(|e| e.to_string())?;
        // 按 id 排序写入，保证落库顺序确定（可复现/可对比）
        let mut exps: Vec<&ExpertDescriptor> = registry.values().collect();
        exps.sort_by(|a, b| a.id.cmp(&b.id));
        for e in exps {
            let data =
                serde_json::to_string(e).map_err(|er| format!("序列化 expert {}: {}", e.id, er))?;
            stmt.execute(params![
                tenant,
                e.id,
                e.name,
                e.title,
                e.organization,
                e.expert_type,
                e.availability.status,
                e.enabled as i64,
                e.metrics.avg_rating,
                e.created_at,
                e.updated_at,
                data,
            ])
            .map_err(|er| format!("insert expert {}: {}", e.id, er))?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 全量同步某租户的专家注册表到 SQLite（单事务；失败仅记录不阻断）
pub fn save_registry(tenant: &str, registry: &HashMap<String, ExpertDescriptor>) {
    let res = open_experts_db().and_then(|conn| save_registry_conn(&conn, tenant, registry));
    if let Err(e) = res {
        log_err("save_registry", &e);
    }
}

/// 从 SQLite 加载某租户的专家注册表（失败返回空表，与历史 JSON 行为一致）
pub fn load_registry(tenant: &str) -> HashMap<String, ExpertDescriptor> {
    let mut map = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_registry", &e);
            return map;
        }
    };
    let rows = conn
        .prepare("SELECT data_json FROM experts WHERE tenant_id = ?1")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        });
    match rows {
        Ok(Ok(list)) => {
            for s in list {
                match serde_json::from_str::<ExpertDescriptor>(&s) {
                    Ok(e) => {
                        map.insert(e.id.clone(), e);
                    }
                    Err(er) => log_err("load_registry 反序列化", &er.to_string()),
                }
            }
        }
        Ok(Err(er)) => log_err("load_registry 查询", &er.to_string()),
        Err(er) => log_err("load_registry 查询", &er.to_string()),
    }
    map
}

/// 启动期加载全部租户的注册表（tenant -> 该租户 id->专家）。
///
/// 内存态 `ExpertsSharedState.registry` 为 per-tenant 结构，启动时一次性把所有
/// 租户行读入并按 `tenant_id` 分组，避免运行期按租户逐次开连接。
pub fn load_all_registries() -> HashMap<String, HashMap<String, ExpertDescriptor>> {
    let mut out: HashMap<String, HashMap<String, ExpertDescriptor>> = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_registries", &e);
            return out;
        }
    };
    let rows = conn
        .prepare("SELECT tenant_id, data_json FROM experts")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let t: String = row.get(0)?;
                let d: String = row.get(1)?;
                Ok((t, d))
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        });
    if let Ok(Ok(list)) = rows {
        for (tenant, s) in list {
            match serde_json::from_str::<ExpertDescriptor>(&s) {
                Ok(e) => {
                    out.entry(tenant).or_default().insert(e.id.clone(), e);
                }
                Err(er) => log_err("load_all_registries 反序列化", &er.to_string()),
            }
        }
    } else if let Err(er) | Ok(Err(er)) = rows {
        log_err("load_all_registries 查询", &er.to_string());
    }
    out
}

// =====================================================================
// 会话（sessions + session_messages）
// =====================================================================

fn save_sessions_conn(
    conn: &Connection,
    sessions: &HashMap<String, ExpertSession>,
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_messages", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM sessions", [])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO sessions (id, title, user_id, session_type, status, created_at, last_active_at, archived_at, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .map_err(|e| e.to_string())?;
        let mut msg_stmt = tx
            .prepare(
                "INSERT INTO session_messages (session_id, seq, msg_id, role, created_at, content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| e.to_string())?;
        let mut ss: Vec<&ExpertSession> = sessions.values().collect();
        ss.sort_by(|a, b| a.id.cmp(&b.id));
        for s in ss {
            let data =
                serde_json::to_string(s).map_err(|er| format!("序列化 session {}: {}", s.id, er))?;
            stmt.execute(params![
                s.id,
                s.title,
                s.user_id,
                s.session_type,
                s.status,
                s.created_at,
                s.last_active_at,
                s.archived_at,
                data,
            ])
            .map_err(|er| format!("insert session {}: {}", s.id, er))?;
            // 消息规范化投影（按消息在会话内的顺序写入 seq）
            for (i, m) in s.messages.iter().enumerate() {
                msg_stmt
                    .execute(params![s.id, i as i64, m.id, m.role, m.created_at, m.content])
                    .map_err(|er| format!("insert message {}#{}: {}", s.id, i, er))?;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 全量同步会话到 SQLite（单事务；消息投影同事务重建）
pub fn save_sessions(sessions: &HashMap<String, ExpertSession>) {
    let res = save_sessions_checked(sessions);
    if let Err(e) = res {
        log_err("save_sessions", &e);
    }
}

/// Callers that publish a success receipt must propagate a failed commit.
pub fn save_sessions_checked(sessions: &HashMap<String, ExpertSession>) -> Result<(), String> {
    open_experts_db().and_then(|conn| save_sessions_conn(&conn, sessions))
}

/// 从 SQLite 加载会话（messages 含在 data_json 中，失败返回空表）
pub fn load_sessions() -> HashMap<String, ExpertSession> {
    let mut map = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_sessions", &e);
            return map;
        }
    };
    let rows = conn
        .prepare("SELECT data_json FROM sessions")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        });
    match rows {
        Ok(Ok(list)) => {
            for s in list {
                match serde_json::from_str::<ExpertSession>(&s) {
                    Ok(sess) => {
                        map.insert(sess.id.clone(), sess);
                    }
                    Err(er) => log_err("load_sessions 反序列化", &er.to_string()),
                }
            }
        }
        Ok(Err(er)) => log_err("load_sessions 查询", &er.to_string()),
        Err(er) => log_err("load_sessions 查询", &er.to_string()),
    }
    map
}

// =====================================================================
// 能力图谱（graph_nodes / graph_edges / graph_meta）
// =====================================================================

fn save_graph_conn(conn: &Connection, tenant: &str, graph: &ExpertGraph) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    // 仅重建本租户的图数据——多租户下不得清空其他租户的节点/边/元信息
    tx.execute("DELETE FROM graph_nodes WHERE tenant_id = ?1", params![tenant])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_edges WHERE tenant_id = ?1", params![tenant])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_meta WHERE tenant_id = ?1", params![tenant])
        .map_err(|e| e.to_string())?;
    {
        let mut node_stmt = tx
            .prepare(
                "INSERT INTO graph_nodes (tenant_id, id, label, node_type, data_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(|e| e.to_string())?;
        let mut nodes: Vec<&GraphNode> = graph.nodes.iter().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        for n in nodes {
            let data =
                serde_json::to_string(n).map_err(|er| format!("序列化 node {}: {}", n.id, er))?;
            node_stmt
                .execute(params![tenant, n.id, n.label, n.node_type, data])
                .map_err(|er| format!("insert node {}: {}", n.id, er))?;
        }

        let mut edge_stmt = tx
            .prepare(
                "INSERT INTO graph_edges (tenant_id, seq, source, target, edge_type, weight, data_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| e.to_string())?;
        for (i, e) in graph.edges.iter().enumerate() {
            let data = serde_json::to_string(e)
                .map_err(|er| format!("序列化 edge #{}: {}", i, er))?;
            edge_stmt
                .execute(params![tenant, i as i64, e.source, e.target, e.edge_type, e.weight, data])
                .map_err(|er| format!("insert edge #{}: {}", i, er))?;
        }

        tx.execute(
            "INSERT INTO graph_meta (tenant_id, k, v) VALUES (?1, 'built_at', ?2), (?1, 'version', ?3)",
            params![tenant, graph.built_at, graph.version.to_string()],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 全量同步某租户的能力图谱到 SQLite（单事务）
pub fn save_graph(tenant: &str, graph: &ExpertGraph) {
    let res = open_experts_db().and_then(|conn| save_graph_conn(&conn, tenant, graph));
    if let Err(e) = res {
        log_err("save_graph", &e);
    }
}

/// 从 SQLite 加载某租户的能力图谱（失败返回默认空图谱）
pub fn load_graph(tenant: &str) -> ExpertGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut built_at = String::new();
    let mut version = 0u64;
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_graph", &e);
            return ExpertGraph::default();
        }
    };
    // 节点
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM graph_nodes WHERE tenant_id = ?1 ORDER BY id")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(n) = serde_json::from_str::<GraphNode>(&s) {
                nodes.push(n);
            }
        }
    }
    // 边（按 seq 保持写入顺序）
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM graph_edges WHERE tenant_id = ?1 ORDER BY seq")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(e) = serde_json::from_str::<GraphEdge>(&s) {
                edges.push(e);
            }
        }
    }
    // 元信息
    if let Ok(v) = conn.query_row(
        "SELECT v FROM graph_meta WHERE tenant_id = ?1 AND k = 'built_at'",
        params![tenant],
        |r| r.get::<_, String>(0),
    ) {
        built_at = v;
    }
    if let Ok(v) = conn.query_row(
        "SELECT v FROM graph_meta WHERE tenant_id = ?1 AND k = 'version'",
        params![tenant],
        |r| r.get::<_, String>(0),
    ) {
        version = v.parse().unwrap_or(0);
    }
    ExpertGraph {
        nodes,
        edges,
        built_at,
        version,
    }
}

/// 启动期加载全部租户的能力图谱（tenant -> 该租户图）。
pub fn load_all_graphs() -> HashMap<String, ExpertGraph> {
    let mut out: HashMap<String, ExpertGraph> = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_graphs", &e);
            return out;
        }
    };
    // 节点按租户分组
    if let Ok(rows) = conn
        .prepare("SELECT tenant_id, data_json FROM graph_nodes")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let t: String = row.get(0)?;
                let d: String = row.get(1)?;
                Ok((t, d))
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for (t, s) in rows.into_iter().flatten() {
            if let Ok(n) = serde_json::from_str::<GraphNode>(&s) {
                out.entry(t).or_default().nodes.push(n);
            }
        }
    }
    // 边按租户分组（按 seq 排序后落位，保持 seq==下标）
    if let Ok(rows) = conn
        .prepare("SELECT tenant_id, data_json FROM graph_edges ORDER BY tenant_id, seq")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let t: String = row.get(0)?;
                let d: String = row.get(1)?;
                Ok((t, d))
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for (t, s) in rows.into_iter().flatten() {
            if let Ok(e) = serde_json::from_str::<GraphEdge>(&s) {
                out.entry(t).or_default().edges.push(e);
            }
        }
    }
    // 元信息
    if let Ok(rows) = conn
        .prepare("SELECT tenant_id, k, v FROM graph_meta")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let t: String = row.get(0)?;
                let k: String = row.get(1)?;
                let v: String = row.get(2)?;
                Ok((t, k, v))
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for (t, k, v) in rows.into_iter().flatten() {
            let g = out.entry(t).or_default();
            if k == "built_at" {
                g.built_at = v;
            } else if k == "version" {
                g.version = v.parse().unwrap_or(0);
            }
        }
    }
    out
}

// =====================================================================
// 图谱增量写（N4 节点级 CRUD）：单条 UPSERT / 级联删 / 边重排
// =====================================================================
//
// 与全量 `save_graph_conn`（DELETE+INSERT，rebuild 语义）互补而不改动它：
// - 节点主键是 `graph_nodes.id`（TEXT PRIMARY KEY），用 INSERT ... ON CONFLICT(id) DO UPDATE
//   实现 UPSERT；
// - 边主键是 `graph_edges.seq`（INTEGER PRIMARY KEY = 行号），且 seq 与内存中
//   `ExpertGraph.edges` 的下标一一对应（load_graph 按 seq 升序落位、save_graph 按下标写入）。
//   追加/原位更新走 ON CONFLICT(seq) DO UPDATE；删除会让后续边下标前移，因此删除后
//   用 `replace_graph_edges_conn` 按当前内存顺序一次性重排 seq，保持「seq == 下标」不变量。
// - 持久化仍遵循本模块约定：内存态是权威源，SQLite 为投影；写失败仅记日志不阻断业务。

/// 单条 UPSERT 节点（ON CONFLICT(tenant_id, id) DO UPDATE）
pub fn upsert_graph_node_conn(conn: &Connection, tenant: &str, node: &GraphNode) -> Result<(), String> {
    let data =
        serde_json::to_string(node).map_err(|er| format!("序列化 node {}: {}", node.id, er))?;
    conn.execute(
        "INSERT INTO graph_nodes (tenant_id, id, label, node_type, data_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(tenant_id, id) DO UPDATE SET label = excluded.label, node_type = excluded.node_type, data_json = excluded.data_json",
        params![tenant, node.id, node.label, node.node_type, data],
    )
    .map_err(|e| format!("upsert node {}: {}", node.id, e))?;
    Ok(())
}

/// 删除节点并级联删除其关联边（单事务，仅限本租户）
pub fn delete_graph_node_cascade_conn(conn: &Connection, tenant: &str, id: &str) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM graph_edges WHERE tenant_id = ?1 AND (source = ?2 OR target = ?2)",
        params![tenant, id],
    )
    .map_err(|e| format!("cascade delete edges of node {}: {}", id, e))?;
    tx.execute(
        "DELETE FROM graph_nodes WHERE tenant_id = ?1 AND id = ?2",
        params![tenant, id],
    )
    .map_err(|e| format!("delete node {}: {}", id, e))?;
    tx.commit().map_err(|e| e.to_string())
}

/// UPSERT 一条边到指定 seq（POST 追加新 seq / PUT 原位更新同一 seq；仅限本租户）
pub fn upsert_graph_edge_conn(conn: &Connection, tenant: &str, seq: i64, edge: &GraphEdge) -> Result<(), String> {
    let data =
        serde_json::to_string(edge).map_err(|er| format!("序列化 edge #{seq}: {er}"))?;
    conn.execute(
        "INSERT INTO graph_edges (tenant_id, seq, source, target, edge_type, weight, data_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(tenant_id, seq) DO UPDATE SET source = excluded.source, target = excluded.target,
             edge_type = excluded.edge_type, weight = excluded.weight, data_json = excluded.data_json",
        params![tenant, seq, edge.source, edge.target, edge.edge_type, edge.weight, data],
    )
    .map_err(|e| format!("upsert edge #{seq}: {e}"))?;
    Ok(())
}

/// 按当前内存边顺序重排某租户的 graph_edges（删除边/节点后调用，保持 seq==下标不变量）
pub fn replace_graph_edges_conn(conn: &Connection, tenant: &str, edges: &[GraphEdge]) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_edges WHERE tenant_id = ?1", params![tenant])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO graph_edges (tenant_id, seq, source, target, edge_type, weight, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| e.to_string())?;
        for (i, e) in edges.iter().enumerate() {
            let data = serde_json::to_string(e)
                .map_err(|er| format!("序列化 edge #{}: {er}", i))?;
            stmt.execute(params![tenant, i as i64, e.source, e.target, e.edge_type, e.weight, data])
                .map_err(|er| format!("insert edge #{}: {er}", i))?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 写入某租户的 graph_meta（built_at / version），UPSERT
pub fn set_graph_meta_conn(conn: &Connection, tenant: &str, version: u64, built_at: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO graph_meta (tenant_id, k, v) VALUES (?1, 'built_at', ?2), (?1, 'version', ?3)
         ON CONFLICT(tenant_id, k) DO UPDATE SET v = excluded.v",
        params![tenant, built_at, version.to_string()],
    )
    .map_err(|e| format!("set graph_meta: {e}"))?;
    Ok(())
}

/// 增量 UPSERT 节点落库（best-effort，失败仅记日志）
pub fn upsert_graph_node(tenant: &str, node: &GraphNode) {
    let res = open_experts_db().and_then(|conn| upsert_graph_node_conn(&conn, tenant, node));
    if let Err(e) = res {
        log_err("upsert_graph_node", &e);
    }
}

/// 删除节点 + 级联边落库（best-effort）
pub fn delete_graph_node_cascade(tenant: &str, id: &str) {
    let res = open_experts_db().and_then(|conn| delete_graph_node_cascade_conn(&conn, tenant, id));
    if let Err(e) = res {
        log_err("delete_graph_node_cascade", &e);
    }
}

/// 增量 UPSERT 边落库（best-effort）
pub fn upsert_graph_edge(tenant: &str, seq: i64, edge: &GraphEdge) {
    let res = open_experts_db().and_then(|conn| upsert_graph_edge_conn(&conn, tenant, seq, edge));
    if let Err(e) = res {
        log_err("upsert_graph_edge", &e);
    }
}

/// 删除边后按当前内存顺序重排落库（best-effort）
pub fn replace_graph_edges(tenant: &str, edges: &[GraphEdge]) {
    let res = open_experts_db().and_then(|conn| replace_graph_edges_conn(&conn, tenant, edges));
    if let Err(e) = res {
        log_err("replace_graph_edges", &e);
    }
}

/// bump graph_meta（best-effort）
pub fn bump_graph_meta(tenant: &str, version: u64, built_at: &str) {
    let res = open_experts_db().and_then(|conn| set_graph_meta_conn(&conn, tenant, version, built_at));
    if let Err(e) = res {
        log_err("bump_graph_meta", &e);
    }
}


// =====================================================================
// 专家广场预约（bookings，供 experts_ext 使用，JSON 文档行存储）
// =====================================================================

fn save_bookings_conn(conn: &Connection, rows: &[Value]) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM bookings", [])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO bookings (id, expert_id, user_id, status, created_at, data_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| e.to_string())?;
        for (i, v) in rows.iter().enumerate() {
            let data = serde_json::to_string(v).map_err(|er| er.to_string())?;
            let id = v
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("__row_{}", i));
            stmt.execute(params![
                id,
                v.get("expert_id").and_then(Value::as_str).unwrap_or(""),
                v.get("user_id").and_then(Value::as_str).unwrap_or(""),
                v.get("status").and_then(Value::as_str).unwrap_or(""),
                v.get("created_at").and_then(Value::as_str).unwrap_or(""),
                data,
            ])
            .map_err(|er| format!("insert booking #{}: {}", i, er))?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 全量同步预约到 SQLite（单事务）
pub fn save_bookings(rows: &[Value]) {
    let res = open_experts_db().and_then(|conn| save_bookings_conn(&conn, rows));
    if let Err(e) = res {
        log_err("save_bookings", &e);
    }
}

/// 从 SQLite 加载预约（保持写入顺序）
pub fn load_bookings() -> Vec<Value> {
    let mut out = Vec::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_bookings", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM bookings ORDER BY rowid")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(v) = serde_json::from_str::<Value>(&s) {
                out.push(v);
            }
        }
    }
    out
}

// =====================================================================
// D4（v3）：进程内三项落盘——协作计划 / 编排历史 / 收藏集
// =====================================================================
//
// 这三项此前为纯内存态（ExpertsSharedState.plans/orchestration_history/favorites），
// 进程崩溃即丢。此处按既有「短连接 + WAL + 事务 + best-effort（失败仅记日志不阻断
// 业务）」约定补齐持久投影：
// - 写时：每次增/改后立即 upsert 单条（数据量小，单条 upsert 足矣，避免全量重写）；
// - 启动：ExpertsSharedState::new() 调 load_all_* 一次读回全部租户分区，重建内存态；
// - 租户：三表均以 (tenant_id, …) 为复合主键，与 A1 多租户行级隔离一致。
//   plans/history 内存态保持 A1 既有「全局扁平 + handler 按 metadata.tenant_id 过滤」
//   语义（plan_id/execution_id 为全局 UUID 不冲突）；favorites 内存态按租户分区。

/// 单条 UPSERT 协作计划（ON CONFLICT(tenant_id, plan_id) DO UPDATE）。
///
/// `tenant` 取请求租户（plan.metadata["tenant_id"] 同源）；完整计划序列化进 data_json。
pub fn upsert_plan_conn(conn: &Connection, tenant: &str, plan: &CollaborationPlan) -> Result<(), String> {
    let data = serde_json::to_string(plan)
        .map_err(|er| format!("序列化 plan {}: {}", plan.plan_id, er))?;
    conn.execute(
        "INSERT INTO collaboration_plans (tenant_id, plan_id, title, status, created_at, data_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(tenant_id, plan_id) DO UPDATE SET
            title = excluded.title, status = excluded.status,
            created_at = excluded.created_at, data_json = excluded.data_json",
        params![tenant, plan.plan_id, plan.title, plan.status, plan.created_at, data],
    )
    .map_err(|e| format!("upsert plan {}: {}", plan.plan_id, e))?;
    Ok(())
}

/// upsert 单条计划落库（best-effort；A2：锁类错误走 busy 重试）
pub fn upsert_plan(tenant: &str, plan: &CollaborationPlan) {
    retry_write("upsert_plan", || {
        open_experts_db().and_then(|conn| upsert_plan_conn(&conn, tenant, plan))
    });
}

/// 启动期加载全部租户的协作计划（重建全局扁平 HashMap<plan_id, plan>）。
///
/// plan_id 为全局 UUID，跨租户不冲突；运行期按 plan.metadata["tenant_id"] 过滤（A1 既有读面）。
pub fn load_all_plans() -> HashMap<String, CollaborationPlan> {
    let mut map = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_plans", &e);
            return map;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM collaboration_plans")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            match serde_json::from_str::<CollaborationPlan>(&s) {
                Ok(p) => {
                    map.insert(p.plan_id.clone(), p);
                }
                Err(er) => log_err("load_all_plans 反序列化", &er.to_string()),
            }
        }
    }
    map
}

/// 追加一条编排执行历史（INSERT；execution_id 全局唯一，冲突即覆盖更新）。
pub fn insert_history_record_conn(
    conn: &Connection,
    tenant: &str,
    rec: &OrchestrationRecord,
) -> Result<(), String> {
    let data = serde_json::to_string(rec)
        .map_err(|er| format!("序列化 history {}: {}", rec.execution_id, er))?;
    conn.execute(
        "INSERT INTO orchestration_history (tenant_id, execution_id, plan_id, status, created_at, data_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(tenant_id, execution_id) DO UPDATE SET
            plan_id = excluded.plan_id, status = excluded.status,
            created_at = excluded.created_at, data_json = excluded.data_json",
        params![tenant, rec.execution_id, rec.plan_id, rec.status, rec.created_at, data],
    )
    .map_err(|e| format!("insert history {}: {}", rec.execution_id, e))?;
    Ok(())
}

/// 追加一条历史落库（best-effort；A2：锁类错误走 busy 重试）
pub fn insert_history_record(tenant: &str, rec: &OrchestrationRecord) {
    retry_write("insert_history_record", || {
        open_experts_db().and_then(|conn| insert_history_record_conn(&conn, tenant, rec))
    });
}

/// 启动期加载全部租户的编排历史（按 created_at + rowid 升序，保持追加时序）。
pub fn load_all_history() -> Vec<OrchestrationRecord> {
    let mut out = Vec::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_history", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM orchestration_history ORDER BY created_at ASC, rowid ASC")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(r) = serde_json::from_str::<OrchestrationRecord>(&s) {
                out.push(r);
            }
        }
    }
    out
}

/// 收藏某专家（租户内），UPSERT
pub fn upsert_favorite_conn(conn: &Connection, tenant: &str, expert_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO favorites (tenant_id, expert_id, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(tenant_id, expert_id) DO NOTHING",
        params![tenant, expert_id, crate::alliance::experts_common::now_iso()],
    )
    .map_err(|e| format!("upsert favorite {tenant}/{expert_id}: {e}"))?;
    Ok(())
}

/// 收藏落库（best-effort；A2：锁类错误走 busy 重试）
pub fn upsert_favorite(tenant: &str, expert_id: &str) {
    retry_write("upsert_favorite", || {
        open_experts_db().and_then(|conn| upsert_favorite_conn(&conn, tenant, expert_id))
    });
}

/// 取消收藏（租户内）
pub fn delete_favorite_conn(conn: &Connection, tenant: &str, expert_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM favorites WHERE tenant_id = ?1 AND expert_id = ?2",
        params![tenant, expert_id],
    )
    .map_err(|e| format!("delete favorite {tenant}/{expert_id}: {e}"))?;
    Ok(())
}

/// 取消收藏落库（best-effort；A2：锁类错误走 busy 重试）
pub fn delete_favorite(tenant: &str, expert_id: &str) {
    retry_write("delete_favorite", || {
        open_experts_db().and_then(|conn| delete_favorite_conn(&conn, tenant, expert_id))
    });
}

/// 启动期加载全部租户的收藏集（tenant -> 该租户收藏的 expert_id 集合）。
pub fn load_all_favorites() -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_favorites", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT tenant_id, expert_id FROM favorites")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                let t: String = row.get(0)?;
                let e: String = row.get(1)?;
                Ok((t, e))
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for (t, e) in rows.into_iter().flatten() {
            out.entry(t).or_default().insert(e);
        }
    }
    out
}

// =====================================================================
// T4（v4）：事件轨迹日志（alliance_event_log）——事件总线消费者落库
// =====================================================================
//
// 进程内事件总线（experts_events）的消费者把每个真实业务事件（计划创建/状态推进/
// 专家注册·禁用）追加到此表。与审计链互补：审计是安全合规视角（SHA-256 哈希链 +
// 多 Sink），本表是**业务生命周期事件流视角**——前端免轮询、外部系统订阅联动、
// 可观测数据骨干（T4 §2.4）。写路径遵循本模块约定：best-effort（失败仅记日志，
// 不阻断业务），锁类错误走 retry_write。

/// 一条事件轨迹记录（读回投影）
#[derive(Debug, Clone)]
pub struct EventLogRow {
    pub tenant_id: String,
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub occurred_at: String,
    pub plan_id: String,
    pub payload: Value,
}

/// 追加一条事件日志（INSERT OR IGNORE：event_id 全局唯一，重复投递幂等）。
#[allow(clippy::too_many_arguments)]
pub fn insert_event_log_conn(
    conn: &Connection,
    tenant: &str,
    event_id: &str,
    event_type: &str,
    source: &str,
    occurred_at: &str,
    plan_id: &str,
    payload: &Value,
) -> Result<(), String> {
    let data = serde_json::to_string(payload)
        .map_err(|er| format!("序列化事件载荷 {event_id}: {er}"))?;
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    conn.execute(
        "INSERT OR IGNORE INTO alliance_event_log
            (tenant_id, event_id, event_type, source, occurred_at, plan_id, payload, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![tenant, event_id, event_type, source, occurred_at, plan_id, data, created_at],
    )
    .map_err(|e| format!("insert event_log {event_id}: {e}"))?;
    Ok(())
}

/// 追加一条事件日志落库（best-effort；A2：锁类错误走 busy 重试）
#[allow(clippy::too_many_arguments)]
pub fn insert_event_log(
    tenant: &str,
    event_id: &str,
    event_type: &str,
    source: &str,
    occurred_at: &str,
    plan_id: &str,
    payload: &Value,
) {
    retry_write("insert_event_log", || {
        open_experts_db().and_then(|conn| {
            insert_event_log_conn(
                &conn, tenant, event_id, event_type, source, occurred_at, plan_id, payload,
            )
        })
    });
}

/// 按租户读事件轨迹日志（按 occurred_at + rowid 升序，保持事件时序）。
///
/// 供 T4 E2E 断言与未来「事件流查询端点」使用；跨租户隔离（WHERE tenant_id=?）。
pub fn load_event_log_by_tenant(tenant: &str) -> Vec<EventLogRow> {
    let mut out = Vec::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_event_log_by_tenant", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare(
            "SELECT tenant_id, event_id, event_type, source, occurred_at, plan_id, payload
             FROM alliance_event_log WHERE tenant_id = ?1
             ORDER BY occurred_at ASC, rowid ASC",
        )
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| {
                let tenant_id: String = row.get(0)?;
                let event_id: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                let source: String = row.get(3)?;
                let occurred_at: String = row.get(4)?;
                let plan_id: String = row.get(5)?;
                let payload: String = row.get(6)?;
                let payload: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
                Ok(EventLogRow {
                    tenant_id,
                    event_id,
                    event_type,
                    source,
                    occurred_at,
                    plan_id,
                    payload,
                })
            }).map(|rows| rows.collect::<Vec<_>>())
        })
    {
        for r in rows.into_iter().flatten() {
            out.push(r);
        }
    }
    out
}

// =====================================================================
// v5（全维终验，2026-10-03）：webhook 外部订阅落盘（alliance_webhooks）
// =====================================================================
//
// T4 webhook 此前为进程内内存 HashMap（experts_events::WebhookTable），重启即失。
// 本表把订阅持久化为「写穿 + 启动读回」：CRUD handler 写后立即 upsert/delete 单行，
// 启动时 `load_all_webhooks` 一次读回重建内存注册表，重启后订阅与派发器自动恢复。
// 与 favorites/event_log 同约定：best-effort（失败仅 log_err 不阻断业务），锁类错误走
// retry_write。event_types 以 JSON 数组字符串存列（空数组 = 全收）。

/// 一条 webhook 订阅（读回投影；event_types 为 JSON 数组字符串，由调用方解析）
#[derive(Debug, Clone)]
pub struct WebhookRow {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub event_types: String,
    pub created_at: String,
}

/// upsert 一条 webhook 订阅（id 全局主键；重复登记同 id 覆盖更新）。
#[allow(clippy::too_many_arguments)]
pub fn upsert_webhook_conn(
    conn: &Connection,
    id: &str,
    tenant: &str,
    url: &str,
    event_types: &str,
    created_at: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO alliance_webhooks (id, tenant_id, url, event_types, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            tenant_id = excluded.tenant_id,
            url = excluded.url,
            event_types = excluded.event_types,
            created_at = excluded.created_at",
        params![id, tenant, url, event_types, created_at],
    )
    .map_err(|e| format!("upsert webhook {id}: {e}"))?;
    Ok(())
}

/// 落盘一条 webhook 订阅（best-effort；锁类错误走 busy 重试）
#[allow(clippy::too_many_arguments)]
pub fn upsert_webhook(id: &str, tenant: &str, url: &str, event_types: &str, created_at: &str) {
    retry_write("upsert_webhook", || {
        open_experts_db().and_then(|conn| {
            upsert_webhook_conn(&conn, id, tenant, url, event_types, created_at)
        })
    });
}

/// 删除一条 webhook 订阅（按 id + 租户，防跨租户删）。
pub fn delete_webhook_row_conn(conn: &Connection, id: &str, tenant: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM alliance_webhooks WHERE id = ?1 AND tenant_id = ?2",
        params![id, tenant],
    )
    .map_err(|e| format!("delete webhook {id}/{tenant}: {e}"))?;
    Ok(())
}

/// 删除一条 webhook 订阅落盘（best-effort；锁类错误走 busy 重试）
pub fn delete_webhook_row(id: &str, tenant: &str) {
    retry_write("delete_webhook_row", || {
        open_experts_db().and_then(|conn| delete_webhook_row_conn(&conn, id, tenant))
    });
}

/// 启动期加载全部 webhook 订阅（重建内存注册表用；失败返回空表，与既有 load_* 容错约定一致）。
pub fn load_all_webhooks() -> Vec<WebhookRow> {
    let mut out = Vec::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_all_webhooks", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT id, tenant_id, url, event_types, created_at FROM alliance_webhooks ORDER BY rowid ASC")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok(WebhookRow {
                    id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    url: row.get(2)?,
                    event_types: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for r in rows.into_iter().flatten() {
            out.push(r);
        }
    }
    out
}

// =====================================================================
// A2（无状态化阶段一，2026-10-02）：读路径实时查 SQLite
// =====================================================================
//
// D4 之前的语义是「内存为主 + SQLite 备份」：多副本各持内存副本，A 写 B 不重启看不到。
// A2 把 SQLite 立为这三项冷/低频数据的**唯一真相**：写已写穿（upsert/insert/delete 单条），
// 此处把读面也改为**每次请求实时查 SQLite（按租户）**，从而跨实例即一致——B 不重启、
// 不刷新本地缓存即可读到 A 刚写穿的行。本实例写后仍同步更新本地内存镜像（保持即时读与
// 既有 handler 形状），但读面权威来源是下面这些函数。
//
// 成本说明：plans / history / favorites 均为冷/低频读（统计、历史列表、收藏开关），
// 短连接 + SQLite 本地文件查询为亚毫秒级，单实例下与「读内存镜像」行为等价（写穿后
// SQLite 与本地镜像一致），故单实例零回归。高频态 registry/graph 仍保持进程内（见报告）。

/// 按租户实时读协作计划（plan_id -> plan）。
///
/// 读路径（orchestration_stats / history 过滤）改为调本函数：跨实例即一致，无需失效广播。
/// 失败返回空表（与既有 load_* 容错约定一致，单实例下退化为「本实例镜像」等价空读）。
pub fn load_plans_by_tenant(tenant: &str) -> HashMap<String, CollaborationPlan> {
    let mut map = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_plans_by_tenant", &e);
            return map;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM collaboration_plans WHERE tenant_id = ?1")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(p) = serde_json::from_str::<CollaborationPlan>(&s) {
                map.insert(p.plan_id.clone(), p);
            }
        }
    }
    map
}

/// 按租户实时统计计划数（全维终验 · DAG 计划配额计数口径，2026-10-03）。
///
/// A2 后 plans 已按 `(tenant_id, plan_id)` 复合键落 SQLite 且有 `idx_plans_tenant` 索引，
/// 故按租户计数为一次 O(index) 查询——这是「DAG 计划数配额」的真实可数基础
/// （A1 阶段二时 plans 还是全局扁平 HashMap、需全表扫描，故当时如实不做）。
/// 失败返回 0（容错约定：DB 不可用时按「无占用」放行，不阻断业务写路径）。
pub fn count_plans_by_tenant(tenant: &str) -> i64 {
    match open_experts_db() {
        Ok(conn) => conn
            .query_row(
                "SELECT COUNT(*) FROM collaboration_plans WHERE tenant_id = ?1",
                params![tenant],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0),
        Err(e) => {
            log_err("count_plans_by_tenant", &e);
            0
        }
    }
}

/// 按 (tenant, plan_id) 单点读计划（execute_plan_handler 读路径）。
///
/// 租户过滤下推到 SQL（`WHERE tenant_id=? AND plan_id=?`），跨租户访问自然 404，
/// 不依赖内存 plan.metadata["tenant_id"] 二次校验。
pub fn get_plan(tenant: &str, plan_id: &str) -> Option<CollaborationPlan> {
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("get_plan", &e);
            return None;
        }
    };
    conn.query_row(
        "SELECT data_json FROM collaboration_plans WHERE tenant_id = ?1 AND plan_id = ?2",
        params![tenant, plan_id],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| serde_json::from_str::<CollaborationPlan>(&s).ok())
}

/// 按租户实时读编排执行历史（按 created_at + rowid 升序，保持追加时序）。
///
/// history 表自带 `tenant_id` 列（D4 写入时即带租户），直接按租户查即可，
/// 等价于既有「按本租户可见 plan 集合过滤 history」语义，且跨实例即一致。
pub fn load_history_by_tenant(tenant: &str) -> Vec<OrchestrationRecord> {
    let mut out = Vec::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_history_by_tenant", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT data_json FROM orchestration_history WHERE tenant_id = ?1 ORDER BY created_at ASC, rowid ASC")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for s in rows.into_iter().flatten() {
            if let Ok(r) = serde_json::from_str::<OrchestrationRecord>(&s) {
                out.push(r);
            }
        }
    }
    out
}

/// 按租户实时读收藏的 expert_id 集合（favorite toggle 的唯一真相判定）。
///
/// toggle handler 据此判断当前是否已收藏（跨实例一致），再回写本地内存镜像。
pub fn load_favorites_by_tenant(tenant: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_favorites_by_tenant", &e);
            return out;
        }
    };
    if let Ok(rows) = conn
        .prepare("SELECT expert_id FROM favorites WHERE tenant_id = ?1")
        .and_then(|mut stmt| {
            stmt.query_map(params![tenant], |row| row.get::<_, String>(0))
                .map(|iter| iter.collect::<Result<Vec<_>, _>>())
        })
    {
        for e in rows.into_iter().flatten() {
            out.insert(e);
        }
    }
    out
}

// =====================================================================
// 历史 JSON → SQLite 一次性迁移（启动期调用，幂等）
// =====================================================================

/// 迁移报告
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    /// 导入的专家数
    pub registry: usize,
    /// 导入的会话数
    pub sessions: usize,
    /// 导入的图谱节点数
    pub graph_nodes: usize,
    /// 导入的图谱边数
    pub graph_edges: usize,
    /// 导入的预约数
    pub bookings: usize,
    /// 归档（改名）后的 JSON 文件路径
    pub archived: Vec<String>,
}

impl MigrationReport {
    /// 导入的记录总数
    pub fn total_imported(&self) -> usize {
        self.registry + self.sessions + self.graph_nodes + self.graph_edges + self.bookings
    }
    /// 是否为无操作（无导入、无归档）
    pub fn is_noop(&self) -> bool {
        self.total_imported() == 0 && self.archived.is_empty()
    }
}

/// 导入成功后把历史 JSON 改名归档（`<原名>.json.migrated-<unix 时间戳>`）
fn archive_json(path: &str, archived: &mut Vec<String>) {
    let target = format!("{}.migrated-{}", path, chrono::Utc::now().timestamp());
    match std::fs::rename(path, &target) {
        Ok(_) => archived.push(target),
        Err(e) => log_err("归档 JSON", &format!("{} → {}: {}", path, target, e)),
    }
}

/// 启动期一次性迁移：历史 JSON → SQLite（幂等）
///
/// 规则：
/// - JSON 文件与数据库同目录（默认 `data/`，与历史路径完全兼容）
/// - 仅当对应表为空时导入（SQLite 已有数据视为权威，跳过导入）
/// - 导入成功（事务提交）后才改名归档 JSON；解析失败则保留原文件不动
/// - 幂等：迁移后 JSON 已归档，再次调用为 noop
pub fn migrate_json_to_sqlite() -> MigrationReport {
    let mut report = MigrationReport::default();

    // 1) 专家注册表
    let path = json_path(JSON_REGISTRY_FILE);
    if let Ok(content) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<HashMap<String, ExpertDescriptor>>(&content) {
            Ok(map) if !map.is_empty() => match open_experts_db().and_then(|conn| {
                if table_count(&conn, "experts")? > 0 {
                    return Ok(false);
                }
                save_registry_conn(&conn, "default", &map)?;
                Ok(true)
            }) {
                Ok(true) => {
                    report.registry = map.len();
                    archive_json(&path, &mut report.archived);
                }
                Ok(false) => {} // SQLite 已有数据，跳过导入且不归档
                Err(e) => log_err("迁移 registry", &e),
            },
            Ok(_) => {} // 空文件：不导入不归档
            Err(e) => log_err("迁移 registry（JSON 解析失败，保留原文件）", &e.to_string()),
        }
    }

    // 2) 会话
    let path = json_path(JSON_SESSIONS_FILE);
    if let Ok(content) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<HashMap<String, ExpertSession>>(&content) {
            Ok(map) if !map.is_empty() => match open_experts_db().and_then(|conn| {
                if table_count(&conn, "sessions")? > 0 {
                    return Ok(false);
                }
                save_sessions_conn(&conn, &map)?;
                Ok(true)
            }) {
                Ok(true) => {
                    report.sessions = map.len();
                    archive_json(&path, &mut report.archived);
                }
                Ok(false) => {}
                Err(e) => log_err("迁移 sessions", &e),
            },
            Ok(_) => {}
            Err(e) => log_err("迁移 sessions（JSON 解析失败，保留原文件）", &e.to_string()),
        }
    }

    // 3) 能力图谱
    let path = json_path(JSON_GRAPH_FILE);
    if let Ok(content) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<ExpertGraph>(&content) {
            Ok(g) if !g.nodes.is_empty() || !g.edges.is_empty() => {
                match open_experts_db().and_then(|conn| {
                    if table_count(&conn, "graph_nodes")? > 0 {
                        return Ok(false);
                    }
                    save_graph_conn(&conn, "default", &g)?;
                    Ok(true)
                }) {
                    Ok(true) => {
                        report.graph_nodes = g.nodes.len();
                        report.graph_edges = g.edges.len();
                        archive_json(&path, &mut report.archived);
                    }
                    Ok(false) => {}
                    Err(e) => log_err("迁移 graph", &e),
                }
            }
            Ok(_) => {}
            Err(e) => log_err("迁移 graph（JSON 解析失败，保留原文件）", &e.to_string()),
        }
    }

    // 4) 专家广场预约
    let path = json_path(JSON_BOOKINGS_FILE);
    if let Ok(content) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<Vec<Value>>(&content) {
            Ok(rows) if !rows.is_empty() => match open_experts_db().and_then(|conn| {
                if table_count(&conn, "bookings")? > 0 {
                    return Ok(false);
                }
                save_bookings_conn(&conn, &rows)?;
                Ok(true)
            }) {
                Ok(true) => {
                    report.bookings = rows.len();
                    archive_json(&path, &mut report.archived);
                }
                Ok(false) => {}
                Err(e) => log_err("迁移 bookings", &e),
            },
            Ok(_) => {}
            Err(e) => log_err("迁移 bookings（JSON 解析失败，保留原文件）", &e.to_string()),
        }
    }

    if !report.is_noop() {
        eprintln!(
            "[experts_db] JSON→SQLite 迁移完成: experts={} sessions={} graph(nodes/edges)={}/{} bookings={} 归档文件={}",
            report.registry,
            report.sessions,
            report.graph_nodes,
            report.graph_edges,
            report.bookings,
            report.archived.len(),
        );
    }
    report
}

/// 检查数据库完整性（运维/自检用）：返回 `PRAGMA integrity_check` 结果
pub fn integrity_check() -> Result<String, String> {
    let conn = open_experts_db()?;
    conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())
}

// =====================================================================
// 增量写（N4）单测：内存 SQLite，全闭环验证 UPSERT / 级联 / 边重排
// =====================================================================

#[cfg(test)]
mod incremental_tests {
    use super::*;
    use std::collections::HashMap;

    /// 新建内存库并建表（不读 env、不落盘）
    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn node(id: &str, t: &str) -> GraphNode {
        GraphNode { id: id.into(), label: id.into(), node_type: t.into(), properties: HashMap::new() }
    }

    fn edge(source: &str, target: &str, w: f64) -> GraphEdge {
        GraphEdge {
            source: source.into(),
            target: target.into(),
            edge_type: "has_domain".into(),
            weight: w,
            properties: HashMap::new(),
        }
    }

    #[test]
    fn upsert_node_insert_then_update() {
        let conn = mem_conn();
        upsert_graph_node_conn(&conn, "default", &node("a", "expert")).unwrap();
        upsert_graph_node_conn(&conn, "default", &node("b", "domain")).unwrap();
        // 再次 upsert 同 id 应覆盖而非冲突
        let mut n = node("a", "expert");
        n.label = "改名".into();
        upsert_graph_node_conn(&conn, "default", &n).unwrap();
        let cnt: i64 = conn.query_row("SELECT COUNT(*) FROM graph_nodes", [], |r| r.get(0)).unwrap();
        assert_eq!(cnt, 2);
        let label: String = conn.query_row(
            "SELECT label FROM graph_nodes WHERE id='a'", [], |r| r.get(0)).unwrap();
        assert_eq!(label, "改名");
    }

    #[test]
    fn delete_node_cascades_edges() {
        let conn = mem_conn();
        upsert_graph_node_conn(&conn, "default", &node("a", "expert")).unwrap();
        upsert_graph_node_conn(&conn, "default", &node("b", "domain")).unwrap();
        upsert_graph_node_conn(&conn, "default", &node("c", "expert")).unwrap();
        replace_graph_edges_conn(&conn, "default", &[edge("a", "b", 1.0), edge("a", "c", 0.5)]).unwrap();
        delete_graph_node_cascade_conn(&conn, "default", "a").unwrap();
        let nodes: i64 = conn.query_row("SELECT COUNT(*) FROM graph_nodes", [], |r| r.get(0)).unwrap();
        let edges: i64 = conn.query_row("SELECT COUNT(*) FROM graph_edges", [], |r| r.get(0)).unwrap();
        assert_eq!(nodes, 2);
        assert_eq!(edges, 0); // a 的两条边都被级联删除
    }

    #[test]
    fn edge_upsert_then_renumber_keeps_seq_order() {
        let conn = mem_conn();
        upsert_graph_node_conn(&conn, "default", &node("a", "expert")).unwrap();
        upsert_graph_node_conn(&conn, "default", &node("b", "domain")).unwrap();
        upsert_graph_node_conn(&conn, "default", &node("c", "expert")).unwrap();
        // 三条边，seq 0/1/2
        replace_graph_edges_conn(&conn, "default", &[edge("a", "b", 1.0), edge("b", "c", 0.4), edge("a", "c", 0.9)]).unwrap();
        // 更新 seq=1 的边
        let mut e = edge("b", "c", 0.4);
        e.weight = 0.7;
        upsert_graph_edge_conn(&conn, "default", 1, &e).unwrap();
        // 删除中间边（b-c），剩余两条重排为 seq 0/1
        replace_graph_edges_conn(&conn, "default", &[edge("a", "b", 1.0), edge("a", "c", 0.9)]).unwrap();
        let rows: Vec<(i64, String, String)> = conn
            .prepare("SELECT seq, source, target FROM graph_edges ORDER BY seq")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (0, "a".into(), "b".into()));
        assert_eq!(rows[1], (1, "a".into(), "c".into()));
    }

    #[test]
    fn bump_meta_upsert() {
        let conn = mem_conn();
        set_graph_meta_conn(&conn, "default", 3, "2026-09-30T00:00:00Z").unwrap();
        set_graph_meta_conn(&conn, "default", 4, "2026-09-30T01:00:00Z").unwrap();
        let v: String = conn.query_row("SELECT v FROM graph_meta WHERE k='version'", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "4");
    }
}
