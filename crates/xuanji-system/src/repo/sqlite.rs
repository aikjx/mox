//! SQLite 后端实现（沿用原 `rusqlite` 单连接 + 写穿模型）

use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::model::*;
use crate::rbac::RoleBinding;
use crate::store::{id_of, State};

use super::Repository;

pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    /// 打开（或创建）SQLite 库。
    pub fn open(path: &str) -> Result<Self, String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("无法创建数据目录 {:?}: {}", parent, e))?;
            }
        }
        let conn = Connection::open(path).map_err(|e| format!("SQLite 打开失败: {}", e))?;
        Ok(Self { conn })
    }

    fn exec<P: rusqlite::Params>(&self, sql: &str, p: P) {
        if let Err(e) = self.conn.execute(sql, p) {
            tracing::error!("sqlite 写穿失败 [{}]: {}", sql, e);
        }
    }

    fn load_into<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(
        &self,
        map: &mut HashMap<String, T>,
        sql: &str,
    ) {
        if let Ok(mut stmt) = self.conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for r in rows.flatten() {
                    if let Ok(v) = serde_json::from_str::<T>(&r) {
                        if let Some(key) = id_of::<T>(&v) {
                            map.insert(key, v);
                        }
                    }
                }
            }
        }
    }
}

impl Repository for SqliteRepository {
    fn migrate(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS xuanjis (id TEXT PRIMARY KEY, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS members (id TEXT PRIMARY KEY, xuanji_id TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, xuanji_id TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS channels (id TEXT PRIMARY KEY, xuanji_id TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, channel_id TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS notifications (id TEXT PRIMARY KEY, member_id TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS bindings (member_id TEXT PRIMARY KEY, data TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS tokens (hash TEXT PRIMARY KEY, member_id TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS audit (seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL, data TEXT NOT NULL, at INTEGER NOT NULL);",
            )
            .map_err(|e| format!("建表失败: {}", e))
    }

    fn load_all(&self) -> State {
        let mut st = State::default();
        if let Ok(mut stmt) = self.conn.prepare("SELECT data FROM xuanjis") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for r in rows.flatten() {
                    if let Ok(v) = serde_json::from_str::<Xuanji>(&r) {
                        st.xuanjis.insert(v.id.clone(), v);
                    }
                }
            }
        }
        self.load_into(&mut st.members, "SELECT data FROM members");
        self.load_into(&mut st.tasks, "SELECT data FROM tasks");
        self.load_into(&mut st.channels, "SELECT data FROM channels");
        self.load_into(&mut st.messages, "SELECT data FROM messages");
        self.load_into(&mut st.notifications, "SELECT data FROM notifications");

        if let Ok(mut stmt) = self.conn.prepare("SELECT member_id, data FROM bindings") {
            if let Ok(rows) =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            {
                for r in rows.flatten() {
                    if let Ok(binds) = serde_json::from_str::<Vec<RoleBinding>>(&r.1) {
                        st.bindings.insert(r.0, binds);
                    }
                }
            }
        }
        if let Ok(mut stmt) = self.conn.prepare("SELECT hash, member_id FROM tokens") {
            if let Ok(rows) =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            {
                for r in rows.flatten() {
                    st.tokens.insert(r.0, r.1);
                }
            }
        }
        if let Ok(mut stmt) = self.conn.prepare("SELECT data FROM audit ORDER BY seq") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for r in rows.flatten() {
                    if let Ok(v) = serde_json::from_str::<AuditRecord>(&r) {
                        st.audit.push(v);
                    }
                }
            }
        }
        st
    }

    fn persist_xuanji(&self, a: &Xuanji) {
        self.exec(
            "INSERT OR REPLACE INTO xuanjis (id, data) VALUES (?1, ?2)",
            params![a.id, serde_json::to_string(a).unwrap_or_default()],
        );
    }
    fn persist_member(&self, m: &Member) {
        self.exec(
            "INSERT OR REPLACE INTO members (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![m.id, m.xuanji_id, serde_json::to_string(m).unwrap_or_default()],
        );
    }
    fn persist_task(&self, t: &Task) {
        self.exec(
            "INSERT OR REPLACE INTO tasks (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![t.id, t.xuanji_id, serde_json::to_string(t).unwrap_or_default()],
        );
    }
    fn persist_channel(&self, c: &Channel) {
        self.exec(
            "INSERT OR REPLACE INTO channels (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![c.id, c.xuanji_id, serde_json::to_string(c).unwrap_or_default()],
        );
    }
    fn persist_message(&self, m: &Message) {
        self.exec(
            "INSERT OR REPLACE INTO messages (id, channel_id, data) VALUES (?1, ?2, ?3)",
            params![m.id, m.channel_id, serde_json::to_string(m).unwrap_or_default()],
        );
    }
    fn persist_notification(&self, n: &Notification) {
        self.exec(
            "INSERT OR REPLACE INTO notifications (id, member_id, data) VALUES (?1, ?2, ?3)",
            params![n.id, n.member_id, serde_json::to_string(n).unwrap_or_default()],
        );
    }
    fn persist_bindings(&self, member_id: &str, bindings: &[RoleBinding]) {
        self.exec(
            "INSERT OR REPLACE INTO bindings (member_id, data) VALUES (?1, ?2)",
            params![member_id, serde_json::to_string(bindings).unwrap_or_default()],
        );
    }
    fn persist_token(&self, hash: &str, member_id: &str) {
        self.exec(
            "INSERT OR REPLACE INTO tokens (hash, member_id) VALUES (?1, ?2)",
            params![hash, member_id],
        );
    }
    fn persist_audit(&self, r: &AuditRecord) {
        self.exec(
            "INSERT INTO audit (id, data, at) VALUES (?1, ?2, ?3)",
            params![
                r.id,
                serde_json::to_string(r).unwrap_or_default(),
                r.at.timestamp()
            ],
        );
    }
}
