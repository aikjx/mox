// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 扩展点模块
//!
//! 提供扩展注册机制，各服务可注册自定义能力供节点处理器使用。

pub mod registry;

pub use registry::ExtensionRegistry;
