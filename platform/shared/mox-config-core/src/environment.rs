// =============================================================================
// 环境隔离（Environment）
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// 环境枚举
// =============================================================================

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// 开发环境
    Dev,
    /// 测试环境
    Test,
    /// 预发布环境
    Staging,
    /// 生产环境
    Prod,
}

impl Environment {
    /// 环境名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Dev => "dev",
            Environment::Test => "test",
            Environment::Staging => "staging",
            Environment::Prod => "prod",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Some(Environment::Dev),
            "test" | "testing" => Some(Environment::Test),
            "staging" | "pre" | "preprod" => Some(Environment::Staging),
            "prod" | "production" => Some(Environment::Prod),
            _ => None,
        }
    }

    /// 是否生产环境
    pub fn is_prod(&self) -> bool {
        matches!(self, Environment::Prod)
    }

    /// 是否开发环境
    pub fn is_dev(&self) -> bool {
        matches!(self, Environment::Dev)
    }

    /// 获取环境配置文件路径
    pub fn config_path(&self, base_path: &str) -> String {
        format!("{}/config.{}.json", base_path, self.as_str())
    }

    /// 获取环境变量前缀
    pub fn env_prefix(&self) -> String {
        format!("MOX_{}_", self.as_str().to_uppercase())
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Dev
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// 环境配置
// =============================================================================

/// 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// 当前环境
    pub environment: Environment,
    /// 应用名称
    pub app_name: String,
    /// 应用版本
    pub app_version: String,
    /// 日志级别
    pub log_level: String,
    /// 是否启用调试模式
    pub debug: bool,
    /// 是否启用详细日志
    pub verbose: bool,
    /// 数据目录
    pub data_dir: String,
    /// 临时目录
    pub temp_dir: String,
}

impl EnvironmentConfig {
    /// 创建新的环境配置
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            app_name: "mox-platform".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            log_level: if environment.is_prod() { "info" } else { "debug" }.to_string(),
            debug: !environment.is_prod(),
            verbose: environment.is_dev(),
            data_dir: format!("./data/{}", environment.as_str()),
            temp_dir: "./temp".to_string(),
        }
    }

    /// 从环境变量加载
    pub fn from_env() -> Self {
        let env_str = std::env::var("MOX_ENV").unwrap_or_else(|_| "dev".to_string());
        let environment = Environment::from_str(&env_str).unwrap_or_default();

        let mut config = Self::new(environment);

        if let Ok(app_name) = std::env::var("MOX_APP_NAME") {
            config.app_name = app_name;
        }
        if let Ok(log_level) = std::env::var("MOX_LOG_LEVEL") {
            config.log_level = log_level;
        }
        if let Ok(data_dir) = std::env::var("MOX_DATA_DIR") {
            config.data_dir = data_dir;
        }

        config
    }

    /// 获取环境特定的配置键前缀
    pub fn config_prefix(&self) -> String {
        format!("{}.", self.environment.as_str())
    }

    /// 检查配置键是否属于当前环境
    pub fn is_env_specific(&self, key: &str) -> bool {
        key.starts_with(&self.config_prefix())
    }

    /// 去除环境前缀
    pub fn strip_env_prefix<'a>(&self, key: &'a str) -> &'a str {
        let prefix = self.config_prefix();
        if key.starts_with(&prefix) {
            &key[prefix.len()..]
        } else {
            key
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self::new(Environment::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_as_str() {
        assert_eq!(Environment::Dev.as_str(), "dev");
        assert_eq!(Environment::Test.as_str(), "test");
        assert_eq!(Environment::Staging.as_str(), "staging");
        assert_eq!(Environment::Prod.as_str(), "prod");
    }

    #[test]
    fn environment_from_str() {
        assert_eq!(Environment::from_str("dev"), Some(Environment::Dev));
        assert_eq!(Environment::from_str("development"), Some(Environment::Dev));
        assert_eq!(Environment::from_str("prod"), Some(Environment::Prod));
        assert_eq!(Environment::from_str("production"), Some(Environment::Prod));
        assert_eq!(Environment::from_str("unknown"), None);
    }

    #[test]
    fn environment_is_prod() {
        assert!(Environment::Prod.is_prod());
        assert!(!Environment::Dev.is_prod());
        assert!(!Environment::Staging.is_prod());
    }

    #[test]
    fn environment_config_path() {
        assert_eq!(
            Environment::Dev.config_path("/etc/mox"),
            "/etc/mox/config.dev.json"
        );
        assert_eq!(
            Environment::Prod.config_path("/etc/mox"),
            "/etc/mox/config.prod.json"
        );
    }

    #[test]
    fn environment_config_new() {
        let config = EnvironmentConfig::new(Environment::Prod);
        assert_eq!(config.environment, Environment::Prod);
        assert_eq!(config.log_level, "info");
        assert!(!config.debug);
        assert_eq!(config.data_dir, "./data/prod");
    }

    #[test]
    fn environment_config_dev() {
        let config = EnvironmentConfig::new(Environment::Dev);
        assert_eq!(config.log_level, "debug");
        assert!(config.debug);
        assert!(config.verbose);
    }

    #[test]
    fn environment_config_prefix() {
        let config = EnvironmentConfig::new(Environment::Dev);
        assert_eq!(config.config_prefix(), "dev.");
        assert!(config.is_env_specific("dev.server.port"));
        assert!(!config.is_env_specific("server.port"));
        assert_eq!(config.strip_env_prefix("dev.server.port"), "server.port");
    }
}
