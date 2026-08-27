// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 事件总线

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 订阅映射类型别名：domain -> (event_type -> [(subscription_id, handler)])
type SubscriptionMap = HashMap<String, HashMap<String, Vec<(String, EventHandlerFn)>>>;

/// 事件总线
pub struct EventBus {
    /// 订阅映射：domain -> (event_type -> [(subscription_id, handler)])
    subscriptions: RwLock<SubscriptionMap>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// 订阅事件，返回带唯一 ID 的订阅凭证（用于精确取消订阅）
    pub async fn subscribe(
        &self,
        domain: String,
        event_type: String,
        handler: EventHandlerFn,
    ) -> Subscription {
        let id = uuid::Uuid::new_v4().to_string();
        let mut subs = self.subscriptions.write();

        subs.entry(domain.clone())
            .or_default()
            .entry(event_type.clone())
            .or_default()
            .push((id.clone(), handler));

        Subscription {
            id,
            domain,
            event_type,
        }
    }

    /// 发送事件（按订阅注册顺序同步分发）
    pub async fn emit(&self, event: Event) {
        let subs = self.subscriptions.read();

        if let Some(domain_subs) = subs.get(&event.domain()) {
            if let Some(handlers) = domain_subs.get(&event.event_type()) {
                for (_id, handler) in handlers {
                    handler(event.payload());
                }
            }
        }
    }

    /// 取消订阅：按订阅 ID 精确移除，返回是否实际移除了订阅
    pub async fn unsubscribe(&self, subscription_id: &str) -> bool {
        let mut subs = self.subscriptions.write();
        for domain_subs in subs.values_mut() {
            for handlers in domain_subs.values_mut() {
                let before = handlers.len();
                handlers.retain(|(id, _)| id != subscription_id);
                if handlers.len() != before {
                    return true;
                }
            }
        }
        false
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
    ProfileLoaded {
        name: String,
        path: String,
    },
    ProfileUnloaded {
        name: String,
    },

    // Bundle事件
    BundleMounted {
        name: String,
        version: String,
    },
    BundleUnmounted {
        name: String,
    },

    // Turn事件
    TurnStarted {
        turn_id: String,
        agent_id: String,
    },
    TurnCompleted {
        turn_id: String,
        summary: super::TurnSummary,
    },

    // Step事件
    StepStarted {
        step_id: String,
        turn_id: String,
    },
    StepCompleted {
        step_id: String,
        turn_id: String,
        success: bool,
    },
    StepFailed {
        step_id: String,
        turn_id: String,
        error: String,
    },

    // 系统事件
    SystemStarted {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    SystemShutdown {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Error {
        domain: String,
        error: String,
    },
}

impl Event {
    /// 获取事件域
    pub fn domain(&self) -> String {
        match self {
            Event::ProfileLoaded { .. } | Event::ProfileUnloaded { .. } => "profile".to_string(),
            Event::BundleMounted { .. } | Event::BundleUnmounted { .. } => "bundle".to_string(),
            Event::TurnStarted { .. } | Event::TurnCompleted { .. } => "turn".to_string(),
            Event::StepStarted { .. } | Event::StepCompleted { .. } | Event::StepFailed { .. } => {
                "step".to_string()
            }
            Event::SystemStarted { .. } | Event::SystemShutdown { .. } | Event::Error { .. } => {
                "system".to_string()
            }
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

        let sub = bus
            .subscribe("turn".to_string(), "started".to_string(), handler)
            .await;

        bus.emit(Event::TurnStarted {
            turn_id: "test-turn".to_string(),
            agent_id: "test-agent".to_string(),
        })
        .await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));

        // 取消订阅后不再收到事件
        assert!(bus.unsubscribe(&sub.id).await);
        called.store(false, std::sync::atomic::Ordering::SeqCst);
        bus.emit(Event::TurnStarted {
            turn_id: "test-turn-2".to_string(),
            agent_id: "test-agent".to_string(),
        })
        .await;
        assert!(!called.load(std::sync::atomic::Ordering::SeqCst));

        // 重复取消返回 false
        assert!(!bus.unsubscribe(&sub.id).await);
    }

    #[tokio::test]
    async fn test_event_bus_isolated_handlers() {
        let bus = EventBus::new();
        let hits = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let mk = |hits: Arc<std::sync::atomic::AtomicU32>| {
            Arc::new(move |_p: serde_json::Value| {
                hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
        };

        let a = bus
            .subscribe("turn".to_string(), "started".to_string(), mk(hits.clone()))
            .await;
        let b = bus
            .subscribe("turn".to_string(), "started".to_string(), mk(hits.clone()))
            .await;

        bus.emit(Event::TurnStarted {
            turn_id: "t".to_string(),
            agent_id: "a".to_string(),
        })
        .await;
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2);

        // 只取消 a，b 仍接收
        assert!(bus.unsubscribe(&a.id).await);
        bus.emit(Event::TurnStarted {
            turn_id: "t2".to_string(),
            agent_id: "a".to_string(),
        })
        .await;
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 3);

        assert!(bus.unsubscribe(&b.id).await);
        bus.emit(Event::TurnStarted {
            turn_id: "t3".to_string(),
            agent_id: "a".to_string(),
        })
        .await;
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
