//! 系统配置管理模块
//!
//! 支持：动态配置 / 功能开关 / 参数配置 / 配置版本管理 / 配置回滚
//! 配置类型：字符串 / 数字 / 布尔 / JSON / 密码 / 选择列表

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 配置项类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigType {
    /// 字符串
    String,
    /// 数字（整数/浮点数）
    Number,
    /// 布尔值
    Boolean,
    /// JSON对象
    Json,
    /// 密码（加密存储）
    Password,
    /// 选择列表（单选）
    Select,
    /// 多选列表
    MultiSelect,
}

impl ConfigType {
    pub fn as_str(&self) -> &str {
        match self {
            ConfigType::String => "string",
            ConfigType::Number => "number",
            ConfigType::Boolean => "boolean",
            ConfigType::Json => "json",
            ConfigType::Password => "password",
            ConfigType::Select => "select",
            ConfigType::MultiSelect => "multi_select",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ConfigType::String => "字符串",
            ConfigType::Number => "数字",
            ConfigType::Boolean => "布尔值",
            ConfigType::Json => "JSON",
            ConfigType::Password => "密码",
            ConfigType::Select => "单选",
            ConfigType::MultiSelect => "多选",
        }
    }
}

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// 配置键（唯一，格式：group.key）
    pub config_key: String,
    /// 配置名称
    pub config_name: String,
    /// 配置描述
    pub description: Option<String>,
    /// 配置分组
    pub config_group: String,
    /// 配置类型
    pub config_type: ConfigType,
    /// 配置值（JSON序列化后的值）
    pub config_value: serde_json::Value,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
    /// 可选值列表（select/multi_select类型使用）
    pub options: Option<Vec<ConfigOption>>,
    /// 是否为系统配置（不可删除）
    pub is_system: bool,
    /// 是否加密（password类型自动加密）
    pub is_encrypted: bool,
    /// 是否可修改
    pub is_editable: bool,
    /// 是否需要重启生效
    pub require_restart: bool,
    /// 验证规则（正则表达式/最小值/最大值等）
    pub validation_rules: Option<HashMap<String, serde_json::Value>>,
    /// 版本号
    pub version: i32,
    /// 租户ID（null表示全局配置）
    pub tenant_id: Option<String>,
    /// 创建人
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
    /// 更新人
    pub updated_by: Option<String>,
    /// 更新时间
    pub updated_at: String,
}

/// 配置可选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOption {
    pub label: String,
    pub value: String,
}

/// 配置版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    /// 版本ID
    pub version_id: String,
    /// 配置键
    pub config_key: String,
    /// 版本号
    pub version: i32,
    /// 配置值
    pub config_value: serde_json::Value,
    /// 修改人
    pub changed_by: String,
    /// 修改时间
    pub changed_at: String,
    /// 修改说明
    pub change_note: Option<String>,
    /// 变更前值
    pub old_value: Option<serde_json::Value>,
    /// 变更后值
    pub new_value: Option<serde_json::Value>,
}

/// 功能开关
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// 开关键
    pub flag_key: String,
    /// 开关名称
    pub flag_name: String,
    /// 开关描述
    pub description: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 灰度发布百分比（0-100，null表示全量）
    pub rollout_percentage: Option<u8>,
    /// 白名单用户ID列表
    pub whitelist_users: Option<Vec<String>>,
    /// 白名单租户ID列表
    pub whitelist_tenants: Option<Vec<String>>,
    /// 过期时间（null表示永久）
    pub expire_at: Option<String>,
    /// 是否为系统开关
    pub is_system: bool,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 配置分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigGroup {
    /// 分组编码
    pub group_code: String,
    /// 分组名称
    pub group_name: String,
    /// 分组描述
    pub description: Option<String>,
    /// 排序
    pub sort_order: i32,
    /// 图标
    pub icon: Option<String>,
}

/// 创建配置项请求
#[derive(Debug, Deserialize)]
pub struct CreateConfigItemRequest {
    pub config_key: String,
    pub config_name: String,
    pub description: Option<String>,
    pub config_group: String,
    pub config_type: ConfigType,
    pub config_value: serde_json::Value,
    pub default_value: Option<serde_json::Value>,
    pub options: Option<Vec<ConfigOption>>,
    pub is_encrypted: Option<bool>,
    pub is_editable: Option<bool>,
    pub require_restart: Option<bool>,
    pub validation_rules: Option<HashMap<String, serde_json::Value>>,
    pub tenant_id: Option<String>,
}

/// 更新配置项请求
#[derive(Debug, Deserialize)]
pub struct UpdateConfigItemRequest {
    pub config_name: Option<String>,
    pub description: Option<String>,
    pub config_value: Option<serde_json::Value>,
    pub options: Option<Vec<ConfigOption>>,
    pub is_editable: Option<bool>,
    pub require_restart: Option<bool>,
    pub validation_rules: Option<HashMap<String, serde_json::Value>>,
    pub change_note: Option<String>,
}

/// 批量更新配置请求
#[derive(Debug, Deserialize)]
pub struct BatchUpdateConfigRequest {
    pub items: Vec<BatchConfigItem>,
    pub change_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchConfigItem {
    pub config_key: String,
    pub config_value: serde_json::Value,
}

/// 创建功能开关请求
#[derive(Debug, Deserialize)]
pub struct CreateFeatureFlagRequest {
    pub flag_key: String,
    pub flag_name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub rollout_percentage: Option<u8>,
    pub whitelist_users: Option<Vec<String>>,
    pub whitelist_tenants: Option<Vec<String>>,
    pub expire_at: Option<String>,
}

/// 系统配置统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigStats {
    /// 配置项总数
    pub total_configs: i64,
    /// 系统配置数
    pub system_configs: i64,
    /// 自定义配置数
    pub custom_configs: i64,
    /// 配置分组数
    pub total_groups: i64,
    /// 功能开关总数
    pub total_flags: i64,
    /// 已启用开关数
    pub enabled_flags: i64,
    /// 已禁用开关数
    pub disabled_flags: i64,
    /// 需要重启的配置数
    pub require_restart_count: i64,
}

/// 内置系统配置
pub fn builtin_system_configs() -> Vec<ConfigItem> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        // 系统基础配置
        ConfigItem {
            config_key: "system.name".to_string(),
            config_name: "系统名称".to_string(),
            description: Some("显示在页面标题和登录页的系统名称".to_string()),
            config_group: "system".to_string(),
            config_type: ConfigType::String,
            config_value: serde_json::json!("MOX 企业级AI平台"),
            default_value: Some(serde_json::json!("MOX 企业级AI平台")),
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        ConfigItem {
            config_key: "system.logo_url".to_string(),
            config_name: "系统Logo".to_string(),
            description: Some("系统Logo图片URL".to_string()),
            config_group: "system".to_string(),
            config_type: ConfigType::String,
            config_value: serde_json::json!("/logo.png"),
            default_value: Some(serde_json::json!("/logo.png")),
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        // 安全配置
        ConfigItem {
            config_key: "security.password_min_length".to_string(),
            config_name: "密码最小长度".to_string(),
            description: Some("用户密码的最小长度要求".to_string()),
            config_group: "security".to_string(),
            config_type: ConfigType::Number,
            config_value: serde_json::json!(8),
            default_value: Some(serde_json::json!(8)),
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: Some(HashMap::from([
                ("min".to_string(), serde_json::json!(6)),
                ("max".to_string(), serde_json::json!(32)),
            ])),
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        ConfigItem {
            config_key: "security.session_timeout".to_string(),
            config_name: "会话超时时间（分钟）".to_string(),
            description: Some("用户登录会话的超时时间，单位为分钟".to_string()),
            config_group: "security".to_string(),
            config_type: ConfigType::Number,
            config_value: serde_json::json!(30),
            default_value: Some(serde_json::json!(30)),
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: Some(HashMap::from([
                ("min".to_string(), serde_json::json!(5)),
                ("max".to_string(), serde_json::json!(1440)),
            ])),
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        ConfigItem {
            config_key: "security.login_max_attempts".to_string(),
            config_name: "登录最大失败次数".to_string(),
            description: Some("连续登录失败达到此次数后锁定账户".to_string()),
            config_group: "security".to_string(),
            config_type: ConfigType::Number,
            config_value: serde_json::json!(5),
            default_value: Some(serde_json::json!(5)),
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: Some(HashMap::from([
                ("min".to_string(), serde_json::json!(3)),
                ("max".to_string(), serde_json::json!(20)),
            ])),
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        // 消息配置
        ConfigItem {
            config_key: "message.smtp_host".to_string(),
            config_name: "SMTP服务器地址".to_string(),
            description: Some("邮件发送SMTP服务器地址".to_string()),
            config_group: "message".to_string(),
            config_type: ConfigType::String,
            config_value: serde_json::json!("smtp.example.com"),
            default_value: None,
            options: None,
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        ConfigItem {
            config_key: "message.smtp_password".to_string(),
            config_name: "SMTP密码".to_string(),
            description: Some("邮件发送SMTP认证密码（加密存储）".to_string()),
            config_group: "message".to_string(),
            config_type: ConfigType::Password,
            config_value: serde_json::json!(""),
            default_value: None,
            options: None,
            is_system: true,
            is_encrypted: true,
            is_editable: true,
            require_restart: false,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        // 存储配置
        ConfigItem {
            config_key: "storage.type".to_string(),
            config_name: "文件存储类型".to_string(),
            description: Some("文件存储方式：local本地/oss阿里云/s3亚马逊/minio".to_string()),
            config_group: "storage".to_string(),
            config_type: ConfigType::Select,
            config_value: serde_json::json!("local"),
            default_value: Some(serde_json::json!("local")),
            options: Some(vec![
                ConfigOption { label: "本地存储".to_string(), value: "local".to_string() },
                ConfigOption { label: "阿里云OSS".to_string(), value: "oss".to_string() },
                ConfigOption { label: "Amazon S3".to_string(), value: "s3".to_string() },
                ConfigOption { label: "MinIO".to_string(), value: "minio".to_string() },
            ]),
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: true,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
        // AI配置
        ConfigItem {
            config_key: "ai.default_model".to_string(),
            config_name: "默认AI模型".to_string(),
            description: Some("系统默认使用的AI大模型".to_string()),
            config_group: "ai".to_string(),
            config_type: ConfigType::Select,
            config_value: serde_json::json!("gpt-4"),
            default_value: Some(serde_json::json!("gpt-4")),
            options: Some(vec![
                ConfigOption { label: "GPT-4".to_string(), value: "gpt-4".to_string() },
                ConfigOption { label: "GPT-3.5 Turbo".to_string(), value: "gpt-3.5-turbo".to_string() },
                ConfigOption { label: "Claude 3".to_string(), value: "claude-3".to_string() },
                ConfigOption { label: "通义千问".to_string(), value: "qwen".to_string() },
            ]),
            is_system: true,
            is_encrypted: false,
            is_editable: true,
            require_restart: false,
            validation_rules: None,
            version: 1,
            tenant_id: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_by: None,
            updated_at: now.clone(),
        },
    ]
}

/// 内置配置分组
pub fn builtin_config_groups() -> Vec<ConfigGroup> {
    vec![
        ConfigGroup { group_code: "system".to_string(), group_name: "系统基础".to_string(), description: Some("系统名称、Logo等基础配置".to_string()), sort_order: 1, icon: Some("setting".to_string()) },
        ConfigGroup { group_code: "security".to_string(), group_name: "安全配置".to_string(), description: Some("密码策略、会话超时等安全配置".to_string()), sort_order: 2, icon: Some("shield".to_string()) },
        ConfigGroup { group_code: "message".to_string(), group_name: "消息配置".to_string(), description: Some("邮件、短信等消息通道配置".to_string()), sort_order: 3, icon: Some("message".to_string()) },
        ConfigGroup { group_code: "storage".to_string(), group_name: "存储配置".to_string(), description: Some("文件存储方式和参数".to_string()), sort_order: 4, icon: Some("folder".to_string()) },
        ConfigGroup { group_code: "ai".to_string(), group_name: "AI配置".to_string(), description: Some("AI模型和参数配置".to_string()), sort_order: 5, icon: Some("robot".to_string()) },
        ConfigGroup { group_code: "workflow".to_string(), group_name: "流程配置".to_string(), description: Some("审批流程相关配置".to_string()), sort_order: 6, icon: Some("flow".to_string()) },
        ConfigGroup { group_code: "custom".to_string(), group_name: "自定义".to_string(), description: Some("用户自定义配置".to_string()), sort_order: 99, icon: Some("edit".to_string()) },
    ]
}

/// 内置功能开关
pub fn builtin_feature_flags() -> Vec<FeatureFlag> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        FeatureFlag {
            flag_key: "feature.low_code_designer".to_string(),
            flag_name: "低代码设计器".to_string(),
            description: Some("启用低代码流程/表单/报表设计器".to_string()),
            enabled: true,
            rollout_percentage: None,
            whitelist_users: None,
            whitelist_tenants: None,
            expire_at: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        FeatureFlag {
            flag_key: "feature.ai_assistant".to_string(),
            flag_name: "AI助手".to_string(),
            description: Some("启用AI智能助手功能".to_string()),
            enabled: true,
            rollout_percentage: Some(100),
            whitelist_users: None,
            whitelist_tenants: None,
            expire_at: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        FeatureFlag {
            flag_key: "feature.mobile_approval".to_string(),
            flag_name: "移动端审批".to_string(),
            description: Some("启用移动端审批功能".to_string()),
            enabled: false,
            rollout_percentage: Some(0),
            whitelist_users: None,
            whitelist_tenants: None,
            expire_at: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        FeatureFlag {
            flag_key: "feature.knowledge_graph".to_string(),
            flag_name: "知识图谱".to_string(),
            description: Some("启用知识图谱可视化功能".to_string()),
            enabled: true,
            rollout_percentage: None,
            whitelist_users: None,
            whitelist_tenants: None,
            expire_at: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}
