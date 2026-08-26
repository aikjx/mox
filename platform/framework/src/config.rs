//! 统一配置管理 — 支持 YAML/JSON/TOML/环境变量，零配置默认值

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 框架统一配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    /// 服务名称
    pub service_name: String,
    /// 服务版本
    pub service_version: String,
    /// 监听地址
    pub listen_addr: String,
    /// gRPC 监听地址
    pub grpc_addr: String,
    /// 日志级别
    pub log_level: String,
    /// 日志格式 (json/text)
    pub log_format: String,
    /// 环境 (dev/staging/prod)
    pub environment: String,
    /// 多租户模式 (none/logical/schema/cluster)
    pub tenant_mode: String,
    /// 认证配置
    pub auth: AuthConfig,
    /// 弹性配置
    pub resilience: ResilienceConfig,
    /// 可观测性配置
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_secs: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub rate_limit_per_sec: u32,
    pub circuit_breaker_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub health_enabled: bool,
    pub otel_endpoint: Option<String>,
}

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            service_name: "mox-service".into(),
            service_version: "1.0.0".into(),
            listen_addr: "0.0.0.0:8080".into(),
            grpc_addr: "0.0.0.0:50051".into(),
            log_level: "info".into(),
            log_format: "json".into(),
            environment: "dev".into(),
            tenant_mode: "logical".into(),
            auth: AuthConfig::default(),
            resilience: ResilienceConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".into(),
            jwt_expiry_secs: 86400,
            enabled: true,
        }
    }
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            rate_limit_per_sec: 1000,
            circuit_breaker_threshold: 0.5,
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            tracing_enabled: true,
            health_enabled: true,
            otel_endpoint: None,
        }
    }
}

impl FrameworkConfig {
    /// 从文件加载配置（支持 yaml/json/toml）
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::from(path.as_ref()).required(false))
            .add_source(config::Environment::with_prefix("MOX").separator("__"));
        builder.build()?.try_deserialize()
    }

    /// 从环境变量加载（零配置，使用默认值+环境变量覆盖）
    pub fn from_env() -> Self {
        config::Config::builder()
            .add_source(config::Environment::with_prefix("MOX").separator("__"))
            .build()
            .and_then(|c| c.try_deserialize())
            .unwrap_or_default()
    }
}
