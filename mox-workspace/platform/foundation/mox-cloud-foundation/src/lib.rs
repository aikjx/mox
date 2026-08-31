//! # mox-cloud-foundation
//!
//! 云存储域抽象 — 定义统一的存储接口，支持多种后端实现
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod storage;
pub mod error;

pub use storage::*;
pub use error::CloudError;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
