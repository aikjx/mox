//! # mox-kg-hub-svc
//!
//! 图谱集成服务 — 多数据源接入/ETL
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod connector;
pub mod pipeline;
pub mod etl;
pub mod error;

pub use connector::*;
pub use pipeline::*;
pub use etl::*;
pub use error::HubError;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
