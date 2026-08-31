//! # mox-kg-fusion-svc
//!
//! 图谱融合服务 — 实体对齐/知识融合
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod alignment;
pub mod fusion;
pub mod matching;
pub mod error;

pub use alignment::*;
pub use fusion::*;
pub use matching::*;
pub use error::FusionError;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
