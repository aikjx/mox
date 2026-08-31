//! # mox-kg-meta-core
//!
//! 图元数据与类型系统 — 节点/边/属性的核心类型定义，纯计算无 IO
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod types;
pub mod schema;
pub mod property;

pub use types::*;
pub use schema::GraphSchema;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
