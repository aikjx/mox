// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RBAC 事件系统
//!
//! 定义 RBAC 引擎运行时产生的各类事件，支持事件监听和审计集成。
//!
//! 事件分类：
//! - **策略变更事件**：角色/策略的增删改
//! - **权限决策事件**：每次权限检查的结果（允许/拒绝）
//! - **缓存事件**：缓存命中/失效
//! - **错误事件**：引擎运行时错误

use std::time::SystemTime;

use crate::types::{Action, EvaluationResult};

/// 事件 ID（唯一标识）
pub type EventId = String;

/// 生成事件 ID
pub(crate) fn new_event_id() -> EventId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("evt-rbac-{n}")
}

/// RBAC 事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RbacEvent {
    // ── 策略变更事件 ────────────────────────────────────────────────────────
    /// 角色已创建
    RoleCreated {
        /// 角色名
        role: String,
        /// 操作者
        operator: Option<String>,
    },
    /// 角色已更新
    RoleUpdated {
        /// 角色名
        role: String,
        /// 变更字段
        changes: Vec<String>,
        /// 操作者
        operator: Option<String>,
    },
    /// 角色已删除
    RoleDeleted {
        /// 角色名
        role: String,
        /// 操作者
        operator: Option<String>,
    },
    /// 角色继承关系变更
    RoleInheritanceChanged {
        /// 角色名
        role: String,
        /// 新增的父角色
        added_parents: Vec<String>,
        /// 移除的父角色
        removed_parents: Vec<String>,
        /// 操作者
        operator: Option<String>,
    },
    /// 策略已创建
    PolicyCreated {
        /// 策略 ID
        policy_id: String,
        /// 策略名称
        policy_name: String,
        /// 操作者
        operator: Option<String>,
    },
    /// 策略已更新
    PolicyUpdated {
        /// 策略 ID
        policy_id: String,
        /// 变更字段
        changes: Vec<String>,
        /// 操作者
        operator: Option<String>,
    },
    /// 策略已删除
    PolicyDeleted {
        /// 策略 ID
        policy_id: String,
        /// 操作者
        operator: Option<String>,
    },
    /// 策略已加载（从存储加载到内存）
    PolicyLoaded {
        /// 加载的策略数量
        policy_count: usize,
        /// 加载的角色数量
        role_count: usize,
    },
    /// 策略已重载
    PolicyReloaded {
        /// 新策略数量
        policy_count: usize,
        /// 新角色数量
        role_count: usize,
    },

    // ── 权限决策事件 ────────────────────────────────────────────────────────
    /// 权限检查 — 允许
    AccessGranted {
        /// 主体 ID
        subject: String,
        /// 资源路径
        resource: String,
        /// 动作
        action: Action,
        /// 匹配到的策略 ID 列表
        matched_policies: Vec<String>,
        /// 耗时（微秒）
        duration_us: u64,
    },
    /// 权限检查 — 拒绝
    AccessDenied {
        /// 主体 ID
        subject: String,
        /// 资源路径
        resource: String,
        /// 动作
        action: Action,
        /// 拒绝原因
        reason: String,
        /// 拒绝策略 ID（如果有）
        denied_by_policy: Option<String>,
        /// 耗时（微秒）
        duration_us: u64,
    },

    // ── 缓存事件 ────────────────────────────────────────────────────────────
    /// 缓存命中
    CacheHit {
        /// 缓存键
        key: String,
    },
    /// 缓存未命中
    CacheMiss {
        /// 缓存键
        key: String,
    },
    /// 缓存已失效（策略变更导致）
    CacheInvalidated {
        /// 失效原因
        reason: String,
        /// 失效的条目数
        invalidated_count: usize,
    },

    // ── 错误事件 ────────────────────────────────────────────────────────────
    /// 引擎错误
    EngineError {
        /// 错误码
        error_code: String,
        /// 错误信息
        error_message: String,
    },
}

impl RbacEvent {
    /// 事件类型名称
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::RoleCreated { .. } => "role.created",
            Self::RoleUpdated { .. } => "role.updated",
            Self::RoleDeleted { .. } => "role.deleted",
            Self::RoleInheritanceChanged { .. } => "role.inheritance_changed",
            Self::PolicyCreated { .. } => "policy.created",
            Self::PolicyUpdated { .. } => "policy.updated",
            Self::PolicyDeleted { .. } => "policy.deleted",
            Self::PolicyLoaded { .. } => "policy.loaded",
            Self::PolicyReloaded { .. } => "policy.reloaded",
            Self::AccessGranted { .. } => "access.granted",
            Self::AccessDenied { .. } => "access.denied",
            Self::CacheHit { .. } => "cache.hit",
            Self::CacheMiss { .. } => "cache.miss",
            Self::CacheInvalidated { .. } => "cache.invalidated",
            Self::EngineError { .. } => "engine.error",
        }
    }

    /// 事件分类
    pub fn category(&self) -> EventCategory {
        match self {
            Self::RoleCreated { .. }
            | Self::RoleUpdated { .. }
            | Self::RoleDeleted { .. }
            | Self::RoleInheritanceChanged { .. }
            | Self::PolicyCreated { .. }
            | Self::PolicyUpdated { .. }
            | Self::PolicyDeleted { .. }
            | Self::PolicyLoaded { .. }
            | Self::PolicyReloaded { .. } => EventCategory::PolicyChange,

            Self::AccessGranted { .. } | Self::AccessDenied { .. } => EventCategory::AccessDecision,

            Self::CacheHit { .. } | Self::CacheMiss { .. } | Self::CacheInvalidated { .. } => {
                EventCategory::Cache
            }

            Self::EngineError { .. } => EventCategory::Error,
        }
    }

    /// 是否为审计敏感事件（需要写入审计日志）
    pub fn is_audit_sensitive(&self) -> bool {
        match self {
            // 策略变更都需要审计
            Self::RoleCreated { .. }
            | Self::RoleUpdated { .. }
            | Self::RoleDeleted { .. }
            | Self::RoleInheritanceChanged { .. }
            | Self::PolicyCreated { .. }
            | Self::PolicyUpdated { .. }
            | Self::PolicyDeleted { .. } => true,

            // 权限拒绝需要审计
            Self::AccessDenied { .. } => true,

            // 其他事件默认不需要审计
            _ => false,
        }
    }

    /// 从评估结果创建访问决策事件
    pub fn from_evaluation(
        subject_id: &str,
        resource_path: &str,
        action: &Action,
        result: &EvaluationResult,
        duration_us: u64,
    ) -> Self {
        match result {
            EvaluationResult::Granted { matched_policies } => Self::AccessGranted {
                subject: subject_id.to_string(),
                resource: resource_path.to_string(),
                action: action.clone(),
                matched_policies: matched_policies.clone(),
                duration_us,
            },
            EvaluationResult::Denied {
                reason,
                denied_by_policy,
            } => Self::AccessDenied {
                subject: subject_id.to_string(),
                resource: resource_path.to_string(),
                action: action.clone(),
                reason: reason.clone(),
                denied_by_policy: denied_by_policy.clone(),
                duration_us,
            },
        }
    }
}

/// 事件分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventCategory {
    /// 策略变更
    PolicyChange,
    /// 访问决策
    AccessDecision,
    /// 缓存
    Cache,
    /// 错误
    Error,
}

impl EventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PolicyChange => "policy_change",
            Self::AccessDecision => "access_decision",
            Self::Cache => "cache",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 事件信封（包含事件元数据）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventEnvelope {
    /// 事件 ID
    pub id: EventId,
    /// 事件类型
    pub event_type: String,
    /// 事件分类
    pub category: EventCategory,
    /// 事件发生时间（Unix 时间戳，毫秒）
    pub timestamp: u64,
    /// 事件数据
    pub payload: RbacEvent,
}

impl EventEnvelope {
    /// 包装事件为信封
    pub fn wrap(event: RbacEvent) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let event_type = event.event_type().to_string();
        let category = event.category();

        Self {
            id: new_event_id(),
            event_type,
            category,
            timestamp,
            payload: event,
        }
    }
}

/// 事件监听器 trait
///
/// 实现此 trait 可以订阅 RBAC 引擎的各种事件。
pub trait EventListener: Send + Sync {
    /// 处理事件
    fn on_event(&self, event: &EventEnvelope);

    /// 是否关心某类事件（用于优化派发）
    fn is_interested(&self, _category: EventCategory) -> bool {
        true
    }
}

/// 简单的函数式事件监听器
pub struct FnEventListener<F>
where
    F: Fn(&EventEnvelope) + Send + Sync,
{
    handler: F,
    filter: Option<EventCategory>,
}

impl<F> FnEventListener<F>
where
    F: Fn(&EventEnvelope) + Send + Sync,
{
    /// 创建函数式监听器
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            filter: None,
        }
    }

    /// 按分类过滤
    pub fn with_filter(mut self, category: EventCategory) -> Self {
        self.filter = Some(category);
        self
    }
}

impl<F> EventListener for FnEventListener<F>
where
    F: Fn(&EventEnvelope) + Send + Sync,
{
    fn on_event(&self, event: &EventEnvelope) {
        (self.handler)(event);
    }

    fn is_interested(&self, category: EventCategory) -> bool {
        match self.filter {
            Some(f) => f == category,
            None => true,
        }
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Action;

    #[test]
    fn event_type_names() {
        assert_eq!(
            RbacEvent::RoleCreated {
                role: "admin".into(),
                operator: None,
            }
            .event_type(),
            "role.created"
        );
        assert_eq!(
            RbacEvent::RoleDeleted {
                role: "viewer".into(),
                operator: None,
            }
            .event_type(),
            "role.deleted"
        );
        assert_eq!(
            RbacEvent::AccessGranted {
                subject: "user:alice".into(),
                resource: "db:test".into(),
                action: Action::Read,
                matched_policies: vec![],
                duration_us: 100,
            }
            .event_type(),
            "access.granted"
        );
        assert_eq!(
            RbacEvent::AccessDenied {
                subject: "user:bob".into(),
                resource: "db:prod".into(),
                action: Action::Write,
                reason: "no permission".into(),
                denied_by_policy: None,
                duration_us: 50,
            }
            .event_type(),
            "access.denied"
        );
        assert_eq!(
            RbacEvent::CacheHit {
                key: "test-key".into()
            }
            .event_type(),
            "cache.hit"
        );
    }

    #[test]
    fn event_categories() {
        assert_eq!(
            RbacEvent::RoleCreated {
                role: "x".into(),
                operator: None,
            }
            .category(),
            EventCategory::PolicyChange
        );
        assert_eq!(
            RbacEvent::PolicyCreated {
                policy_id: "p1".into(),
                policy_name: "n1".into(),
                operator: None,
            }
            .category(),
            EventCategory::PolicyChange
        );
        assert_eq!(
            RbacEvent::AccessGranted {
                subject: "s".into(),
                resource: "r".into(),
                action: Action::Read,
                matched_policies: vec![],
                duration_us: 0,
            }
            .category(),
            EventCategory::AccessDecision
        );
        assert_eq!(
            RbacEvent::CacheHit {
                key: "k".into()
            }
            .category(),
            EventCategory::Cache
        );
        assert_eq!(
            RbacEvent::EngineError {
                error_code: "E001".into(),
                error_message: "err".into(),
            }
            .category(),
            EventCategory::Error
        );
    }

    #[test]
    fn audit_sensitive_events() {
        // 策略变更应审计
        assert!(RbacEvent::RoleCreated {
            role: "admin".into(),
            operator: None,
        }
        .is_audit_sensitive());
        assert!(RbacEvent::RoleDeleted {
            role: "viewer".into(),
            operator: None,
        }
        .is_audit_sensitive());
        assert!(RbacEvent::PolicyCreated {
            policy_id: "p1".into(),
            policy_name: "n1".into(),
            operator: None,
        }
        .is_audit_sensitive());

        // 权限拒绝应审计
        assert!(RbacEvent::AccessDenied {
            subject: "s".into(),
            resource: "r".into(),
            action: Action::Write,
            reason: "no".into(),
            denied_by_policy: None,
            duration_us: 0,
        }
        .is_audit_sensitive());

        // 权限允许不应审计
        assert!(!RbacEvent::AccessGranted {
            subject: "s".into(),
            resource: "r".into(),
            action: Action::Read,
            matched_policies: vec![],
            duration_us: 0,
        }
        .is_audit_sensitive());

        // 缓存事件不应审计
        assert!(!RbacEvent::CacheHit {
            key: "k".into()
        }
        .is_audit_sensitive());
    }

    #[test]
    fn event_envelope_wrap() {
        let event = RbacEvent::RoleCreated {
            role: "test_role".into(),
            operator: Some("admin".into()),
        };
        let envelope = EventEnvelope::wrap(event);

        assert!(!envelope.id.is_empty());
        assert!(envelope.id.starts_with("evt-rbac-"));
        assert_eq!(envelope.event_type, "role.created");
        assert_eq!(envelope.category, EventCategory::PolicyChange);
        assert!(envelope.timestamp > 0);
    }

    #[test]
    fn from_evaluation_granted() {
        let result = EvaluationResult::Granted {
            matched_policies: vec!["p1".into(), "p2".into()],
        };
        let event =
            RbacEvent::from_evaluation("user:alice", "db:test/data", &Action::Read, &result, 150);

        match event {
            RbacEvent::AccessGranted {
                subject,
                resource,
                action,
                matched_policies,
                duration_us,
            } => {
                assert_eq!(subject, "user:alice");
                assert_eq!(resource, "db:test/data");
                assert_eq!(action, Action::Read);
                assert_eq!(matched_policies, vec!["p1", "p2"]);
                assert_eq!(duration_us, 150);
            }
            _ => panic!("expected AccessGranted"),
        }
    }

    #[test]
    fn from_evaluation_denied() {
        let result = EvaluationResult::Denied {
            reason: "insufficient permissions".into(),
            denied_by_policy: Some("deny-policy".into()),
        };
        let event = RbacEvent::from_evaluation(
            "user:bob",
            "db:prod/secret",
            &Action::Write,
            &result,
            200,
        );

        match event {
            RbacEvent::AccessDenied {
                subject,
                resource,
                action,
                reason,
                denied_by_policy,
                duration_us,
            } => {
                assert_eq!(subject, "user:bob");
                assert_eq!(resource, "db:prod/secret");
                assert_eq!(action, Action::Write);
                assert_eq!(reason, "insufficient permissions");
                assert_eq!(denied_by_policy, Some("deny-policy".into()));
                assert_eq!(duration_us, 200);
            }
            _ => panic!("expected AccessDenied"),
        }
    }

    #[test]
    fn fn_event_listener_works() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let listener = FnEventListener::new(move |_evt| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

        let event = EventEnvelope::wrap(RbacEvent::CacheHit {
            key: "k".into(),
        });

        assert!(listener.is_interested(EventCategory::Cache));
        assert!(listener.is_interested(EventCategory::PolicyChange));
        listener.on_event(&event);
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn fn_event_listener_with_filter() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let listener =
            FnEventListener::new(move |_evt| {
                count_clone.fetch_add(1, Ordering::Relaxed);
            })
            .with_filter(EventCategory::AccessDecision);

        assert!(listener.is_interested(EventCategory::AccessDecision));
        assert!(!listener.is_interested(EventCategory::Cache));
        assert!(!listener.is_interested(EventCategory::PolicyChange));
    }

    #[test]
    fn event_category_display() {
        assert_eq!(format!("{}", EventCategory::PolicyChange), "policy_change");
        assert_eq!(format!("{}", EventCategory::AccessDecision), "access_decision");
        assert_eq!(format!("{}", EventCategory::Cache), "cache");
        assert_eq!(format!("{}", EventCategory::Error), "error");
    }
}
