//! # mox-data-norm-core
//!
//! 数据归一化核心 — 数据清洗/标准化/质量评估
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod cleaning;
pub mod normalization;
pub mod quality;
pub mod types;

pub use types::*;
pub use cleaning::*;
pub use normalization::*;
pub use quality::*;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
