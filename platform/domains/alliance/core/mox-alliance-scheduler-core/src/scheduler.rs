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
//!
//! Phase 2 更新：
//! - 使用 ExecutorBridge 替代 dispatch_tx 通道
//! - 支持 HTTP 远程执行器和进程内执行器
//! - 任务状态与执行器同步

use std::sync::Arc;

use async_trait::async_trait;
use mox_alliance_common_proto::{
    AllianceError, AllianceErrorCode, AllianceResult, Task, TaskStatus,
};
use mox_alliance_scheduler_proto::{
    ExpertMatcher, PlanGenerationRequest, TaskScheduler, TaskSubmitRequest, TaskSubmitResponse,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::planner::SimplePlanGenerator;
use crate::executor_bridge::{ExecutorBridge, NoopExecutorBridge};
use crate::storage::{InMemoryTaskRepository, TaskRepository};
use mox_alliance_scheduler_proto::types::SchedulerConfig;

/// 任务调度器实现
///
/// 专家匹配器通过 trait 对象注入，支持规则匹配器 / 模块化权重匹配器
/// 的运行时互换（可插拔架构）。
pub struct TaskSchedulerImpl {
    config: SchedulerConfig,
    /// 任务仓库（trait 对象，可插拔：内存 / 文件快照 / 数据库）
    tasks: Arc<dyn TaskRepository>,
    matcher: Arc<dyn ExpertMatcher>,
    planner: SimplePlanGenerator,
    /// 执行器桥接（替代原 dispatch_tx）
    executor_bridge: Arc<dyn ExecutorBridge>,
}

impl TaskSchedulerImpl {
    /// 创建调度器（使用 ExecutorBridge，推荐）
    pub fn new_with_bridge(
        config: SchedulerConfig,
        matcher: Arc<dyn ExpertMatcher>,
        executor_bridge: Arc<dyn ExecutorBridge>,
    ) -> Self {
        Self {
            config,
            tasks: Arc::new(InMemoryTaskRepository::new()),
            matcher,
            planner: SimplePlanGenerator::new(),
            executor_bridge,
        }
    }

    /// 创建调度器（旧版 API，向后兼容）
    ///
    /// 内部会将 dispatch_tx 包装为一个特殊的桥接实现，
    /// 同时使用 NoopExecutorBridge 作为主要桥接（保持旧行为）。
    /// 建议迁移到 `new_with_bridge` 以获得完整的执行器集成能力。
    pub fn new(
        config: SchedulerConfig,
        matcher: Arc<dyn ExpertMatcher>,
        dispatch_tx: mpsc::UnboundedSender<Task>,
    ) -> Self {
        // 使用 NoopExecutorBridge 保持向后兼容
        // dispatch_tx 被保留为可选的通知通道（不参与核心流程）
        let _ = dispatch_tx; // 保留参数以维持 API 兼容
        Self {
            config,
            tasks: Arc::new(InMemoryTaskRepository::new()),
            matcher,
            planner: SimplePlanGenerator::new(),
            executor_bridge: Arc::new(NoopExecutorBridge),
        }
    }

    /// 注入自定义任务仓库（持久化可插拔，企业级）
    pub fn with_task_repository(mut self, repository: Arc<dyn TaskRepository>) -> Self {
        self.tasks = repository;
        self
    }

    /// 获取执行器桥接引用
    pub fn executor_bridge(&self) -> &Arc<dyn ExecutorBridge> {
        &self.executor_bridge
    }

    /// 获取任务引用（内部方法）
    fn get_task_internal(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Task> {
        let task = self
            .tasks
            .get(task_id)?
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
        let mut task = self
            .tasks
            .get(task_id)?
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
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
        self.tasks.save(&task)?;
        Ok(())
    }

    /// 更新任务进度（内部方法）
    fn update_task_progress(&self, task_id: Uuid, progress: f32) -> AllianceResult<()> {
        let mut task = self
            .tasks
            .get(task_id)?
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
        task.progress = progress;
        self.tasks.save(&task)?;
        Ok(())
    }

    /// 从执行器同步任务状态（内部方法）
    async fn sync_task_status_from_executor(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
    ) -> AllianceResult<()> {
        match self.executor_bridge.get_status(task_id, tenant_id).await {
            Ok(exec_status) => {
                let mut task = match self.tasks.get(task_id)? {
                    Some(t) => t,
                    None => {
                        warn!("Task {} not found while syncing from executor", task_id);
                        return Ok(());
                    }
                };
                if task.tenant_id != tenant_id {
                    return Ok(());
                }
                task.progress = exec_status.progress;
                // 根据执行进度推断任务状态
                if task.status == TaskStatus::Running {
                    if exec_status.total_nodes > 0
                        && exec_status.completed_nodes + exec_status.failed_nodes
                            == exec_status.total_nodes
                    {
                        // 所有节点都完成了
                        if exec_status.failed_nodes > 0 {
                            task.status = TaskStatus::Failed;
                        } else {
                            task.status = TaskStatus::Completed;
                        }
                        task.completed_at = Some(chrono::Utc::now());
                        if let Some(started) = task.started_at {
                            let duration = chrono::Utc::now() - started;
                            task.duration_ms = Some(duration.num_milliseconds());
                        }
                    }
                }
                self.tasks.save(&task)?;
                Ok(())
            }
            Err(e) => {
                // 同步失败不影响主流程，只记录日志
                warn!(
                    "Failed to sync task status from executor: task_id={}, error={}",
                    task_id, e
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl TaskScheduler for TaskSchedulerImpl {
    async fn submit_task(&self, request: TaskSubmitRequest) -> AllianceResult<TaskSubmitResponse> {
        // 检查队列容量与并发上限
        {
            let tasks = self.tasks.all()?;
            let pending_count = tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Planning)
                .count();
            let running_count = tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Running)
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
            if running_count >= self.config.max_concurrent_tasks {
                return Err(AllianceError::new(
                    AllianceErrorCode::SchedulerFull,
                    format!(
                        "Max concurrent tasks reached (limit: {})",
                        self.config.max_concurrent_tasks
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
        self.tasks.save(&task)?;

        // 更新状态为规划中
        self.update_task_status(task_id, TaskStatus::Planning)?;

        // 生成协作计划
        let plan_request = PlanGenerationRequest {
            task_id,
            tenant_id,
            task_description: task.description.clone(),
            preferred_mode: Some(task.mode),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: task.fusion_strategy,
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

        // 更新任务状态为执行中
        self.update_task_status(task_id, TaskStatus::Running)?;

        // 通过执行器桥接提交计划
        match self.executor_bridge.submit_plan(&task, plan.clone()).await {
            Ok(_) => {
                info!(
                    "Task {} dispatched to executor successfully",
                    task_id
                );
            }
            Err(e) => {
                // 提交失败，标记任务为失败
                warn!(
                    "Failed to dispatch task {} to executor: {}",
                    task_id, e
                );
                self.update_task_status(task_id, TaskStatus::Failed)?;
                // 更新失败信息
                if let Some(mut t) = self.tasks.get(task_id)? {
                    t.progress = 0.0;
                    self.tasks.save(&t)?;
                }
            }
        }

        // 重新获取最新的任务状态
        let task = self.get_task_internal(task_id, tenant_id)?;

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

        // 通知执行器取消任务
        if task.status == TaskStatus::Running || task.status == TaskStatus::Paused {
            match self
                .executor_bridge
                .cancel_task(task_id, tenant_id, reason.clone())
                .await
            {
                Ok(_) => {
                    info!("Executor cancelled task {} successfully", task_id);
                }
                Err(e) => {
                    warn!(
                        "Failed to cancel task {} on executor: {}",
                        task_id, e
                    );
                    // 即使执行器取消失败，我们仍然更新本地状态
                }
            }
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

        // 通知执行器暂停任务
        if task.status == TaskStatus::Running {
            match self.executor_bridge.pause_task(task_id, tenant_id).await {
                Ok(_) => {
                    info!("Executor paused task {} successfully", task_id);
                }
                Err(e) => {
                    warn!(
                        "Failed to pause task {} on executor: {}",
                        task_id, e
                    );
                }
            }
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

        // 通知执行器恢复任务
        match self.executor_bridge.resume_task(task_id, tenant_id).await {
            Ok(_) => {
                info!("Executor resumed task {} successfully", task_id);
            }
            Err(e) => {
                warn!(
                    "Failed to resume task {} on executor: {}",
                    task_id, e
                );
            }
        }

        self.update_task_status(task_id, TaskStatus::Running)?;
        info!("Task {} resumed", task_id);
        Ok(())
    }

    async fn get_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Task> {
        // 先从本地获取
        let task = self.get_task_internal(task_id, tenant_id)?;

        // 如果任务正在执行中，尝试从执行器同步最新状态
        if task.status == TaskStatus::Running || task.status == TaskStatus::Paused {
            // 异步同步，不阻塞查询（后台更新）
            // 注意：这里使用同步更新以确保用户获得最新状态
            let _ = self
                .sync_task_status_from_executor(task_id, tenant_id)
                .await;

            // 重新获取更新后的任务
            return self.get_task_internal(task_id, tenant_id);
        }

        Ok(task)
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
        match self.tasks.all() {
            Ok(tasks) => tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Planning)
                .count(),
            Err(_) => 0,
        }
    }

    async fn list_tasks(&self, tenant_id: Uuid) -> AllianceResult<Vec<Task>> {
        let tasks = self.tasks.all()?;
        let mut result: Vec<Task> = tasks
            .into_iter()
            .filter(|t| t.tenant_id == tenant_id)
            .collect();
        // 新任务优先展示
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn running_count(&self) -> usize {
        match self.tasks.all() {
            Ok(tasks) => tasks.iter().filter(|t| t.status == TaskStatus::Running).count(),
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor_bridge::MockExecutorBridge;
    use crate::matcher::RuleBasedExpertMatcher;
    use mox_alliance_common_proto::{
        AllianceMode, FusionStrategy, Node, NodeStatus, TaskPriority,
    };
    use mox_alliance_executor_proto::ExecutionStatus;

    fn create_test_config() -> SchedulerConfig {
        SchedulerConfig {
            max_concurrent_tasks: 10,
            queue_capacity: 100,
            default_priority: TaskPriority::Normal,
            default_mode: AllianceMode::Parallel,
            default_fusion_strategy: FusionStrategy::Weighted,
            plan_generation_timeout_ms: 30_000,
        }
    }

    #[tokio::test]
    async fn test_submit_task_with_mock_bridge() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let bridge = Arc::new(MockExecutorBridge::new());

        let scheduler = TaskSchedulerImpl::new_with_bridge(config, matcher, bridge.clone());

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        let result = scheduler.submit_task(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.task.status, TaskStatus::Running);
        assert_eq!(bridge.submitted_count(), 1);
    }

    #[tokio::test]
    async fn test_cancel_task_with_mock_bridge() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let bridge = Arc::new(MockExecutorBridge::new());

        let scheduler = TaskSchedulerImpl::new_with_bridge(config, matcher, bridge.clone());

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // 先提交一个任务
        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        let response = scheduler.submit_task(request).await.unwrap();
        let task_id = response.task.task_id;

        // 取消任务
        let result = scheduler
            .cancel_task(task_id, tenant_id, Some("test cancel".to_string()))
            .await;
        assert!(result.is_ok());
        assert_eq!(bridge.cancelled_count(), 1);

        // 验证任务状态
        let task = scheduler.get_task(task_id, tenant_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_pause_resume_task_with_mock_bridge() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let bridge = Arc::new(MockExecutorBridge::new());

        let scheduler = TaskSchedulerImpl::new_with_bridge(config, matcher, bridge.clone());

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        let response = scheduler.submit_task(request).await.unwrap();
        let task_id = response.task.task_id;

        // 暂停
        let result = scheduler.pause_task(task_id, tenant_id).await;
        assert!(result.is_ok());
        assert_eq!(bridge.paused.lock().unwrap().len(), 1);

        let task = scheduler.get_task(task_id, tenant_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Paused);

        // 恢复
        let result = scheduler.resume_task(task_id, tenant_id).await;
        assert!(result.is_ok());
        assert_eq!(bridge.resumed.lock().unwrap().len(), 1);

        let task = scheduler.get_task(task_id, tenant_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_backward_compatible_new() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let (dispatch_tx, mut dispatch_rx) = mpsc::unbounded_channel::<Task>();

        let scheduler = TaskSchedulerImpl::new(config, matcher, dispatch_tx);

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        let result = scheduler.submit_task(request).await;
        assert!(result.is_ok());

        // 旧版 API 中 dispatch_tx 不再用于核心派发
        // 验证 channel 为空（向后兼容但行为已变更）
        assert!(dispatch_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_executor_bridge_failure() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let bridge = Arc::new(MockExecutorBridge::new());
        bridge.set_should_fail(true);

        let scheduler = TaskSchedulerImpl::new_with_bridge(config, matcher, bridge.clone());

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        // 即使执行器失败，submit_task 也应该返回 Ok（任务已创建但执行失败）
        let result = scheduler.submit_task(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        // 任务状态应该是 Failed（因为执行器提交失败）
        assert_eq!(response.task.status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_task_syncs_executor_status() {
        let config = create_test_config();
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        let bridge = Arc::new(MockExecutorBridge::new());

        let scheduler = TaskSchedulerImpl::new_with_bridge(config.clone(), matcher, bridge.clone());

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let request = TaskSubmitRequest {
            tenant_id,
            user_id,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            task_type: None,
            priority: None,
            mode: None,
            fusion_strategy: None,
        };

        let response = scheduler.submit_task(request).await.unwrap();
        let task_id = response.task.task_id;

        // 设置执行器返回的状态（模拟部分完成）
        bridge.set_status(ExecutionStatus {
            task_id,
            total_nodes: 5,
            completed_nodes: 2,
            running_nodes: 1,
            failed_nodes: 0,
            pending_nodes: 2,
            skipped_nodes: 0,
            cancelled_nodes: 0,
            progress: 0.4,
            started_at: None,
            estimated_remaining_ms: None,
        });

        // 获取任务时应该同步执行器状态
        let task = scheduler.get_task(task_id, tenant_id).await.unwrap();
        assert_eq!(task.progress, 0.4);
    }
}
