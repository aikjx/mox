//! 璇玑系统核心库
//!
//! 设计对齐 OUS 分层业务架构：
//! - 接入层：server（HTTP + WebSocket）
//! - 运行时/鉴权：rbac + 鉴权中间件
//! - 编排层：orchestrator（事件反应器，把领域事件翻译为通信/通知）
//! - 核心域：model + services（成员 / 任务 / 权限 / 通信）
//! - 数据层：store（可替换为持久化实现）
pub mod config;
pub mod crypto;
pub mod error;
pub mod event;
pub mod metrics;
pub mod model;
pub mod orchestrator;
pub mod rbac;
pub mod ratelimit;
pub mod repo;
pub mod server;
pub mod services;
pub mod store;

pub use orchestrator::XuanjiSystem;
