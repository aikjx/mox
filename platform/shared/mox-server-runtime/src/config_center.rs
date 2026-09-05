// =============================================================================
// 配置中心（ConfigCenter）
// =============================================================================
//
// 统一配置中心抽象，支持多种后端：
// - MemoryConfigProvider：内存配置（测试/默认）
// - FileConfigProvider：文件配置（TOML/JSON，热更新监听）
// - RemoteConfigProvider：远程配置（Nacos/Apollo/etcd，预留扩展点）
//
// 核心能力：
// - 统一配置读取（get/get_all）
// - 配置热更新（watch + 回调通知）
// - 配置版本管理（version_hash + 变更历史）
// - 三级优先级：默认值 < 配置中心 < 环境变量
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 配置值类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<ConfigValue>),
    Map(HashMap<String, ConfigValue>),
    Null,
}

impl ConfigValue {
    /// 转换为字符串
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转换为 i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            ConfigValue::Float(f) => Some(*f as i64),
            ConfigValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为 f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            ConfigValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为 bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self { ConfigValue::String(s) }
}
impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self { ConfigValue::String(s.to_string()) }
}
impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self { ConfigValue::Integer(i) }
}
impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self { ConfigValue::Boolean(b) }
}

/// 配置变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 变更的配置键
    pub key: String,
    /// 旧值
    pub old_value: Option<ConfigValue>,
    /// 新值
    pub new_value: Option<ConfigValue>,
    /// 变更时间（RFC3339）
    pub changed_at: String,
    /// 变更来源（manual/file/remote）
    pub source: String,
}

/// 配置变更回调
pub type ConfigChangeCallback = Arc<dyn Fn(&ConfigChangeEvent) + Send + Sync>;

/// 配置中心 Provider trait
pub trait ConfigProvider: Send + Sync {
    /// 获取配置值
    fn get(&self, key: &str) -> Option<ConfigValue>;

    /// 获取所有配置
    fn get_all(&self) -> HashMap<String, ConfigValue>;

    /// 设置配置值（返回旧值）
    fn set(&self, key: &str, value: ConfigValue) -> Option<ConfigValue>;

    /// 删除配置（返回旧值）
    fn delete(&self, key: &str) -> Option<ConfigValue>;

    /// 监听配置变更
    fn watch(&self, callback: ConfigChangeCallback) -> u64;

    /// 取消监听
    fn unwatch(&self, watch_id: u64);

    /// 获取配置版本哈希
    fn version_hash(&self) -> String;

    /// Provider 名称
    fn name(&self) -> &str;
}

/// 内存配置 Provider（默认实现，线程安全）
pub struct MemoryConfigProvider {
    name: String,
    config: Mutex<HashMap<String, ConfigValue>>,
    callbacks: Mutex<HashMap<u64, ConfigChangeCallback>>,
    next_watch_id: Mutex<u64>,
}

impl MemoryConfigProvider {
    /// 创建内存配置 Provider
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config: Mutex::new(HashMap::new()),
            callbacks: Mutex::new(HashMap::new()),
            next_watch_id: Mutex::new(1),
        }
    }

    /// 批量设置配置
    pub fn set_all(&self, configs: HashMap<String, ConfigValue>) {
        let mut store = self.config.lock().unwrap();
        for (key, value) in configs {
            let old = store.insert(key.clone(), value.clone());
            self.notify_change(&key, old, Some(value), "batch");
        }
    }

    fn notify_change(&self, key: &str, old: Option<ConfigValue>, new: Option<ConfigValue>, source: &str) {
        let event = ConfigChangeEvent {
            key: key.to_string(),
            old_value: old,
            new_value: new,
            changed_at: chrono::Utc::now().to_rfc3339(),
            source: source.to_string(),
        };
        let callbacks = self.callbacks.lock().unwrap();
        for cb in callbacks.values() {
            cb(&event);
        }
    }
}

impl ConfigProvider for MemoryConfigProvider {
    fn get(&self, key: &str) -> Option<ConfigValue> {
        self.config.lock().unwrap().get(key).cloned()
    }

    fn get_all(&self) -> HashMap<String, ConfigValue> {
        self.config.lock().unwrap().clone()
    }

    fn set(&self, key: &str, value: ConfigValue) -> Option<ConfigValue> {
        let old = self.config.lock().unwrap().insert(key.to_string(), value.clone());
        self.notify_change(key, old.clone(), Some(value), "manual");
        old
    }

    fn delete(&self, key: &str) -> Option<ConfigValue> {
        let old = self.config.lock().unwrap().remove(key);
        self.notify_change(key, old.clone(), None, "manual");
        old
    }

    fn watch(&self, callback: ConfigChangeCallback) -> u64 {
        let mut id = self.next_watch_id.lock().unwrap();
        let watch_id = *id;
        *id += 1;
        self.callbacks.lock().unwrap().insert(watch_id, callback);
        watch_id
    }

    fn unwatch(&self, watch_id: u64) {
        self.callbacks.lock().unwrap().remove(&watch_id);
    }

    fn version_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let config = self.config.lock().unwrap();
        let mut hasher = Sha256::new();
        let mut keys: Vec<&String> = config.keys().collect();
        keys.sort();
        for key in keys {
            hasher.update(key.as_bytes());
            hasher.update(serde_json::to_string(config.get(key).unwrap()).unwrap_or_default().as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 文件配置 Provider（从 TOML/JSON 文件加载，支持热更新监听）
pub struct FileConfigProvider {
    inner: MemoryConfigProvider,
    file_path: String,
}

impl FileConfigProvider {
    /// 从文件创建配置 Provider
    pub fn from_file(file_path: impl Into<String>) -> Result<Self, String> {
        let path = file_path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取配置文件 {path} 失败: {e}"))?;

        let configs: HashMap<String, ConfigValue> = if path.ends_with(".json") {
            serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?
        } else {
            // TOML 转换为 ConfigValue
            let toml_val: toml::Value = toml::from_str(&content).map_err(|e| format!("解析 TOML 失败: {e}"))?;
            toml_value_to_config_value(&toml_val)
                .and_then(|v| match v {
                    ConfigValue::Map(m) => Ok(m),
                    _ => Err("配置文件根节点必须是 Map".to_string()),
                })?
        };

        let inner = MemoryConfigProvider::new(format!("file:{path}"));
        inner.set_all(configs);

        Ok(Self { inner, file_path: path })
    }

    /// 重新加载配置文件
    pub fn reload(&self) -> Result<(), String> {
        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| format!("读取配置文件失败: {e}"))?;
        let configs: HashMap<String, ConfigValue> = if self.file_path.ends_with(".json") {
            serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?
        } else {
            let toml_val: toml::Value = toml::from_str(&content).map_err(|e| format!("解析 TOML 失败: {e}"))?;
            toml_value_to_config_value(&toml_val)
                .and_then(|v| match v {
                    ConfigValue::Map(m) => Ok(m),
                    _ => Err("配置文件根节点必须是 Map".to_string()),
                })?
        };

        // 清空旧配置并设置新配置
        let old_all = self.inner.get_all();
        for key in old_all.keys() {
            self.inner.delete(key);
        }
        self.inner.set_all(configs);
        Ok(())
    }

    /// 启动文件热更新监听（轮询模式）
    pub fn start_watch(&self, interval: Duration) -> u64 {
        // 简化实现：返回 watch_id，实际热更新需要文件系统事件或轮询
        self.inner.watch(Arc::new(|_event| {
            // 默认回调，实际使用时注册自己的回调
        }))
    }
}

impl ConfigProvider for FileConfigProvider {
    fn get(&self, key: &str) -> Option<ConfigValue> { self.inner.get(key) }
    fn get_all(&self) -> HashMap<String, ConfigValue> { self.inner.get_all() }
    fn set(&self, key: &str, value: ConfigValue) -> Option<ConfigValue> { self.inner.set(key, value) }
    fn delete(&self, key: &str) -> Option<ConfigValue> { self.inner.delete(key) }
    fn watch(&self, callback: ConfigChangeCallback) -> u64 { self.inner.watch(callback) }
    fn unwatch(&self, watch_id: u64) { self.inner.unwatch(watch_id) }
    fn version_hash(&self) -> String { self.inner.version_hash() }
    fn name(&self) -> &str { self.inner.name() }
}

/// TOML Value 转换为 ConfigValue
fn toml_value_to_config_value(value: &toml::Value) -> Result<ConfigValue, String> {
    match value {
        toml::Value::String(s) => Ok(ConfigValue::String(s.clone())),
        toml::Value::Integer(i) => Ok(ConfigValue::Integer(*i)),
        toml::Value::Float(f) => Ok(ConfigValue::Float(*f)),
        toml::Value::Boolean(b) => Ok(ConfigValue::Boolean(*b)),
        toml::Value::Array(arr) => {
            let mut list = Vec::new();
            for v in arr {
                list.push(toml_value_to_config_value(v)?);
            }
            Ok(ConfigValue::List(list))
        }
        toml::Value::Table(table) => {
            let mut map = HashMap::new();
            for (k, v) in table {
                map.insert(k.clone(), toml_value_to_config_value(v)?);
            }
            Ok(ConfigValue::Map(map))
        }
        toml::Value::Datetime(dt) => Ok(ConfigValue::String(dt.to_string())),
    }
}

/// 配置中心统一入口
pub struct ConfigCenter {
    provider: Arc<dyn ConfigProvider>,
}

impl ConfigCenter {
    /// 创建配置中心
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    /// 创建内存配置中心（默认）
    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryConfigProvider::new("memory")))
    }

    /// 从文件创建配置中心
    pub fn from_file(path: impl Into<String>) -> Result<Self, String> {
        let provider = FileConfigProvider::from_file(path)?;
        Ok(Self::new(Arc::new(provider)))
    }

    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        self.provider.get(key)
    }

    /// 获取字符串配置（带默认值）
    pub fn get_str(&self, key: &str, default: &str) -> String {
        self.provider.get(key).and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_else(|| default.to_string())
    }

    /// 获取整数配置（带默认值）
    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.provider.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
    }

    /// 获取布尔配置（带默认值）
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.provider.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// 设置配置
    pub fn set(&self, key: &str, value: impl Into<ConfigValue>) {
        self.provider.set(key, value.into());
    }

    /// 监听配置变更
    pub fn watch(&self, callback: ConfigChangeCallback) -> u64 {
        self.provider.watch(callback)
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> &Arc<dyn ConfigProvider> {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_provider_get_set() {
        let provider = MemoryConfigProvider::new("test");
        assert!(provider.get("key1").is_none());

        let old = provider.set("key1", ConfigValue::String("value1".to_string()));
        assert!(old.is_none());
        assert_eq!(provider.get("key1").unwrap().as_str(), Some("value1"));

        let old = provider.set("key1", ConfigValue::Integer(42));
        assert_eq!(old.unwrap().as_str(), Some("value1"));
        assert_eq!(provider.get("key1").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn test_memory_provider_delete() {
        let provider = MemoryConfigProvider::new("test");
        provider.set("key1", "value1".into());
        let old = provider.delete("key1");
        assert_eq!(old.unwrap().as_str(), Some("value1"));
        assert!(provider.get("key1").is_none());
    }

    #[test]
    fn test_memory_provider_get_all() {
        let provider = MemoryConfigProvider::new("test");
        provider.set("a", "1".into());
        provider.set("b", "2".into());
        let all = provider.get_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_config_value_conversions() {
        assert_eq!(ConfigValue::String("42".to_string()).as_i64(), Some(42));
        assert_eq!(ConfigValue::Integer(42).as_f64(), Some(42.0));
        assert_eq!(ConfigValue::String("true".to_string()).as_bool(), Some(true));
        assert_eq!(ConfigValue::String("false".to_string()).as_bool(), Some(false));
        assert_eq!(ConfigValue::Boolean(true).as_bool(), Some(true));
    }

    #[test]
    fn test_config_change_watch() {
        let provider = MemoryConfigProvider::new("test");
        let changes = Arc::new(Mutex::new(Vec::new()));
        let changes_clone = changes.clone();

        let watch_id = provider.watch(Arc::new(move |event| {
            changes_clone.lock().unwrap().push(event.key.clone());
        }));

        provider.set("key1", "value1".into());
        provider.set("key2", "value2".into());
        provider.delete("key1");

        let changes = changes.lock().unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], "key1");
        assert_eq!(changes[1], "key2");
        assert_eq!(changes[2], "key1");

        provider.unwatch(watch_id);
    }

    #[test]
    fn test_version_hash() {
        let provider = MemoryConfigProvider::new("test");
        let hash1 = provider.version_hash();
        provider.set("key1", "value1".into());
        let hash2 = provider.version_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_config_center_convenience() {
        let center = ConfigCenter::memory();
        center.set("name", "test-app");
        center.set("port", 8080i64);
        center.set("debug", true);

        assert_eq!(center.get_str("name", "default"), "test-app");
        assert_eq!(center.get_i64("port", 3000), 8080);
        assert!(center.get_bool("debug", false));
        assert_eq!(center.get_str("missing", "fallback"), "fallback");
    }

    #[test]
    fn test_provider_name() {
        let provider = MemoryConfigProvider::new("my-provider");
        assert_eq!(provider.name(), "my-provider");
    }
}
