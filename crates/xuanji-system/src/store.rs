//! 存储层：内存热缓存 + 可选 SQLite 系统记录（持久化 / WAL 重放）
//!
//! 设计要点（企业级 I-01 / I-02）：
//! - **热缓存**：所有读操作命中内存 `State`（低延迟、高并发）。
//! - **系统记录（System of Record）**：当配置 `persist=true` 时，所有写操作**写穿（write-through）**
//!   到 SQLite 单文件数据库，保证进程重启后数据不丢失。
//! - **启动重放（Replay）**：`Store::open` 在启动时从 SQLite 全量加载并重建内存状态，
//!   等价于「事件/实体日志重放」，是审计可追溯与灾难恢复的基础。
//! - **令牌哈希**：令牌落盘前做 SHA-256（见 `crypto`），库中只存哈希，明文不落地。
//! - 无 SQLite 时为纯内存模式（`Store::new`），用于测试与演示，接口完全一致。
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, params};
use tokio::sync::RwLock;

use crate::crypto::sha256_hex;
use crate::error::AppError;
use crate::model::*;
use crate::rbac::RoleBinding;

#[derive(Default)]
pub struct State {
    pub xuanjis: HashMap<String, Xuanji>,
    pub members: HashMap<String, Member>,
    pub tokens: HashMap<String, String>, // token_hash -> member_id
    pub tasks: HashMap<String, Task>,
    pub channels: HashMap<String, Channel>,
    pub messages: HashMap<String, Message>,
    pub notifications: HashMap<String, Notification>,
    pub bindings: HashMap<String, Vec<RoleBinding>>, // member_id -> bindings
    /// 审计流：按发生顺序追加，只增不改（BR-18）
    pub audit: Vec<AuditRecord>,
}

pub struct Store {
    pub state: RwLock<State>,
    /// SQLite 后台（系统记录）。`None` = 纯内存模式。
    sql: Option<Mutex<Connection>>,
    /// 审计记录计数器（与 Metrics.audit_records 共享同一原子）
    pub audit_counter: Arc<AtomicU64>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    /// 纯内存模式（测试 / 演示）
    pub fn new() -> Self {
        Self {
            state: RwLock::new(State::default()),
            sql: None,
            audit_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 持久化模式：打开（或创建）SQLite 库，建表并**重放**现有数据到内存
    pub fn open(path: &str) -> Result<Store, AppError> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Internal(format!("无法创建数据目录 {:?}: {}", parent, e))
                })?;
            }
        }
        let conn = Connection::open(path)
            .map_err(|e| AppError::Internal(format!("SQLite 打开失败: {}", e)))?;
        create_tables(&conn)
            .map_err(|e| AppError::Internal(format!("建表失败: {}", e)))?;
        let state = load_state(&conn);
        tracing::info!(
            "Store::open 重放完成: 璇玑={} 成员={} 任务={} 审计={}",
            state.xuanjis.len(),
            state.members.len(),
            state.tasks.len(),
            state.audit.len()
        );
        let audit_len = state.audit.len() as u64;
        Ok(Self {
            state: RwLock::new(state),
            sql: Some(Mutex::new(conn)),
            audit_counter: Arc::new(AtomicU64::new(audit_len)),
        })
    }

    /// 是否持久化模式
    pub fn is_persistent(&self) -> bool {
        self.sql.is_some()
    }

    pub async fn xuanji_count(&self) -> usize {
        self.state.read().await.xuanjis.len()
    }

    // 内部：执行一条写穿 SQL（仅持久化模式生效）
    fn exec<P: rusqlite::Params>(&self, sql: &str, p: P) {
        if let Some(lock) = &self.sql {
            if let Ok(conn) = lock.lock() {
                if let Err(e) = conn.execute(sql, p) {
                    tracing::error!("sqlite 写穿失败 [{}]: {}", sql, e);
                }
            }
        }
    }

    fn p_xuanji(&self, a: &Xuanji) {
        self.exec(
            "INSERT OR REPLACE INTO xuanjis (id, data) VALUES (?1, ?2)",
            params![a.id, serde_json::to_string(a).unwrap_or_default()],
        );
    }
    fn p_member(&self, m: &Member) {
        self.exec(
            "INSERT OR REPLACE INTO members (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![
                m.id,
                m.xuanji_id,
                serde_json::to_string(m).unwrap_or_default()
            ],
        );
    }
    fn p_task(&self, t: &Task) {
        self.exec(
            "INSERT OR REPLACE INTO tasks (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![
                t.id,
                t.xuanji_id,
                serde_json::to_string(t).unwrap_or_default()
            ],
        );
    }
    fn p_channel(&self, c: &Channel) {
        self.exec(
            "INSERT OR REPLACE INTO channels (id, xuanji_id, data) VALUES (?1, ?2, ?3)",
            params![
                c.id,
                c.xuanji_id,
                serde_json::to_string(c).unwrap_or_default()
            ],
        );
    }
    fn p_message(&self, m: &Message) {
        self.exec(
            "INSERT OR REPLACE INTO messages (id, channel_id, data) VALUES (?1, ?2, ?3)",
            params![
                m.id,
                m.channel_id,
                serde_json::to_string(m).unwrap_or_default()
            ],
        );
    }
    fn p_notification(&self, n: &Notification) {
        self.exec(
            "INSERT OR REPLACE INTO notifications (id, member_id, data) VALUES (?1, ?2, ?3)",
            params![
                n.id,
                n.member_id,
                serde_json::to_string(n).unwrap_or_default()
            ],
        );
    }
    fn p_bindings(&self, member_id: &str, bindings: &[RoleBinding]) {
        self.exec(
            "INSERT OR REPLACE INTO bindings (member_id, data) VALUES (?1, ?2)",
            params![member_id, serde_json::to_string(bindings).unwrap_or_default()],
        );
    }
    fn p_token(&self, hash: &str, member_id: &str) {
        self.exec(
            "INSERT OR REPLACE INTO tokens (hash, member_id) VALUES (?1, ?2)",
            params![hash, member_id],
        );
    }
    fn p_audit(&self, r: &AuditRecord) {
        self.exec(
            "INSERT INTO audit (id, data, at) VALUES (?1, ?2, ?3)",
            params![
                r.id,
                serde_json::to_string(r).unwrap_or_default(),
                r.at.timestamp()
            ],
        );
    }

    // ---------- 璇玑 ----------
    pub async fn create_xuanji(&self, a: Xuanji) {
        self.state.write().await.xuanjis.insert(a.id.clone(), a.clone());
        self.p_xuanji(&a);
    }
    pub async fn get_xuanji(&self, id: &str) -> Option<Xuanji> {
        self.state.read().await.xuanjis.get(id).cloned()
    }

    // ---------- 成员 ----------
    pub async fn create_member(&self, m: Member) {
        self.state.write().await.members.insert(m.id.clone(), m.clone());
        self.p_member(&m);
    }
    pub async fn get_member(&self, id: &str) -> Option<Member> {
        self.state.read().await.members.get(id).cloned()
    }
    pub async fn list_members(&self, xuanji_id: &str) -> Vec<Member> {
        self.state
            .read()
            .await
            .members
            .values()
            .filter(|m| m.xuanji_id == xuanji_id)
            .cloned()
            .collect()
    }
    pub async fn update_member<F: FnOnce(&mut Member)>(&self, id: &str, f: F) -> Option<Member> {
        let updated = {
            let mut s = self.state.write().await;
            match s.members.get_mut(id) {
                Some(m) => {
                    f(m);
                    Some(m.clone())
                }
                None => None,
            }
        };
        if let Some(ref m) = updated {
            self.p_member(m);
        }
        updated
    }
    pub async fn set_token(&self, token: &str, member_id: &str) {
        let hash = sha256_hex(token.as_bytes());
        self.state
            .write()
            .await
            .tokens
            .insert(hash.clone(), member_id.to_string());
        self.p_token(&hash, member_id);
    }
    pub async fn member_by_token(&self, token: &str) -> Option<String> {
        let hash = sha256_hex(token.as_bytes());
        self.state.read().await.tokens.get(&hash).cloned()
    }

    // ---------- 角色绑定 ----------
    pub async fn set_bindings(&self, member_id: &str, bindings: Vec<RoleBinding>) {
        self.state
            .write()
            .await
            .bindings
            .insert(member_id.to_string(), bindings.clone());
        self.p_bindings(member_id, &bindings);
    }
    pub async fn get_bindings(&self, member_id: &str) -> Vec<RoleBinding> {
        self.state
            .read()
            .await
            .bindings
            .get(member_id)
            .cloned()
            .unwrap_or_default()
    }

    // ---------- 任务 ----------
    pub async fn create_task(&self, t: Task) {
        self.state.write().await.tasks.insert(t.id.clone(), t.clone());
        self.p_task(&t);
    }
    pub async fn get_task(&self, id: &str) -> Option<Task> {
        self.state.read().await.tasks.get(id).cloned()
    }
    pub async fn update_task<F: FnOnce(&mut Task)>(&self, id: &str, f: F) -> Option<Task> {
        let updated = {
            let mut s = self.state.write().await;
            match s.tasks.get_mut(id) {
                Some(t) => {
                    f(t);
                    t.updated_at = chrono::Utc::now();
                    Some(t.clone())
                }
                None => None,
            }
        };
        if let Some(ref t) = updated {
            self.p_task(t);
        }
        updated
    }
    pub async fn list_tasks(&self, xuanji_id: &str) -> Vec<Task> {
        self.state
            .read()
            .await
            .tasks
            .values()
            .filter(|t| t.xuanji_id == xuanji_id)
            .cloned()
            .collect()
    }

    // ---------- 频道 ----------
    pub async fn create_channel(&self, c: Channel) {
        self.state.write().await.channels.insert(c.id.clone(), c.clone());
        self.p_channel(&c);
    }
    pub async fn get_channel(&self, id: &str) -> Option<Channel> {
        self.state.read().await.channels.get(id).cloned()
    }
    pub async fn list_channels(&self, xuanji_id: &str) -> Vec<Channel> {
        self.state
            .read()
            .await
            .channels
            .values()
            .filter(|c| c.xuanji_id == xuanji_id)
            .cloned()
            .collect()
    }
    pub async fn task_channel(&self, xuanji_id: &str, task_id: &str) -> Channel {
        {
            let s = self.state.read().await;
            for c in s.channels.values() {
                if let ChannelKind::Task(tid) = &c.kind {
                    if tid == task_id {
                        return c.clone();
                    }
                }
            }
        }
        let ch = Channel {
            id: new_id("chan"),
            xuanji_id: xuanji_id.to_string(),
            kind: ChannelKind::Task(task_id.to_string()),
            name: format!("任务 #{task_id}"),
            members: vec![],
        };
        self.create_channel(ch.clone()).await;
        ch
    }
    pub async fn ensure_xuanji_channel(&self, xuanji_id: &str) -> Channel {
        {
            let s = self.state.read().await;
            for c in s.channels.values() {
                if let ChannelKind::Xuanji = &c.kind {
                    if c.xuanji_id == xuanji_id {
                        return c.clone();
                    }
                }
            }
        }
        let ch = Channel {
            id: new_id("chan"),
            xuanji_id: xuanji_id.to_string(),
            kind: ChannelKind::Xuanji,
            name: "璇玑大厅".to_string(),
            members: vec![],
        };
        self.create_channel(ch.clone()).await;
        ch
    }
    pub async fn add_channel_member(&self, channel_id: &str, member_id: &str) {
        let changed = {
            let mut s = self.state.write().await;
            match s.channels.get_mut(channel_id) {
                Some(c) => {
                    if !c.members.contains(&member_id.to_string()) {
                        c.members.push(member_id.to_string());
                        Some(c.clone())
                    } else {
                        None
                    }
                }
                None => None,
            }
        };
        if let Some(c) = changed {
            self.p_channel(&c);
        }
    }

    // ---------- 消息 ----------
    pub async fn add_message(&self, m: Message) {
        self.state.write().await.messages.insert(m.id.clone(), m.clone());
        self.p_message(&m);
    }
    pub async fn list_messages(&self, channel_id: &str) -> Vec<Message> {
        self.state
            .read()
            .await
            .messages
            .values()
            .filter(|m| m.channel_id == channel_id)
            .cloned()
            .collect()
    }

    // ---------- 通知 ----------
    pub async fn add_notification(&self, n: Notification) {
        self.state
            .write()
            .await
            .notifications
            .insert(n.id.clone(), n.clone());
        self.p_notification(&n);
    }
    pub async fn list_notifications(&self, member_id: &str) -> Vec<Notification> {
        let mut v: Vec<Notification> = self
            .state
            .read()
            .await
            .notifications
            .values()
            .filter(|n| n.member_id == member_id)
            .cloned()
            .collect();
        v.sort_by_key(|n| std::cmp::Reverse(n.created_at));
        v
    }
    pub async fn mark_notification_read(&self, id: &str, member_id: &str) -> bool {
        let updated = {
            let mut s = self.state.write().await;
            match s.notifications.get_mut(id) {
                Some(n) if n.member_id == member_id => {
                    n.read = true;
                    Some(n.clone())
                }
                _ => None,
            }
        };
        match updated {
            Some(n) => {
                self.p_notification(&n);
                true
            }
            None => false,
        }
    }

    // ---------- 审计（BR-18：只增不改） ----------
    pub async fn append_audit(&self, r: AuditRecord) {
        self.audit_counter.fetch_add(1, Ordering::Relaxed);
        self.state.write().await.audit.push(r.clone());
        self.p_audit(&r);
    }
    pub async fn list_audit(&self) -> Vec<AuditRecord> {
        self.state.read().await.audit.clone()
    }
    pub async fn list_audit_by_action(&self, action: AuditAction) -> Vec<AuditRecord> {
        self.state
            .read()
            .await
            .audit
            .iter()
            .filter(|r| r.action == action)
            .cloned()
            .collect()
    }
}

// ---------------- SQLite 辅助 ----------------
fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
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
}

/// 从 SQLite 全量加载并重建内存状态（WAL 重放语义）
fn load_state(conn: &Connection) -> State {
    let mut st = State::default();

    if let Ok(mut stmt) = conn.prepare("SELECT data FROM xuanjis") {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for r in rows.flatten() {
                if let Ok(v) = serde_json::from_str::<Xuanji>(&r) {
                    st.xuanjis.insert(v.id.clone(), v);
                }
            }
        }
    }
    load_into(&mut st.members, conn, "SELECT data FROM members");
    load_into(&mut st.tasks, conn, "SELECT data FROM tasks");
    load_into(&mut st.channels, conn, "SELECT data FROM channels");
    load_into(&mut st.messages, conn, "SELECT data FROM messages");
    load_into(&mut st.notifications, conn, "SELECT data FROM notifications");

    // 角色绑定：member_id -> Vec<RoleBinding>
    if let Ok(mut stmt) = conn.prepare("SELECT member_id, data FROM bindings") {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        {
            for r in rows.flatten() {
                if let Ok(binds) = serde_json::from_str::<Vec<RoleBinding>>(&r.1) {
                    st.bindings.insert(r.0, binds);
                }
            }
        }
    }
    // 令牌：hash -> member_id
    if let Ok(mut stmt) = conn.prepare("SELECT hash, member_id FROM tokens") {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        {
            for r in rows.flatten() {
                st.tokens.insert(r.0, r.1);
            }
        }
    }
    // 审计：保持追加顺序
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

fn load_into<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(
    map: &mut HashMap<String, T>,
    conn: &Connection,
    sql: &str,
) {
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for r in rows.flatten() {
                if let Ok(v) = serde_json::from_str::<T>(&r) {
                    // 用 id 字段作为键：实体均含 `id`
                    if let Some(key) = id_of::<T>(&v) {
                        map.insert(key, v);
                    }
                }
            }
        }
    }
}

/// 从含 `id` 字段的实体提取主键（通过序列化再读取，避免为每个类型特化）
fn id_of<T: serde::Serialize>(v: &T) -> Option<String> {
    let val = serde_json::to_value(v).ok()?;
    val.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
}
