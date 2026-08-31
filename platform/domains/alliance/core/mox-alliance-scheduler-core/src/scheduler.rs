// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 任务调度器实现
//!
//! 实现 `TaskScheduler` trait，提供任务提交、排队、调度、计划生成等功能。
//!
//! Phase 1 实现：
//! - 内存队列（优先队列，按优先级 + FIFO）
//! - 简单任务状态管理
//! - 调用匹配器和计划生成器

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use mox_alliance_common_proto::{
    AllianceError, AllianceResult, Task, TaskStatus,
};
use mox_alliance_scheduler_proto::{
    ExpertMatcher, PlanGenerationRequest, TaskScheduler, TaskSubmitRequest, TaskSubmitResponse,
};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::planner::SimplePlanGenerator;
use crate::matcher::RuleBasedExpertMatcher;
use mox_alliance_scheduler_proto::types::SchedulerConfig;

/// 任务调度器实现
pub struct TaskSchedulerImpl {
    config: SchedulerConfig,
    tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
    matcher: Arc<RuleBasedExpertMatcher>,
    planner: SimplePlanGenerator,
    /// 任务调度发送端（通知执行器有新任务）
    dispatch_tx: mpsc::UnboundedSender<Task>,
}

impl TaskSchedulerImpl {
    pub fn new(
        config: SchedulerConfig,
        matcher: Arc<RuleBasedExpertMatcher>,
        dispatch_tx: mpsc::UnboundedSender<Task>,
    ) -> Self {
        Self {
            config,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            matcher,
            planner: SimplePlanGenerator::new(),
            dispatch_tx,
        }
    }

    /// 获取任务引用（内部方法）
    fn get_task_internal(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Task> {
        let tasks = self.tasks.read();
        let task = tasks
            .get(&task_id)
            .cloned()
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;

        if task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }

        Ok(task)
    }

    /// 更新任务状态（内部方法）
    fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> AllianceResult<()> {
        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = status;
            match status {
                TaskStatus::Running => {
                    task.started_at = Some(chrono::Utc::now());
                }
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    task.completed_at = Some(chrono::Utc::now());
                    if let Some(started) = task.started_at {
                        let duration = chrono::Utc::now() - started;
                        task.duration_ms = Some(duration.num_milliseconds());
                    }
                }
                _ => {}
            }
            Ok(())
        } else {
            Err(AllianceError::not_found("Task", &task_id.to_string()))
        }
    }
}

// 需要从 error 模块导入错误码
use mox_alliance_common_proto::AllianceErrorCode;

#[async_trait]
impl TaskScheduler for TaskSchedulerImpl {
    async fn submit_task(&self, request: TaskSubmitRequest) -> AllianceResult<TaskSubmitResponse> {
        // 检查队列容量
        {
            let tasks = self.tasks.read();
            let pending_count = tasks
                .values()
                .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Planning)
                .count();
            if pending_count >= self.config.queue_capacity {
                return Err(AllianceError::new(
                    AllianceErrorCode::SchedulerFull,
                    format!(
                        "Task queue is full (capacity: {})",
                        self.config.queue_capacity
                    ),
                ));
            }
        }

        // 创建任务
        let mut task = Task::new(
            request.tenant_id,
            request.user_id,
            request.title,
            request.description,
        );
        task.task_type = request.task_type.unwrap_or_else(|| "custom".to_string());
        task.priority = request.priority.unwrap_or(self.config.default_priority);
        task.mode = request.mode.unwrap_or(self.config.default_mode);
        task.fusion_strategy = request
            .fusion_strategy
            .unwrap_or(self.config.default_fusion_strategy);

        let task_id = task.task_id;
        let tenant_id = task.tenant_id;

        info!(
            "Task submitted: {} ({}) by tenant {}",
            task.title, task_id, tenant_id
        );

        // 存入任务表
        {
            let mut tasks = self.tasks.write();
            tasks.insert(task_id, task.clone());
        }

        // 更新状态为规划中
        self.update_task_status(task_id, TaskStatus::Planning)?;

        // 生成协作计划（异步触发，这里先同步简化版）
        let plan_request = PlanGenerationRequest {
            task_id,
            tenant_id,
            task_description: task.description.clone(),
            preferred_mode: Some(task.mode),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
        };

        // 匹配专家
        let match_query = mox_alliance_scheduler_proto::ExpertMatchQuery {
            tenant_id: tenant_id.to_string(),
            task_description: task.description.clone(),
            required_domains: vec![],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 5,
        };

        let match_result = self.matcher.match_experts(match_query).await?;
        debug!(
            "Matched {} experts for task {}",
            match_result.matches.len(),
            task_id
        );

        // 生成计划
        let plan = self.planner.generate(&plan_request, &match_result.matches)?;
        debug!(
            "Generated plan for task {}: {} nodes, mode={:?}",
            task_id, plan.nodes.len(), plan.mode
        );

        // 更新任务状态为待执行（Phase 1 简化：直接派发）
        self.update_task_status(task_id, TaskStatus::Running)?;

        // 派发给执行器
        if self.dispatch_tx.send(task.clone()).is_err() {
            warn!("Failed to dispatch task {}: receiver dropped", task_id);
        }

        Ok(TaskSubmitResponse {
            task,
            estimated_duration_ms: None,
        })
    }

    async fn cancel_task(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()> {
        let task = self.get_task_internal(task_id, tenant_id)?;

        if task.status.is_terminal() {
            return Err(AllianceError::new(
                AllianceErrorCode::TaskAlreadyTerminal,
                format!("Task is already in terminal state: {:?}", task.status),
            ));
        }

        self.update_task_status(task_id, TaskStatus::Cancelled)?;
        info!("Task {} cancelled, reason: {:?}", task_id, reason);
        Ok(())
    }

    async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        let task = self.get_task_internal(task_id, tenant_id)?;

        if task.status.is_terminal() {
            return Err(AllianceError::new(
                AllianceErrorCode::TaskAlreadyTerminal,
                format!("Task is already in terminal state: {:?}", task.status),
            ));
        }

        self.update_task_status(task_id, TaskStatus::Paused)?;
        info!("Task {} paused", task_id);
        Ok(())
    }

    async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        let task = self.get_task_internal(task_id, tenant_id)?;

        if task.status != TaskStatus::Paused {
            return Err(AllianceError::new(
                AllianceErrorCode::InvalidTaskStatus,
                format!("Can only resume paused task, current status: {:?}", task.status),
            ));
        }

        self.update_task_status(task_id, TaskStatus::Running)?;
        info!("Task {} resumed", task_id);
        Ok(())
    }

    async fn get_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Task> {
        self.get_task_internal(task_id, tenant_id)
    }

    async fn generate_plan(
        &self,
        request: PlanGenerationRequest,
    ) -> AllianceResult<mox_alliance_common_proto::CollaborationPlan> {
        let match_query = mox_alliance_scheduler_proto::ExpertMatchQuery {
            tenant_id: request.tenant_id.to_string(),
            task_description: request.task_description.clone(),
            required_domains: vec![],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 5,
        };

        let match_result = self.matcher.match_experts(match_query).await?;
        let plan = self.planner.generate(&request, &match_result.matches)?;

        Ok(plan)
    }

    async fn queue_length(&self) -> usize {
        let tasks = self.tasks.read();
        tasks
            .values()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Planning)
            .count()
    }

    async fn running_count(&self) -> usize {
        let tasks = self.tasks.read();
        tasks
            .values()
            .filter(|t| t.status == TaskStatus::Running)
            .count()
    }
}
