//! 璇玑系统核心库
//!
//! 设计对齐 OUS 分层业务架构：
//! - 接入层：server（HTTP + WebSocket）
//! - 运行时/鉴权：rbac + 鉴权中间件
//! - 编排层：orchestrator（事件反应器，把领域事件翻译为通信/通知）
//! - 核心域：model + services（成员 / 任务 / 权限 / 通信）
//! - 数据层：store（可替换为持久化实现）
pub mod config;

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "xuanji-system";

/// 璇玑系统 Crate 结构化元数据。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    pub uuid: &'static str,
    pub ais_layers: &'static [&'static str],
    pub owner_project: &'static str,
    pub capabilities: &'static [&'static str],
    pub data_tables_read: &'static [&'static str],
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "5c28b4d3-96f2-45a6-a1c2-d3e4f5a6b7c8",
    ais_layers: &["L1-Ingress", "L2-Gateway", "L3-Domain", "L5-Infra", "L6-Kernel"],
    owner_project: "proj-xuanji-core",
    capabilities: &[],
    data_tables_read: &["members.json", "tasks.json", "projects.json", "permissions.json"],
    data_tables_write: &["tasks.json", "members.json"],
};

pub mod crypto;
pub mod domain_traits;
pub mod error;
pub mod event;
pub mod metrics;
pub mod model;
pub mod orchestrator;
pub mod persistence_provider;
pub mod rbac;
pub mod ratelimit;
pub mod repo;
pub mod server;
pub mod services;
pub mod sqlite_provider;
pub mod store;

pub use orchestrator::XuanjiSystem;
