// =============================================================================
// 订阅者（Subscriber）
// =============================================================================
//
// 事件订阅者负责处理特定类型的事件：
//
// - SubscriberId：订阅者唯一标识符
// - EventHandler：事件处理器 trait，支持异步处理
// - HandlerResult：处理结果（成功/失败/重试）
// - Subscriber：订阅者配置，包含过滤条件和处理器
// =============================================================================

use crate::event::{Event, EventType};
use crate::metadata::EventMetadata;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// 订阅者 ID
///
/// 唯一标识一个订阅者，用于管理和调试。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriberId(String);

impl SubscriberId {
    /// 创建新的订阅者 ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// 获取订阅者 ID 字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SubscriberId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// 事件处理结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResult {
    /// 处理成功
    Success,
    /// 处理失败，不重试（进入死信队列）
    Failed(String),
    /// 处理失败，需要重试
    Retry(String),
    /// 跳过此事件（不处理，也不报错）
    Skipped,
}

impl HandlerResult {
    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Skipped)
    }

    /// 是否需要重试
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Retry(_))
    }

    /// 是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// 获取错误消息
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed(msg) | Self::Retry(msg) => Some(msg),
            _ => None,
        }
    }
}

/// 异步事件处理器函数类型
pub type AsyncHandler<E> = Arc<
    dyn Fn(E, EventMetadata) -> Pin<Box<dyn Future<Output = HandlerResult> + Send>>
        + Send
        + Sync,
>;

/// 事件处理器 trait
///
/// 实现此 trait 的类型可以作为事件处理器注册到事件总线。
///
/// # 示例
/// ```
/// use mox_event_core::{EventHandler, HandlerResult, EventMetadata, Event, EventType};
/// use async_trait::async_trait;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct TestEvent { message: String }
/// impl Event for TestEvent {
///     fn event_type(&self) -> EventType { EventType::new("test.event") }
/// }
///
/// struct TestHandler;
///
/// #[async_trait]
/// impl EventHandler<TestEvent> for TestHandler {
///     async fn handle(&self, event: TestEvent, metadata: EventMetadata) -> HandlerResult {
///         println!("处理事件: {}", event.message);
///         HandlerResult::Success
///     }
/// }
/// ```
pub trait EventHandler<E: Event>: Send + Sync {
    /// 处理事件
    fn handle<'a>(
        &'a self,
        event: E,
        metadata: EventMetadata,
    ) -> Pin<Box<dyn Future<Output = HandlerResult> + Send + 'a>>;
}

/// 订阅者配置
///
/// 包含订阅者 ID、订阅的事件类型列表和事件处理器。
pub struct Subscriber<E: Event> {
    /// 订阅者 ID
    id: SubscriberId,
    /// 订阅的事件类型列表（支持通配符）
    event_types: Vec<EventType>,
    /// 事件处理器
    handler: AsyncHandler<E>,
    /// 最大并发处理数
    max_concurrency: usize,
    /// 处理超时（毫秒）
    timeout_ms: Option<u64>,
}

impl<E: Event> Subscriber<E> {
    /// 创建新的订阅者
    pub fn new<F>(id: SubscriberId, event_types: Vec<EventType>, handler: F) -> Self
    where
        F: Fn(E, EventMetadata) -> Pin<Box<dyn Future<Output = HandlerResult> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            id,
            event_types,
            handler: Arc::new(handler),
            max_concurrency: 10,
            timeout_ms: Some(30000),
        }
    }

    /// 获取订阅者 ID
    pub fn id(&self) -> &SubscriberId {
        &self.id
    }

    /// 获取订阅的事件类型
    pub fn event_types(&self) -> &[EventType] {
        &self.event_types
    }

    /// 检查是否订阅指定事件类型
    pub fn subscribes_to(&self, event_type: &EventType) -> bool {
        self.event_types.iter().any(|et| et.matches(event_type))
    }

    /// 获取最大并发数
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// 设置最大并发数
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    /// 获取处理超时
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    /// 设置处理超时
    pub fn with_timeout_ms(mut self, timeout_ms: Option<u64>) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// 处理事件
    pub async fn handle(&self, event: E, metadata: EventMetadata) -> HandlerResult {
        if let Some(timeout_ms) = self.timeout_ms {
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                (self.handler)(event, metadata),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => HandlerResult::Failed("处理超时".to_string()),
            }
        } else {
            (self.handler)(event, metadata).await
        }
    }
}

/// 类型擦除的订阅者（用于在事件总线中存储不同类型的订阅者）
pub(crate) struct TypeErasedSubscriber {
    pub id: SubscriberId,
    pub event_types: Vec<EventType>,
    pub max_concurrency: usize,
    pub timeout_ms: Option<u64>,
    /// 处理函数（接收序列化后的事件载荷和元数据）
    pub handler: Arc<
        dyn Fn(
                crate::event::EventPayload,
                EventMetadata,
            ) -> Pin<Box<dyn Future<Output = HandlerResult> + Send>>
            + Send
            + Sync,
    >,
}

impl TypeErasedSubscriber {
    /// 从具体订阅者创建类型擦除订阅者
    pub fn from_subscriber<E: Event>(subscriber: Subscriber<E>) -> Self {
        let handler = subscriber.handler.clone();
        let erased_handler: Arc<
            dyn Fn(
                    crate::event::EventPayload,
                    EventMetadata,
                ) -> Pin<Box<dyn Future<Output = HandlerResult> + Send>>
                + Send
                + Sync,
        > = Arc::new(move |payload, metadata| {
            let handler = handler.clone();
            Box::pin(async move {
                match E::from_payload(&payload) {
                    Ok(event) => handler(event, metadata).await,
                    Err(e) => HandlerResult::Failed(format!("事件反序列化失败: {}", e)),
                }
            })
        });

        Self {
            id: subscriber.id,
            event_types: subscriber.event_types,
            max_concurrency: subscriber.max_concurrency,
            timeout_ms: subscriber.timeout_ms,
            handler: erased_handler,
        }
    }

    /// 检查是否订阅指定事件类型
    pub fn subscribes_to(&self, event_type: &EventType) -> bool {
        self.event_types.iter().any(|et| et.matches(event_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        message: String,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::new("test.event")
        }
    }

    #[tokio::test]
    async fn test_subscriber_handle() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let subscriber = Subscriber::new(
            SubscriberId::new("test-sub"),
            vec![EventType::new("test.event")],
            move |_evt: TestEvent, _meta| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    HandlerResult::Success
                })
            },
        );

        assert_eq!(subscriber.id().as_str(), "test-sub");
        assert!(subscriber.subscribes_to(&EventType::new("test.event")));
        assert!(!subscriber.subscribes_to(&EventType::new("other.event")));

        let result = subscriber
            .handle(
                TestEvent {
                    message: "test".to_string(),
                },
                EventMetadata::new(crate::metadata::EventSource::new("test")),
            )
            .await;
        assert!(result.is_success());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handler_result() {
        assert!(HandlerResult::Success.is_success());
        assert!(HandlerResult::Skipped.is_success());
        assert!(!HandlerResult::Failed("err".to_string()).is_success());
        assert!(HandlerResult::Retry("err".to_string()).should_retry());
        assert_eq!(
            HandlerResult::Failed("test error".to_string()).error_message(),
            Some("test error")
        );
    }

    #[tokio::test]
    async fn test_subscriber_timeout() {
        let subscriber = Subscriber::new(
            SubscriberId::new("timeout-sub"),
            vec![EventType::new("test.event")],
            move |_evt: TestEvent, _meta| {
                Box::pin(async {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    HandlerResult::Success
                })
            },
        )
        .with_timeout_ms(Some(100));

        let result = subscriber
            .handle(
                TestEvent {
                    message: "test".to_string(),
                },
                EventMetadata::new(crate::metadata::EventSource::new("test")),
            )
            .await;
        assert!(result.is_failed());
        assert!(result.error_message().unwrap().contains("超时"));
    }
}
