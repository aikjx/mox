// =============================================================================
// 集成测试：EventScheduler（定时触发）→ EventBus（路由）→ 订阅者（响应）
// =============================================================================
//
// 验证"到点发事件"闭环：注册一次性/周期任务 → 到点发布事件 → 订阅者收到并计数；
// 取消后不再触发；事件统计一致。审批超时等业务语义由订阅者侧（如
// mox-alliance-scheduler-core 的 approval_gate reap_expired 定时驱动）实现，
// 本测试只验证调度器与总线基础设施链路本身。
// =============================================================================

use mox_event_core::{
    Event, EventType, EventMetadata, EventBus, HandlerResult, MemoryEventBus, SubscriberId,
    EventScheduler, ScheduledTaskId, SchedulerConfig,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 业务事件（示例：审批超时重估信号）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DueCheckEvent {
    subject_id: String,
}

impl Event for DueCheckEvent {
    fn event_type(&self) -> EventType {
        EventType::new("approval.due.check")
    }
}

#[tokio::test]
async fn test_scheduler_event_bus_subscriber_chain_once() {
    let bus = MemoryEventBus::default();
    let counter = Arc::new(AtomicUsize::new(0));
    let c2 = counter.clone();
    bus.subscribe(
        SubscriberId::new("due-check-handler"),
        vec![EventType::new("approval.due.check")],
        move |_evt: DueCheckEvent, _meta: EventMetadata| {
            let c = c2.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                HandlerResult::Success
            })
        },
    )
    .await
    .unwrap();

    let sched = EventScheduler::new(
        bus.clone(),
        SchedulerConfig {
            default_source: "approval-due-scheduler".to_string(),
            max_tasks: 100,
        },
    );
    sched.schedule_once(
        ScheduledTaskId::new("due-check-1"),
        EventType::new("approval.due.check"),
        Duration::from_millis(30),
        || DueCheckEvent { subject_id: "approval-1".to_string() },
    )
    .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    let stats = sched.stats();
    assert_eq!(stats.fired_total, 1);
    // once 任务自然结束并从活跃表移除
    assert_eq!(stats.active_tasks, 0);
}

#[tokio::test]
async fn test_scheduler_event_bus_chain_repeat_and_cancel() {
    let bus = MemoryEventBus::default();
    let counter = Arc::new(AtomicUsize::new(0));
    let c2 = counter.clone();
    bus.subscribe(
        SubscriberId::new("due-check-repeat"),
        vec![EventType::new("approval.due.check")],
        move |_evt: DueCheckEvent, _meta: EventMetadata| {
            let c = c2.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                HandlerResult::Success
            })
        },
    )
    .await
    .unwrap();

    let sched = EventScheduler::default(bus);
    sched.schedule_repeat(
        ScheduledTaskId::new("due-check-repeat"),
        EventType::new("approval.due.check"),
        Duration::from_millis(20),
        || DueCheckEvent { subject_id: "approval-2".to_string() },
    )
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    let fired = counter.load(Ordering::SeqCst);
    assert!(fired >= 3, "周期任务应多次触发, 实际 {fired}");

    let before = counter.load(Ordering::SeqCst);
    assert!(sched.cancel(&ScheduledTaskId::new("due-check-repeat")));
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(before, counter.load(Ordering::SeqCst), "取消后不应再触发");
    assert_eq!(sched.stats().cancelled_total, 1);
    assert_eq!(sched.stats().active_tasks, 0);
}
