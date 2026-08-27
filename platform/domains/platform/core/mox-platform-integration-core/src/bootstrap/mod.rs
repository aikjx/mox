// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 统一启动组装 — Integration Bootstrap
//!
//! 企业级应用启动入口，统一组装4大对接能力，
//! 提供Builder模式配置，支持按需启用各能力。

use crate::config::IntegrationConfig;
use crate::coordinator::{CapabilityHandle, CapabilityType, IntegrationCoordinator};
use crate::extension::ExtensionRegistry;
use crate::factory::{AutoAssembler, AutoAssemblyResult, FactoryRegistry};
use crate::health::IntegrationHealthChecker;
use std::sync::Arc;

/// 集成运行时（所有对接能力的统一持有者）
pub struct IntegrationRuntime {
    /// 配置
    config: IntegrationConfig,
    /// 扩展点注册表
    extensions: Arc<ExtensionRegistry>,
    /// 协调器
    coordinator: Arc<IntegrationCoordinator>,
    /// 健康检查器
    health_checker: Arc<IntegrationHealthChecker>,
    /// 工厂注册中心（零改动核心架构的关键）
    factory_registry: Arc<FactoryRegistry>,
    /// AI Provider注册表
    ai_registry: Arc<crate::ai::registry::ProviderRegistry>,
    /// Connector注册表
    connector_registry: Arc<crate::connector::registry::ConnectorRegistry>,
    /// 自动组装结果
    auto_assembly: Option<AutoAssemblyResult>,
    /// 是否已启动
    started: bool,
}

impl IntegrationRuntime {
    /// 创建Builder
    pub fn builder() -> IntegrationRuntimeBuilder {
        IntegrationRuntimeBuilder::new()
    }

    /// 获取配置
    pub fn config(&self) -> &IntegrationConfig { &self.config }

    /// 获取扩展点注册表
    pub fn extensions(&self) -> &Arc<ExtensionRegistry> { &self.extensions }

    /// 获取协调器
    pub fn coordinator(&self) -> &Arc<IntegrationCoordinator> { &self.coordinator }

    /// 获取健康检查器
    pub fn health_checker(&self) -> &Arc<IntegrationHealthChecker> { &self.health_checker }

    /// 获取工厂注册中心
    pub fn factory_registry(&self) -> &Arc<FactoryRegistry> { &self.factory_registry }

    /// 获取AI Provider注册表
    pub fn ai_registry(&self) -> &Arc<crate::ai::registry::ProviderRegistry> { &self.ai_registry }

    /// 获取Connector注册表
    pub fn connector_registry(&self) -> &Arc<crate::connector::registry::ConnectorRegistry> { &self.connector_registry }

    /// 获取自动组装结果
    pub fn auto_assembly_result(&self) -> Option<&AutoAssemblyResult> { self.auto_assembly.as_ref() }

    /// 是否已启动
    pub fn is_started(&self) -> bool { self.started }

    /// 执行健康检查
    pub async fn health_check(&self) -> crate::health::IntegrationHealth {
        self.health_checker.check_all(&self.config.runtime_name).await
    }

    /// 关闭运行时
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        tracing::info!("integration runtime shutting down: {}", self.config.runtime_name);
        // 发送关闭事件
        self.coordinator.emit(
            "runtime.shutdown",
            CapabilityType::Custom,
            serde_json::json!({"runtime": self.config.runtime_name}),
        )?;
        Ok(())
    }
}

/// 集成运行时构建器
pub struct IntegrationRuntimeBuilder {
    config: Option<IntegrationConfig>,
    enable_ai: bool,
    enable_plugin: bool,
    enable_enterprise: bool,
    enable_connector: bool,
    custom_extensions: Vec<crate::extension::ExtensionPoint>,
}

impl IntegrationRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            enable_ai: false,
            enable_plugin: false,
            enable_enterprise: false,
            enable_connector: false,
            custom_extensions: Vec::new(),
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: IntegrationConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 启用AI能力
    pub fn with_ai(mut self) -> Self {
        self.enable_ai = true;
        self
    }

    /// 启用插件系统
    pub fn with_plugin(mut self) -> Self {
        self.enable_plugin = true;
        self
    }

    /// 启用政企适配
    pub fn with_enterprise(mut self) -> Self {
        self.enable_enterprise = true;
        self
    }

    /// 启用连接器
    pub fn with_connector(mut self) -> Self {
        self.enable_connector = true;
        self
    }

    /// 启用全部4大能力
    pub fn with_all_capabilities(mut self) -> Self {
        self.enable_ai = true;
        self.enable_plugin = true;
        self.enable_enterprise = true;
        self.enable_connector = true;
        self
    }

    /// 添加自定义扩展点
    pub fn with_extension(mut self, extension: crate::extension::ExtensionPoint) -> Self {
        self.custom_extensions.push(extension);
        self
    }

    /// 构建运行时（异步，因为需要初始化各能力）
    pub async fn build(self) -> anyhow::Result<IntegrationRuntime> {
        let config = self.config.unwrap_or_default();

        tracing::info!("building integration runtime: {}", config.runtime_name);
        tracing::info!("  environment: {}", config.environment);
        tracing::info!("  ai: {}, plugin: {}, enterprise: {}, connector: {}",
            self.enable_ai, self.enable_plugin, self.enable_enterprise, self.enable_connector);

        // 1. 创建核心组件
        let extensions = Arc::new(ExtensionRegistry::new());
        let coordinator = Arc::new(IntegrationCoordinator::new());
        let health_checker = Arc::new(IntegrationHealthChecker::new());
        // 工厂注册中心（零改动核心架构的关键）
        let factory_registry = Arc::new(FactoryRegistry::new());
        // 默认注册内置Factory（开箱即用）
        crate::builtin::register_all_builtin_factories(&factory_registry);
        // AI Provider注册表
        let ai_registry = Arc::new(crate::ai::registry::ProviderRegistry::new());
        // Connector注册表
        let connector_registry = Arc::new(crate::connector::registry::ConnectorRegistry::new());

        // 2. 注册自定义扩展点
        for ext in self.custom_extensions {
            if let Err(e) = extensions.register(ext) {
                tracing::warn!("failed to register custom extension: {}", e);
            }
        }

        // 3. 注册能力到协调器
        if self.enable_ai && config.ai.enabled {
            coordinator.register_capability(
                CapabilityHandle::new(CapabilityType::Ai, "AI Provider Gateway", "1.0.0")
                    .with_metadata("default_provider", &config.ai.default_provider)
                    .with_metadata("default_model", &config.ai.default_model)
            );
            // 注册AI扩展点
            let _ = extensions.register(crate::extension::ExtensionPoint::new(
                "integration.ai.gateway", "AI Gateway", crate::extension::ExtensionPointType::AiProvider, "1.0.0"
            ));
        }

        if self.enable_plugin && config.plugin.enabled {
            coordinator.register_capability(
                CapabilityHandle::new(CapabilityType::Plugin, "Plugin System", "1.0.0")
                    .with_metadata("plugin_dir", &config.plugin.plugin_dir)
                    .with_metadata("hot_reload", &config.plugin.hot_reload.to_string())
            );
            let _ = extensions.register(crate::extension::ExtensionPoint::new(
                "integration.plugin.system", "Plugin System", crate::extension::ExtensionPointType::Plugin, "1.0.0"
            ));
        }

        if self.enable_enterprise && config.enterprise.enabled {
            coordinator.register_capability(
                CapabilityHandle::new(CapabilityType::Enterprise, "Enterprise Adapter", "1.0.0")
                    .with_metadata("sso_enabled", &config.enterprise.sso.enabled.to_string())
                    .with_metadata("audit_enabled", &config.enterprise.compliance.audit_log_enabled.to_string())
            );
            let _ = extensions.register(crate::extension::ExtensionPoint::new(
                "integration.enterprise.adapter", "Enterprise Adapter", crate::extension::ExtensionPointType::SsoProvider, "1.0.0"
            ));
        }

        if self.enable_connector && config.connector.enabled {
            coordinator.register_capability(
                CapabilityHandle::new(CapabilityType::Connector, "Connector Framework", "1.0.0")
                    .with_metadata("connector_count", &config.connector.connectors.len().to_string())
            );
            let _ = extensions.register(crate::extension::ExtensionPoint::new(
                "integration.connector.framework", "Connector Framework", crate::extension::ExtensionPointType::Connector, "1.0.0"
            ));
        }

        // 4. 自动组装（从配置创建并注册所有实例，零改动核心架构的关键）
        let auto_assembler = AutoAssembler::new(factory_registry.clone());
        let auto_assembly = auto_assembler.assemble(
            &config, &ai_registry, &connector_registry, &extensions,
        ).await;
        if !auto_assembly.errors.is_empty() {
            tracing::warn!("auto-assembly had {} errors", auto_assembly.errors.len());
        }

        // 5. 发送启动事件
        coordinator.emit(
            "runtime.started",
            CapabilityType::Custom,
            serde_json::json!({
                "runtime": config.runtime_name,
                "environment": config.environment,
                "capabilities": {
                    "ai": self.enable_ai,
                    "plugin": self.enable_plugin,
                    "enterprise": self.enable_enterprise,
                    "connector": self.enable_connector,
                },
                "auto_assembly": {
                    "ai_created": auto_assembly.ai_created,
                    "connector_created": auto_assembly.connector_created,
                    "extension_created": auto_assembly.extension_created,
                }
            }),
        )?;

        tracing::info!("integration runtime built successfully: {}", config.runtime_name);

        Ok(IntegrationRuntime {
            config,
            extensions,
            coordinator,
            health_checker,
            factory_registry,
            ai_registry,
            connector_registry,
            auto_assembly: Some(auto_assembly),
            started: true,
        })
    }
}

impl Default for IntegrationRuntimeBuilder {
    fn default() -> Self { Self::new() }
}

/// 启动引导（便捷入口）
pub struct IntegrationBootstrap;

impl IntegrationBootstrap {
    /// 从配置文件快速启动全部能力
    pub async fn from_config_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<IntegrationRuntime> {
        let config = IntegrationConfig::load_from_file(path).await?;
        Self::from_config(config).await
    }

    /// 从配置快速启动全部能力
    pub async fn from_config(config: IntegrationConfig) -> anyhow::Result<IntegrationRuntime> {
        IntegrationRuntime::builder()
            .with_config(config)
            .with_all_capabilities()
            .build()
            .await
    }

    /// 快速启动（默认配置 + 全部能力）
    pub async fn quick_start() -> anyhow::Result<IntegrationRuntime> {
        Self::from_config(IntegrationConfig::default()).await
    }
}
