// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 领域事件与事件总线（数据流核心）
//!
//! 所有写操作在执行后发出领域事件；编排层的反应器（reactor）订阅总线，
//! 将事件转换为系统消息与成员通知，实现「任务变更 → 通信」的自动数据流。
use crate::model::TaskStatus;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// 领域事件
#[derive(Clone, Debug, Serialize)]
pub enum DomainEvent {
    MemberInvited {
        member_id: String,
        mox_id: String,
        by: String,
    },
    MemberActivated {
        member_id: String,
    },
    MemberStatusChanged {
        member_id: String,
        status: String,
    },
    TaskCreated {
        task_id: String,
        mox_id: String,
        by: String,
    },
    TaskAssigned {
        task_id: String,
        assignees: Vec<String>,
        by: String,
    },
    TaskStatusChanged {
        task_id: String,
        from: TaskStatus,
        to: TaskStatus,
        by: String,
    },
    TaskCommented {
        task_id: String,
        by: String,
        channel_id: String,
    },
    MessagePosted {
        channel_id: String,
        message_id: String,
        sender_id: String,
    },
    /// 鉴权被拒（BR-18）：越权尝试的可观测信号，同时落审计流
    AuthzDenied {
        member_id: String,
        permission: String,
        scope: String,
        reason: String,
    },
}

impl DomainEvent {
    /// 该事件关心的成员（用于实时推送与通知路由）
    pub fn interested_members(&self) -> Vec<String> {
        match self {
            DomainEvent::MemberInvited { member_id, .. } => vec![member_id.clone()],
            DomainEvent::MemberActivated { member_id } => vec![member_id.clone()],
            DomainEvent::MemberStatusChanged { member_id, .. } => vec![member_id.clone()],
            DomainEvent::TaskCreated { by, .. } => vec![by.clone()],
            DomainEvent::TaskAssigned { assignees, .. } => assignees.clone(),
            DomainEvent::TaskStatusChanged { by, .. } => vec![by.clone()],
            DomainEvent::TaskCommented { by, .. } => vec![by.clone()],
            DomainEvent::MessagePosted { sender_id, .. } => vec![sender_id.clone()],
            DomainEvent::AuthzDenied { member_id, .. } => vec![member_id.clone()],
        }
    }
}

/// 事件总线（多播）
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<DomainEvent>,
    /// 与 Metrics 共享的事件发布计数器
    events_published: Arc<AtomicU64>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self::with_metrics(Arc::new(AtomicU64::new(0)))
    }

    /// 使用共享计数器构造（与 Metrics.events_published 同源，保证计数一致）
    pub fn with_metrics(events_published: Arc<AtomicU64>) -> Self {
        let (tx, _rx) = broadcast::channel(1024);
        Self {
            tx,
            events_published,
        }
    }

    pub fn publish(&self, event: DomainEvent) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
        // 忽略无订阅者的情况
        let _ = self.tx.send(event);
    }

    pub fn events_published(&self) -> u64 {
        self.events_published.load(Ordering::Relaxed)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.tx.subscribe()
    }
}
