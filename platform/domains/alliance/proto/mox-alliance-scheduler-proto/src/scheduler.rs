// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 任务调度器 trait 抽象

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceResult, Task};
use uuid::Uuid;

use crate::types::PlanGenerationRequest;

/// 任务提交请求
#[derive(Debug, Clone)]
pub struct TaskSubmitRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub task_type: Option<String>,
    pub priority: Option<mox_alliance_common_proto::TaskPriority>,
    pub mode: Option<mox_alliance_common_proto::AllianceMode>,
    pub fusion_strategy: Option<mox_alliance_common_proto::FusionStrategy>,
}

/// 任务提交响应
#[derive(Debug, Clone)]
pub struct TaskSubmitResponse {
    pub task: Task,
    pub estimated_duration_ms: Option<u64>,
}

/// 任务调度器 trait
///
/// 负责接收任务、排队、调度、生成协作计划。
/// 这是调度器的核心抽象，所有实现都遵循这个接口。
#[async_trait]
pub trait TaskScheduler: Send + Sync {
    /// 提交任务
    async fn submit_task(&self, request: TaskSubmitRequest) -> AllianceResult<TaskSubmitResponse>;

    /// 取消任务
    async fn cancel_task(&self, task_id: Uuid, tenant_id: Uuid, reason: Option<String>) -> AllianceResult<()>;

    /// 暂停任务
    async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 恢复任务
    async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 获取任务状态
    async fn get_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Task>;

    /// 生成协作计划
    async fn generate_plan(&self, request: PlanGenerationRequest) -> AllianceResult<mox_alliance_common_proto::CollaborationPlan>;

    /// 获取队列长度
    async fn queue_length(&self) -> usize;

    /// 列出指定租户的任务（按创建时间倒序）
    async fn list_tasks(&self, tenant_id: Uuid) -> AllianceResult<Vec<Task>>;

    /// 获取当前运行中任务数
    async fn running_count(&self) -> usize;
}
