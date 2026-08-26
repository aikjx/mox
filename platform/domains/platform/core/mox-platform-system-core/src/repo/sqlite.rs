//! SQLite 后端实现（沿用原 `rusqlite` 单连接 + 写穿模型）

use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::model::*;
use crate::rbac::RoleBinding;
use crate::store::{id_of, State};

use super::Repository;

pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

impl SqliteRepository {
    /// 打开（或创建）SQLite 库。
    pub fn open(path: &str) -> Result<Self, String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("无法创建数据目录 {:?}: {}", parent, e))?;
            }
        }
        let conn = Connection::open(path).map_err(|e| format!("SQLite 打开失败: {}", e))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn exec<P: rusqlite::Params>(&self, sql: &str, p: P) {
        if let Ok(conn) = self.conn.lock() {
            if let Err(e) = conn.execute(sql, p) {
                tracing::error!("sqlite 写穿失败 [{}]: {}", sql, e);
            }
        }
    }

    fn load_into<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(
        conn: &Connection,
        map: &mut HashMap<String, T>,
        sql: &str,
    ) {
        if let Ok(mut stmt) = conn.prepare(sql) {
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

#[async_trait::async_trait]
impl Repository for SqliteRepository {
    async fn migrate(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "sqlite 连接锁 poisoned".to_string())?;
        // 委托给方言层生成建表 SQL（当前后端为 Sqlite）
        for sql in crate::repo::schema::create_tables_sql(crate::config::Backend::Sqlite) {
            conn.execute_batch(&sql)
                .map_err(|e| format!("建表失败 [{}]: {}", sql, e))?;
        }
        Ok(())
    }

    async fn load_all(&self) -> State {
        let mut st = State::default();
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return st,
        };
        if let Ok(mut stmt) = conn.prepare("SELECT data FROM moxs") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for r in rows.flatten() {
                    if let Ok(v) = serde_json::from_str::<Mox>(&r) {
                        st.moxs.insert(v.id.clone(), v);
                    }
                }
            }
        }
        Self::load_into(&conn, &mut st.members, "SELECT data FROM members");
        Self::load_into(&conn, &mut st.tasks, "SELECT data FROM tasks");
        Self::load_into(&conn, &mut st.channels, "SELECT data FROM channels");
        Self::load_into(&conn, &mut st.messages, "SELECT data FROM messages");
        Self::load_into(
            &conn,
            &mut st.notifications,
            "SELECT data FROM notifications",
        );

        if let Ok(mut stmt) = conn.prepare("SELECT member_id, data FROM bindings") {
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
        if let Ok(mut stmt) = conn.prepare("SELECT hash, member_id FROM tokens") {
            if let Ok(rows) =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            {
                for r in rows.flatten() {
                    st.tokens.insert(r.0, r.1);
                }
            }
        }
        if let Ok(mut stmt) = conn.prepare("SELECT data FROM audit ORDER BY seq") {
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

    async fn persist_mox(&self, a: &Mox) {
        self.exec(
            "INSERT OR REPLACE INTO moxs (id, data) VALUES (?1, ?2)",
            params![a.id, serde_json::to_string(a).unwrap_or_default()],
        );
    }
    async fn persist_member(&self, m: &Member) {
        self.exec(
            "INSERT OR REPLACE INTO members (id, mox_id, data) VALUES (?1, ?2, ?3)",
            params![m.id, m.mox_id, serde_json::to_string(m).unwrap_or_default()],
        );
    }
    async fn persist_task(&self, t: &Task) {
        self.exec(
            "INSERT OR REPLACE INTO tasks (id, mox_id, data) VALUES (?1, ?2, ?3)",
            params![t.id, t.mox_id, serde_json::to_string(t).unwrap_or_default()],
        );
    }
    async fn persist_channel(&self, c: &Channel) {
        self.exec(
            "INSERT OR REPLACE INTO channels (id, mox_id, data) VALUES (?1, ?2, ?3)",
            params![c.id, c.mox_id, serde_json::to_string(c).unwrap_or_default()],
        );
    }
    async fn persist_message(&self, m: &Message) {
        self.exec(
            "INSERT OR REPLACE INTO messages (id, channel_id, data) VALUES (?1, ?2, ?3)",
            params![
                m.id,
                m.channel_id,
                serde_json::to_string(m).unwrap_or_default()
            ],
        );
    }
    async fn persist_notification(&self, n: &Notification) {
        self.exec(
            "INSERT OR REPLACE INTO notifications (id, member_id, data) VALUES (?1, ?2, ?3)",
            params![
                n.id,
                n.member_id,
                serde_json::to_string(n).unwrap_or_default()
            ],
        );
    }
    async fn persist_bindings(&self, member_id: &str, bindings: &[RoleBinding]) {
        self.exec(
            "INSERT OR REPLACE INTO bindings (member_id, data) VALUES (?1, ?2)",
            params![
                member_id,
                serde_json::to_string(bindings).unwrap_or_default()
            ],
        );
    }
    async fn persist_token(&self, hash: &str, member_id: &str) {
        self.exec(
            "INSERT OR REPLACE INTO tokens (hash, member_id) VALUES (?1, ?2)",
            params![hash, member_id],
        );
    }
    async fn persist_audit(&self, r: &AuditRecord) {
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
