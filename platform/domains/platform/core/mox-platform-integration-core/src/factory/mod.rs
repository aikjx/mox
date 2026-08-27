// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 统一工厂注册中心 — Factory Registry
//!
//! 企业级"零改动核心架构"的核心机制。
//! 新增扩展只需：实现Factory + 注册 + 加配置，核心代码零改动。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// ─── 工厂配置（通用）─────────────────────────────────────────────────────────

/// 工厂配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    pub id: String,
    pub name: String,
    pub factory_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_true() -> bool { true }
fn default_priority() -> u32 { 100 }

impl FactoryConfig {
    pub fn parse_config<T: serde::de::DeserializeOwned>(&self) -> anyhow::Result<T> {
        serde_json::from_value(self.config.clone())
            .map_err(|e| anyhow::anyhow!("parse config for {}: {}", self.id, e))
    }
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(|v| v.as_str())
    }
}

// ─── 具体Factory trait（非泛型，避免trait object限制）────────────────────────

/// AI Provider工厂trait
#[async_trait]
pub trait AiProviderFactory: Send + Sync {
    fn factory_type(&self) -> &'static str;
    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn crate::ai::providers::traits::AiProvider>>;
}

/// Connector工厂trait
#[async_trait]
pub trait ConnectorFactory: Send + Sync {
    fn factory_type(&self) -> &'static str;
    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn crate::connector::traits::Connector>>;
}

/// SSO工厂trait
#[async_trait]
pub trait SsoFactory: Send + Sync {
    fn factory_type(&self) -> &'static str;
    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn crate::enterprise::sso::SsoProvider>>;
}

/// 扩展点工厂trait
#[async_trait]
pub trait ExtensionFactory: Send + Sync {
    fn factory_type(&self) -> &'static str;
    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<crate::extension::ExtensionPoint>;
}

// ─── 工厂注册中心 ────────────────────────────────────────────────────────────

/// 工厂注册中心
pub struct FactoryRegistry {
    ai_factories: RwLock<HashMap<String, Arc<dyn AiProviderFactory>>>,
    connector_factories: RwLock<HashMap<String, Arc<dyn ConnectorFactory>>>,
    sso_factories: RwLock<HashMap<String, Arc<dyn SsoFactory>>>,
    extension_factories: RwLock<HashMap<String, Arc<dyn ExtensionFactory>>>,
}

impl FactoryRegistry {
    pub fn new() -> Self {
        Self {
            ai_factories: RwLock::new(HashMap::new()),
            connector_factories: RwLock::new(HashMap::new()),
            sso_factories: RwLock::new(HashMap::new()),
            extension_factories: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_ai_factory(&self, f: Arc<dyn AiProviderFactory>) {
        let ft = f.factory_type().to_string();
        tracing::info!("register AI factory: {}", ft);
        self.ai_factories.write().insert(ft, f);
    }
    pub fn register_connector_factory(&self, f: Arc<dyn ConnectorFactory>) {
        let ft = f.factory_type().to_string();
        tracing::info!("register connector factory: {}", ft);
        self.connector_factories.write().insert(ft, f);
    }
    pub fn register_sso_factory(&self, f: Arc<dyn SsoFactory>) {
        let ft = f.factory_type().to_string();
        tracing::info!("register SSO factory: {}", ft);
        self.sso_factories.write().insert(ft, f);
    }
    pub fn register_extension_factory(&self, f: Arc<dyn ExtensionFactory>) {
        let ft = f.factory_type().to_string();
        tracing::info!("register extension factory: {}", ft);
        self.extension_factories.write().insert(ft, f);
    }

    pub fn get_ai_factory(&self, ft: &str) -> Option<Arc<dyn AiProviderFactory>> {
        self.ai_factories.read().get(ft).cloned()
    }
    pub fn get_connector_factory(&self, ft: &str) -> Option<Arc<dyn ConnectorFactory>> {
        self.connector_factories.read().get(ft).cloned()
    }
    pub fn get_sso_factory(&self, ft: &str) -> Option<Arc<dyn SsoFactory>> {
        self.sso_factories.read().get(ft).cloned()
    }

    pub fn list_types(&self) -> Vec<String> {
        let mut v = Vec::new();
        v.extend(self.ai_factories.read().keys().map(|k| format!("ai:{}", k)));
        v.extend(self.connector_factories.read().keys().map(|k| format!("connector:{}", k)));
        v.extend(self.sso_factories.read().keys().map(|k| format!("sso:{}", k)));
        v.extend(self.extension_factories.read().keys().map(|k| format!("ext:{}", k)));
        v
    }
}

impl Default for FactoryRegistry {
    fn default() -> Self { Self::new() }
}

// ─── 自动组装器 ───────────────────────────────────────────────────────────────

/// 自动组装结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoAssemblyResult {
    pub ai_created: u32,
    pub ai_failed: u32,
    pub connector_created: u32,
    pub connector_failed: u32,
    pub sso_created: u32,
    pub sso_failed: u32,
    pub extension_created: u32,
    pub extension_failed: u32,
    pub errors: Vec<String>,
}

/// 自动组装器 — 从配置自动创建并注册所有实例
pub struct AutoAssembler {
    factory_registry: Arc<FactoryRegistry>,
}

impl AutoAssembler {
    pub fn new(fr: Arc<FactoryRegistry>) -> Self { Self { factory_registry: fr } }

    pub async fn assemble(
        &self,
        config: &crate::config::IntegrationConfig,
        ai_registry: &crate::ai::registry::ProviderRegistry,
        connector_registry: &crate::connector::registry::ConnectorRegistry,
        extension_registry: &crate::extension::ExtensionRegistry,
    ) -> AutoAssemblyResult {
        let mut r = AutoAssemblyResult::default();

        // AI Provider自动组装
        for pc in &config.ai.providers {
            if !pc.enabled { continue; }
            if let Some(factory) = self.factory_registry.get_ai_factory(&pc.provider_type) {
                let fc = FactoryConfig {
                    id: pc.id.clone(), name: pc.name.clone(),
                    factory_type: pc.provider_type.clone(),
                    enabled: pc.enabled, priority: pc.priority,
                    config: serde_json::json!({
                        "api_base": pc.api_base, "api_key": pc.api_key,
                        "models": pc.models, "extra": pc.extra,
                    }),
                    metadata: HashMap::new(),
                };
                match factory.create(&fc).await {
                    Ok(p) => { ai_registry.register(p); r.ai_created += 1; }
                    Err(e) => { r.ai_failed += 1; r.errors.push(format!("AI {}: {}", pc.id, e)); }
                }
            } else {
                r.ai_failed += 1;
                r.errors.push(format!("AI factory not found: {}", pc.provider_type));
            }
        }

        // Connector自动组装
        for cc in &config.connector.connectors {
            if !cc.enabled { continue; }
            if let Some(factory) = self.factory_registry.get_connector_factory(&cc.connector_type) {
                let fc = FactoryConfig {
                    id: cc.id.clone(), name: cc.name.clone(),
                    factory_type: cc.connector_type.clone(),
                    enabled: cc.enabled, priority: 100,
                    config: serde_json::json!({
                        "protocol": cc.protocol, "endpoint": cc.endpoint,
                        "auth_type": cc.auth_type, "credentials": cc.credentials,
                        "headers": cc.headers, "extra": cc.extra,
                    }),
                    metadata: HashMap::new(),
                };
                match factory.create(&fc).await {
                    Ok(c) => { connector_registry.register(c); r.connector_created += 1; }
                    Err(e) => { r.connector_failed += 1; r.errors.push(format!("Connector {}: {}", cc.id, e)); }
                }
            } else {
                r.connector_failed += 1;
                r.errors.push(format!("Connector factory not found: {}", cc.connector_type));
            }
        }

        // 扩展点自动组装（直接从配置创建，简化版）
        for ec in &config.extensions {
            if !ec.enabled { continue; }
            let ext = crate::extension::ExtensionPointBuilder::new(
                ec.id.clone(), ec.name.clone(), crate::extension::ExtensionPointType::Custom,
            )
            .version(ec.version.clone())
            .description(ec.description.clone())
            .build();
            match extension_registry.register(ext) {
                Ok(_) => { r.extension_created += 1; }
                Err(e) => { r.extension_failed += 1; r.errors.push(format!("Ext {}: {}", ec.id, e)); }
            }
        }

        tracing::info!("auto-assembly: AI+{} Connector+{} Ext+{}", r.ai_created, r.connector_created, r.extension_created);
        r
    }
}
