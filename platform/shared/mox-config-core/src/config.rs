// =============================================================================
// 配置模型（Config Model）
// =============================================================================

use crate::ConfigError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

// =============================================================================
// 配置值
// =============================================================================

/// 配置值（支持多种类型）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ConfigValue {
    /// 字符串
    String(String),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 列表
    List(Vec<ConfigValue>),
    /// 对象（嵌套配置）
    Object(BTreeMap<String, ConfigValue>),
    /// 空值
    Null,
}

impl ConfigValue {
    /// 获取字符串值
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 获取整数值
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            ConfigValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// 获取浮点数值
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// 获取列表
    pub fn as_list(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// 获取对象
    pub fn as_object(&self) -> Option<&BTreeMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// 类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            ConfigValue::String(_) => "string",
            ConfigValue::Integer(_) => "integer",
            ConfigValue::Float(_) => "float",
            ConfigValue::Boolean(_) => "boolean",
            ConfigValue::List(_) => "list",
            ConfigValue::Object(_) => "object",
            ConfigValue::Null => "null",
        }
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        ConfigValue::String(s)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue::String(s.to_string())
    }
}

impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self {
        ConfigValue::Integer(i)
    }
}

impl From<f64> for ConfigValue {
    fn from(f: f64) -> Self {
        ConfigValue::Float(f)
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        ConfigValue::Boolean(b)
    }
}

// =============================================================================
// 配置键
// =============================================================================

/// 配置键（点分隔，如 "server.port"）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfigKey(String);

impl ConfigKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 拆分为路径段
    pub fn parts(&self) -> Vec<&str> {
        self.0.split('.').collect()
    }

    /// 拼接子键
    pub fn join(&self, sub: &str) -> ConfigKey {
        ConfigKey(format!("{}.{}", self.0, sub))
    }
}

impl std::fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ConfigKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ConfigKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// =============================================================================
// 配置快照
// =============================================================================

/// 配置快照（不可变的配置视图）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// 配置数据
    pub data: BTreeMap<String, ConfigValue>,
    /// 快照版本
    pub version: u64,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 环境
    pub environment: String,
}

impl ConfigSnapshot {
    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        // 先尝试直接查找
        if let Some(value) = self.data.get(key) {
            return Some(value);
        }

        // 再尝试嵌套查找
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() > 1 {
            let mut current = self.data.get(parts[0]);
            for part in &parts[1..] {
                match current {
                    Some(ConfigValue::Object(obj)) => {
                        current = obj.get(*part);
                    }
                    _ => return None,
                }
            }
            return current;
        }

        None
    }

    /// 获取字符串配置
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// 获取整数配置
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    /// 获取浮点数配置
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }

    /// 获取布尔配置
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// 获取配置或默认值
    pub fn get_or<T: From<ConfigValue>>(&self, key: &str, default: T) -> T {
        self.get(key)
            .and_then(|v| {
                // 尝试转换
                None
            })
            .unwrap_or(default)
    }
}

// =============================================================================
// 配置管理器
// =============================================================================

/// 配置管理器（线程安全，支持热更新）
pub struct ConfigManager {
    /// 当前配置
    config: Arc<RwLock<Config>>,
    /// 环境
    environment: String,
}

/// 配置（可变的配置数据）
#[derive(Debug, Clone)]
pub struct Config {
    /// 配置数据
    pub data: BTreeMap<String, ConfigValue>,
    /// 版本号
    pub version: u64,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            version: 0,
            updated_at: chrono::Utc::now(),
        }
    }

    /// 设置配置值
    pub fn set(&mut self, key: impl Into<String>, value: ConfigValue) {
        self.data.insert(key.into(), value);
        self.version += 1;
        self.updated_at = chrono::Utc::now();
    }

    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.data.get(key)
    }

    /// 合并另一个配置（后者覆盖前者）
    pub fn merge(&mut self, other: &Config) {
        for (key, value) in &other.data {
            self.data.insert(key.clone(), value.clone());
        }
        self.version += 1;
        self.updated_at = chrono::Utc::now();
    }

    /// 创建快照
    pub fn snapshot(&self, environment: &str) -> ConfigSnapshot {
        ConfigSnapshot {
            data: self.data.clone(),
            version: self.version,
            created_at: chrono::Utc::now(),
            environment: environment.to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(environment: impl Into<String>) -> Self {
        Self {
            config: Arc::new(RwLock::new(Config::new())),
            environment: environment.into(),
        }
    }

    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        self.config.read().get(key).cloned()
    }

    /// 获取字符串配置
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.config.read().get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// 获取整数配置
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.config.read().get(key).and_then(|v| v.as_i64())
    }

    /// 获取浮点数配置
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.config.read().get(key).and_then(|v| v.as_f64())
    }

    /// 获取布尔配置
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.config.read().get(key).and_then(|v| v.as_bool())
    }

    /// 获取配置或默认值
    pub fn get_string_or(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|| default.to_string())
    }

    pub fn get_i64_or(&self, key: &str, default: i64) -> i64 {
        self.get_i64(key).unwrap_or(default)
    }

    pub fn get_f64_or(&self, key: &str, default: f64) -> f64 {
        self.get_f64(key).unwrap_or(default)
    }

    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get_bool(key).unwrap_or(default)
    }

    /// 设置配置值
    pub fn set(&self, key: impl Into<String>, value: ConfigValue) {
        self.config.write().set(key, value);
    }

    /// 合并配置
    pub fn merge(&self, other: &Config) {
        self.config.write().merge(other);
    }

    /// 创建快照
    pub fn snapshot(&self) -> ConfigSnapshot {
        self.config.read().snapshot(&self.environment)
    }

    /// 获取环境
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// 获取版本号
    pub fn version(&self) -> u64 {
        self.config.read().version
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            environment: self.environment.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_value_types() {
        assert_eq!(ConfigValue::from("test").as_str(), Some("test"));
        assert_eq!(ConfigValue::from(42i64).as_i64(), Some(42));
        assert_eq!(ConfigValue::from(3.14f64).as_f64(), Some(3.14));
        assert_eq!(ConfigValue::from(true).as_bool(), Some(true));
    }

    #[test]
    fn config_key_parts() {
        let key = ConfigKey::new("server.port");
        assert_eq!(key.parts(), vec!["server", "port"]);
        assert_eq!(key.join("host").as_str(), "server.port.host");
    }

    #[test]
    fn config_set_and_get() {
        let mut config = Config::new();
        config.set("server.port", ConfigValue::from(8080i64));
        config.set("server.host", ConfigValue::from("localhost"));

        assert_eq!(config.get("server.port").and_then(|v| v.as_i64()), Some(8080));
        assert_eq!(config.get("server.host").and_then(|v| v.as_str()), Some("localhost"));
        assert_eq!(config.version, 2);
    }

    #[test]
    fn config_merge() {
        let mut base = Config::new();
        base.set("a", ConfigValue::from(1i64));
        base.set("b", ConfigValue::from("base"));

        let mut override_config = Config::new();
        override_config.set("b", ConfigValue::from("override"));
        override_config.set("c", ConfigValue::from(true));

        base.merge(&override_config);

        assert_eq!(base.get("a").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(base.get("b").and_then(|v| v.as_str()), Some("override"));
        assert_eq!(base.get("c").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn config_manager_thread_safe() {
        let manager = ConfigManager::new("test");
        manager.set("key", ConfigValue::from("value"));

        let manager2 = manager.clone();
        assert_eq!(manager2.get_string("key"), Some("value".to_string()));
    }

    #[test]
    fn config_snapshot() {
        let manager = ConfigManager::new("test");
        manager.set("key", ConfigValue::from("value"));

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.environment, "test");
        assert_eq!(snapshot.get_string("key"), Some("value".to_string()));
    }

    #[test]
    fn config_manager_defaults() {
        let manager = ConfigManager::new("test");
        assert_eq!(manager.get_string_or("missing", "default"), "default");
        assert_eq!(manager.get_i64_or("missing", 42), 42);
        assert_eq!(manager.get_bool_or("missing", true), true);
    }
}
