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
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::crypto::sha256_hex;
use crate::error::AppError;
use crate::model::*;
use crate::rbac::RoleBinding;
use crate::repo::Repository;

#[derive(Default)]
pub struct State {
    pub moxs: HashMap<String, Mox>,
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
    /// 持久化仓库（系统记录）。`None` = 纯内存模式。
    repo: Option<Box<dyn Repository>>,
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
            repo: None,
            audit_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 持久化模式：按后端类型打开对应仓库，建表并**重放**现有数据到内存
    pub async fn open(backend: crate::config::Backend, url: &str) -> Result<Store, AppError> {
        use crate::config::Backend;
        use crate::repo::Repository;
        let repo: Box<dyn Repository> = match backend {
            Backend::Sqlite => {
                let r =
                    crate::repo::sqlite::SqliteRepository::open(url).map_err(AppError::Internal)?;
                Box::new(r)
            }
            Backend::Postgres => {
                let r = crate::repo::postgres::PostgresRepository::open(url)
                    .await
                    .map_err(AppError::Internal)?;
                Box::new(r)
            }
            Backend::MySql => {
                let r = crate::repo::mysql::MySqlRepository::open(url)
                    .await
                    .map_err(AppError::Internal)?;
                Box::new(r)
            }
        };
        repo.migrate()
            .await
            .map_err(|e| AppError::Internal(format!("建表失败: {}", e)))?;
        let state = repo.load_all().await;
        tracing::info!(
            "Store::open 重放完成({:?}): 璇玑={} 成员={} 任务={} 审计={}",
            backend,
            state.moxs.len(),
            state.members.len(),
            state.tasks.len(),
            state.audit.len()
        );
        let audit_len = state.audit.len() as u64;
        Ok(Self {
            state: RwLock::new(state),
            repo: Some(repo),
            audit_counter: Arc::new(AtomicU64::new(audit_len)),
        })
    }

    /// 是否持久化模式
    pub fn is_persistent(&self) -> bool {
        self.repo.is_some()
    }

    pub async fn mox_count(&self) -> usize {
        self.state.read().await.moxs.len()
    }

    // 内部：写穿到仓库（仅持久化模式生效）
    async fn p_mox(&self, a: &Mox) {
        if let Some(repo) = &self.repo {
            repo.persist_mox(a).await;
        }
    }
    async fn p_member(&self, m: &Member) {
        if let Some(repo) = &self.repo {
            repo.persist_member(m).await;
        }
    }
    async fn p_task(&self, t: &Task) {
        if let Some(repo) = &self.repo {
            repo.persist_task(t).await;
        }
    }
    async fn p_channel(&self, c: &Channel) {
        if let Some(repo) = &self.repo {
            repo.persist_channel(c).await;
        }
    }
    async fn p_message(&self, m: &Message) {
        if let Some(repo) = &self.repo {
            repo.persist_message(m).await;
        }
    }
    async fn p_notification(&self, n: &Notification) {
        if let Some(repo) = &self.repo {
            repo.persist_notification(n).await;
        }
    }
    async fn p_bindings(&self, member_id: &str, bindings: &[RoleBinding]) {
        if let Some(repo) = &self.repo {
            repo.persist_bindings(member_id, bindings).await;
        }
    }
    async fn p_token(&self, hash: &str, member_id: &str) {
        if let Some(repo) = &self.repo {
            repo.persist_token(hash, member_id).await;
        }
    }
    async fn p_audit(&self, r: &AuditRecord) {
        if let Some(repo) = &self.repo {
            repo.persist_audit(r).await;
        }
    }

    // ---------- 璇玑 ----------
    pub async fn create_mox(&self, a: Mox) {
        self.state
            .write()
            .await
            .moxs
            .insert(a.id.clone(), a.clone());
        self.p_mox(&a).await;
    }
    pub async fn get_mox(&self, id: &str) -> Option<Mox> {
        self.state.read().await.moxs.get(id).cloned()
    }

    // ---------- 成员 ----------
    pub async fn create_member(&self, m: Member) {
        self.state
            .write()
            .await
            .members
            .insert(m.id.clone(), m.clone());
        self.p_member(&m).await;
    }
    pub async fn get_member(&self, id: &str) -> Option<Member> {
        self.state.read().await.members.get(id).cloned()
    }
    pub async fn list_members(&self, mox_id: &str) -> Vec<Member> {
        self.state
            .read()
            .await
            .members
            .values()
            .filter(|m| m.mox_id == mox_id)
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
            self.p_member(m).await;
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
        self.p_token(&hash, member_id).await;
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
        self.p_bindings(member_id, &bindings).await;
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
        self.state
            .write()
            .await
            .tasks
            .insert(t.id.clone(), t.clone());
        self.p_task(&t).await;
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
            self.p_task(t).await;
        }
        updated
    }
    pub async fn list_tasks(&self, mox_id: &str) -> Vec<Task> {
        self.state
            .read()
            .await
            .tasks
            .values()
            .filter(|t| t.mox_id == mox_id)
            .cloned()
            .collect()
    }

    // ---------- 频道 ----------
    pub async fn create_channel(&self, c: Channel) {
        self.state
            .write()
            .await
            .channels
            .insert(c.id.clone(), c.clone());
        self.p_channel(&c).await;
    }
    pub async fn get_channel(&self, id: &str) -> Option<Channel> {
        self.state.read().await.channels.get(id).cloned()
    }
    pub async fn list_channels(&self, mox_id: &str) -> Vec<Channel> {
        self.state
            .read()
            .await
            .channels
            .values()
            .filter(|c| c.mox_id == mox_id)
            .cloned()
            .collect()
    }
    pub async fn task_channel(&self, mox_id: &str, task_id: &str) -> Channel {
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
            mox_id: mox_id.to_string(),
            kind: ChannelKind::Task(task_id.to_string()),
            name: format!("任务 #{task_id}"),
            members: vec![],
        };
        self.create_channel(ch.clone()).await;
        ch
    }
    pub async fn ensure_mox_channel(&self, mox_id: &str) -> Channel {
        {
            let s = self.state.read().await;
            for c in s.channels.values() {
                if let ChannelKind::Mox = &c.kind {
                    if c.mox_id == mox_id {
                        return c.clone();
                    }
                }
            }
        }
        let ch = Channel {
            id: new_id("chan"),
            mox_id: mox_id.to_string(),
            kind: ChannelKind::Mox,
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
            self.p_channel(&c).await;
        }
    }

    // ---------- 消息 ----------
    pub async fn add_message(&self, m: Message) {
        self.state
            .write()
            .await
            .messages
            .insert(m.id.clone(), m.clone());
        self.p_message(&m).await;
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
        self.p_notification(&n).await;
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
                self.p_notification(&n).await;
                true
            }
            None => false,
        }
    }

    // ---------- 审计（BR-18：只增不改） ----------
    pub async fn append_audit(&self, r: AuditRecord) {
        self.audit_counter.fetch_add(1, Ordering::Relaxed);
        self.state.write().await.audit.push(r.clone());
        self.p_audit(&r).await;
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

/// 从含 `id` 字段的实体提取主键（通过序列化再读取，避免为每个类型特化）
/// 被 `repo::sqlite` 复用，故导出为 `pub(crate)`。
pub(crate) fn id_of<T: serde::Serialize>(v: &T) -> Option<String> {
    let val = serde_json::to_value(v).ok()?;
    val.get("id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}
