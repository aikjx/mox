// =============================================================================
// 定时触发调度器（EventScheduler）
// =============================================================================
//
// 企业级"定时触发器"抽象：到点通过 EventBus 发布事件，业务由订阅者响应。
// 定时器只负责"到点发事件"，监听器只负责"响应事件"——两者靠事件总线解耦。
//
// 设计要点：
// - 一次性触发（delay）与周期触发（interval）两类任务
// - 触发载荷由工厂闭包构造（到点才构造事件，避免提前序列化）
// - 取消：abort 后台任务句柄
// - 多副本语义：本调度器是"单写者"触发源，分布式多副本场景应由
//   租约选主（mox-alliance-scheduler-core / leadership.rs 同族）限定
//   唯一 leader 驱动本调度器，再配合任务幂等实现"到点只执行一次"
// - 持久化接缝：内存任务表重启即失；需要跨重启恢复时接入存储实现
//   （SQLite/WAL，见 MOX_ALLIANCE_STORAGE_MODE=sqlite 同族约定）
// - 超时/审批类决策：到点仅发布事件（如 ApprovalDue），具体处理由
//   订阅者执行（与 mox-approval-core 的 overdue_check 接缝）
// =============================================================================

use crate::bus::EventBus;
use crate::event::{Event, EventType};
use crate::metadata::EventMetadata;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::task::JoinHandle;

/// 调度任务 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScheduledTaskId(pub String);

impl ScheduledTaskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ScheduledTaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 调度任务元数据（用于统计与审计）
#[derive(Debug, Clone)]
pub struct ScheduledTaskInfo {
    pub id: ScheduledTaskId,
    pub event_type: String,
    pub kind: ScheduleKind,
    pub created_at_ms: u64,
}

/// 任务种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleKind {
    /// 一次性（到点触发一次后移除）
    Once,
    /// 周期（按固定间隔重复触发，直到取消）
    Repeat,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 事件源名称（默认 mox-event-scheduler）
    pub default_source: String,
    /// 任务表容量上限（防失控注册）
    pub max_tasks: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            default_source: "mox-event-scheduler".to_string(),
            max_tasks: 10000,
        }
    }
}

/// 调度器统计
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub scheduled_total: u64,
    pub fired_total: u64,
    pub cancelled_total: u64,
    pub active_tasks: usize,
}

/// 后台任务上下文（与调度器结构解耦，避免闭包持有 Self 引用）
struct SchedulerCtx<B> {
    bus: Arc<B>,
    source: String,
    fired: Arc<AtomicU64>,
}

/// 定时触发调度器
///
/// 持有事件总线引用（泛型 B：任何实现 EventBus 的总线，当前为 MemoryEventBus，
/// 预留 Redis/Kafka 实现），所有触发均通过总线发布事件。
/// 本调度器自身不做多副本选主——分布式语义由调用方（leader 调度器）保证。
pub struct EventScheduler<B: EventBus> {
    bus: Arc<B>,
    config: SchedulerConfig,
    tasks: Arc<RwLock<HashMap<ScheduledTaskId, JoinHandle<()>>>>,
    infos: RwLock<HashMap<ScheduledTaskId, ScheduledTaskInfo>>,
    scheduled_total: AtomicU64,
    fired_total: Arc<AtomicU64>,
    cancelled_total: AtomicU64,
}

impl<B: EventBus + 'static> EventScheduler<B> {
    /// 创建调度器（挂到事件总线）
    pub fn new(bus: Arc<B>, config: SchedulerConfig) -> Arc<Self> {
        Arc::new(Self {
            bus,
            config,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            infos: RwLock::new(HashMap::new()),
            scheduled_total: AtomicU64::new(0),
            fired_total: Arc::new(AtomicU64::new(0)),
            cancelled_total: AtomicU64::new(0),
        })
    }

    /// 使用默认配置创建
    #[allow(clippy::should_implement_trait)]
    pub fn default(bus: Arc<B>) -> Arc<Self> {
        Self::new(bus, SchedulerConfig::default())
    }

    /// 注册一次性任务：delay 后触发一次
    pub fn schedule_once<E, F>(
        &self,
        id: ScheduledTaskId,
        event_type: EventType,
        delay: Duration,
        factory: F,
    ) -> crate::EventResult<()>
    where
        E: Event + Send + 'static,
        F: Fn() -> E + Send + Sync + 'static,
    {
        let ctx = SchedulerCtx {
            bus: Arc::clone(&self.bus),
            source: self.config.default_source.clone(),
            fired: Arc::clone(&self.fired_total),
        };
        self.spawn_task(id.clone(), event_type.clone(), ScheduleKind::Once, move || {
            let bus = Arc::clone(&ctx.bus);
            let source = ctx.source.clone();
            let fired = Arc::clone(&ctx.fired);
            let task_id = id.clone();
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                let evt = factory();
                let meta = EventMetadata::new(crate::metadata::EventSource::new(&source));
                match bus.publish_with_metadata(evt, meta).await {
                    Ok(()) => {
                        fired.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => tracing::error!(error = %e, task = %task_id, "定时任务发布失败"),
                }
            })
        })
    }

    /// 注册周期任务：按 interval 重复触发，直到取消或关停
    pub fn schedule_repeat<E, F>(
        &self,
        id: ScheduledTaskId,
        event_type: EventType,
        interval: Duration,
        factory: F,
    ) -> crate::EventResult<()>
    where
        E: Event + Send + 'static,
        F: Fn() -> E + Send + Sync + 'static,
    {
        let ctx = SchedulerCtx {
            bus: Arc::clone(&self.bus),
            source: self.config.default_source.clone(),
            fired: Arc::clone(&self.fired_total),
        };
        self.spawn_task(id.clone(), event_type.clone(), ScheduleKind::Repeat, move || {
            let bus = Arc::clone(&ctx.bus);
            let source = ctx.source.clone();
            let fired = Arc::clone(&ctx.fired);
            let task_id = id.clone();
            Box::pin(async move {
                let mut ticker = tokio::time::interval(interval);
                // 首次触发不等一个完整周期
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    let evt = factory();
                    let meta = EventMetadata::new(crate::metadata::EventSource::new(&source));
                    match bus.publish_with_metadata(evt, meta).await {
                        Ok(()) => {
                            fired.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(e) => tracing::error!(error = %e, task = %task_id, "周期任务发布失败"),
                    }
                }
            })
        })
    }

    /// 取消任务（abort 后台任务并移除；统计计入 cancelled_total）
    pub fn cancel(&self, id: &ScheduledTaskId) -> bool {
        let removed = self.tasks.write().remove(id);
        match removed {
            Some(handle) => {
                handle.abort();
                self.cancelled_total.fetch_add(1, Ordering::SeqCst);
                self.infos.write().remove(id);
                true
            }
            None => false,
        }
    }

    /// 查询任务信息
    pub fn info(&self, id: &ScheduledTaskId) -> Option<ScheduledTaskInfo> {
        self.infos.read().get(id).cloned()
    }

    /// 当前活跃任务数
    pub fn active_count(&self) -> usize {
        self.tasks.read().len()
    }

    /// 统计
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            scheduled_total: self.scheduled_total.load(Ordering::SeqCst),
            fired_total: self.fired_total.load(Ordering::SeqCst),
            cancelled_total: self.cancelled_total.load(Ordering::SeqCst),
            active_tasks: self.tasks.read().len(),
        }
    }

    fn spawn_task<F>(
        &self,
        id: ScheduledTaskId,
        event_type: EventType,
        kind: ScheduleKind,
        run: F,
    ) -> crate::EventResult<()>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + 'static,
    {
        {
            let mut tasks = self.tasks.write();
            if tasks.len() >= self.config.max_tasks {
                return Err(crate::EventError::InternalError(format!(
                    "调度任务已达上限 {}",
                    self.config.max_tasks
                )));
            }
            if tasks.contains_key(&id) {
                return Err(crate::EventError::SubscriberAlreadyExists(format!(
                    "调度任务已存在: {id}"
                )));
            }
            let h: JoinHandle<()> = {
                let tasks = Arc::clone(&self.tasks);
                let task_id = id.clone();
                tokio::spawn(async move {
                    (run)().await;
                    // 任务自然结束后从活跃表移除（once 触发完成；repeat 由 cancel abort）
                    tasks.write().remove(&task_id);
                })
            };
            tasks.insert(id.clone(), h);
        }
        self.infos.write().insert(
            id.clone(),
            ScheduledTaskInfo {
                id,
                event_type: event_type.as_str().to_string(),
                kind,
                created_at_ms: now_ms(),
            },
        );
        self.scheduled_total.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{MemoryEventBus, BusConfig};
    use crate::subscriber::{HandlerResult, SubscriberId};
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TickEvent {
        seq: u64,
    }

    impl Event for TickEvent {
        fn event_type(&self) -> EventType {
            EventType::new("scheduler.tick")
        }
    }

    #[tokio::test]
    async fn test_schedule_once_fires_after_delay() {
        let bus = MemoryEventBus::new(BusConfig::default());
        let counter = Arc::new(AtomicUsize::new(0));
        let c2 = counter.clone();
        bus.subscribe(
            SubscriberId::new("sched-test"),
            vec![EventType::new("scheduler.tick")],
            move |_e: TickEvent, _m| {
                let c = c2.clone();
                Box::pin(async move {
                    c.fetch_add(1, AtomicOrdering::SeqCst);
                    HandlerResult::Success
                })
            },
        )
        .await
        .unwrap();

        let sched = EventScheduler::default(bus.clone());
        sched.schedule_once(
            ScheduledTaskId::new("t1"),
            EventType::new("scheduler.tick"),
            Duration::from_millis(30),
            || TickEvent { seq: 1 },
        )
        .unwrap();

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        let stats = sched.stats();
        assert_eq!(stats.fired_total, 1);
        assert_eq!(stats.scheduled_total, 1);
    }

    #[tokio::test]
    async fn test_schedule_repeat_fires_multiple_and_cancel() {
        let bus = MemoryEventBus::new(BusConfig::default());
        let counter = Arc::new(AtomicUsize::new(0));
        let c2 = counter.clone();
        bus.subscribe(
            SubscriberId::new("sched-repeat"),
            vec![EventType::new("scheduler.tick")],
            move |_e: TickEvent, _m| {
                let c = c2.clone();
                Box::pin(async move {
                    c.fetch_add(1, AtomicOrdering::SeqCst);
                    HandlerResult::Success
                })
            },
        )
        .await
        .unwrap();

        let sched = EventScheduler::default(bus);
        sched.schedule_repeat(
            ScheduledTaskId::new("r1"),
            EventType::new("scheduler.tick"),
            Duration::from_millis(20),
            || TickEvent { seq: 2 },
        )
        .unwrap();

        tokio::time::sleep(Duration::from_millis(120)).await;
        let fired = counter.load(AtomicOrdering::SeqCst);
        assert!(fired >= 3, "周期任务应多次触发, 实际 {fired}");

        let before = counter.load(AtomicOrdering::SeqCst);
        assert!(sched.cancel(&ScheduledTaskId::new("r1")));
        tokio::time::sleep(Duration::from_millis(60)).await;
        let after = counter.load(AtomicOrdering::SeqCst);
        assert_eq!(before, after, "取消后不应再触发");
        assert_eq!(sched.stats().cancelled_total, 1);
    }

    #[tokio::test]
    async fn test_duplicate_task_id_rejected() {
        let bus = MemoryEventBus::default();
        let sched = EventScheduler::default(bus);
        sched.schedule_once(
            ScheduledTaskId::new("dup"),
            EventType::new("scheduler.tick"),
            Duration::from_millis(1),
            || TickEvent { seq: 1 },
        )
        .unwrap();
        let err = sched.schedule_once(
            ScheduledTaskId::new("dup"),
            EventType::new("scheduler.tick"),
            Duration::from_millis(1),
            || TickEvent { seq: 2 },
        );
        assert!(err.is_err(), "重复任务 ID 应被拒绝");
    }
}
