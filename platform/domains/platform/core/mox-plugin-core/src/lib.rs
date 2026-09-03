// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # Mox Plugin Core — 双运行时混合插件框架
//!
//! 企业级插件系统，支持 **WASM** 和 **VSCode** 双运行时（方案 C），
//! 提供元数据兼容、统一插件市场、多运行时抽象、热重载、权限控制、宿主API、生命周期管理。
//!
//! ## 双运行时架构（方案 C）
//!
//! ```text
//!                    ┌─────────────────────────────┐
//!                    │       PluginManifest         │
//!                    │  (统一元数据 / 能力 / 权限)  │
//!                    └──────────────┬──────────────┘
//!                                   │
//!                    ┌──────────────▼──────────────┐
//!                    │       RuntimeRegistry         │
//!                    │  (运行时注册 / 查找 / 调度)   │
//!                    └──────────────┬──────────────┘
//!                                   │
//!              ┌────────────────────┴────────────────────┐
//!              │                                         │
//!     ┌────────▼────────┐                     ┌────────▼────────┐
//!     │   WasmRuntime    │                     │  VsCodeRuntime   │
//!     │ (wasmer+cranelift)│                     │ (deno_core 阶段2)│
//!     │  WASM 沙箱执行    │                     │  VSCode API 兼容  │
//!     └─────────────────┘                     └─────────────────┘
//! ```
//!
//! ## 核心组件
//! - [`manifest`] — 插件描述符（manifest.json）+ VSCode package.json 兼容解析 + 权限 + 依赖 + 能力声明
//! - [`lifecycle`] — 插件生命周期状态机（Unloaded→Loaded→Initialized→Running）
//! - [`registry`] — 插件注册表（实例管理 + 状态 + 能力查找）
//! - [`loader`] — 插件加载器（目录扫描 + WASM加载 + VSIX解压 + 热重载）
//! - [`host_api`] — 宿主API（插件可调用的平台能力 + 权限检查）
//! - [`runtime`] — 多运行时抽象（Runtime trait + WasmRuntime + VsCodeRuntime + RuntimeRegistry）
//! - [`market`] — 插件市场（远程发现/安装/版本管理 + VSIX 市场支持）
//!
//! ## 阶段 1 完成内容
//! - VSCode package.json 元数据解析与 MOX PluginManifest 转换
//! - VSIX 包解压加载（ZIP 格式）
//! - 多运行时抽象骨架（Runtime trait + WasmRuntime + VsCodeRuntime）
//! - VSIX 市场支持骨架（搜索/安装/已安装列表）
//!
//! ## 阶段 2 规划
//! - VsCodeRuntime 的 deno_core 集成（JS 执行环境）
//! - VSCode API 兼容层（vscode namespace shim）
//! - 真实 Open VSX Registry API 对接
//! - VSCode 扩展激活事件调度
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
//! // 3. 加载所有插件（WASM 目录 + VSIX 包）
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
//! ├── com.vendor.ocr/           # WASM 插件目录
//! │   ├── manifest.json          # 插件描述符（必需）
//! │   └── plugin.wasm            # WASM模块（必需，路径在manifest.entry指定）
//! ├── vscode.ms-python.python/   # VSCode 扩展目录（VSIX解压后）
//! │   └── extension/
//! │       ├── package.json       # VSCode 扩展描述符
//! │       └── out/extension.js   # JS 入口
//! └── vendor.extension.vsix      # VSIX 包（直接放置，加载时自动解压）
//! ```

pub mod host_api;
pub mod lifecycle;
pub mod loader;
pub mod manifest;
pub mod market;
pub mod registry;
pub mod runtime;

// ─── 统一重导出 ──────────────────────────────────────────────────────────────

// Manifest（含 VSCode 兼容类型）
pub use manifest::{
    ConfigField, PluginCapability, PluginConfig, PluginDependency, PluginManifest,
    PluginPermission, VsCodeManifest,
};

// Lifecycle
pub use lifecycle::{LifecycleError, LifecycleEvent, PluginState};

// Registry
pub use registry::{PluginInstance, PluginRegistry};

// Loader（含 VSIX 加载器）
pub use loader::{PluginLoader, VsixLoader};

// Host API
pub use host_api::{
    AiChatDelegate, AiChatHostApi, EventPublishHostApi, HostApi, HostApiContext,
    HostApiError, HostApiRegistry, HostApiResult,
};

// Runtime（多运行时抽象）
pub use runtime::{
    Runtime, RuntimeHandle, RuntimeInternal, RuntimeRegistry, RuntimeType, VsCodeRuntime,
    WasmRuntime,
};

// Market（插件市场：远程发现/安装/版本管理 + VSIX 市场）
pub use market::{
    client::MarketClient,
    installer::{InstallResult, InstalledPluginInfo, PluginInstaller, UninstallResult},
    remote_registry::{ListQuery, RemoteDependency, RemotePluginInfo, RemotePluginRegistry, RemotePluginVersion},
    version::{DependencyStatus, SemVer, VersionLockFile, VersionManager, VersionUpdateInfo, classify_update, is_version_greater, version_matches_constraint},
    vsix::{VsixMarketplace, VsixPackageInfo},
};

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}
