// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 插件市场模块
//!
//! 基于 PluginLoader 扩展的远程发现、安装、版本管理能力。
//!
//! ## 核心组件
//! - [`client`] — 市场API客户端（HTTP封装 + 认证 + 缓存）
//! - [`remote_registry`] — 远程插件注册表（列表/搜索/详情/分类）
//! - [`installer`] — 插件安装器（下载/验证/安装/卸载/升级）
//! - [`version`] — 版本管理（语义化版本/依赖解析/升级检查/回滚）
//! - [`vsix`] — VSIX 市场支持（VSCode 扩展搜索/安装/已安装列表，阶段 1 骨架）
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_plugin_core::prelude::*;
//! use std::sync::Arc;
//!
//! // 1. 创建市场客户端
//! let client = MarketClient::new("https://market.mox.local/api/v1".into())
//!     .with_api_token("sk-xxx".into());
//!
//! // 2. 远程发现
//! let registry = RemotePluginRegistry::new(client.clone());
//! let plugins = registry.list(None).await?;
//! let ocr_plugins = registry.search("ocr").await?;
//!
//! // 3. 安装插件
//! let installer = PluginInstaller::new(client, "./plugins".into());
//! installer.install("com.vendor.ocr", "1.2.0").await?;
//!
//! // 4. 版本管理
//! let version_mgr = VersionManager::new("./plugins".into());
//! let updates = version_mgr.check_updates().await?;
//! version_mgr.upgrade("com.vendor.ocr").await?;
//! version_mgr.rollback("com.vendor.ocr").await?;
//! ```

pub mod client;
pub mod installer;
pub mod remote_registry;
pub mod version;
pub mod vsix;

// 重导出
pub use client::MarketClient;
pub use installer::{InstallResult, PluginInstaller, UninstallResult};
pub use remote_registry::{RemotePluginInfo, RemotePluginRegistry, RemotePluginVersion};
pub use version::{VersionManager, VersionUpdateInfo};
pub use vsix::{VsixMarketplace, VsixPackageInfo};
