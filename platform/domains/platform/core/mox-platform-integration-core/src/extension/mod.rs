// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 统一扩展点机制 — Extension Point Registry
//!
//! 企业级插件化架构的核心。所有可扩展能力通过扩展点注册，
//! 支持运行时动态注册/查找/卸载，实现"对扩展开放，对修改关闭"。

pub mod registry;

// 重导出
pub use registry::{
    ExtensionPoint, ExtensionPointBuilder, ExtensionPointId, ExtensionPointMetadata,
    ExtensionPointType, ExtensionRegistry, ExtensionRegistryError,
};
