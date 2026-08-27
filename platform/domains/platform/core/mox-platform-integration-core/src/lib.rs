// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # Mox Platform Integration Core — 统一集成层
//!
//! 企业级架构的核心枢纽，统一管理4大对接能力：
//! - **AI Provider Gateway** (`mox-ai-core`) — 多模型路由/降级/熔断
//! - **Plugin System** (`mox-plugin-core`) — WASM插件/热加载/市场
//! - **Enterprise Adapter** (`mox-enterprise-core`) — SSO/合规/白标定制
//! - **Connector Framework** (`mox-connector-core`) — 第三方系统即插即用
//!
//! ## 核心能力
//! - [`extension`] — 统一扩展点注册表（ExtensionPoint Registry）
//! - [`bootstrap`] — 统一启动组装（Integration Bootstrap）
//! - [`config`] — 统一集成配置（Integration Config）
//! - [`health`] — 统一健康检查（Health Check）
//! - [`coordinator`] — 跨能力协调器（Cross-capability Coordinator）
//!
//! ## 架构定位
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  L6 接入层  Gateway / API                            │
//! ├─────────────────────────────────────────────────────┤
//! │  L5 集成层  mox-platform-integration-core  ★本层    │
//! │           (统一扩展点/组装/健康/协调)                 │
//! ├─────────────────────────────────────────────────────┤
//! │  L4 对接层  AI / Plugin / Enterprise / Connector    │
//! ├─────────────────────────────────────────────────────┤
//! │  L3 领域服务层  8域 svc                              │
//! ├─────────────────────────────────────────────────────┤
//! │  L2 平台核心层  iam/system/meta/orchestrator        │
//! ├─────────────────────────────────────────────────────┤
//! │  L1 基础框架层  framework/foundation/observability   │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_platform_integration_core::prelude::*;
//! use std::sync::Arc;
//!
//! // 1. 加载集成配置
//! let config = IntegrationConfig::load_from_file("config/integration.yaml").await?;
//!
//! // 2. 创建集成运行时（统一组装所有对接能力）
//! let runtime = IntegrationRuntime::builder()
//!     .with_config(config)
//!     .with_ai_providers()      // 自动注册AI Provider
//!     .with_plugin_system()      // 自动初始化插件系统
//!     .with_enterprise_adapter() // 自动加载政企适配
//!     .with_connectors()         // 自动注册连接器
//!     .build()
//!     .await?;
//!
//! // 3. 统一健康检查
//! let health = runtime.health_check().await;
//! println!("integration health: {:?}", health);
//!
//! // 4. 注册自定义扩展点
//! runtime.extensions().register(
//!     "custom.ai.hook",
//!     ExtensionPoint::new("My AI Hook", "1.0.0"),
//! );
//! ```

pub mod bootstrap;
pub mod builtin;
pub mod config;
pub mod coordinator;
pub mod extension;
pub mod factory;
pub mod flow;
pub mod health;
pub mod protocol;

// ─── 统一重导出 ──────────────────────────────────────────────────────────────

// 扩展点
pub use extension::{
    ExtensionPoint, ExtensionPointBuilder, ExtensionPointId, ExtensionPointMetadata,
    ExtensionPointType, ExtensionRegistry, ExtensionRegistryError,
};

// 启动组装
pub use bootstrap::{IntegrationBootstrap, IntegrationRuntime, IntegrationRuntimeBuilder};

// 配置
pub use config::{
    AiIntegrationConfig, ConnectorIntegrationConfig, EnterpriseIntegrationConfig,
    IntegrationConfig, PluginIntegrationConfig,
};

// 健康检查
pub use health::{
    CapabilityHealth, HealthStatus, IntegrationHealth, IntegrationHealthChecker,
};

// 协调器
pub use coordinator::{
    CapabilityHandle, CapabilityType, IntegrationCoordinator,
};

// 工厂注册中心
pub use factory::{
    AiProviderFactory, AutoAssembler, AutoAssemblyResult, ConnectorFactory,
    ExtensionFactory, FactoryConfig, FactoryRegistry, SsoFactory,
};

// 内置Factory（开箱即用）
pub use builtin::{
    register_all_builtin_factories, register_ai_factories, register_connector_factories,
    register_sso_factories,
};

// 企业级处理流程
pub use flow::{
    ConfigHotReloader, ConfigUpdateEvent, ErrorCategory, ErrorCode, PlatformError,
    RateLimitConfig, RateLimitResult, RateLimiter, TraceContext, TraceId,
    current_trace_id, error_code, with_trace,
};

// 多协议网关
pub use protocol::{
    GraphQLEndpoint, GraphQLSchema, GraphQLSchemaRegistry, GrpcService, GrpcServiceRegistry,
    ProtocolHandler, ProtocolRequest, ProtocolResponse, ProtocolRouter, ProtocolType,
    RoutingResult, WebSocketConnection, WebSocketManager, WebSocketMessage,
};

// 重导出4大对接能力（方便上层统一引用）
pub use mox_ai_core as ai;
pub use mox_connector_core as connector;
pub use mox_enterprise_core as enterprise;
pub use mox_plugin_core as plugin;

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}
