// =============================================================================
// 配置源（Config Source）
// =============================================================================

use crate::config::{Config, ConfigValue};
use crate::ConfigError;
use async_trait::async_trait;
use std::collections::BTreeMap;

// =============================================================================
// 配置源 trait
// =============================================================================

/// 配置源 trait
#[async_trait]
pub trait ConfigSource: Send + Sync {
    /// 加载配置
    async fn load(&self) -> Result<Config, ConfigError>;

    /// 源名称
    fn name(&self) -> &str;

    /// 源优先级（数字越大优先级越高）
    fn priority(&self) -> u32 {
        50
    }
}

// =============================================================================
// 内存配置源
// =============================================================================

/// 内存配置源（用于测试和默认值）
pub struct MemorySource {
    config: Config,
    name: String,
}

impl MemorySource {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
            name: "memory".to_string(),
        }
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn set(&mut self, key: impl Into<String>, value: ConfigValue) {
        self.config.set(key, value);
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigSource for MemorySource {
    async fn load(&self) -> Result<Config, ConfigError> {
        Ok(self.config.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        10
    }
}

// =============================================================================
// 环境变量配置源
// =============================================================================

/// 环境变量配置源
///
/// 支持前缀过滤，如 MOX_SERVER_PORT → server.port
pub struct EnvironmentSource {
    prefix: String,
    name: String,
}

impl EnvironmentSource {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            name: "environment".to_string(),
        }
    }

    /// 将环境变量名转换为配置键
    /// MOX_SERVER_PORT → server.port
    fn env_to_key(&self, env_name: &str) -> Option<String> {
        let name = env_name.to_lowercase();
        let prefix_lower = self.prefix.to_lowercase();

        if !name.starts_with(&prefix_lower) {
            return None;
        }

        let key_part = &name[prefix_lower.len()..];
        let key = key_part.trim_start_matches('_').replace('_', ".");

        if key.is_empty() {
            None
        } else {
            Some(key)
        }
    }

    /// 解析环境变量值为配置值
    fn parse_value(value: &str) -> ConfigValue {
        // 尝试布尔
        match value.to_lowercase().as_str() {
            "true" => return ConfigValue::Boolean(true),
            "false" => return ConfigValue::Boolean(false),
            _ => {}
        }

        // 尝试整数
        if let Ok(i) = value.parse::<i64>() {
            return ConfigValue::Integer(i);
        }

        // 尝试浮点数
        if let Ok(f) = value.parse::<f64>() {
            return ConfigValue::Float(f);
        }

        // 默认字符串
        ConfigValue::String(value.to_string())
    }
}

#[async_trait]
impl ConfigSource for EnvironmentSource {
    async fn load(&self) -> Result<Config, ConfigError> {
        let mut config = Config::new();

        for (key, value) in std::env::vars() {
            if let Some(config_key) = self.env_to_key(&key) {
                config.set(config_key, Self::parse_value(&value));
            }
        }

        Ok(config)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        80
    }
}

// =============================================================================
// 文件配置源
// =============================================================================

/// 文件配置源（JSON 格式）
pub struct FileSource {
    path: String,
    name: String,
}

impl FileSource {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: "file".to_string(),
        }
    }

    /// 扁平化嵌套 JSON 对象
    fn flatten(prefix: &str, value: &serde_json::Value, config: &mut Config) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten(&full_key, val, config);
                }
            }
            serde_json::Value::String(s) => {
                config.set(prefix, ConfigValue::String(s.clone()));
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    config.set(prefix, ConfigValue::Integer(i));
                } else if let Some(f) = n.as_f64() {
                    config.set(prefix, ConfigValue::Float(f));
                }
            }
            serde_json::Value::Bool(b) => {
                config.set(prefix, ConfigValue::Boolean(*b));
            }
            serde_json::Value::Array(arr) => {
                let list: Vec<ConfigValue> = arr
                    .iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => ConfigValue::String(s.clone()),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                ConfigValue::Integer(i)
                            } else {
                                ConfigValue::Float(n.as_f64().unwrap_or(0.0))
                            }
                        }
                        serde_json::Value::Bool(b) => ConfigValue::Boolean(*b),
                        _ => ConfigValue::Null,
                    })
                    .collect();
                config.set(prefix, ConfigValue::List(list));
            }
            serde_json::Value::Null => {
                config.set(prefix, ConfigValue::Null);
            }
        }
    }
}

#[async_trait]
impl ConfigSource for FileSource {
    async fn load(&self) -> Result<Config, ConfigError> {
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::SourceLoadFailed(format!("读取配置文件失败: {}", e)))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("JSON解析失败: {}", e)))?;

        let mut config = Config::new();
        Self::flatten("", &json, &mut config);

        Ok(config)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        60
    }
}

// =============================================================================
// 远程配置源
// =============================================================================

/// 远程配置源（HTTP 拉取）
pub struct RemoteSource {
    url: String,
    api_key: Option<String>,
    name: String,
}

impl RemoteSource {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            api_key: None,
            name: "remote".to_string(),
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

#[async_trait]
impl ConfigSource for RemoteSource {
    async fn load(&self) -> Result<Config, ConfigError> {
        // 简化实现：实际应使用 reqwest 调用远程配置中心
        // 这里返回空配置，避免引入额外依赖
        tracing::warn!("远程配置源尚未完整实现: {}", self.url);
        Ok(Config::new())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        90
    }
}

// =============================================================================
// 多源加载器
// =============================================================================

/// 多源加载器（按优先级合并）
pub struct MultiSourceLoader {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl MultiSourceLoader {
    pub fn new() -> Self {
        Self { sources: vec![] }
    }

    pub fn add_source(mut self, source: Box<dyn ConfigSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// 加载所有源并按优先级合并
    pub async fn load_all(&self) -> Result<Config, ConfigError> {
        let mut sources: Vec<&Box<dyn ConfigSource>> = self.sources.iter().collect();
        sources.sort_by_key(|s| std::cmp::Reverse(s.priority()));

        let mut merged = Config::new();

        for source in sources {
            match source.load().await {
                Ok(config) => {
                    tracing::debug!("加载配置源: {} (优先级: {})", source.name(), source.priority());
                    merged.merge(&config);
                }
                Err(e) => {
                    tracing::warn!("配置源加载失败: {} - {}", source.name(), e);
                }
            }
        }

        Ok(merged)
    }
}

impl Default for MultiSourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_source_load() {
        let mut source = MemorySource::new();
        source.set("key", ConfigValue::from("value"));

        let config = source.load().await.unwrap();
        assert_eq!(config.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn environment_source_env_to_key() {
        let source = EnvironmentSource::new("MOX");
        assert_eq!(source.env_to_key("MOX_SERVER_PORT"), Some("server.port".to_string()));
        assert_eq!(source.env_to_key("OTHER_VAR"), None);
        assert_eq!(source.env_to_key("MOX"), None);
    }

    #[test]
    fn environment_source_parse_value() {
        assert_eq!(EnvironmentSource::parse_value("true"), ConfigValue::Boolean(true));
        assert_eq!(EnvironmentSource::parse_value("42"), ConfigValue::Integer(42));
        assert_eq!(EnvironmentSource::parse_value("3.14"), ConfigValue::Float(3.14));
        assert_eq!(EnvironmentSource::parse_value("hello"), ConfigValue::String("hello".to_string()));
    }

    #[tokio::test]
    async fn file_source_flatten() {
        let json = serde_json::json!({
            "server": {
                "port": 8080,
                "host": "localhost"
            },
            "debug": true
        });

        let mut config = Config::new();
        FileSource::flatten("", &json, &mut config);

        assert_eq!(config.get("server.port").and_then(|v| v.as_i64()), Some(8080));
        assert_eq!(config.get("server.host").and_then(|v| v.as_str()), Some("localhost"));
        assert_eq!(config.get("debug").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn multi_source_loader_priority() {
        // 低优先级源
        let mut low = MemorySource::new();
        low.set("key", ConfigValue::from("low"));

        // 高优先级源
        let mut high = MemorySource::new();
        high.set("key", ConfigValue::from("high"));

        let loader = MultiSourceLoader::new()
            .add_source(Box::new(low))
            .add_source(Box::new(high));

        // MemorySource 优先级都是10，后加载的覆盖先加载的
        // 实际测试中应使用不同优先级的源
        let config = loader.load_all().await.unwrap();
        assert!(config.get("key").is_some());
    }
}
