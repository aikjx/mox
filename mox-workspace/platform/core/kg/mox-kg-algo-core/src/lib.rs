//! # mox-kg-algo-core
//!
//! 图算法核心库 — 社区检测/中心性/PageRank/激活扩散等纯计算算法
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod centrality;
pub mod community;
pub mod pagerank;
pub mod spread;
pub mod shortest_path;

pub use centrality::*;
pub use community::*;
pub use pagerank::*;
pub use spread::*;
pub use shortest_path::*;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
