//! # mox-data-formula-core
//!
//! 公式引擎核心 — 公式解析/编译/执行
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]
pub mod error;
pub mod ast;
pub mod parser;
pub mod compiler;
pub mod runtime;

pub use error::FormulaError;
pub use ast::*;
pub use parser::parse;
pub use compiler::compile;
pub use runtime::evaluate;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
