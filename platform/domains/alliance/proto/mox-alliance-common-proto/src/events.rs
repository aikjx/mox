// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 联盟领域事件协议
//!
//! 定义专家联盟系统中所有领域事件，用于事件驱动架构和 CDC。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{NodeStatus, TaskStatus};

/// 联盟事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AllianceEvent {
    Task(TaskEvent),
    Node(NodeEvent),
    Expert(ExpertEvent),
}

/// 任务事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event_id: Uuid,
    pub task_id: Uuid,
    pub tenant_id: Uuid,
    pub action: TaskAction,
    pub status: TaskStatus,
    pub progress: f32,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
}

/// 任务动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    Created,
    PlanningStarted,
    PlanGenerated,
    ExecutionStarted,
    ProgressUpdated,
    Paused,
    Resumed,
    Completed,
    Failed,
    Cancelled,
}

impl TaskEvent {
    pub fn new(task_id: Uuid, tenant_id: Uuid, action: TaskAction, status: TaskStatus) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            task_id,
            tenant_id,
            action,
            status,
            progress: 0.0,
            timestamp: Utc::now(),
            payload: None,
        }
    }
}

/// 节点事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEvent {
    pub event_id: Uuid,
    pub task_id: Uuid,
    pub node_id: String,
    pub expert_id: String,
    pub action: NodeAction,
    pub status: NodeStatus,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
}

/// 节点动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeAction {
    Scheduled,
    Started,
    Retrying,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl NodeEvent {
    pub fn new(
        task_id: Uuid,
        node_id: String,
        expert_id: String,
        action: NodeAction,
        status: NodeStatus,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            task_id,
            node_id,
            expert_id,
            action,
            status,
            timestamp: Utc::now(),
            payload: None,
        }
    }
}

/// 专家事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertEvent {
    pub event_id: Uuid,
    pub expert_id: String,
    pub action: ExpertAction,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
}

/// 专家动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpertAction {
    Registered,
    Updated,
    Activated,
    Deactivated,
    Deregistered,
    Heartbeat,
    HealthChanged,
}

/// 事件主题命名空间（NATS subject 前缀）
pub mod event_subjects {
    /// 任务事件前缀: alliance.task.{tenant_id}.{task_id}.{action}
    pub const TASK_PREFIX: &str = "alliance.task";
    /// 节点事件前缀: alliance.node.{tenant_id}.{task_id}.{node_id}
    pub const NODE_PREFIX: &str = "alliance.node";
    /// 专家事件前缀: alliance.expert.{expert_id}.{action}
    pub const EXPERT_PREFIX: &str = "alliance.expert";

    pub fn task_subject(tenant_id: &str, task_id: &str) -> String {
        format!("{}.{}.{}", TASK_PREFIX, tenant_id, task_id)
    }

    pub fn expert_subject(expert_id: &str) -> String {
        format!("{}.{}", EXPERT_PREFIX, expert_id)
    }
}
