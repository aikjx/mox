//! 持久化仓库抽象层（等价 Spring Boot 的 Repository / JpaRepository）
//!
//! 设计目标：把"落盘后端"从 `Store` 中解耦，`Store` 只关心内存热缓存 + 调用仓库接口。
//! 当前仅 SQLite 实现；后续可加 Postgres / MySQL 实现而**不动 `Store` 上层逻辑**。
//!
//! 接口语义严格对应原 `store.rs` 的写穿方法 + 启动重放：
//! - `migrate`     : 建表（对应 `create_tables`）
//! - `load_all`    : 全量加载并重建内存 `State`（对应 `load_state`）
//! - 8 个 `persist_*`: 写穿单个实体/集合（对应 `p_*`）

use crate::model::*;
use crate::rbac::RoleBinding;
use crate::store::State;

pub mod sqlite;

/// 持久化仓库统一接口。所有方法同步（沿用原 rusqlite 单连接模型），
/// 由 `Store` 在持有锁的上下文中调用。
pub trait Repository: Send + Sync {
    /// 建表 / 迁移
    fn migrate(&self) -> Result<(), String>;

    /// 全量加载并重建内存状态（WAL 重放语义）
    fn load_all(&self) -> State;

    // ---- 实体写穿 ----
    fn persist_xuanji(&self, a: &Xuanji);
    fn persist_member(&self, m: &Member);
    fn persist_task(&self, t: &Task);
    fn persist_channel(&self, c: &Channel);
    fn persist_message(&self, m: &Message);
    fn persist_notification(&self, n: &Notification);
    fn persist_bindings(&self, member_id: &str, bindings: &[RoleBinding]);
    fn persist_token(&self, hash: &str, member_id: &str);
    fn persist_audit(&self, r: &AuditRecord);
}
