// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 插件描述符（Manifest）— 插件的元数据声明

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 插件权限（沙箱控制）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// 文件读取
    FileRead,
    /// 文件写入
    FileWrite,
    /// 网络API调用
    NetworkApi,
    /// 网络监听（服务器）
    NetworkServer,
    /// AI能力调用
    AiChat,
    /// 数据库访问
    Database,
    /// 缓存访问
    Cache,
    /// 事件发布
    EventPublish,
    /// 事件订阅
    EventSubscribe,
    /// 系统命令执行（高危）
    SystemCommand,
    /// 环境变量读取
    EnvRead,
}

impl PluginPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginPermission::FileRead => "file:read",
            PluginPermission::FileWrite => "file:write",
            PluginPermission::NetworkApi => "network:api",
            PluginPermission::NetworkServer => "network:server",
            PluginPermission::AiChat => "ai:chat",
            PluginPermission::Database => "database",
            PluginPermission::Cache => "cache",
            PluginPermission::EventPublish => "event:publish",
            PluginPermission::EventSubscribe => "event:subscribe",
            PluginPermission::SystemCommand => "system:command",
            PluginPermission::EnvRead => "env:read",
        }
    }
}

/// 插件依赖声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// 依赖的插件ID
    pub id: String,
    /// 版本约束（如 ">=1.0.0", "^2.0"）
    pub version: String,
    /// 是否可选依赖
    #[serde(default)]
    pub optional: bool,
}

/// 插件配置Schema（JSON Schema子集）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String, // string/number/boolean/object
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: String,
    /// 是否敏感字段（如API Key，存储时加密）
    #[serde(default)]
    pub secret: bool,
}

/// 插件能力声明（插件向平台注册的能力）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    /// 能力唯一标识（如 "ocr.extract", "translate.text"）
    pub id: String,
    /// 能力名称
    pub name: String,
    /// 能力描述
    pub description: String,
    /// 输入参数Schema
    pub input_schema: serde_json::Value,
    /// 输出参数Schema
    pub output_schema: serde_json::Value,
}

/// 插件描述符（manifest.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件唯一ID（反向域名风格，如 "com.vendor.ocr"）
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本（语义化版本）
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 入口文件（WASM模块路径）
    pub entry: String,
    /// 权限列表
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    /// 依赖列表
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// 配置Schema
    #[serde(default)]
    pub config_schema: Vec<ConfigField>,
    /// 能力列表
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// 标签（用于分类搜索）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 主页URL
    #[serde(default)]
    pub homepage: Option<String>,
    /// 仓库URL
    #[serde(default)]
    pub repository: Option<String>,
    /// 许可证
    #[serde(default)]
    pub license: Option<String>,
    /// 最低平台版本
    #[serde(default = "default_min_platform_version")]
    pub min_platform_version: String,
}

fn default_min_platform_version() -> String { "3.0.0".into() }

impl PluginManifest {
    /// 从JSON字符串解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 序列化为JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 检查是否有权限
    pub fn has_permission(&self, perm: PluginPermission) -> bool {
        self.permissions.contains(&perm)
    }

    /// 语义化版本比较（简化）
    pub fn version_matches(&self, constraint: &str) -> bool {
        // 简化：支持 ">=x.y.z", "^x.y", "x.y.z"
        let current = parse_version(&self.version);
        if constraint.starts_with(">=") {
            let required = parse_version(constraint.trim_start_matches(">="));
            return current >= required;
        }
        if constraint.starts_with("^") {
            let required = parse_version(constraint.trim_start_matches("^"));
            return current.0 == required.0 && current >= required;
        }
        let required = parse_version(constraint);
        current == required
    }
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = v.trim()
        .split('.')
        .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
        .collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

/// 插件运行时配置（用户配置的实际值）
#[derive(Debug, Clone, Default)]
pub struct PluginConfig {
    pub values: HashMap<String, serde_json::Value>,
}

impl PluginConfig {
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(|v| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let json = r#"{
            "id": "com.test.ocr",
            "name": "OCR Plugin",
            "version": "1.2.0",
            "author": "TestCorp",
            "description": "OCR文字识别",
            "entry": "plugin.wasm",
            "permissions": ["file:read", "ai:chat"],
            "capabilities": [{"id": "ocr.extract", "name": "提取文字", "description": "", "input_schema": {}, "output_schema": {}}]
        }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        assert_eq!(manifest.id, "com.test.ocr");
        assert_eq!(manifest.version, "1.2.0");
        assert!(manifest.has_permission(PluginPermission::FileRead));
        assert!(!manifest.has_permission(PluginPermission::FileWrite));
        assert_eq!(manifest.capabilities.len(), 1);
    }

    #[test]
    fn test_version_matches() {
        let manifest = PluginManifest {
            id: "test".into(), name: "test".into(), version: "1.5.3".into(),
            author: "test".into(), description: "test".into(), entry: "test.wasm".into(),
            permissions: vec![], dependencies: vec![], config_schema: vec![],
            capabilities: vec![], tags: vec![], homepage: None, repository: None,
            license: None, min_platform_version: "3.0.0".into(),
        };
        assert!(manifest.version_matches(">=1.0.0"));
        assert!(manifest.version_matches("^1.0"));
        assert!(!manifest.version_matches("^2.0"));
        assert!(manifest.version_matches("1.5.3"));
    }
}
