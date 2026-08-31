//! # mox-platform-foundation
//!
//! Mox 平台基础库 — 公共类型、错误码、元数据、通用工具
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod id;
pub mod time;
pub mod tenant;
pub mod common;

pub use error::MoxError;
pub use id::MoxId;
pub use id::TenantId;
pub use common::*;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
