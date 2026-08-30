// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 统一配置中心
//!
//! 六大归一化体系的配置统一管理：
//! - 分层配置（全局/租户/用户）
//! - 配置热更新
//! - 配置变更事件
//! - 配置校验

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{PlatformError, PlatformResult};
use crate::types::NormalizationSystem;

/// 配置层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLevel {
    /// 全局默认配置
    Global = 0,
    /// 体系级配置
    System = 1,
    /// 租户级配置
    Tenant = 2,
    /// 用户级配置
    User = 3,
}

impl ConfigLevel {
    pub fn name(&self) -> &'static str {
        match self {
            ConfigLevel::Global => "global",
            ConfigLevel::System => "system",
            ConfigLevel::Tenant => "tenant",
            ConfigLevel::User => "user",
        }
    }
}

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// 配置键
    pub key: String,
    /// 配置值
    pub value: serde_json::Value,
    /// 所属体系
    pub system: Option<NormalizationSystem>,
    /// 配置层级
    pub level: ConfigLevel,
    /// 租户 ID（租户级配置）
    pub tenant_id: Option<String>,
    /// 用户 ID（用户级配置）
    pub user_id: Option<String>,
    /// 描述
    pub description: String,
    /// 是否可修改
    pub mutable: bool,
    /// 版本号
    pub version: u64,
    /// 更新时间戳
    pub updated_at: u64,
}

/// 配置变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 变更的配置键
    pub key: String,
    /// 旧值
    pub old_value: Option<serde_json::Value>,
    /// 新值
    pub new_value: serde_json::Value,
    /// 变更层级
    pub level: ConfigLevel,
    /// 变更时间
    pub changed_at: u64,
}

/// 配置变更回调
pub type ConfigChangeHandler = Box<dyn Fn(&ConfigChangeEvent) + Send + Sync>;

/// 统一配置中心
pub struct UnifiedConfigCenter {
    /// 配置存储（按层级组织）
    configs: RwLock<HashMap<String, ConfigItem>>,
    /// 变更监听器
    listeners: RwLock<Vec<ConfigChangeHandler>>,
    /// 配置 schema 校验规则
    schemas: RwLock<HashMap<String, ConfigSchema>>,
}

/// 配置 schema 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    /// 配置键
    pub key: String,
    /// 值类型
    pub value_type: ConfigValueType,
    /// 默认值
    pub default_value: serde_json::Value,
    /// 描述
    pub description: String,
    /// 所属体系
    pub system: NormalizationSystem,
    /// 是否可修改
    pub mutable: bool,
    /// 枚举值（可选）
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// 最小值（数值类型）
    pub min_value: Option<f64>,
    /// 最大值（数值类型）
    pub max_value: Option<f64>,
}

/// 配置值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigValueType {
    /// 字符串
    String,
    /// 整数
    Integer,
    /// 浮点数
    Float,
    /// 布尔值
    Boolean,
    /// 对象
    Object,
    /// 数组
    Array,
}

impl UnifiedConfigCenter {
    /// 创建配置中心
    pub fn new() -> Self {
        let center = Self {
            configs: RwLock::new(HashMap::new()),
            listeners: RwLock::new(Vec::new()),
            schemas: RwLock::new(HashMap::new()),
        };
        center.register_builtin_schemas();
        center.init_default_configs();
        center
    }

    /// 注册内置配置 schema
    fn register_builtin_schemas(&self) {
        let schemas = vec![
            // 架构归一化配置
            ConfigSchema {
                key: "arch.default_protocol".to_string(),
                value_type: ConfigValueType::String,
                default_value: serde_json::json!("rest"),
                description: "默认接入协议".to_string(),
                system: NormalizationSystem::Architecture,
                mutable: true,
                enum_values: Some(vec![
                    serde_json::json!("rest"),
                    serde_json::json!("graphql"),
                    serde_json::json!("grpc"),
                    serde_json::json!("websocket"),
                ]),
                min_value: None,
                max_value: None,
            },
            ConfigSchema {
                key: "arch.request_timeout_ms".to_string(),
                value_type: ConfigValueType::Integer,
                default_value: serde_json::json!(30000),
                description: "请求超时时间（毫秒）".to_string(),
                system: NormalizationSystem::Architecture,
                mutable: true,
                enum_values: None,
                min_value: Some(1000.0),
                max_value: Some(300000.0),
            },
            // 权限配置
            ConfigSchema {
                key: "perm.rbac_enabled".to_string(),
                value_type: ConfigValueType::Boolean,
                default_value: serde_json::json!(true),
                description: "是否启用RBAC".to_string(),
                system: NormalizationSystem::Permission,
                mutable: true,
                enum_values: None,
                min_value: None,
                max_value: None,
            },
            ConfigSchema {
                key: "perm.abac_enabled".to_string(),
                value_type: ConfigValueType::Boolean,
                default_value: serde_json::json!(true),
                description: "是否启用ABAC".to_string(),
                system: NormalizationSystem::Permission,
                mutable: true,
                enum_values: None,
                min_value: None,
                max_value: None,
            },
            ConfigSchema {
                key: "perm.super_role".to_string(),
                value_type: ConfigValueType::String,
                default_value: serde_json::json!("admin"),
                description: "超级管理员角色名".to_string(),
                system: NormalizationSystem::Permission,
                mutable: true,
                enum_values: None,
                min_value: None,
                max_value: None,
            },
            // 低代码配置
            ConfigSchema {
                key: "lowcode.form_validation_mode".to_string(),
                value_type: ConfigValueType::String,
                default_value: serde_json::json!("on_submit"),
                description: "表单校验模式".to_string(),
                system: NormalizationSystem::Lowcode,
                mutable: true,
                enum_values: Some(vec![
                    serde_json::json!("on_submit"),
                    serde_json::json!("on_blur"),
                    serde_json::json!("on_change"),
                ]),
                min_value: None,
                max_value: None,
            },
            ConfigSchema {
                key: "lowcode.max_fields_per_form".to_string(),
                value_type: ConfigValueType::Integer,
                default_value: serde_json::json!(100),
                description: "单表单最大字段数".to_string(),
                system: NormalizationSystem::Lowcode,
                mutable: true,
                enum_values: None,
                min_value: Some(1.0),
                max_value: Some(1000.0),
            },
            // 流程算法配置
            ConfigSchema {
                key: "process.rule_max_fires".to_string(),
                value_type: ConfigValueType::Integer,
                default_value: serde_json::json!(1000),
                description: "规则引擎最大触发次数".to_string(),
                system: NormalizationSystem::ProcessAlgo,
                mutable: true,
                enum_values: None,
                min_value: Some(1.0),
                max_value: Some(100000.0),
            },
            ConfigSchema {
                key: "process.auto_tune_enabled".to_string(),
                value_type: ConfigValueType::Boolean,
                default_value: serde_json::json!(true),
                description: "是否启用算法自动调优".to_string(),
                system: NormalizationSystem::ProcessAlgo,
                mutable: true,
                enum_values: None,
                min_value: None,
                max_value: None,
            },
            // 前端配置
            ConfigSchema {
                key: "frontend.theme".to_string(),
                value_type: ConfigValueType::String,
                default_value: serde_json::json!("light"),
                description: "默认主题".to_string(),
                system: NormalizationSystem::Frontend,
                mutable: true,
                enum_values: Some(vec![
                    serde_json::json!("light"),
                    serde_json::json!("dark"),
                    serde_json::json!("auto"),
                ]),
                min_value: None,
                max_value: None,
            },
            ConfigSchema {
                key: "frontend.primary_color".to_string(),
                value_type: ConfigValueType::String,
                default_value: serde_json::json!("#1890ff"),
                description: "主题色".to_string(),
                system: NormalizationSystem::Frontend,
                mutable: true,
                enum_values: None,
                min_value: None,
                max_value: None,
            },
            // AI配置
            ConfigSchema {
                key: "ai.max_concurrent_tasks".to_string(),
                value_type: ConfigValueType::Integer,
                default_value: serde_json::json!(10),
                description: "最大并发AI任务数".to_string(),
                system: NormalizationSystem::AiAssistant,
                mutable: true,
                enum_values: None,
                min_value: Some(1.0),
                max_value: Some(1000.0),
            },
            ConfigSchema {
                key: "ai.intent_confidence_threshold".to_string(),
                value_type: ConfigValueType::Float,
                default_value: serde_json::json!(0.7),
                description: "意图识别置信度阈值".to_string(),
                system: NormalizationSystem::AiAssistant,
                mutable: true,
                enum_values: None,
                min_value: Some(0.0),
                max_value: Some(1.0),
            },
        ];

        let mut schema_map = self.schemas.write();
        for schema in schemas {
            schema_map.insert(schema.key.clone(), schema);
        }
    }

    /// 初始化默认配置
    fn init_default_configs(&self) {
        let schemas = self.schemas.read();
        let mut configs = self.configs.write();

        for (key, schema) in schemas.iter() {
            let item = ConfigItem {
                key: key.clone(),
                value: schema.default_value.clone(),
                system: Some(schema.system),
                level: ConfigLevel::Global,
                tenant_id: None,
                user_id: None,
                description: schema.description.clone(),
                mutable: schema.mutable,
                version: 1,
                updated_at: now_ms(),
            };
            let lookup_key = format!("global:{}", key);
            configs.insert(lookup_key, item);
        }
    }

    /// 获取配置（按层级合并，高层级覆盖低层级）
    pub fn get(
        &self,
        key: &str,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Option<serde_json::Value> {
        let configs = self.configs.read();
        let mut result: Option<serde_json::Value> = None;
        let mut current_level = ConfigLevel::Global;

        // 从最低层级到最高层级，高的覆盖低的
        for level in [
            ConfigLevel::Global,
            ConfigLevel::System,
            ConfigLevel::Tenant,
            ConfigLevel::User,
        ] {
            let lookup_key = self.build_lookup_key(key, level, tenant_id, user_id);
            if let Some(config) = configs.get(&lookup_key) {
                if level >= current_level {
                    result = Some(config.value.clone());
                    current_level = level;
                }
            }
        }

        result
    }

    /// 获取全局配置
    pub fn get_global(&self, key: &str) -> Option<serde_json::Value> {
        self.get(key, None, None)
    }

    /// 设置配置
    pub fn set(
        &self,
        key: &str,
        value: serde_json::Value,
        level: ConfigLevel,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
    ) -> PlatformResult<()> {
        // 校验
        self.validate_config(key, &value)?;

        let lookup_key = self.build_lookup_key(key, level, tenant_id, user_id);
        let mut configs = self.configs.write();

        let old_value = configs.get(&lookup_key).map(|c| c.value.clone());
        let version = configs.get(&lookup_key).map(|c| c.version + 1).unwrap_or(1);

        let schema = self.schemas.read();
        let system = schema.get(key).map(|s| s.system);
        let description = schema.get(key).map(|s| s.description.clone()).unwrap_or_default();
        let mutable = schema.get(key).map(|s| s.mutable).unwrap_or(true);

        let item = ConfigItem {
            key: key.to_string(),
            value: value.clone(),
            system,
            level,
            tenant_id: tenant_id.map(|s| s.to_string()),
            user_id: user_id.map(|s| s.to_string()),
            description,
            mutable,
            version,
            updated_at: now_ms(),
        };

        configs.insert(lookup_key, item);

        // 触发变更事件
        let event = ConfigChangeEvent {
            key: key.to_string(),
            old_value,
            new_value: value,
            level,
            changed_at: now_ms(),
        };
        self.notify_listeners(&event);

        Ok(())
    }

    /// 设置全局配置
    pub fn set_global(&self, key: &str, value: serde_json::Value) -> PlatformResult<()> {
        self.set(key, value, ConfigLevel::Global, None, None)
    }

    /// 校验配置值
    fn validate_config(&self, key: &str, value: &serde_json::Value) -> PlatformResult<()> {
        let schemas = self.schemas.read();
        let schema = match schemas.get(key) {
            Some(s) => s,
            None => return Ok(()), // 无 schema 的配置不校验
        };

        // 类型校验
        let value_type = match value {
            serde_json::Value::String(_) => ConfigValueType::String,
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    ConfigValueType::Integer
                } else {
                    ConfigValueType::Float
                }
            }
            serde_json::Value::Bool(_) => ConfigValueType::Boolean,
            serde_json::Value::Object(_) => ConfigValueType::Object,
            serde_json::Value::Array(_) => ConfigValueType::Array,
            serde_json::Value::Null => {
                return Err(PlatformError::ValidationError(format!(
                    "config '{}' value cannot be null",
                    key
                )))
            }
        };

        if value_type != schema.value_type
            && !(schema.value_type == ConfigValueType::Float
                && value_type == ConfigValueType::Integer)
        {
            return Err(PlatformError::ValidationError(format!(
                "config '{}' type mismatch: expected {:?}, got {:?}",
                key, schema.value_type, value_type
            )));
        }

        // 枚举值校验
        if let Some(enum_values) = &schema.enum_values {
            if !enum_values.contains(value) {
                return Err(PlatformError::ValidationError(format!(
                    "config '{}' value {:?} not in allowed values",
                    key, value
                )));
            }
        }

        // 范围校验
        if let (Some(min), Some(num)) = (schema.min_value, value.as_f64()) {
            if num < min {
                return Err(PlatformError::ValidationError(format!(
                    "config '{}' value {} is below minimum {}",
                    key, num, min
                )));
            }
        }
        if let (Some(max), Some(num)) = (schema.max_value, value.as_f64()) {
            if num > max {
                return Err(PlatformError::ValidationError(format!(
                    "config '{}' value {} is above maximum {}",
                    key, num, max
                )));
            }
        }

        Ok(())
    }

    /// 添加变更监听器
    pub fn add_listener(&self, handler: ConfigChangeHandler) {
        self.listeners.write().push(handler);
    }

    /// 通知所有监听器
    fn notify_listeners(&self, event: &ConfigChangeEvent) {
        let listeners = self.listeners.read();
        for handler in listeners.iter() {
            handler(event);
        }
    }

    /// 获取某体系的所有配置 schema
    pub fn list_schemas_by_system(&self, system: NormalizationSystem) -> Vec<ConfigSchema> {
        self.schemas
            .read()
            .values()
            .filter(|s| s.system == system)
            .cloned()
            .collect()
    }

    /// 获取所有配置键
    pub fn list_keys(&self) -> Vec<String> {
        self.schemas.read().keys().cloned().collect()
    }

    /// schema 数量
    pub fn schema_count(&self) -> usize {
        self.schemas.read().len()
    }

    /// 配置项数量
    pub fn config_count(&self) -> usize {
        self.configs.read().len()
    }

    fn build_lookup_key(
        &self,
        key: &str,
        level: ConfigLevel,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
    ) -> String {
        match level {
            ConfigLevel::Global => format!("global:{}", key),
            ConfigLevel::System => format!("system:{}", key),
            ConfigLevel::Tenant => format!("tenant:{}:{}", tenant_id.unwrap_or(""), key),
            ConfigLevel::User => format!(
                "user:{}:{}:{}",
                tenant_id.unwrap_or(""),
                user_id.unwrap_or(""),
                key
            ),
        }
    }
}

impl Default for UnifiedConfigCenter {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_schemas() {
        let center = UnifiedConfigCenter::new();
        assert_eq!(center.schema_count(), 13); // 13 个内置配置项
        assert_eq!(center.config_count(), 13); // 默认都有全局配置
    }

    #[test]
    fn test_get_global_config() {
        let center = UnifiedConfigCenter::new();

        let theme = center.get_global("frontend.theme").unwrap();
        assert_eq!(theme, serde_json::json!("light"));

        let timeout = center.get_global("arch.request_timeout_ms").unwrap();
        assert_eq!(timeout, serde_json::json!(30000));
    }

    #[test]
    fn test_set_global_config() {
        let center = UnifiedConfigCenter::new();

        center
            .set_global("frontend.theme", serde_json::json!("dark"))
            .unwrap();

        let theme = center.get_global("frontend.theme").unwrap();
        assert_eq!(theme, serde_json::json!("dark"));
    }

    #[test]
    fn test_tenant_override() {
        let center = UnifiedConfigCenter::new();

        // 租户级覆盖
        center
            .set(
                "frontend.theme",
                serde_json::json!("dark"),
                ConfigLevel::Tenant,
                Some("t1"),
                None,
            )
            .unwrap();

        // 全局仍为 light
        assert_eq!(
            center.get_global("frontend.theme").unwrap(),
            serde_json::json!("light")
        );

        // 租户 t1 为 dark
        assert_eq!(
            center.get("frontend.theme", Some("t1"), None).unwrap(),
            serde_json::json!("dark")
        );
    }

    #[test]
    fn test_user_override_tenant() {
        let center = UnifiedConfigCenter::new();

        center
            .set(
                "frontend.theme",
                serde_json::json!("dark"),
                ConfigLevel::Tenant,
                Some("t1"),
                None,
            )
            .unwrap();

        center
            .set(
                "frontend.theme",
                serde_json::json!("auto"),
                ConfigLevel::User,
                Some("t1"),
                Some("u1"),
            )
            .unwrap();

        assert_eq!(
            center.get("frontend.theme", Some("t1"), Some("u1")).unwrap(),
            serde_json::json!("auto")
        );
    }

    #[test]
    fn test_validation_enum() {
        let center = UnifiedConfigCenter::new();

        let result = center.set_global("frontend.theme", serde_json::json!("invalid_theme"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_type() {
        let center = UnifiedConfigCenter::new();

        let result = center.set_global("arch.request_timeout_ms", serde_json::json!("not_a_number"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_range() {
        let center = UnifiedConfigCenter::new();

        // 超过最大值
        let result = center.set_global("arch.request_timeout_ms", serde_json::json!(999999));
        assert!(result.is_err());

        // 低于最小值
        let result = center.set_global("arch.request_timeout_ms", serde_json::json!(100));
        assert!(result.is_err());
    }

    #[test]
    fn test_list_schemas_by_system() {
        let center = UnifiedConfigCenter::new();

        let frontend_schemas = center.list_schemas_by_system(NormalizationSystem::Frontend);
        assert_eq!(frontend_schemas.len(), 2); // theme + primary_color

        let ai_schemas = center.list_schemas_by_system(NormalizationSystem::AiAssistant);
        assert_eq!(ai_schemas.len(), 2);
    }

    #[test]
    fn test_config_change_listener() {
        let center = UnifiedConfigCenter::new();
        let changed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let changed_clone = changed.clone();

        center.add_listener(Box::new(move |_| {
            changed_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        }));

        center
            .set_global("frontend.theme", serde_json::json!("dark"))
            .unwrap();

        assert!(changed.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_list_keys() {
        let center = UnifiedConfigCenter::new();
        let keys = center.list_keys();
        assert_eq!(keys.len(), 13);
    }

    #[test]
    fn test_config_version_increments() {
        let center = UnifiedConfigCenter::new();

        center
            .set_global("frontend.theme", serde_json::json!("dark"))
            .unwrap();
        center
            .set_global("frontend.theme", serde_json::json!("light"))
            .unwrap();

        // 版本号应该增加
        let configs = center.configs.read();
        let config = configs.get("global:frontend.theme").unwrap();
        assert_eq!(config.version, 3); // 初始1 + 两次修改 = 3
    }

    #[test]
    fn test_get_nonexistent_config() {
        let center = UnifiedConfigCenter::new();
        assert!(center.get_global("nonexistent.key").is_none());
    }
}
