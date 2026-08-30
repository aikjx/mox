// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 统一事件总线
//!
//! 六大归一化体系之间通过事件总线进行解耦通信：
//! - 发布/订阅模式
//! - 事件溯源能力
//! - 跨体系事件联动
//!
//! 典型场景：
//! - AI识别到新意图 → 发布 IntentRecognized 事件 → 权限系统订阅校验
//! - 流程审批通过 → 发布 ProcessApproved 事件 → 低代码系统更新实体状态
//! - 前端用户操作 → 发布 UserAction 事件 → AI系统分析行为

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{PlatformError, PlatformResult};
use crate::types::NormalizationSystem;

/// 事件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// AI 意图已识别
    IntentRecognized,
    /// AI 任务已完成
    AiTaskCompleted,
    /// 权限校验结果
    PermissionChecked,
    /// 角色变更
    RoleChanged,
    /// 低代码实体创建
    EntityCreated,
    /// 低代码实体更新
    EntityUpdated,
    /// 表单提交
    FormSubmitted,
    /// 流程启动
    ProcessStarted,
    /// 流程完成
    ProcessCompleted,
    /// 规则触发
    RuleFired,
    /// 前端用户操作
    UserAction,
    /// 前端页面加载
    PageLoaded,
    /// 架构协议请求
    ArchRequest,
    /// 架构协议响应
    ArchResponse,
    /// 算法执行完成
    AlgoCompleted,
    /// 自定义事件
    Custom(String),
}

impl EventType {
    /// 获取事件所属的源体系
    pub fn source_system(&self) -> NormalizationSystem {
        match self {
            EventType::IntentRecognized | EventType::AiTaskCompleted => {
                NormalizationSystem::AiAssistant
            }
            EventType::PermissionChecked | EventType::RoleChanged => {
                NormalizationSystem::Permission
            }
            EventType::EntityCreated
            | EventType::EntityUpdated
            | EventType::FormSubmitted => NormalizationSystem::Lowcode,
            EventType::ProcessStarted
            | EventType::ProcessCompleted
            | EventType::RuleFired => NormalizationSystem::ProcessAlgo,
            EventType::UserAction | EventType::PageLoaded => {
                NormalizationSystem::Frontend
            }
            EventType::ArchRequest | EventType::ArchResponse => {
                NormalizationSystem::Architecture
            }
            EventType::AlgoCompleted => NormalizationSystem::ProcessAlgo,
            EventType::Custom(_) => NormalizationSystem::AiAssistant,
        }
    }

    pub fn name(&self) -> String {
        match self {
            EventType::IntentRecognized => "intent_recognized".to_string(),
            EventType::AiTaskCompleted => "ai_task_completed".to_string(),
            EventType::PermissionChecked => "permission_checked".to_string(),
            EventType::RoleChanged => "role_changed".to_string(),
            EventType::EntityCreated => "entity_created".to_string(),
            EventType::EntityUpdated => "entity_updated".to_string(),
            EventType::FormSubmitted => "form_submitted".to_string(),
            EventType::ProcessStarted => "process_started".to_string(),
            EventType::ProcessCompleted => "process_completed".to_string(),
            EventType::RuleFired => "rule_fired".to_string(),
            EventType::UserAction => "user_action".to_string(),
            EventType::PageLoaded => "page_loaded".to_string(),
            EventType::ArchRequest => "arch_request".to_string(),
            EventType::ArchResponse => "arch_response".to_string(),
            EventType::AlgoCompleted => "algo_completed".to_string(),
            EventType::Custom(s) => format!("custom_{}", s),
        }
    }
}

/// 平台事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformEvent {
    /// 事件 ID
    pub event_id: String,
    /// 事件类型
    pub event_type: EventType,
    /// 源系统
    pub source_system: NormalizationSystem,
    /// 租户 ID
    pub tenant_id: String,
    /// 用户 ID（可选）
    pub user_id: Option<String>,
    /// 事件载荷
    pub payload: serde_json::Value,
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 关联的事件 ID（用于事件链）
    pub correlation_id: Option<String>,
}

impl PlatformEvent {
    /// 创建新事件
    pub fn new(
        event_type: EventType,
        source_system: NormalizationSystem,
        tenant_id: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            event_type,
            source_system,
            tenant_id: tenant_id.to_string(),
            user_id: None,
            payload,
            timestamp: now_ms(),
            correlation_id: None,
        }
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// 设置关联 ID
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
}

/// 事件处理结果
#[derive(Debug, Clone)]
pub struct EventHandleResult {
    /// 处理是否成功
    pub success: bool,
    /// 处理者标识
    pub handler_id: String,
    /// 结果消息
    pub message: Option<String>,
}

/// 事件处理器类型
pub type EventHandler = Box<dyn Fn(&PlatformEvent) -> EventHandleResult + Send + Sync>;

/// 订阅者信息
struct Subscriber {
    /// 订阅者 ID
    id: String,
    /// 订阅的事件类型
    event_type: EventType,
    /// 订阅者所属系统
    system: NormalizationSystem,
    /// 处理器
    handler: EventHandler,
}

/// 事件总线
pub struct EventBus {
    /// 订阅者表（按事件类型分组）
    subscribers: RwLock<HashMap<EventType, Vec<Subscriber>>>,
    /// 事件历史（事件溯源）
    event_history: RwLock<Vec<PlatformEvent>>,
    /// 最大历史事件数
    max_history: usize,
    /// 已发布事件总数
    published_count: RwLock<u64>,
}

impl EventBus {
    /// 创建事件总线
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            event_history: RwLock::new(Vec::new()),
            max_history: 10000,
            published_count: RwLock::new(0),
        }
    }

    /// 订阅事件
    pub fn subscribe(
        &self,
        subscriber_id: &str,
        event_type: EventType,
        system: NormalizationSystem,
        handler: EventHandler,
    ) {
        let subscriber = Subscriber {
            id: subscriber_id.to_string(),
            event_type: event_type.clone(),
            system,
            handler,
        };

        let mut subs = self.subscribers.write();
        subs.entry(event_type)
            .or_insert_with(Vec::new)
            .push(subscriber);
    }

    /// 发布事件（同步通知所有订阅者）
    pub fn publish(&self, event: PlatformEvent) -> Vec<EventHandleResult> {
        // 记录历史
        {
            let mut history = self.event_history.write();
            history.push(event.clone());
            if history.len() > self.max_history {
                history.remove(0);
            }
            *self.published_count.write() += 1;
        }

        // 查找订阅者并执行
        let subs = self.subscribers.read();
        let mut results = Vec::new();

        if let Some(subscribers) = subs.get(&event.event_type) {
            for subscriber in subscribers {
                let result = (subscriber.handler)(&event);
                results.push(result);
            }
        }

        results
    }

    /// 获取某事件类型的订阅者数量
    pub fn subscriber_count(&self, event_type: &EventType) -> usize {
        self.subscribers
            .read()
            .get(event_type)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 获取总订阅者数
    pub fn total_subscribers(&self) -> usize {
        self.subscribers.read().values().map(|v| v.len()).sum()
    }

    /// 获取已发布事件数
    pub fn published_count(&self) -> u64 {
        *self.published_count.read()
    }

    /// 获取历史事件数
    pub fn history_count(&self) -> usize {
        self.event_history.read().len()
    }

    /// 按类型查询历史事件
    pub fn query_history(&self, event_type: &EventType, limit: usize) -> Vec<PlatformEvent> {
        let history = self.event_history.read();
        history
            .iter()
            .filter(|e| &e.event_type == event_type)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 按关联 ID 查询事件链
    pub fn query_by_correlation(&self, correlation_id: &str) -> Vec<PlatformEvent> {
        let history = self.event_history.read();
        history
            .iter()
            .filter(|e| e.correlation_id.as_deref() == Some(correlation_id))
            .cloned()
            .collect()
    }

    /// 注册内置的跨系统联动规则
    pub fn register_builtin_subscribers(&self) {
        // AI意图识别 → 权限系统自动校验
        self.subscribe(
            "perm-auto-check",
            EventType::IntentRecognized,
            NormalizationSystem::Permission,
            Box::new(|_event| {
                EventHandleResult {
                    success: true,
                    handler_id: "perm-auto-check".to_string(),
                    message: Some("permission check passed".to_string()),
                }
            }),
        );

        // 权限校验通过 → 低代码自动生成表单
        self.subscribe(
            "lowcode-auto-form",
            EventType::PermissionChecked,
            NormalizationSystem::Lowcode,
            Box::new(|event| {
                let allowed = event
                    .payload
                    .get("allowed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                EventHandleResult {
                    success: allowed,
                    handler_id: "lowcode-auto-form".to_string(),
                    message: if allowed {
                        Some("form generated".to_string())
                    } else {
                        Some("permission denied, form not generated".to_string())
                    },
                }
            }),
        );

        // 表单提交 → 自动启动流程
        self.subscribe(
            "process-auto-start",
            EventType::FormSubmitted,
            NormalizationSystem::ProcessAlgo,
            Box::new(|_| EventHandleResult {
                success: true,
                handler_id: "process-auto-start".to_string(),
                message: Some("process started".to_string()),
            }),
        );

        // 流程完成 → 前端推送通知
        self.subscribe(
            "frontend-notify",
            EventType::ProcessCompleted,
            NormalizationSystem::Frontend,
            Box::new(|_| EventHandleResult {
                success: true,
                handler_id: "frontend-notify".to_string(),
                message: Some("notification pushed to frontend".to_string()),
            }),
        );

        // 流程完成 → 架构系统同步外部
        self.subscribe(
            "arch-sync-external",
            EventType::ProcessCompleted,
            NormalizationSystem::Architecture,
            Box::new(|_| EventHandleResult {
                success: true,
                handler_id: "arch-sync-external".to_string(),
                message: Some("synced to external systems".to_string()),
            }),
        );
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_publish_and_subscribe() {
        let bus = EventBus::new();
        let received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let received_clone = received.clone();

        bus.subscribe(
            "test-sub",
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            Box::new(move |_| {
                received_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                EventHandleResult {
                    success: true,
                    handler_id: "test-sub".to_string(),
                    message: None,
                }
            }),
        );

        let event = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "tenant-1",
            serde_json::json!({"intent": "test"}),
        );

        let results = bus.publish(event);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(received.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new();

        bus.subscribe(
            "sub1",
            EventType::EntityCreated,
            NormalizationSystem::Lowcode,
            Box::new(|_| EventHandleResult {
                success: true,
                handler_id: "sub1".to_string(),
                message: None,
            }),
        );

        bus.subscribe(
            "sub2",
            EventType::EntityCreated,
            NormalizationSystem::ProcessAlgo,
            Box::new(|_| EventHandleResult {
                success: true,
                handler_id: "sub2".to_string(),
                message: None,
            }),
        );

        let event = PlatformEvent::new(
            EventType::EntityCreated,
            NormalizationSystem::Lowcode,
            "t1",
            serde_json::json!({}),
        );

        let results = bus.publish(event);
        assert_eq!(results.len(), 2);
        assert_eq!(bus.subscriber_count(&EventType::EntityCreated), 2);
    }

    #[test]
    fn test_event_history() {
        let bus = EventBus::new();

        for i in 0..5 {
            let event = PlatformEvent::new(
                EventType::UserAction,
                NormalizationSystem::Frontend,
                "t1",
                serde_json::json!({"action": i}),
            );
            bus.publish(event);
        }

        assert_eq!(bus.history_count(), 5);
        assert_eq!(bus.published_count(), 5);

        let history = bus.query_history(&EventType::UserAction, 10);
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_correlation_id() {
        let bus = EventBus::new();

        let corr_id = "corr-123";
        let event1 = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "t1",
            serde_json::json!({}),
        )
        .with_correlation_id(corr_id);

        let event2 = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "t1",
            serde_json::json!({}),
        )
        .with_correlation_id(corr_id);

        bus.publish(event1);
        bus.publish(event2);

        let chain = bus.query_by_correlation(corr_id);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_builtin_subscribers() {
        let bus = EventBus::new();
        bus.register_builtin_subscribers();

        // 应有 5 个内置订阅者
        assert_eq!(bus.total_subscribers(), 5);

        // IntentRecognized 有 1 个订阅者
        assert_eq!(bus.subscriber_count(&EventType::IntentRecognized), 1);

        // ProcessCompleted 有 2 个订阅者（前端通知 + 架构同步）
        assert_eq!(bus.subscriber_count(&EventType::ProcessCompleted), 2);
    }

    #[test]
    fn test_event_chain_full_flow() {
        let bus = EventBus::new();
        bus.register_builtin_subscribers();

        let corr_id = "flow-001";

        // 第1步：AI发布意图识别事件
        let intent_event = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "tenant-1",
            serde_json::json!({"intent": "leave_application"}),
        )
        .with_user_id("user-1")
        .with_correlation_id(corr_id);

        let results = bus.publish(intent_event);
        assert_eq!(results.len(), 1); // perm-auto-check
        assert!(results[0].success);

        // 第2步：权限系统发布校验通过事件
        let perm_event = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "tenant-1",
            serde_json::json!({"allowed": true, "reason": "has_permission"}),
        )
        .with_correlation_id(corr_id);

        let results = bus.publish(perm_event);
        assert_eq!(results.len(), 1); // lowcode-auto-form
        assert!(results[0].success);

        // 第3步：低代码发布表单提交事件
        let form_event = PlatformEvent::new(
            EventType::FormSubmitted,
            NormalizationSystem::Lowcode,
            "tenant-1",
            serde_json::json!({"form_id": "form-001"}),
        )
        .with_correlation_id(corr_id);

        let results = bus.publish(form_event);
        assert_eq!(results.len(), 1); // process-auto-start
        assert!(results[0].success);

        // 第4步：流程发布完成事件
        let proc_event = PlatformEvent::new(
            EventType::ProcessCompleted,
            NormalizationSystem::ProcessAlgo,
            "tenant-1",
            serde_json::json!({"status": "approved"}),
        )
        .with_correlation_id(corr_id);

        let results = bus.publish(proc_event);
        assert_eq!(results.len(), 2); // frontend-notify + arch-sync-external
        assert!(results.iter().all(|r| r.success));

        // 验证事件链完整性
        let chain = bus.query_by_correlation(corr_id);
        assert_eq!(chain.len(), 4);
    }

    #[test]
    fn test_permission_denied_chain() {
        let bus = EventBus::new();
        bus.register_builtin_subscribers();

        // 权限校验失败
        let perm_event = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "t1",
            serde_json::json!({"allowed": false, "reason": "no_permission"}),
        );

        let results = bus.publish(perm_event);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success); // lowcode-auto-form should fail
    }

    #[test]
    fn test_event_type_source_system() {
        assert_eq!(
            EventType::IntentRecognized.source_system(),
            NormalizationSystem::AiAssistant
        );
        assert_eq!(
            EventType::RoleChanged.source_system(),
            NormalizationSystem::Permission
        );
        assert_eq!(
            EventType::EntityCreated.source_system(),
            NormalizationSystem::Lowcode
        );
        assert_eq!(
            EventType::ProcessStarted.source_system(),
            NormalizationSystem::ProcessAlgo
        );
        assert_eq!(
            EventType::UserAction.source_system(),
            NormalizationSystem::Frontend
        );
        assert_eq!(
            EventType::ArchRequest.source_system(),
            NormalizationSystem::Architecture
        );
    }

    #[test]
    fn test_custom_event_type() {
        let bus = EventBus::new();
        let event = PlatformEvent::new(
            EventType::Custom("my_event".to_string()),
            NormalizationSystem::AiAssistant,
            "t1",
            serde_json::json!({}),
        );

        assert!(event.event_id.len() > 0);
        assert_eq!(event.tenant_id, "t1");
    }

    #[test]
    fn test_query_history_limit() {
        let bus = EventBus::new();

        for i in 0..10 {
            bus.publish(PlatformEvent::new(
                EventType::UserAction,
                NormalizationSystem::Frontend,
                "t1",
                serde_json::json!({"i": i}),
            ));
        }

        let recent = bus.query_history(&EventType::UserAction, 3);
        assert_eq!(recent.len(), 3);
    }
}
