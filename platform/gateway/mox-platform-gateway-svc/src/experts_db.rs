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

use crate::experts_common::{ExpertDescriptor, ExpertGraph, ExpertSession, GraphEdge, GraphNode};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::collections::HashMap;
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
    Ok(conn)
}

/// 幂等建表（列投影 + JSON 文档混合建模）
fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        -- 专家注册表：热查询字段建列，完整描述符存 data_json
        CREATE TABLE IF NOT EXISTS experts (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL DEFAULT '',
            title        TEXT NOT NULL DEFAULT '',
            organization TEXT NOT NULL DEFAULT '',
            expert_type  TEXT NOT NULL DEFAULT 'ai',
            status       TEXT NOT NULL DEFAULT 'online',
            enabled      INTEGER NOT NULL DEFAULT 1,
            avg_rating   REAL NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT '',
            updated_at   TEXT NOT NULL DEFAULT '',
            data_json    TEXT NOT NULL
        );
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

        -- 能力图谱：节点/边投影 + 元信息
        CREATE TABLE IF NOT EXISTS graph_nodes (
            id        TEXT PRIMARY KEY,
            label     TEXT NOT NULL DEFAULT '',
            node_type TEXT NOT NULL DEFAULT '',
            data_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS graph_edges (
            seq       INTEGER PRIMARY KEY,
            source    TEXT NOT NULL DEFAULT '',
            target    TEXT NOT NULL DEFAULT '',
            edge_type TEXT NOT NULL DEFAULT '',
            weight    REAL NOT NULL DEFAULT 0,
            data_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS graph_meta (
            k TEXT PRIMARY KEY,
            v TEXT NOT NULL
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
        "#,
    )
    .map_err(|e| format!("初始化 schema 失败: {}", e))
}

fn log_err(op: &str, err: &str) {
    eprintln!("[experts_db] {} 失败: {}", op, err);
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
    registry: &HashMap<String, ExpertDescriptor>,
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM experts", [])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO experts (id, name, title, organization, expert_type, status, enabled, avg_rating, created_at, updated_at, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .map_err(|e| e.to_string())?;
        // 按 id 排序写入，保证落库顺序确定（可复现/可对比）
        let mut exps: Vec<&ExpertDescriptor> = registry.values().collect();
        exps.sort_by(|a, b| a.id.cmp(&b.id));
        for e in exps {
            let data =
                serde_json::to_string(e).map_err(|er| format!("序列化 expert {}: {}", e.id, er))?;
            stmt.execute(params![
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

/// 全量同步专家注册表到 SQLite（单事务；失败仅记录不阻断）
pub fn save_registry(registry: &HashMap<String, ExpertDescriptor>) {
    let res = open_experts_db().and_then(|conn| save_registry_conn(&conn, registry));
    if let Err(e) = res {
        log_err("save_registry", &e);
    }
}

/// 从 SQLite 加载专家注册表（失败返回空表，与历史 JSON 行为一致）
pub fn load_registry() -> HashMap<String, ExpertDescriptor> {
    let mut map = HashMap::new();
    let conn = match open_experts_db() {
        Ok(c) => c,
        Err(e) => {
            log_err("load_registry", &e);
            return map;
        }
    };
    let rows = conn
        .prepare("SELECT data_json FROM experts")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
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
    let res = open_experts_db().and_then(|conn| save_sessions_conn(&conn, sessions));
    if let Err(e) = res {
        log_err("save_sessions", &e);
    }
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

fn save_graph_conn(conn: &Connection, graph: &ExpertGraph) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_nodes", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_edges", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM graph_meta", [])
        .map_err(|e| e.to_string())?;
    {
        let mut node_stmt = tx
            .prepare(
                "INSERT INTO graph_nodes (id, label, node_type, data_json) VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(|e| e.to_string())?;
        let mut nodes: Vec<&GraphNode> = graph.nodes.iter().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        for n in nodes {
            let data =
                serde_json::to_string(n).map_err(|er| format!("序列化 node {}: {}", n.id, er))?;
            node_stmt
                .execute(params![n.id, n.label, n.node_type, data])
                .map_err(|er| format!("insert node {}: {}", n.id, er))?;
        }

        let mut edge_stmt = tx
            .prepare(
                "INSERT INTO graph_edges (seq, source, target, edge_type, weight, data_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| e.to_string())?;
        for (i, e) in graph.edges.iter().enumerate() {
            let data = serde_json::to_string(e)
                .map_err(|er| format!("序列化 edge #{}: {}", i, er))?;
            edge_stmt
                .execute(params![i as i64, e.source, e.target, e.edge_type, e.weight, data])
                .map_err(|er| format!("insert edge #{}: {}", i, er))?;
        }

        tx.execute(
            "INSERT INTO graph_meta (k, v) VALUES ('built_at', ?1), ('version', ?2)",
            params![graph.built_at, graph.version.to_string()],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// 全量同步能力图谱到 SQLite（单事务）
pub fn save_graph(graph: &ExpertGraph) {
    let res = open_experts_db().and_then(|conn| save_graph_conn(&conn, graph));
    if let Err(e) = res {
        log_err("save_graph", &e);
    }
}

/// 从 SQLite 加载能力图谱（失败返回默认空图谱）
pub fn load_graph() -> ExpertGraph {
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
        .prepare("SELECT data_json FROM graph_nodes ORDER BY id")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
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
        .prepare("SELECT data_json FROM graph_edges ORDER BY seq")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))
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
    if let Ok(v) = conn.query_row("SELECT v FROM graph_meta WHERE k = 'built_at'", [], |r| {
        r.get::<_, String>(0)
    }) {
        built_at = v;
    }
    if let Ok(v) = conn.query_row("SELECT v FROM graph_meta WHERE k = 'version'", [], |r| {
        r.get::<_, String>(0)
    }) {
        version = v.parse().unwrap_or(0);
    }
    ExpertGraph {
        nodes,
        edges,
        built_at,
        version,
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
                save_registry_conn(&conn, &map)?;
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
                    save_graph_conn(&conn, &g)?;
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
