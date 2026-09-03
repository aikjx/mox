//! M3 持久化层：SQLite（rusqlite bundled）为专家注册表 / 会话 / 计划 / 调度配置提供
//! 重启后数据保留 + 幂等 seed。
//!
//! 表结构：
//! - `experts(id, meta_json, updated_at)`：注册/更新专家（内置专家首次 seed）
//! - `sessions(id, data_json, updated_at)`：专家会话
//! - `plans(id, data_json, updated_at)`：生成的计划（plan_execute 依赖）
//! - `kv(key, value_json, updated_at)`：调度配置等键值
//!
//! 幂等 seed：`kv['mox_experts_seeded']=1` 标记已 seed；首次建库写内置专家，
//! 之后重启即使专家表被删空也不重复 seed（保证"重复 seed 0 新增"语义）。

use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde_json::Value;

use crate::types::ExpertMeta;

/// SQLite 持久化句柄（内部 Mutex 保护连接，同步调用、不跨 await 持锁）。
pub struct PersistenceDb {
    conn: Mutex<Connection>,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl PersistenceDb {
    /// 打开/创建数据库并建表。`path` 若含目录且不存在则自动创建。
    pub fn open(path: &str) -> anyhow::Result<Self> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS experts (
                 id         TEXT PRIMARY KEY,
                 meta_json  TEXT NOT NULL,
                 updated_at TEXT
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 id         TEXT PRIMARY KEY,
                 data_json  TEXT NOT NULL,
                 updated_at TEXT
             );
             CREATE TABLE IF NOT EXISTS plans (
                 id         TEXT PRIMARY KEY,
                 data_json  TEXT NOT NULL,
                 updated_at TEXT
             );
             CREATE TABLE IF NOT EXISTS kv (
                 key         TEXT PRIMARY KEY,
                 value_json  TEXT NOT NULL,
                 updated_at  TEXT
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ---------------- experts ----------------

    pub fn upsert_expert(&self, meta: &ExpertMeta) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        let j = serde_json::to_string(meta)?;
        c.execute(
            "INSERT INTO experts(id, meta_json, updated_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET meta_json = excluded.meta_json, updated_at = excluded.updated_at",
            params![meta.id, j, now()],
        )?;
        Ok(())
    }

    pub fn delete_expert(&self, id: &str) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM experts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn load_experts(&self) -> Vec<ExpertMeta> {
        let c = self.conn.lock().unwrap();
        let mut stmt = match c.prepare("SELECT meta_json FROM experts") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |r| r.get::<_, String>(0)) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|j| serde_json::from_str::<ExpertMeta>(&j).ok())
            .collect()
    }

    // ---------------- sessions ----------------

    pub fn upsert_session(&self, id: &str, data: &Value) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        let j = serde_json::to_string(data)?;
        c.execute(
            "INSERT INTO sessions(id, data_json, updated_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET data_json = excluded.data_json, updated_at = excluded.updated_at",
            params![id, j, now()],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn load_sessions(&self) -> Vec<(String, Value)> {
        self.load_json_rows("sessions", "id", "data_json")
    }

    // ---------------- plans ----------------

    pub fn upsert_plan(&self, id: &str, data: &Value) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        let j = serde_json::to_string(data)?;
        c.execute(
            "INSERT INTO plans(id, data_json, updated_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET data_json = excluded.data_json, updated_at = excluded.updated_at",
            params![id, j, now()],
        )?;
        Ok(())
    }

    pub fn load_plans(&self) -> Vec<(String, Value)> {
        self.load_json_rows("plans", "id", "data_json")
    }

    // ---------------- kv ----------------

    pub fn delete_kv(&self, key: &str) {
        if let Ok(guard) = self.conn.lock() {
            let _ = guard.execute("DELETE FROM kv WHERE key = ?1", rusqlite::params![key]);
        }
    }

    pub fn save_kv(&self, key: &str, v: &Value) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        let j = serde_json::to_string(v)?;
        c.execute(
            "INSERT INTO kv(key, value_json, updated_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![key, j, now()],
        )?;
        Ok(())
    }

    pub fn load_kv(&self, key: &str) -> Option<Value> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT value_json FROM kv WHERE key = ?1").ok()?;
        let mut rows = stmt.query_map(params![key], |r| r.get::<_, String>(0)).ok()?;
        rows.next()?
            .ok()
            .and_then(|j| serde_json::from_str(&j).ok())
    }

    pub fn kv_exists(&self, key: &str) -> bool {
        self.load_kv(key).is_some()
    }

    /// M4：按前缀加载 kv（如 `metrics:`），返回 (key, Value)
    pub fn load_kv_prefix(&self, prefix: &str) -> Vec<(String, Value)> {
        let c = self.conn.lock().unwrap();
        let mut stmt = match c.prepare("SELECT key, value_json FROM kv WHERE key LIKE ?1") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let pat = format!("{}%", prefix);
        let rows = match stmt.query_map(params![pat], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|(k, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (k, v)))
            .collect()
    }

    // ---------------- generic ----------------

    fn load_json_rows(&self, table: &str, id_col: &str, val_col: &str) -> Vec<(String, Value)> {
        let c = self.conn.lock().unwrap();
        let sql = format!("SELECT {}, {} FROM {}", id_col, val_col, table);
        let mut stmt = match c.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_kv_prefix_roundtrip() {
        let db = PersistenceDb::open(":memory:").unwrap();
        db.save_kv("metrics:alpha", &json!({"consultations": 2, "rating_sum": 1.8, "latency_sum": 5})).unwrap();
        db.save_kv("dispatcher_cfg", &json!({"x": 1})).unwrap();
        let rows = db.load_kv_prefix("metrics:");
        assert_eq!(rows.len(), 1, "expected 1 metrics row, got {:?}", rows);
        assert_eq!(rows[0].0, "metrics:alpha");
        assert_eq!(rows[0].1["consultations"], 2);
    }

    #[test]
    fn test_kv_prefix_file_reopen() {
        // 复现 M4：文件库写入 → 关闭 → 重新打开 → load_kv_prefix 应仍能读到
        let path = std::env::temp_dir()
            .join("mox_m4_prefix_test.db")
            .to_string_lossy()
            .to_string();
        let _ = std::fs::remove_file(&path);
        {
            let db = PersistenceDb::open(&path).unwrap();
            db.save_kv("metrics:algorithm", &json!({"consultations": 2, "rating_sum": 2.0, "latency_sum": 0}))
                .unwrap();
            db.save_kv("dispatcher_cfg", &json!({"strategy": "multi-consult"})).unwrap();
            let rows = db.load_kv_prefix("metrics:");
            assert_eq!(rows.len(), 1, "same-conn load_kv_prefix failed: {:?}", rows);
        } // close
        let db2 = PersistenceDb::open(&path).unwrap();
        let rows2 = db2.load_kv_prefix("metrics:");
        let disp = db2.load_kv("dispatcher_cfg");
        assert!(disp.is_some(), "load_kv after reopen failed");
        assert_eq!(rows2.len(), 1, "reopen load_kv_prefix failed: {:?}", rows2);
        assert_eq!(rows2[0].0, "metrics:algorithm");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }
}

