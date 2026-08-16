//! 事件总线

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 事件总线
pub struct EventBus {
    /// 订阅映射：domain -> (event_type -> handlers)
    subscriptions: RwLock<HashMap<String, HashMap<String, Vec<EventHandlerFn>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// 订阅事件
    pub async fn subscribe(
        &self,
        domain: String,
        event_type: String,
        handler: EventHandlerFn,
    ) {
        let mut subs = self.subscriptions.write();

        subs.entry(domain.clone())
            .or_insert_with(HashMap::new)
            .entry(event_type.clone())
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// 发送事件
    pub async fn emit(&self, event: Event) {
        let subs = self.subscriptions.read();

        if let Some(domain_subs) = subs.get(&event.domain()) {
            if let Some(handlers) = domain_subs.get(&event.event_type()) {
                for handler in handlers {
                    handler(event.payload());
                }
            }
        }
    }

    /// 取消订阅
    pub async fn unsubscribe(&self, _subscription_id: &str) {
        // TODO: 实现取消订阅逻辑
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    // Profile事件
    ProfileLoaded { name: String, path: String },
    ProfileUnloaded { name: String },

    // Bundle事件
    BundleMounted { name: String, version: String },
    BundleUnmounted { name: String },

    // Turn事件
    TurnStarted { turn_id: String, agent_id: String },
    TurnCompleted { turn_id: String, summary: super::TurnSummary },

    // Step事件
    StepStarted { step_id: String, turn_id: String },
    StepCompleted { step_id: String, turn_id: String, success: bool },
    StepFailed { step_id: String, turn_id: String, error: String },

    // 系统事件
    SystemStarted { timestamp: chrono::DateTime<chrono::Utc> },
    SystemShutdown { timestamp: chrono::DateTime<chrono::Utc> },
    Error { domain: String, error: String },
}

impl Event {
    /// 获取事件域
    pub fn domain(&self) -> String {
        match self {
            Event::ProfileLoaded { .. } | Event::ProfileUnloaded { .. } => "profile".to_string(),
            Event::BundleMounted { .. } | Event::BundleUnmounted { .. } => "bundle".to_string(),
            Event::TurnStarted { .. } | Event::TurnCompleted { .. } => "turn".to_string(),
            Event::StepStarted { .. } | Event::StepCompleted { .. } | Event::StepFailed { .. } => "step".to_string(),
            Event::SystemStarted { .. } | Event::SystemShutdown { .. } | Event::Error { .. } => "system".to_string(),
        }
    }

    /// 获取事件类型
    pub fn event_type(&self) -> String {
        match self {
            Event::ProfileLoaded { .. } => "loaded".to_string(),
            Event::ProfileUnloaded { .. } => "unloaded".to_string(),
            Event::BundleMounted { .. } => "mounted".to_string(),
            Event::BundleUnmounted { .. } => "unmounted".to_string(),
            Event::TurnStarted { .. } => "started".to_string(),
            Event::TurnCompleted { .. } => "completed".to_string(),
            Event::StepStarted { .. } => "started".to_string(),
            Event::StepCompleted { .. } => "completed".to_string(),
            Event::StepFailed { .. } => "failed".to_string(),
            Event::SystemStarted { .. } => "started".to_string(),
            Event::SystemShutdown { .. } => "shutdown".to_string(),
            Event::Error { .. } => "error".to_string(),
        }
    }

    /// 获取事件载荷
    pub fn payload(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!(null))
    }
}

/// 事件域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventDomain {
    Profile,
    Bundle,
    Turn,
    Step,
    System,
    Agent,
    Tool,
}

/// 事件处理器函数
pub type EventHandlerFn = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// 订阅
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub domain: String,
    pub event_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let handler = {
            let called = called.clone();
            Arc::new(move |_payload: serde_json::Value| {
                called.store(true, std::sync::atomic::Ordering::SeqCst);
            })
        };

        bus.subscribe("turn".to_string(), "started".to_string(), handler).await;

        bus.emit(Event::TurnStarted {
            turn_id: "test-turn".to_string(),
            agent_id: "test-agent".to_string(),
        }).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
