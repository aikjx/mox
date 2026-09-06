// =============================================================================
// 事件总线（EventBus）
// =============================================================================
//
// 事件总线是事件驱动架构的核心，负责：
//
// - 订阅管理：注册/注销订阅者，支持按事件类型过滤
// - 事件发布：同步/异步发布事件，支持通配符匹配
// - 事件路由：将事件路由到所有匹配的订阅者
// - 错误处理：失败事件进入死信队列，支持自动重试
// - 并发控制：每个订阅者独立的并发控制和超时
// - 优雅关停：等待所有正在处理的事件完成
//
// 设计参考 minio event/pubsub 架构，支持多种实现：
// - MemoryEventBus：内存实现，适用于单进程
// - （预留）RedisEventBus：Redis 实现，适用于分布式
// - （预留）KafkaEventBus：Kafka 实现，适用于高吞吐场景
// =============================================================================

use crate::dead_letter::{DeadLetterQueue, DeadLetterEntry};
use crate::event::{Event, EventPayload, EventType};
use crate::metadata::{EventMetadata, EventSource};
use crate::subscriber::{Subscriber, SubscriberId, TypeErasedSubscriber, HandlerResult};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Semaphore;

/// 事件总线配置
#[derive(Debug, Clone)]
pub struct BusConfig {
    /// 是否异步发布（默认 true）
    pub async_publish: bool,
    /// 发布超时（毫秒，0 表示不超时）
    pub publish_timeout_ms: u64,
    /// 事件源名称
    pub default_source: String,
    /// 是否启用死信队列
    pub dead_letter_enabled: bool,
    /// 最大并发发布数
    pub max_concurrent_publish: usize,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            async_publish: true,
            publish_timeout_ms: 30000,
            default_source: "mox-event-bus".to_string(),
            dead_letter_enabled: true,
            max_concurrent_publish: 100,
        }
    }
}

/// 事件总线统计
#[derive(Debug, Clone, Default)]
pub struct BusStats {
    /// 已发布事件总数
    pub published_total: u64,
    /// 已处理事件总数
    pub handled_total: u64,
    /// 处理成功总数
    pub success_total: u64,
    /// 处理失败总数
    pub failed_total: u64,
    /// 重试总数
    pub retried_total: u64,
    /// 当前订阅者数
    pub subscriber_count: usize,
    /// 当前活跃处理数
    pub active_handlers: u64,
}

/// 事件总线 trait
///
/// 所有事件总线实现必须实现此 trait。
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    /// 发布事件
    async fn publish<E: Event>(&self, event: E) -> crate::EventResult<()>;

    /// 发布事件（带元数据）
    async fn publish_with_metadata<E: Event>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> crate::EventResult<()>;

    /// 订阅事件
    async fn subscribe<E: Event, F>(
        &self,
        id: SubscriberId,
        event_types: Vec<EventType>,
        handler: F,
    ) -> crate::EventResult<()>
    where
        F: Fn(E, EventMetadata) -> std::pin::Pin<Box<dyn std::future::Future<Output = HandlerResult> + Send>>
            + Send
            + Sync
            + 'static;

    /// 取消订阅
    async fn unsubscribe(&self, id: &SubscriberId) -> crate::EventResult<bool>;

    /// 获取统计信息
    fn stats(&self) -> BusStats;

    /// 优雅关停（等待所有处理完成）
    async fn shutdown(&self);
}

/// 事件总线内部状态（可在异步任务中安全共享）
struct BusInner {
    /// 配置
    config: BusConfig,
    /// 订阅者（按 ID 存储）
    subscribers: RwLock<HashMap<SubscriberId, Arc<TypeErasedSubscriber>>>,
    /// 死信队列
    dead_letter: Option<Arc<DeadLetterQueue>>,
    /// 统计
    published_total: AtomicU64,
    handled_total: AtomicU64,
    success_total: AtomicU64,
    failed_total: AtomicU64,
    retried_total: AtomicU64,
    active_handlers: AtomicU64,
    /// 是否正在关停
    shutting_down: AtomicBool,
}

/// 内存事件总线
///
/// 单进程内的事件总线实现，适用于单体应用和测试。
pub struct MemoryEventBus {
    inner: Arc<BusInner>,
    /// 发布信号量（控制并发）
    publish_semaphore: Arc<Semaphore>,
}

impl MemoryEventBus {
    /// 创建新的内存事件总线
    pub fn new(config: BusConfig) -> Arc<Self> {
        let dead_letter = if config.dead_letter_enabled {
            Some(DeadLetterQueue::default())
        } else {
            None
        };

        let inner = Arc::new(BusInner {
            config,
            subscribers: RwLock::new(HashMap::new()),
            dead_letter,
            published_total: AtomicU64::new(0),
            handled_total: AtomicU64::new(0),
            success_total: AtomicU64::new(0),
            failed_total: AtomicU64::new(0),
            retried_total: AtomicU64::new(0),
            active_handlers: AtomicU64::new(0),
            shutting_down: AtomicBool::new(false),
        });

        let max_concurrent = inner.config.max_concurrent_publish;
        Arc::new(Self {
            inner,
            publish_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }

    /// 创建默认配置的内存事件总线
    pub fn default() -> Arc<Self> {
        Self::new(BusConfig::default())
    }

    /// 获取死信队列引用
    pub fn dead_letter(&self) -> Option<&Arc<DeadLetterQueue>> {
        self.inner.dead_letter.as_ref()
    }

    /// 获取匹配指定事件类型的订阅者
    fn get_matching_subscribers(inner: &BusInner, event_type: &EventType) -> Vec<Arc<TypeErasedSubscriber>> {
        let subscribers = inner.subscribers.read();
        subscribers
            .values()
            .filter(|s| s.subscribes_to(event_type))
            .cloned()
            .collect()
    }

    /// 处理单个订阅者的事件（静态方法，可在异步任务中安全调用）
    async fn dispatch_to_subscriber(
        inner: Arc<BusInner>,
        subscriber: Arc<TypeErasedSubscriber>,
        payload: EventPayload,
        metadata: EventMetadata,
        event_type: EventType,
    ) {
        inner.active_handlers.fetch_add(1, Ordering::SeqCst);

        let result = if let Some(timeout_ms) = subscriber.timeout_ms {
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                (subscriber.handler)(payload.clone(), metadata.clone()),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => HandlerResult::Failed("处理超时".to_string()),
            }
        } else {
            (subscriber.handler)(payload.clone(), metadata.clone()).await
        };

        inner.handled_total.fetch_add(1, Ordering::SeqCst);
        inner.active_handlers.fetch_sub(1, Ordering::SeqCst);

        match result {
            HandlerResult::Success | HandlerResult::Skipped => {
                inner.success_total.fetch_add(1, Ordering::SeqCst);
            }
            HandlerResult::Failed(msg) => {
                inner.failed_total.fetch_add(1, Ordering::SeqCst);
                if let Some(dlq) = &inner.dead_letter {
                    let entry = DeadLetterEntry::new(
                        metadata.event_id().clone(),
                        event_type.clone(),
                        payload,
                        metadata,
                        subscriber.id.clone(),
                        msg,
                    );
                    if let Err(e) = dlq.push(entry) {
                        tracing::error!(error = %e, "死信队列写入失败");
                    }
                }
            }
            HandlerResult::Retry(msg) => {
                inner.retried_total.fetch_add(1, Ordering::SeqCst);
                inner.failed_total.fetch_add(1, Ordering::SeqCst);
                if let Some(dlq) = &inner.dead_letter {
                    let entry = DeadLetterEntry::new(
                        metadata.event_id().clone(),
                        event_type.clone(),
                        payload,
                        metadata,
                        subscriber.id.clone(),
                        msg,
                    );
                    if let Err(e) = dlq.push(entry) {
                        tracing::error!(error = %e, "死信队列写入失败");
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl EventBus for MemoryEventBus {
    async fn publish<E: Event>(&self, event: E) -> crate::EventResult<()> {
        let metadata = EventMetadata::new(EventSource::new(&self.inner.config.default_source));
        self.publish_with_metadata(event, metadata).await
    }

    async fn publish_with_metadata<E: Event>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> crate::EventResult<()> {
        if self.inner.shutting_down.load(Ordering::SeqCst) {
            return Err(crate::EventError::BusClosed);
        }

        let event_type = event.event_type();
        let payload = event
            .to_payload()
            .map_err(|e| crate::EventError::SerializationFailed(e.to_string()))?;

        // 获取匹配的订阅者
        let matching = Self::get_matching_subscribers(&self.inner, &event_type);
        self.inner.published_total.fetch_add(1, Ordering::SeqCst);

        if matching.is_empty() {
            tracing::debug!(event_type = %event_type, "没有匹配的订阅者，事件已丢弃");
            return Ok(());
        }

        // 并发控制
        let semaphore = self.publish_semaphore.clone();
        let _permit = semaphore
            .acquire()
            .await
            .map_err(|e| crate::EventError::InternalError(e.to_string()))?;

        if self.inner.config.async_publish {
            // 异步发布：不等待处理完成
            for subscriber in matching {
                let inner = self.inner.clone();
                let payload = payload.clone();
                let metadata = metadata.clone();
                let event_type = event_type.clone();
                tokio::spawn(async move {
                    Self::dispatch_to_subscriber(inner, subscriber, payload, metadata, event_type)
                        .await;
                });
            }
        } else {
            // 同步发布：等待所有处理完成
            let mut handles = Vec::new();
            for subscriber in matching {
                let inner = self.inner.clone();
                let payload = payload.clone();
                let metadata = metadata.clone();
                let event_type = event_type.clone();
                handles.push(tokio::spawn(async move {
                    Self::dispatch_to_subscriber(inner, subscriber, payload, metadata, event_type)
                        .await;
                }));
            }
            for handle in handles {
                if let Err(e) = handle.await {
                    tracing::error!(error = %e, "事件处理任务失败");
                }
            }
        }

        Ok(())
    }

    async fn subscribe<E: Event, F>(
        &self,
        id: SubscriberId,
        event_types: Vec<EventType>,
        handler: F,
    ) -> crate::EventResult<()>
    where
        F: Fn(E, EventMetadata) -> std::pin::Pin<Box<dyn std::future::Future<Output = HandlerResult> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let subscriber = Subscriber::new(id.clone(), event_types, handler);
        let erased = TypeErasedSubscriber::from_subscriber(subscriber);

        let mut subscribers = self.inner.subscribers.write();
        if subscribers.contains_key(&id) {
            return Err(crate::EventError::SubscriberAlreadyExists(id.to_string()));
        }
        subscribers.insert(id, Arc::new(erased));
        Ok(())
    }

    async fn unsubscribe(&self, id: &SubscriberId) -> crate::EventResult<bool> {
        let mut subscribers = self.inner.subscribers.write();
        Ok(subscribers.remove(id).is_some())
    }

    fn stats(&self) -> BusStats {
        BusStats {
            published_total: self.inner.published_total.load(Ordering::SeqCst),
            handled_total: self.inner.handled_total.load(Ordering::SeqCst),
            success_total: self.inner.success_total.load(Ordering::SeqCst),
            failed_total: self.inner.failed_total.load(Ordering::SeqCst),
            retried_total: self.inner.retried_total.load(Ordering::SeqCst),
            subscriber_count: self.inner.subscribers.read().len(),
            active_handlers: self.inner.active_handlers.load(Ordering::SeqCst),
        }
    }

    async fn shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::SeqCst);
        // 等待所有活跃处理完成（最多等待30秒）
        for _ in 0..300 {
            if self.inner.active_handlers.load(Ordering::SeqCst) == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        tracing::info!("事件总线已关停");
    }
}

// 为 Arc<MemoryEventBus> 实现 EventBus，方便使用
#[async_trait::async_trait]
impl EventBus for Arc<MemoryEventBus> {
    async fn publish<E: Event>(&self, event: E) -> crate::EventResult<()> {
        (**self).publish(event).await
    }

    async fn publish_with_metadata<E: Event>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> crate::EventResult<()> {
        (**self).publish_with_metadata(event, metadata).await
    }

    async fn subscribe<E: Event, F>(
        &self,
        id: SubscriberId,
        event_types: Vec<EventType>,
        handler: F,
    ) -> crate::EventResult<()>
    where
        F: Fn(E, EventMetadata) -> std::pin::Pin<Box<dyn std::future::Future<Output = HandlerResult> + Send>>
            + Send
            + Sync
            + 'static,
    {
        (**self).subscribe(id, event_types, handler).await
    }

    async fn unsubscribe(&self, id: &SubscriberId) -> crate::EventResult<bool> {
        (**self).unsubscribe(id).await
    }

    fn stats(&self) -> BusStats {
        (**self).stats()
    }

    async fn shutdown(&self) {
        (**self).shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct OtherEvent {
        data: String,
    }

    impl Event for OtherEvent {
        fn event_type(&self) -> EventType {
            EventType::new("other.event")
        }
    }

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = MemoryEventBus::new(BusConfig {
            async_publish: false,
            ..Default::default()
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.subscribe(
            SubscriberId::new("test-sub"),
            vec![EventType::new("test.event")],
            move |_evt: TestEvent, _meta| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    HandlerResult::Success
                })
            },
        )
        .await
        .unwrap();

        bus.publish(TestEvent {
            message: "hello".to_string(),
            value: 42,
        })
        .await
        .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_wildcard_subscription() {
        let bus = MemoryEventBus::new(BusConfig {
            async_publish: false,
            ..Default::default()
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.subscribe(
            SubscriberId::new("wildcard-sub"),
            vec![EventType::new("test.*")],
            move |_evt: TestEvent, _meta| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    HandlerResult::Success
                })
            },
        )
        .await
        .unwrap();

        bus.publish(TestEvent {
            message: "test".to_string(),
            value: 1,
        })
        .await
        .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_no_matching_subscriber() {
        let bus = MemoryEventBus::default();

        // 没有订阅者，发布应该成功但不处理
        let result = bus
            .publish(TestEvent {
                message: "test".to_string(),
                value: 1,
            })
            .await;
        assert!(result.is_ok());

        let stats = bus.stats();
        assert_eq!(stats.published_total, 1);
        assert_eq!(stats.handled_total, 0);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let bus = MemoryEventBus::new(BusConfig {
            async_publish: false,
            ..Default::default()
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let sub_id = SubscriberId::new("test-sub");
        bus.subscribe(
            sub_id.clone(),
            vec![EventType::new("test.event")],
            move |_evt: TestEvent, _meta| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    HandlerResult::Success
                })
            },
        )
        .await
        .unwrap();

        // 取消订阅
        let removed = bus.unsubscribe(&sub_id).await.unwrap();
        assert!(removed);

        // 再次发布应该没有处理
        bus.publish(TestEvent {
            message: "test".to_string(),
            value: 1,
        })
        .await
        .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_duplicate_subscriber() {
        let bus = MemoryEventBus::default();

        bus.subscribe(
            SubscriberId::new("test-sub"),
            vec![EventType::new("test.event")],
            |_evt: TestEvent, _meta| Box::pin(async { HandlerResult::Success }),
        )
        .await
        .unwrap();

        // 重复订阅应该失败
        let result = bus
            .subscribe(
                SubscriberId::new("test-sub"),
                vec![EventType::new("test.event")],
                |_evt: TestEvent, _meta| Box::pin(async { HandlerResult::Success }),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_failed_handler_dead_letter() {
        let bus = MemoryEventBus::new(BusConfig {
            async_publish: false,
            ..Default::default()
        });

        bus.subscribe(
            SubscriberId::new("failing-sub"),
            vec![EventType::new("test.event")],
            |_evt: TestEvent, _meta| {
                Box::pin(async { HandlerResult::Failed("处理失败".to_string()) })
            },
        )
        .await
        .unwrap();

        bus.publish(TestEvent {
            message: "test".to_string(),
            value: 1,
        })
        .await
        .unwrap();

        let stats = bus.stats();
        assert_eq!(stats.failed_total, 1);

        // 检查死信队列
        if let Some(dlq) = bus.dead_letter() {
            let dlq_stats = dlq.stats();
            assert_eq!(dlq_stats.total, 1);
        }
    }

    #[tokio::test]
    async fn test_bus_stats() {
        let bus = MemoryEventBus::default();
        let stats = bus.stats();
        assert_eq!(stats.published_total, 0);
        assert_eq!(stats.subscriber_count, 0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let bus = MemoryEventBus::default();
        bus.shutdown().await;

        // 关停后发布应该失败
        let result = bus
            .publish(TestEvent {
                message: "test".to_string(),
                value: 1,
            })
            .await;
        assert!(result.is_err());
    }
}
