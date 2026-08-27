//! 宿主API绑定 — Host API Bindings
//!
//! 插件通过PluginContext调用宿主平台提供的能力。
//! 在WASM运行时中，这些调用通过wasmer的host function实现。

use crate::error::{PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 插件日志级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// AI聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub usage: AiUsage,
}

/// AI token使用量
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 插件上下文（插件运行时环境，提供宿主API访问）
pub struct PluginContext {
    /// 插件ID
    pub plugin_id: String,
    /// 插件名称
    pub plugin_name: String,
    /// 插件版本
    pub plugin_version: String,
    /// 租户ID
    pub tenant_id: Option<String>,
    /// 插件配置（从manifest的config_schema读取用户配置）
    pub config: HashMap<String, String>,
    /// 宿主API绑定（实际调用宿主的接口）
    host_api: HostApiBinding,
}

impl PluginContext {
    /// 创建插件上下文（由宿主运行时调用）
    pub fn new(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        plugin_version: impl Into<String>,
        host_api: HostApiBinding,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            plugin_name: plugin_name.into(),
            plugin_version: plugin_version.into(),
            tenant_id: None,
            config: HashMap::new(),
            host_api,
        }
    }

    /// 调用AI聊天（同步）
    pub async fn ai_chat(&self, prompt: impl Into<String>) -> PluginResult<AiChatResponse> {
        self.host_api.ai_chat(prompt.into(), None).await
    }

    /// 调用AI聊天（指定模型）
    pub async fn ai_chat_with_model(&self, prompt: impl Into<String>, model: impl Into<String>) -> PluginResult<AiChatResponse> {
        self.host_api.ai_chat(prompt.into(), Some(model.into())).await
    }

    /// 发布事件
    pub fn publish_event(&self, event_type: impl Into<String>, payload: serde_json::Value) -> PluginResult<()> {
        self.host_api.publish_event(event_type.into(), payload)
    }

    /// 记录日志
    pub fn log(&self, level: PluginLogLevel, message: impl Into<String>) {
        self.host_api.log(level, message.into());
    }

    /// 记录Info日志
    pub fn log_info(&self, message: impl Into<String>) {
        self.log(PluginLogLevel::Info, message);
    }

    /// 记录Warn日志
    pub fn log_warn(&self, message: impl Into<String>) {
        self.log(PluginLogLevel::Warn, message);
    }

    /// 记录Error日志
    pub fn log_error(&self, message: impl Into<String>) {
        self.log(PluginLogLevel::Error, message);
    }

    /// 记录Debug日志
    pub fn log_debug(&self, message: impl Into<String>) {
        self.log(PluginLogLevel::Debug, message);
    }

    /// 获取配置值
    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    /// 获取配置值（带默认值）
    pub fn get_config_or(&self, key: &str, default: &str) -> String {
        self.config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
}

/// 宿主API绑定（实际调用宿主的接口）
///
/// 在WASM运行时中，这些方法通过wasmer的import object绑定到宿主函数。
/// 在测试中，可以使用Mock实现。
#[derive(Clone)]
pub struct HostApiBinding {
    /// AI聊天函数（异步）
    ai_chat_fn: Arc<dyn Fn(String, Option<String>) -> PluginResult<AiChatResponse> + Send + Sync>,
    /// 事件发布函数
    publish_event_fn: Arc<dyn Fn(String, serde_json::Value) -> PluginResult<()> + Send + Sync>,
    /// 日志函数
    log_fn: Arc<dyn Fn(PluginLogLevel, String) + Send + Sync>,
}

use std::sync::Arc;

impl HostApiBinding {
    /// 创建宿主API绑定（由宿主运行时调用）
    pub fn new(
        ai_chat_fn: impl Fn(String, Option<String>) -> PluginResult<AiChatResponse> + Send + Sync + 'static,
        publish_event_fn: impl Fn(String, serde_json::Value) -> PluginResult<()> + Send + Sync + 'static,
        log_fn: impl Fn(PluginLogLevel, String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            ai_chat_fn: Arc::new(ai_chat_fn),
            publish_event_fn: Arc::new(publish_event_fn),
            log_fn: Arc::new(log_fn),
        }
    }

    /// 创建Mock宿主API绑定（用于测试）
    pub fn mock() -> Self {
        Self::new(
            |prompt, model| Ok(AiChatResponse {
                content: format!("Mock AI response to: {}", prompt),
                model: model.unwrap_or_else(|| "mock-model".into()),
                provider: "mock".into(),
                usage: AiUsage::default(),
            }),
            |_event_type, _payload| Ok(()),
            |level, msg| println!("[{:?}] {}", level, msg),
        )
    }

    /// 调用AI聊天
    pub async fn ai_chat(&self, prompt: String, model: Option<String>) -> PluginResult<AiChatResponse> {
        (self.ai_chat_fn)(prompt, model)
    }

    /// 发布事件
    pub fn publish_event(&self, event_type: String, payload: serde_json::Value) -> PluginResult<()> {
        (self.publish_event_fn)(event_type, payload)
    }

    /// 记录日志
    pub fn log(&self, level: PluginLogLevel, message: String) {
        (self.log_fn)(level, message);
    }
}
