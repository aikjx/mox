// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 持久化仓库抽象层（等价 Spring Boot 的 Repository / JpaRepository）
//!
//! 设计目标：把"落盘后端"从 `Store` 中解耦，`Store` 只关心内存热缓存 + 调用仓库接口。
//! 支持后端：SQLite（rusqlite）、Postgres / MySQL（sqlx）。
//!
//! 接口语义严格对应原 `store.rs` 的写穿方法 + 启动重放：
//! - `migrate`     : 建表（对应 `create_tables`）
//! - `load_all`    : 全量加载并重建内存 `State`（对应 `load_state`）
//! - 8 个 `persist_*`: 写穿单个实体/集合（对应 `p_*`）
//!
//! 全异步签名（原生 async fn in trait，Rust 1.85+ 支持 dyn 兼容）：
//! `Store` 在 async 上下文中调用仓库接口。

use crate::model::*;
use crate::rbac::RoleBinding;
use crate::store::State;

pub mod mysql;
pub mod postgres;
pub mod schema;
pub mod sqlite;

/// 持久化仓库统一接口。全异步（等价 Spring Boot 的 Repository），
/// 由 `Store` 在 async 上下文中调用。
#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    /// 建表 / 迁移
    async fn migrate(&self) -> Result<(), String>;

    /// 全量加载并重建内存状态（WAL 重放语义）
    async fn load_all(&self) -> State;

    // ---- 实体写穿 ----
    async fn persist_mox(&self, a: &Mox);
    async fn persist_member(&self, m: &Member);
    async fn persist_task(&self, t: &Task);
    async fn persist_channel(&self, c: &Channel);
    async fn persist_message(&self, m: &Message);
    async fn persist_notification(&self, n: &Notification);
    async fn persist_bindings(&self, member_id: &str, bindings: &[RoleBinding]);
    async fn persist_token(&self, hash: &str, member_id: &str);
    async fn persist_audit(&self, r: &AuditRecord);
}
