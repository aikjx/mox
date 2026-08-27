// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 跨能力协调器 — Cross-capability Coordinator
//!
//! 协调4大对接能力之间的交互：
//! - Plugin调用AI（插件内AI能力代理）
//! - Enterprise调用Connector（政企数据同步到第三方系统）
//! - AI调用Connector（AI Agent调用外部工具）
//! - 统一事件总线（各能力间事件通信）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// 能力类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    /// AI Provider Gateway
    Ai,
    /// Plugin System
    Plugin,
    /// Enterprise Adapter
    Enterprise,
    /// Connector Framework
    Connector,
    /// 自定义能力
    Custom,
}

impl CapabilityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityType::Ai => "ai",
            CapabilityType::Plugin => "plugin",
            CapabilityType::Enterprise => "enterprise",
            CapabilityType::Connector => "connector",
            CapabilityType::Custom => "custom",
        }
    }
}

/// 能力句柄（运行时持有各能力的引用）
#[derive(Clone)]
pub struct CapabilityHandle {
    pub capability_type: CapabilityType,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    /// 能力元数据
    pub metadata: HashMap<String, String>,
}

impl CapabilityHandle {
    pub fn new(capability_type: CapabilityType, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            capability_type,
            name: name.into(),
            version: version.into(),
            enabled: true,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn is_enabled(&self) -> bool { self.enabled }
}

/// 协调事件（各能力间通信）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationEvent {
    /// 事件ID
    pub event_id: String,
    /// 事件类型
    pub event_type: String,
    /// 来源能力
    pub source: CapabilityType,
    /// 目标能力（None表示广播）
    pub target: Option<CapabilityType>,
    /// 事件载荷
    pub payload: serde_json::Value,
    /// 时间戳
    pub timestamp: String,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// 事件监听器
pub type EventListener = Arc<dyn Fn(&CoordinationEvent) -> anyhow::Result<()> + Send + Sync>;

/// 集成协调器
pub struct IntegrationCoordinator {
    /// 已注册的能力句柄
    capabilities: RwLock<HashMap<CapabilityType, CapabilityHandle>>,
    /// 事件监听器（按事件类型分组）
    listeners: RwLock<HashMap<String, Vec<EventListener>>>,
    /// 事件历史（最近N条）
    event_history: RwLock<Vec<CoordinationEvent>>,
    /// 最大历史记录数
    max_history: usize,
}

impl IntegrationCoordinator {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
            listeners: RwLock::new(HashMap::new()),
            event_history: RwLock::new(Vec::new()),
            max_history: 1000,
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// 注册能力
    pub fn register_capability(&self, handle: CapabilityHandle) {
        tracing::info!("coordinator register capability: {} ({}) v{}", handle.name, handle.capability_type.as_str(), handle.version);
        self.capabilities.write().insert(handle.capability_type, handle);
    }

    /// 获取能力句柄
    pub fn get_capability(&self, capability_type: CapabilityType) -> Option<CapabilityHandle> {
        self.capabilities.read().get(&capability_type).cloned()
    }

    /// 检查能力是否可用
    pub fn is_capability_available(&self, capability_type: CapabilityType) -> bool {
        self.capabilities.read()
            .get(&capability_type)
            .map(|h| h.is_enabled())
            .unwrap_or(false)
    }

    /// 列出所有已注册能力
    pub fn list_capabilities(&self) -> Vec<CapabilityHandle> {
        self.capabilities.read().values().cloned().collect()
    }

    /// 注册事件监听器
    pub fn on_event(&self, event_type: impl Into<String>, listener: EventListener) {
        let event_type = event_type.into();
        self.listeners.write().entry(event_type).or_default().push(listener);
    }

    /// 发布事件（同步通知所有监听器）
    pub fn publish_event(&self, event: CoordinationEvent) -> anyhow::Result<()> {
        tracing::debug!("coordinator event: {} from {} to {:?}", event.event_type, event.source.as_str(), event.target);

        // 记录历史
        {
            let mut history = self.event_history.write();
            history.push(event.clone());
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        // 通知监听器
        let listeners = self.listeners.read();
        if let Some(listeners) = listeners.get(&event.event_type) {
            for listener in listeners {
                if let Err(e) = listener(&event) {
                    tracing::error!("event listener error for {}: {}", event.event_type, e);
                }
            }
        }

        // 广播监听器（event_type = "*"）
        if let Some(broadcast_listeners) = listeners.get("*") {
            for listener in broadcast_listeners {
                if let Err(e) = listener(&event) {
                    tracing::error!("broadcast listener error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 便捷：创建并发布事件
    pub fn emit(&self, event_type: impl Into<String>, source: CapabilityType, payload: serde_json::Value) -> anyhow::Result<()> {
        let event = CoordinationEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            source,
            target: None,
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
        };
        self.publish_event(event)
    }

    /// 获取事件历史
    pub fn event_history(&self) -> Vec<CoordinationEvent> {
        self.event_history.read().clone()
    }

    /// 按类型获取事件历史
    pub fn event_history_by_type(&self, event_type: &str) -> Vec<CoordinationEvent> {
        self.event_history.read()
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// 能力数量
    pub fn capability_count(&self) -> usize {
        self.capabilities.read().len()
    }
}

impl Default for IntegrationCoordinator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_capability() {
        let coordinator = IntegrationCoordinator::new();
        let handle = CapabilityHandle::new(CapabilityType::Ai, "AI Gateway", "1.0.0");
        coordinator.register_capability(handle);
        assert!(coordinator.is_capability_available(CapabilityType::Ai));
        assert_eq!(coordinator.capability_count(), 1);
    }

    #[test]
    fn test_emit_and_listen() {
        let coordinator = IntegrationCoordinator::new();
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        coordinator.on_event("test.event", Arc::new(move |e| {
            received_clone.write().push(e.event_type.clone());
            Ok(())
        }));

        coordinator.emit("test.event", CapabilityType::Ai, serde_json::json!({"key": "value"})).unwrap();
        assert_eq!(received.read().len(), 1);
    }
}
