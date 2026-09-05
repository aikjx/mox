// =============================================================================
// 统一服务配置（ServerConfig）
// =============================================================================
//
// 三级配置加载：默认值 < TOML 配置文件 < 环境变量
// 环境变量前缀：MOX_，如 MOX_SERVER_PORT、MOX_DATABASE_URL
// =============================================================================

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 服务端配置（所有独立服务统一使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 服务元信息
    #[serde(default)]
    pub server: ServerMeta,
    /// 数据库配置
    #[serde(default)]
    pub database: DatabaseConfig,
    /// 缓存配置
    #[serde(default)]
    pub cache: CacheConfig,
    /// 认证配置
    #[serde(default)]
    pub auth: AuthConfig,
    /// 可观测性配置
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

/// 服务元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMeta {
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,
    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,
    /// 工作线程数（0 = CPU 核数）
    #[serde(default)]
    pub workers: usize,
    /// 请求体大小限制（字节）
    #[serde(default = "default_body_limit")]
    pub body_limit: usize,
    /// 请求超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for ServerMeta {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: 0,
            body_limit: default_body_limit(),
            timeout_secs: default_timeout(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_body_limit() -> usize {
    10 * 1024 * 1024 // 10MB
}
fn default_timeout() -> u64 {
    30
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 连接 URL（sqlite:///path 或 postgres://user:pass@host/db 或 mysql://...）
    #[serde(default = "default_db_url")]
    pub url: String,
    /// 最大连接数
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    /// 最小连接数
    #[serde(default)]
    pub min_connections: u32,
    /// 连接超时（秒）
    #[serde(default = "default_conn_timeout")]
    pub connect_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_db_url(),
            max_connections: default_max_conn(),
            min_connections: 0,
            connect_timeout_secs: default_conn_timeout(),
        }
    }
}

fn default_db_url() -> String {
    "sqlite:///var/lib/mox/service.db".to_string()
}
fn default_max_conn() -> u32 {
    10
}
fn default_conn_timeout() -> u64 {
    5
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 缓存后端：memory / redis / none
    #[serde(default = "default_cache_backend")]
    pub backend: String,
    /// Redis 连接 URL（backend=redis 时使用）
    #[serde(default)]
    pub redis_url: String,
    /// L1 内存缓存最大容量
    #[serde(default = "default_l1_capacity")]
    pub l1_max_capacity: usize,
    /// L1 默认 TTL（秒）
    #[serde(default = "default_l1_ttl")]
    pub l1_default_ttl_secs: u64,
    /// 缓存 key 前缀
    #[serde(default)]
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            backend: default_cache_backend(),
            redis_url: String::new(),
            l1_max_capacity: default_l1_capacity(),
            l1_default_ttl_secs: default_l1_ttl(),
            key_prefix: String::new(),
        }
    }
}

fn default_cache_backend() -> String {
    "memory".to_string()
}
fn default_l1_capacity() -> usize {
    10_000
}
fn default_l1_ttl() -> u64 {
    300
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 是否启用认证
    #[serde(default = "default_auth_enabled")]
    pub enabled: bool,
    /// JWT 密钥（环境变量 MOX_JWT_SECRET 优先）
    #[serde(default)]
    pub jwt_secret: String,
    /// JWT 签发者
    #[serde(default = "default_jwt_issuer")]
    pub jwt_issuer: String,
    /// JWT 有效期（秒）
    #[serde(default = "default_jwt_ttl")]
    pub jwt_ttl_secs: u64,
    /// 公钥路径（用于验证外部 JWT）
    #[serde(default)]
    pub public_key_path: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_auth_enabled(),
            jwt_secret: String::new(),
            jwt_issuer: default_jwt_issuer(),
            jwt_ttl_secs: default_jwt_ttl(),
            public_key_path: String::new(),
        }
    }
}

fn default_auth_enabled() -> bool {
    true
}
fn default_jwt_issuer() -> String {
    "mox-platform".to_string()
}
fn default_jwt_ttl() -> u64 {
    3600
}

/// 可观测性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// 日志级别：trace/debug/info/warn/error
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 是否使用 JSON 格式日志
    #[serde(default = "default_json_log")]
    pub json_log: bool,
    /// Prometheus 指标端口（0 = 与主端口共用 /metrics 路径）
    #[serde(default)]
    pub metrics_port: u16,
    /// 是否启用分布式追踪
    #[serde(default = "default_tracing_enabled")]
    pub tracing_enabled: bool,
    /// 采样率（0.0 ~ 1.0）
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            json_log: default_json_log(),
            metrics_port: 0,
            tracing_enabled: default_tracing_enabled(),
            sample_rate: default_sample_rate(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_json_log() -> bool {
    true
}
fn default_tracing_enabled() -> bool {
    true
}
fn default_sample_rate() -> f64 {
    0.1
}

impl ServerConfig {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取配置文件 {} 失败: {e}", path.display()))?;
        let config: ServerConfig =
            toml::from_str(&content).map_err(|e| format!("解析配置文件失败: {e}"))?;
        Ok(config)
    }

    /// 从环境变量覆盖配置（MOX_ 前缀）
    pub fn apply_env_overrides(&mut self) {
        if let Ok(port) = std::env::var("MOX_SERVER_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                self.server.port = p;
            }
        }
        if let Ok(host) = std::env::var("MOX_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(url) = std::env::var("MOX_DATABASE_URL") {
            self.database.url = url;
        }
        if let Ok(url) = std::env::var("MOX_REDIS_URL") {
            self.cache.redis_url = url;
        }
        if let Ok(secret) = std::env::var("MOX_JWT_SECRET") {
            self.auth.jwt_secret = secret;
        }
        if let Ok(level) = std::env::var("MOX_LOG_LEVEL") {
            self.observability.log_level = level;
        }
    }

    /// 获取监听地址
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerMeta::default(),
            database: DatabaseConfig::default(),
            cache: CacheConfig::default(),
            auth: AuthConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
        assert!(config.auth.enabled);
        assert_eq!(config.cache.backend, "memory");
    }

    #[test]
    fn test_listen_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.listen_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_env_override() {
        std::env::set_var("MOX_SERVER_PORT", "9999");
        let mut config = ServerConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.server.port, 9999);
        std::env::remove_var("MOX_SERVER_PORT");
    }
}
