//! # Mox Plugin SDK — WASM插件开发工具包
//!
//! 第三方开发者使用本SDK开发Mox平台的WASM插件。
//!
//! ## 核心能力
//! - [`host_api`] — 宿主API绑定（插件可调用的平台能力：AI聊天/事件发布/日志）
//! - [`manifest`] — 插件Manifest生成工具（构建插件描述符）
//! - [`macros`] — 插件宏（#[plugin_entry]标注入口函数）
//! - [`error`] — 插件错误类型
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_plugin_sdk::prelude::*;
//!
//! // 1. 定义插件入口
//! #[plugin_entry]
//! async fn plugin_main(ctx: PluginContext) -> PluginResult<()> {
//!     // 2. 调用宿主API
//!     let response = ctx.ai_chat("你好，请介绍一下自己").await?;
//!     ctx.log_info(&format!("AI回复: {}", response.content));
//!
//!     // 3. 发布事件
//!     ctx.publish_event("plugin.initialized", json!({"status": "ok"}))?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 插件目录结构
//!
//! ```text
//! my-plugin/
//! ├── Cargo.toml          # 依赖 mox-plugin-sdk
//! ├── src/
//! │   └── lib.rs          # 插件入口（#[plugin_entry]）
//! └── manifest.json       # 插件描述符（可用SDK生成）
//! ```

pub mod error;
pub mod host_api;
pub mod macros;
pub mod manifest;

// 重导出
pub use error::{PluginError, PluginResult};
pub use host_api::{HostApiBinding, PluginContext, PluginLogLevel};
pub use manifest::{PluginManifest, PluginManifestBuilder, PluginPermission};

/// 便捷预导入
pub mod prelude {
    pub use super::*;
    pub use serde_json::json;
}
