//! # mox-ai-core
//!
//! AI 核心类型与接口 — LLM Provider 抽象、消息、Embedding、工具调用
//!
//! ## 功能特性
//! - 统一的 LLM Provider 抽象接口
//! - 聊天消息与工具调用类型
//! - Embedding 与余弦相似度计算
//! - Completion 参数配置

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod types;
pub mod provider;
pub mod message;
pub mod tool;

pub use error::AiError;
pub use types::*;
pub use provider::*;
pub use message::*;
pub use tool::*;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
