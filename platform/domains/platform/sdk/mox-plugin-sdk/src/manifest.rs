//! 插件Manifest生成工具 — Plugin Manifest Builder
//!
//! 用于在构建时或运行时生成插件描述符（manifest.json）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 插件权限（与mox-plugin-core中的PluginPermission保持一致）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    FileRead,
    FileWrite,
    NetworkApi,
    NetworkServer,
    AiChat,
    Database,
    Cache,
    EventPublish,
    EventSubscribe,
    SystemCommand,
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

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub optional: bool,
}

/// 插件配置字段（JSON Schema子集）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub secret: bool,
}

/// 插件能力声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
}

/// 插件Manifest（描述符）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件唯一ID（反向域名格式，如 com.vendor.ocr）
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号（语义化）
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// WASM入口文件路径
    pub entry: String,
    /// 权限列表
    pub permissions: Vec<PluginPermission>,
    /// 依赖列表
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// 配置Schema
    #[serde(default)]
    pub config_schema: Vec<ConfigField>,
    /// 能力声明
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 最低平台版本要求
    #[serde(default = "default_min_platform")]
    pub min_platform_version: String,
    /// 主页URL
    #[serde(default)]
    pub homepage: Option<String>,
    /// 仓库URL
    #[serde(default)]
    pub repository: Option<String>,
    /// 许可证
    #[serde(default)]
    pub license: Option<String>,
}

fn default_min_platform() -> String { "3.0.0".into() }

/// 插件Manifest构建器
pub struct PluginManifestBuilder {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    entry: String,
    permissions: Vec<PluginPermission>,
    dependencies: Vec<PluginDependency>,
    config_schema: Vec<ConfigField>,
    capabilities: Vec<PluginCapability>,
    tags: Vec<String>,
    min_platform_version: String,
    homepage: Option<String>,
    repository: Option<String>,
    license: Option<String>,
}

impl PluginManifestBuilder {
    /// 创建构建器
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            author: String::new(),
            description: String::new(),
            entry: "plugin.wasm".into(),
            permissions: Vec::new(),
            dependencies: Vec::new(),
            config_schema: Vec::new(),
            capabilities: Vec::new(),
            tags: Vec::new(),
            min_platform_version: default_min_platform(),
            homepage: None,
            repository: None,
            license: None,
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self { self.author = author.into(); self }
    pub fn description(mut self, desc: impl Into<String>) -> Self { self.description = desc.into(); self }
    pub fn entry(mut self, entry: impl Into<String>) -> Self { self.entry = entry.into(); self }
    pub fn permission(mut self, perm: PluginPermission) -> Self { self.permissions.push(perm); self }
    pub fn permissions(mut self, perms: Vec<PluginPermission>) -> Self { self.permissions = perms; self }
    pub fn dependency(mut self, id: impl Into<String>, version: impl Into<String>) -> Self {
        self.dependencies.push(PluginDependency { id: id.into(), version: version.into(), optional: false });
        self
    }
    pub fn config_field(mut self, field: ConfigField) -> Self { self.config_schema.push(field); self }
    pub fn capability(mut self, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.capabilities.push(PluginCapability { name: name.into(), description: desc.into(), version: "1.0.0".into() });
        self
    }
    pub fn tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn min_platform_version(mut self, v: impl Into<String>) -> Self { self.min_platform_version = v.into(); self }
    pub fn homepage(mut self, url: impl Into<String>) -> Self { self.homepage = Some(url.into()); self }
    pub fn repository(mut self, url: impl Into<String>) -> Self { self.repository = Some(url.into()); self }
    pub fn license(mut self, license: impl Into<String>) -> Self { self.license = Some(license.into()); self }

    /// 构建Manifest
    pub fn build(self) -> PluginManifest {
        PluginManifest {
            id: self.id,
            name: self.name,
            version: self.version,
            author: self.author,
            description: self.description,
            entry: self.entry,
            permissions: self.permissions,
            dependencies: self.dependencies,
            config_schema: self.config_schema,
            capabilities: self.capabilities,
            tags: self.tags,
            min_platform_version: self.min_platform_version,
            homepage: self.homepage,
            repository: self.repository,
            license: self.license,
        }
    }

    /// 构建并序列化为JSON字符串
    pub fn build_json(self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_builder() {
        let manifest = PluginManifestBuilder::new("com.example.ocr", "OCR Plugin", "1.0.0")
            .author("Example Inc")
            .description("OCR text extraction plugin")
            .permission(PluginPermission::AiChat)
            .permission(PluginPermission::NetworkApi)
            .tag("ocr")
            .tag("ai")
            .build();

        assert_eq!(manifest.id, "com.example.ocr");
        assert_eq!(manifest.permissions.len(), 2);
        assert_eq!(manifest.tags.len(), 2);
    }
}
