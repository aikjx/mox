//! # mox-kg-storage-svc
//!
//! 图谱存储服务 — 图数据库访问层
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod storage;
pub mod error;

pub use storage::*;
pub use error::StorageError;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
