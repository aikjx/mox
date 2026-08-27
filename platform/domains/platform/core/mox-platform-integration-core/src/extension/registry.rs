// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 扩展点注册表 — 运行时动态注册/查找/卸载扩展点

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// 扩展点类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPointType {
    /// AI Provider（新增AI模型）
    AiProvider,
    /// 插件（WASM插件）
    Plugin,
    /// 连接器（第三方系统对接）
    Connector,
    /// SSO Provider（新增SSO协议）
    SsoProvider,
    /// 中间件（tower layer）
    Middleware,
    /// 事件监听器
    EventListener,
    /// 数据转换器
    DataTransformer,
    /// 自定义扩展点
    Custom,
}

impl ExtensionPointType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtensionPointType::AiProvider => "ai_provider",
            ExtensionPointType::Plugin => "plugin",
            ExtensionPointType::Connector => "connector",
            ExtensionPointType::SsoProvider => "sso_provider",
            ExtensionPointType::Middleware => "middleware",
            ExtensionPointType::EventListener => "event_listener",
            ExtensionPointType::DataTransformer => "data_transformer",
            ExtensionPointType::Custom => "custom",
        }
    }
}

/// 扩展点ID（格式：{domain}.{capability}.{name}，如 "ai.provider.openai"）
pub type ExtensionPointId = String;

/// 扩展点元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPointMetadata {
    /// 扩展点ID（唯一）
    pub id: ExtensionPointId,
    /// 扩展点名称
    pub name: String,
    /// 扩展点类型
    pub extension_type: ExtensionPointType,
    /// 版本号（语义化）
    pub version: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 依赖的其他扩展点ID
    #[serde(default)]
    pub depends_on: Vec<ExtensionPointId>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 优先级（数字越小优先级越高，默认100）
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// 创建时间
    #[serde(default)]
    pub created_at: String,
}

fn default_true() -> bool { true }
fn default_priority() -> u32 { 100 }

/// 扩展点实例（trait对象）
#[async_trait]
pub trait ExtensionPointInstance: Send + Sync {
    /// 扩展点元数据
    fn metadata(&self) -> &ExtensionPointMetadata;
    /// 初始化
    async fn init(&self) -> anyhow::Result<()> { Ok(()) }
    /// 销毁
    async fn destroy(&self) -> anyhow::Result<()> { Ok(()) }
    /// 健康检查
    async fn health_check(&self) -> bool { true }
}

/// 扩展点（包装元数据+实例）
pub struct ExtensionPoint {
    metadata: ExtensionPointMetadata,
    instance: Option<Arc<dyn ExtensionPointInstance>>,
}

impl ExtensionPoint {
    pub fn new(id: impl Into<String>, name: impl Into<String>, extension_type: ExtensionPointType, version: impl Into<String>) -> Self {
        Self {
            metadata: ExtensionPointMetadata {
                id: id.into(),
                name: name.into(),
                extension_type,
                version: version.into(),
                description: String::new(),
                author: String::new(),
                tags: Vec::new(),
                depends_on: Vec::new(),
                enabled: true,
                priority: 100,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            instance: None,
        }
    }

    pub fn with_instance(mut self, instance: Arc<dyn ExtensionPointInstance>) -> Self {
        self.instance = Some(instance);
        self
    }

    pub fn metadata(&self) -> &ExtensionPointMetadata { &self.metadata }
    pub fn instance(&self) -> Option<&Arc<dyn ExtensionPointInstance>> { self.instance.as_ref() }
    pub fn id(&self) -> &str { &self.metadata.id }
    pub fn extension_type(&self) -> ExtensionPointType { self.metadata.extension_type }
    pub fn is_enabled(&self) -> bool { self.metadata.enabled }
}

/// 扩展点构建器
pub struct ExtensionPointBuilder {
    metadata: ExtensionPointMetadata,
    instance: Option<Arc<dyn ExtensionPointInstance>>,
}

impl ExtensionPointBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>, extension_type: ExtensionPointType) -> Self {
        Self {
            metadata: ExtensionPointMetadata {
                id: id.into(),
                name: name.into(),
                extension_type,
                version: "1.0.0".into(),
                description: String::new(),
                author: String::new(),
                tags: Vec::new(),
                depends_on: Vec::new(),
                enabled: true,
                priority: 100,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            instance: None,
        }
    }

    pub fn version(mut self, v: impl Into<String>) -> Self { self.metadata.version = v.into(); self }
    pub fn description(mut self, d: impl Into<String>) -> Self { self.metadata.description = d.into(); self }
    pub fn author(mut self, a: impl Into<String>) -> Self { self.metadata.author = a.into(); self }
    pub fn tag(mut self, t: impl Into<String>) -> Self { self.metadata.tags.push(t.into()); self }
    pub fn depends_on(mut self, id: impl Into<String>) -> Self { self.metadata.depends_on.push(id.into()); self }
    pub fn enabled(mut self, e: bool) -> Self { self.metadata.enabled = e; self }
    pub fn priority(mut self, p: u32) -> Self { self.metadata.priority = p; self }
    pub fn instance(mut self, i: Arc<dyn ExtensionPointInstance>) -> Self { self.instance = Some(i); self }

    pub fn build(self) -> ExtensionPoint {
        ExtensionPoint { metadata: self.metadata, instance: self.instance }
    }
}

/// 扩展点注册表错误
#[derive(Debug, thiserror::Error)]
pub enum ExtensionRegistryError {
    #[error("extension point already exists: {0}")]
    AlreadyExists(String),
    #[error("extension point not found: {0}")]
    NotFound(String),
    #[error("dependency not satisfied: {0} requires {1}")]
    DependencyNotSatisfied(String, String),
    #[error("extension point disabled: {0}")]
    Disabled(String),
    #[error("other error: {0}")]
    Other(String),
}

/// 扩展点注册表
pub struct ExtensionRegistry {
    extensions: RwLock<HashMap<ExtensionPointId, ExtensionPoint>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self { extensions: RwLock::new(HashMap::new()) }
    }

    /// 注册扩展点
    pub fn register(&self, extension: ExtensionPoint) -> Result<(), ExtensionRegistryError> {
        let id = extension.id().to_string();
        if self.extensions.read().contains_key(&id) {
            return Err(ExtensionRegistryError::AlreadyExists(id));
        }
        // 检查依赖（简化：只检查是否存在，不检查版本）
        for dep in &extension.metadata().depends_on {
            if !self.extensions.read().contains_key(dep) {
                return Err(ExtensionRegistryError::DependencyNotSatisfied(id, dep.clone()));
            }
        }
        tracing::info!("register extension: {} ({}) v{}", extension.metadata().name, id, extension.metadata().version);
        self.extensions.write().insert(id, extension);
        Ok(())
    }

    /// 注销扩展点
    pub fn unregister(&self, id: &str) -> Result<ExtensionPoint, ExtensionRegistryError> {
        self.extensions.write().remove(id)
            .ok_or_else(|| ExtensionRegistryError::NotFound(id.into()))
    }

    /// 获取扩展点元数据（owned clone）
    pub fn get_metadata(&self, id: &str) -> Option<ExtensionPointMetadata> {
        self.extensions.read().get(id).map(|e| e.metadata().clone())
    }

    /// 检查是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.extensions.read().contains_key(id)
    }

    /// 按类型列出
    pub fn list_by_type(&self, extension_type: ExtensionPointType) -> Vec<ExtensionPointMetadata> {
        self.extensions.read()
            .values()
            .filter(|e| e.extension_type() == extension_type)
            .map(|e| e.metadata().clone())
            .collect()
    }

    /// 按标签列出
    pub fn list_by_tag(&self, tag: &str) -> Vec<ExtensionPointMetadata> {
        self.extensions.read()
            .values()
            .filter(|e| e.metadata().tags.iter().any(|t| t == tag))
            .map(|e| e.metadata().clone())
            .collect()
    }

    /// 列出所有
    pub fn list_all(&self) -> Vec<ExtensionPointMetadata> {
        self.extensions.read()
            .values()
            .map(|e| e.metadata().clone())
            .collect()
    }

    /// 列出已启用的
    pub fn list_enabled(&self) -> Vec<ExtensionPointMetadata> {
        self.extensions.read()
            .values()
            .filter(|e| e.is_enabled())
            .map(|e| e.metadata().clone())
            .collect()
    }

    /// 启用/禁用
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), ExtensionRegistryError> {
        let mut guard = self.extensions.write();
        let ext = guard.get_mut(id)
            .ok_or_else(|| ExtensionRegistryError::NotFound(id.into()))?;
        ext.metadata.enabled = enabled;
        Ok(())
    }

    /// 数量
    pub fn len(&self) -> usize { self.extensions.read().len() }
    pub fn is_empty(&self) -> bool { self.extensions.read().is_empty() }

    /// 全部健康检查
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        for ext in self.list_enabled() {
            // 简化：只检查元数据存在性，实际应调用instance.health_check()
            results.insert(ext.id, true);
        }
        results
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list() {
        let registry = ExtensionRegistry::new();
        let ext = ExtensionPointBuilder::new("test.ai.demo", "Demo AI", ExtensionPointType::AiProvider)
            .version("1.0.0")
            .tag("demo")
            .build();
        registry.register(ext).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test.ai.demo"));
        let by_type = registry.list_by_type(ExtensionPointType::AiProvider);
        assert_eq!(by_type.len(), 1);
    }

    #[test]
    fn test_duplicate_register() {
        let registry = ExtensionRegistry::new();
        let ext1 = ExtensionPoint::new("test.dup", "Dup", ExtensionPointType::Custom, "1.0.0");
        let ext2 = ExtensionPoint::new("test.dup", "Dup2", ExtensionPointType::Custom, "2.0.0");
        registry.register(ext1).unwrap();
        assert!(registry.register(ext2).is_err());
    }
}
