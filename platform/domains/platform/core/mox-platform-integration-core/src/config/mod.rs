// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一集成配置 — Integration Config
//!
//! 集中管理4大对接能力的配置，支持YAML/JSON文件加载、环境变量覆盖。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 顶层集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// 集成运行时名称
    #[serde(default = "default_runtime_name")]
    pub runtime_name: String,

    /// 环境（dev/staging/prod）
    #[serde(default = "default_env")]
    pub environment: String,

    /// AI集成配置
    #[serde(default)]
    pub ai: AiIntegrationConfig,

    /// 插件系统配置
    #[serde(default)]
    pub plugin: PluginIntegrationConfig,

    /// 政企适配配置
    #[serde(default)]
    pub enterprise: EnterpriseIntegrationConfig,

    /// 连接器配置
    #[serde(default)]
    pub connector: ConnectorIntegrationConfig,

    /// 扩展点配置（自定义扩展点列表）
    #[serde(default)]
    pub extensions: Vec<ExtensionPointConfig>,

    /// 全局超时（秒）
    #[serde(default = "default_timeout")]
    pub global_timeout_secs: u64,

    /// 是否启用遥测
    #[serde(default = "default_true")]
    pub telemetry_enabled: bool,
}

fn default_runtime_name() -> String { "mox-integration".into() }
fn default_env() -> String { "dev".into() }
fn default_timeout() -> u64 { 30 }
fn default_true() -> bool { true }

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            runtime_name: default_runtime_name(),
            environment: default_env(),
            ai: AiIntegrationConfig::default(),
            plugin: PluginIntegrationConfig::default(),
            enterprise: EnterpriseIntegrationConfig::default(),
            connector: ConnectorIntegrationConfig::default(),
            extensions: Vec::new(),
            global_timeout_secs: default_timeout(),
            telemetry_enabled: true,
        }
    }
}

impl IntegrationConfig {
    /// 从YAML文件加载
    pub async fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse integration config: {}", e))?;
        Ok(config)
    }

    /// 从YAML字符串加载
    pub fn from_yaml(yaml: &str) -> anyhow::Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 从JSON字符串加载
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 应用环境变量覆盖（MOX_INTEGRATION__前缀）
    pub fn apply_env_overrides(&mut self) {
        // 简化实现：只覆盖environment和runtime_name
        if let Ok(env) = std::env::var("MOX_INTEGRATION_ENV") {
            self.environment = env;
        }
        if let Ok(name) = std::env::var("MOX_INTEGRATION_NAME") {
            self.runtime_name = name;
        }
    }
}

/// AI集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiIntegrationConfig {
    /// 是否启用AI能力
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 默认Provider ID
    #[serde(default = "default_ai_provider")]
    pub default_provider: String,

    /// 默认模型
    #[serde(default = "default_ai_model")]
    pub default_model: String,

    /// 路由策略（round_robin / priority / latency / cost）
    #[serde(default = "default_ai_route")]
    pub routing_strategy: String,

    /// 是否启用自动降级
    #[serde(default = "default_true")]
    pub auto_fallback: bool,

    /// 是否启用熔断
    #[serde(default = "default_true")]
    pub circuit_breaker: bool,

    /// Provider配置列表
    #[serde(default)]
    pub providers: Vec<AiProviderConfig>,

    /// 全局请求超时（秒）
    #[serde(default = "default_ai_timeout")]
    pub request_timeout_secs: u64,

    /// 最大重试次数
    #[serde(default = "default_ai_retries")]
    pub max_retries: u32,
}

fn default_ai_provider() -> String { "openai".into() }
fn default_ai_model() -> String { "gpt-4o".into() }
fn default_ai_route() -> String { "priority".into() }
fn default_ai_timeout() -> u64 { 60 }
fn default_ai_retries() -> u32 { 2 }

impl Default for AiIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_provider: default_ai_provider(),
            default_model: default_ai_model(),
            routing_strategy: default_ai_route(),
            auto_fallback: true,
            circuit_breaker: true,
            providers: Vec::new(),
            request_timeout_secs: default_ai_timeout(),
            max_retries: default_ai_retries(),
        }
    }
}

/// AI Provider配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: String, // openai / anthropic / qwen / custom
    pub api_base: String,
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

fn default_priority() -> u32 { 100 }

/// 插件系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginIntegrationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 插件目录
    #[serde(default = "default_plugin_dir")]
    pub plugin_dir: String,

    /// 是否启用热重载
    #[serde(default = "default_true")]
    pub hot_reload: bool,

    /// 热重载检查间隔（秒）
    #[serde(default = "default_hot_reload_interval")]
    pub hot_reload_interval_secs: u64,

    /// 插件市场配置
    #[serde(default)]
    pub market: PluginMarketConfig,

    /// 最大插件数量
    #[serde(default = "default_max_plugins")]
    pub max_plugins: u32,

    /// 单个插件最大内存（MB）
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u32,
}

fn default_plugin_dir() -> String { "./plugins".into() }
fn default_hot_reload_interval() -> u64 { 10 }
fn default_max_plugins() -> u32 { 100 }
fn default_max_memory() -> u32 { 256 }

impl Default for PluginIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugin_dir: default_plugin_dir(),
            hot_reload: true,
            hot_reload_interval_secs: default_hot_reload_interval(),
            market: PluginMarketConfig::default(),
            max_plugins: default_max_plugins(),
            max_memory_mb: default_max_memory(),
        }
    }
}

/// 插件市场配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_token: String,
    #[serde(default = "default_true")]
    pub auto_update_check: bool,
    #[serde(default = "default_update_interval")]
    pub update_check_interval_hours: u64,
}

fn default_update_interval() -> u64 { 24 }

impl Default for PluginMarketConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            api_token: String::new(),
            auto_update_check: true,
            update_check_interval_hours: default_update_interval(),
        }
    }
}

/// 政企适配配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseIntegrationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// SSO配置
    #[serde(default)]
    pub sso: SsoConfig,

    /// 合规配置
    #[serde(default)]
    pub compliance: ComplianceConfig,

    /// 白标定制配置
    #[serde(default)]
    pub whitelabel: WhitelabelConfig,
}

impl Default for EnterpriseIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sso: SsoConfig::default(),
            compliance: ComplianceConfig::default(),
            whitelabel: WhitelabelConfig::default(),
        }
    }
}

/// SSO配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SsoConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub default_provider: String,
    #[serde(default)]
    pub providers: Vec<HashMap<String, String>>,
}

/// 合规配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    #[serde(default = "default_true")]
    pub audit_log_enabled: bool,
    #[serde(default = "default_true")]
    pub data_masking_enabled: bool,
    #[serde(default)]
    pub data_residency_region: String,
    #[serde(default = "default_true")]
    pub cross_border_control: bool,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self {
            audit_log_enabled: true,
            data_masking_enabled: true,
            data_residency_region: String::new(),
            cross_border_control: true,
        }
    }
}

/// 白标定制配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhitelabelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub brand_name: String,
    #[serde(default)]
    pub logo_url: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub custom_fields: Vec<HashMap<String, String>>,
}

/// 连接器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorIntegrationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 连接器配置列表
    #[serde(default)]
    pub connectors: Vec<ConnectorConfig>,

    /// 全局超时（秒）
    #[serde(default = "default_connector_timeout")]
    pub global_timeout_secs: u64,

    /// 全局重试次数
    #[serde(default = "default_connector_retries")]
    pub global_max_retries: u32,
}

fn default_connector_timeout() -> u64 { 30 }
fn default_connector_retries() -> u32 { 2 }

impl Default for ConnectorIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            connectors: Vec::new(),
            global_timeout_secs: default_connector_timeout(),
            global_max_retries: default_connector_retries(),
        }
    }
}

/// 单个连接器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    pub name: String,
    pub connector_type: String,
    pub protocol: String,
    pub endpoint: String,
    #[serde(default)]
    pub auth_type: String,
    #[serde(default)]
    pub credentials: HashMap<String, String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// 扩展点配置（用于配置文件中声明自定义扩展点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPointConfig {
    pub id: String,
    pub name: String,
    pub extension_type: String,
    #[serde(default = "default_ext_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

fn default_ext_version() -> String { "1.0.0".into() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IntegrationConfig::default();
        assert_eq!(config.runtime_name, "mox-integration");
        assert_eq!(config.environment, "dev");
        assert!(config.ai.enabled);
        assert!(config.plugin.enabled);
        assert!(config.enterprise.enabled);
        assert!(config.connector.enabled);
    }

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
runtime_name: test-runtime
environment: staging
ai:
  default_provider: qwen
  default_model: qwen-max
plugin:
  plugin_dir: /data/plugins
"#;
        let config = IntegrationConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.runtime_name, "test-runtime");
        assert_eq!(config.environment, "staging");
        assert_eq!(config.ai.default_provider, "qwen");
        assert_eq!(config.plugin.plugin_dir, "/data/plugins");
    }
}
