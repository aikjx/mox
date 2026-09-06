// =============================================================================
// MOX 统一事件总线核心（mox-event-core）
// =============================================================================
//
// 企业级事件驱动架构基础设施，参考 minio event/pubsub 架构设计：
//
// 1. **事件模型**（Event）— 所有事件的基础接口，携带元数据和载荷
// 2. **事件总线**（EventBus）— 管理订阅者，支持同步/异步发布
// 3. **订阅者**（Subscriber）— 处理事件，支持过滤和错误处理
// 4. **死信队列**（DeadLetterQueue）— 处理失败的事件，支持重试
// 5. **事件溯源**（EventSourcing）— 事件持久化和重放
//
// 设计原则：
// - 解耦：发布者和订阅者完全解耦，通过事件类型关联
// - 异步：支持异步事件处理，不阻塞主流程
// - 可靠：失败事件进入死信队列，支持重试和人工介入
// - 可观测：事件处理指标、日志、追踪全覆盖
// - 可扩展：支持多种事件总线实现（内存/Redis/Kafka）
// =============================================================================

pub mod event;
pub mod bus;
pub mod subscriber;
pub mod dead_letter;
pub mod metadata;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use event::{Event, EventPayload, EventType};
pub use bus::{EventBus, MemoryEventBus, BusConfig};
pub use subscriber::{Subscriber, SubscriberId, HandlerResult, EventHandler};
pub use dead_letter::{DeadLetterQueue, DeadLetterEntry, RetryPolicy};
pub use metadata::{EventMetadata, EventId, EventSource};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-event-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use thiserror::Error;

/// 事件总线错误
#[derive(Debug, Error)]
pub enum EventError {
    #[error("事件发布失败: {0}")]
    PublishFailed(String),

    #[error("订阅者不存在: {0}")]
    SubscriberNotFound(String),

    #[error("订阅者已存在: {0}")]
    SubscriberAlreadyExists(String),

    #[error("事件处理失败: {0}")]
    HandlerFailed(String),

    #[error("事件序列化失败: {0}")]
    SerializationFailed(String),

    #[error("事件反序列化失败: {0}")]
    DeserializationFailed(String),

    #[error("死信队列已满")]
    DeadLetterQueueFull,

    #[error("总线已关闭")]
    BusClosed,

    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 事件总线结果类型
pub type EventResult<T> = Result<T, EventError>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        message: String,
        value: i32,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::new("test.event")
        }
    }

    #[tokio::test]
    async fn test_event_basic() {
        let evt = TestEvent {
            message: "hello".to_string(),
            value: 42,
        };
        assert_eq!(evt.event_type().as_str(), "test.event");
    }

    #[tokio::test]
    async fn test_memory_event_bus_publish_subscribe() {
        let bus = MemoryEventBus::new(BusConfig::default());
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.subscribe(
            SubscriberId::new("test-subscriber"),
            vec![EventType::new("test.event")],
            move |_evt: TestEvent, _meta: EventMetadata| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    HandlerResult::Success
                })
            },
        ).await.unwrap();

        bus.publish(TestEvent {
            message: "test".to_string(),
            value: 1,
        }).await.unwrap();

        // 等待异步处理
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_metadata() {
        let metadata = EventMetadata::new(EventSource::new("test-service"));
        assert!(!metadata.event_id().as_str().is_empty());
        assert_eq!(metadata.source().as_str(), "test-service");
        assert!(metadata.timestamp() > 0);
    }

    #[test]
    fn test_event_error_display() {
        let err = EventError::PublishFailed("test".to_string());
        assert!(err.to_string().contains("事件发布失败"));
    }
}
