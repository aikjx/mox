//! # mox-flow-primiflow-svc
//!
//! 流程编排服务 — DAG 执行引擎
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod engine;
pub mod scheduler;
pub mod executor;
pub mod dag;
pub mod error;

pub use engine::*;
pub use scheduler::*;
pub use executor::*;
pub use dag::*;
pub use error::FlowError;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
