// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 宿主API — 插件可调用的平台能力绑定
//!
//! 插件通过WASM导入函数调用宿主API，宿主API在此定义和注册。
//! 所有API调用都经过权限检查。

use crate::manifest::PluginPermission;
use crate::registry::PluginInstance;
use std::sync::Arc;

/// 宿主API调用上下文
pub struct HostApiContext {
    /// 当前调用的插件实例
    pub plugin: Arc<PluginInstance>,
    /// 调用追踪ID
    pub trace_id: String,
}

impl HostApiContext {
    pub fn new(plugin: Arc<PluginInstance>) -> Self {
        Self {
            plugin,
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// 权限检查
    pub fn require_permission(&self, perm: PluginPermission) -> Result<(), HostApiError> {
        if !self.plugin.manifest.has_permission(perm) {
            tracing::warn!(
                "plugin {} denied permission: {}",
                self.plugin.id(),
                perm.as_str()
            );
            return Err(HostApiError::PermissionDenied(perm.as_str().into()));
        }
        Ok(())
    }

    /// 检查插件是否处于运行状态
    pub fn require_running(&self) -> Result<(), HostApiError> {
        if !self.plugin.is_running() {
            return Err(HostApiError::PluginNotRunning(self.plugin.id().into()));
        }
        Ok(())
    }
}

/// 宿主API错误
#[derive(Debug, thiserror::Error)]
pub enum HostApiError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("plugin not running: {0}")]
    PluginNotRunning(String),

    #[error("API not implemented: {0}")]
    NotImplemented(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// 宿主API结果
pub type HostApiResult<T> = Result<T, HostApiError>;

/// 宿主API trait — 所有宿主能力实现此接口
#[async_trait::async_trait]
pub trait HostApi: Send + Sync {
    /// API名称（如 "ai.chat", "file.read", "event.publish"）
    fn api_name(&self) -> &'static str;

    /// 所需权限
    fn required_permission(&self) -> Option<PluginPermission>;

    /// 调用API
    async fn call(&self, ctx: &HostApiContext, args: serde_json::Value) -> HostApiResult<serde_json::Value>;
}

/// AI聊天宿主API
pub struct AiChatHostApi {
    /// AI路由器（运行时注入）
    pub ai_router: Arc<tokio::sync::RwLock<Option<Arc<dyn AiChatDelegate>>>>,
}

/// AI聊天委托（由平台层注入实际实现）
#[async_trait::async_trait]
pub trait AiChatDelegate: Send + Sync {
    async fn chat(&self, messages: Vec<serde_json::Value>, model: Option<String>) -> Result<String, String>;
}

impl AiChatHostApi {
    pub fn new() -> Self {
        Self { ai_router: Arc::new(tokio::sync::RwLock::new(None)) }
    }

    pub async fn set_delegate(&self, delegate: Arc<dyn AiChatDelegate>) {
        *self.ai_router.write().await = Some(delegate);
    }
}

impl Default for AiChatHostApi {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl HostApi for AiChatHostApi {
    fn api_name(&self) -> &'static str { "ai.chat" }

    fn required_permission(&self) -> Option<PluginPermission> {
        Some(PluginPermission::AiChat)
    }

    async fn call(&self, ctx: &HostApiContext, args: serde_json::Value) -> HostApiResult<serde_json::Value> {
        ctx.require_running()?;
        ctx.require_permission(PluginPermission::AiChat)?;

        let messages = args.get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| HostApiError::InvalidArgument("missing messages".into()))?;

        let model = args.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

        let router = self.ai_router.read().await;
        let delegate = router.as_ref()
            .ok_or_else(|| HostApiError::NotImplemented("ai router not configured".into()))?;

        let result = delegate.chat(messages, model).await
            .map_err(|e| HostApiError::Internal(e))?;

        Ok(serde_json::json!({ "content": result }))
    }
}

/// 事件发布宿主API
pub struct EventPublishHostApi {
    pub event_bus: Arc<flume::Sender<(String, serde_json::Value)>>,
}

impl EventPublishHostApi {
    pub fn new(sender: flume::Sender<(String, serde_json::Value)>) -> Self {
        Self { event_bus: Arc::new(sender) }
    }
}

#[async_trait::async_trait]
impl HostApi for EventPublishHostApi {
    fn api_name(&self) -> &'static str { "event.publish" }

    fn required_permission(&self) -> Option<PluginPermission> {
        Some(PluginPermission::EventPublish)
    }

    async fn call(&self, ctx: &HostApiContext, args: serde_json::Value) -> HostApiResult<serde_json::Value> {
        ctx.require_running()?;
        ctx.require_permission(PluginPermission::EventPublish)?;

        let event_type = args.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HostApiError::InvalidArgument("missing event type".into()))?
            .to_string();

        let payload = args.get("payload").cloned().unwrap_or(serde_json::Value::Null);

        self.event_bus.send_async((event_type.clone(), payload.clone())).await
            .map_err(|e| HostApiError::Internal(format!("event bus send failed: {}", e)))?;

        let event_id = uuid::Uuid::new_v4().to_string();
        Ok(serde_json::json!({
            "ok": true,
            "event_id": event_id,
            "event_type": event_type,
            "published_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

/// 宿主API注册表 — 管理所有可用宿主API
pub struct HostApiRegistry {
    apis: parking_lot::RwLock<std::collections::HashMap<String, Arc<dyn HostApi>>>,
}

impl HostApiRegistry {
    pub fn new() -> Self {
        Self { apis: parking_lot::RwLock::new(std::collections::HashMap::new()) }
    }

    pub fn register(&self, api: Arc<dyn HostApi>) {
        let name = api.api_name().to_string();
        tracing::info!("register host API: {}", name);
        self.apis.write().insert(name, api);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn HostApi>> {
        self.apis.read().get(name).cloned()
    }

    pub fn list_names(&self) -> Vec<String> {
        self.apis.read().keys().cloned().collect()
    }

    /// 调用宿主API（带权限检查）
    pub async fn call(&self, ctx: &HostApiContext, api_name: &str, args: serde_json::Value) -> HostApiResult<serde_json::Value> {
        let api = self.get(api_name)
            .ok_or_else(|| HostApiError::NotImplemented(api_name.into()))?;
        api.call(ctx, args).await
    }
}

impl Default for HostApiRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        let manifest = crate::manifest::PluginManifest {
            id: "test".into(), name: "test".into(), version: "1.0.0".into(),
            author: "test".into(), description: "test".into(), entry: "test.wasm".into(),
            permissions: vec![PluginPermission::AiChat],
            dependencies: vec![], config_schema: vec![], capabilities: vec![],
            tags: vec![], homepage: None, repository: None, license: None,
            min_platform_version: "3.0.0".into(),
        };
        let instance = Arc::new(crate::registry::PluginInstance::new(manifest));
        let ctx = HostApiContext::new(instance);
        assert!(ctx.require_permission(PluginPermission::AiChat).is_ok());
        assert!(ctx.require_permission(PluginPermission::FileWrite).is_err());
    }
}
