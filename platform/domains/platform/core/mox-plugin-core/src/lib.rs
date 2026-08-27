//! # Mox Plugin Core — WASM插件框架
//!
//! 企业级插件系统，支持WASM沙箱、热重载、权限控制、宿主API、生命周期管理。
//!
//! ## 核心组件
//! - [`manifest`] — 插件描述符（manifest.json）+ 权限 + 依赖 + 能力声明
//! - [`lifecycle`] — 插件生命周期状态机（Unloaded→Loaded→Initialized→Running）
//! - [`registry`] — 插件注册表（实例管理 + 状态 + 能力查找）
//! - [`loader`] — 插件加载器（目录扫描 + WASM加载 + 热重载）
//! - [`host_api`] — 宿主API（插件可调用的平台能力 + 权限检查）
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_plugin_core::prelude::*;
//! use std::sync::Arc;
//!
//! // 1. 创建注册表
//! let registry = Arc::new(PluginRegistry::new());
//!
//! // 2. 创建加载器，指定插件目录
//! let loader = PluginLoader::new(registry.clone(), "./plugins");
//!
//! // 3. 加载所有插件
//! let count = loader.load_all().await.unwrap();
//! println!("loaded {} plugins", count);
//!
//! // 4. 初始化并启动插件
//! for plugin in registry.list() {
//!     plugin.transition_to(PluginState::Initialized).unwrap();
//!     plugin.transition_to(PluginState::Running).unwrap();
//! }
//!
//! // 5. 按能力查找插件
//! let ocr_plugins = registry.find_by_capability("ocr.extract");
//! ```
//!
//! ## 插件目录结构
//!
//! ```text
//! plugins/
//! ├── com.vendor.ocr/
//! │   ├── manifest.json      # 插件描述符（必需）
//! │   └── plugin.wasm        # WASM模块（必需，路径在manifest.entry指定）
//! └── com.vendor.translate/
//!     ├── manifest.json
//!     └── plugin.wasm
//! ```

pub mod host_api;
pub mod lifecycle;
pub mod loader;
pub mod manifest;
pub mod registry;

// ─── 统一重导出 ──────────────────────────────────────────────────────────────

// Manifest
pub use manifest::{
    ConfigField, PluginCapability, PluginConfig, PluginDependency, PluginManifest,
    PluginPermission,
};

// Lifecycle
pub use lifecycle::{LifecycleError, LifecycleEvent, PluginState};

// Registry
pub use registry::{PluginInstance, PluginRegistry};

// Loader
pub use loader::PluginLoader;

// Host API
pub use host_api::{
    AiChatDelegate, AiChatHostApi, EventPublishHostApi, HostApi, HostApiContext,
    HostApiError, HostApiRegistry, HostApiResult,
};

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}
