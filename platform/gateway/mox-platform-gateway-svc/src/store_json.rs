// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 通用 JSON 集合 SQLite 持久化（归一化层）
//!
//! 替代 monitor / notification / workspace / kb_ext / misc / projects_ext
//! 各模块散落的 `data/*.json` 文件读写，统一收敛到 `data/store.db`，
//! 与专家联盟域 [`crate::experts_db`] 的持久化风格保持一致：
//!
//! - **WAL 模式** + **busy_timeout(5s)** + **synchronous=NORMAL**：读写并发、
//!   跨连接写竞争自动等待、崩溃原子（事务内全量同步）。
//! - **集合模型**：每个模块的持久化数据视为一个「集合」（`Vec<T>`），以
//!   `name`（如 `monitor.alert_rules`）为键存入 `collections` 表，整集以
//!   `data_json` 文档形式存储——结构演进零 DDL 迁移成本。
//! - **自动迁移**：首次启动时若对应旧 JSON 文件存在且 SQLite 为空，则导入
//!   并改名归档（`*.json.migrated-<ts>`）。
//!
//! 容错策略与原 JSON 持久化一致：持久化失败仅 `eprintln!` 不阻断业务，
//! 内存态仍是权威数据源，SQLite 为持久投影。

use rusqlite::{Connection, params};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;

/// 数据库路径环境变量（测试/部署隔离用）
pub const ENV_STORE_DB_PATH: &str = "MOX_STORE_DB_PATH";
/// 默认数据库路径（相对 cwd，与 data/ 约定一致）
pub const DEFAULT_STORE_DB_PATH: &str = "data/store.db";
/// 跨连接写锁等待超时（毫秒）
const BUSY_TIMEOUT_MS: u64 = 5000;

/// 解析当前数据库路径（每次调用读取，保证测试/运行时可覆盖）
pub fn store_db_path() -> String {
    std::env::var(ENV_STORE_DB_PATH).unwrap_or_else(|_| DEFAULT_STORE_DB_PATH.to_string())
}

/// 打开数据库连接并初始化 schema（幂等）。短连接 + WAL + busy_timeout。
pub fn open_store_db() -> Result<Connection, String> {
    let path = store_db_path();
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

/// 幂等建表
fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS collections (
            name       TEXT PRIMARY KEY,
            data_json  TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT ''
        );",
    )
    .map_err(|e| format!("建表 collections 失败: {}", e))
}

/// 读取某集合（name 如 `monitor.alert_rules`），返回 `Vec<T>`。
pub fn load_collection<T: DeserializeOwned>(name: &str) -> Vec<T> {
    let conn = match open_store_db() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[store_json] 打开失败 {}: {}", name, e);
            return Vec::new();
        }
    };
    match conn.query_row(
        "SELECT data_json FROM collections WHERE name = ?1",
        params![name],
        |row| row.get::<_, String>(0),
    ) {
        Ok(json_str) => serde_json::from_str::<Vec<T>>(&json_str).unwrap_or_default(),
        Err(rusqlite::Error::QueryReturnedNoRows) => Vec::new(),
        Err(e) => {
            eprintln!("[store_json] 读取 {} 失败: {}", name, e);
            Vec::new()
        }
    }
}

/// 事务内全量覆写某集合（Vec<T> → data_json）。
pub fn save_collection<T: Serialize>(name: &str, items: &[T]) -> Result<(), String> {
    let json_str = serde_json::to_string(items).map_err(|e| e.to_string())?;
    let mut conn = open_store_db()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO collections(name, data_json, updated_at) VALUES(?1, ?2, datetime('now'))
         ON CONFLICT(name) DO UPDATE SET data_json = excluded.data_json, updated_at = datetime('now')",
        params![name, json_str],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// 通用迁移：若 SQLite 集合为空且旧 JSON 文件存在，则导入（Vec<T>）并归档。
/// 用于「单集合 = 一个 JSON 数组」的模块（monitor/notification/workspace/kb_ext）。
pub fn try_migrate_json<T: DeserializeOwned + Serialize>(name: &str, json_path: &str) -> Vec<T> {
    let existing = load_collection::<T>(name);
    if !existing.is_empty() {
        return existing;
    }
    if let Ok(content) = std::fs::read_to_string(json_path) {
        if let Ok(items) = serde_json::from_str::<Vec<T>>(&content) {
            if !items.is_empty() {
                if save_collection(name, &items).is_ok() {
                    archive_json(json_path);
                }
                return items;
            }
        }
    }
    existing
}

/// 将已迁移的 JSON 文件改名归档（`.migrated-<unix_ts>`），避免重复迁移。
pub fn archive_json(json_path: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = std::fs::rename(json_path, format!("{}.migrated-{}", json_path, ts));
}
