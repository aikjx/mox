// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG 执行引擎实现
//!
//! 实现 `DagEngine` trait，负责：
//! - 接收协作计划（DAG）
//! - 按依赖关系调度节点执行
//! - 管理节点状态流转
//! - 追踪执行进度
//!
//! Phase 1 实现：
//! - 内存状态管理
//! - 基于轮询的调度循环
//! - 支持暂停/恢复/取消

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use mox_alliance_common_proto::{
    AllianceError, AllianceErrorCode, AllianceResult, CollaborationPlan, Node, NodeStatus, Task,
    TaskStatus,
};
use mox_alliance_executor_proto::{
    DagEngine, ExecutionOptions, ExecutionStatus, NodeExecutor, NodeExecutionRequest,
    NodeExecutionResult,
};
use mox_alliance_core::dag;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use mox_alliance_executor_proto::types::ExecutorConfig;

use crate::fusion::{FusionEngine, FusionInput, FusionItem, FusionOutput};

/// 任务执行状态（内部完整状态）
pub(crate) struct TaskExecutionState {
    task: Task,
    plan: CollaborationPlan,
    nodes: HashMap<String, Node>,
    #[allow(dead_code)] // 预留：任务级执行选项，供后续控制逻辑扩展使用
    options: ExecutionOptions,
    /// 节点执行结果（node_id -> result，供融合与结果获取）
    outputs: HashMap<String, NodeExecutionResult>,
    /// DAG 尾部融合结论（全部节点成功完成后生成）
    fusion_output: Option<FusionOutput>,
}

/// DAG 执行引擎实现
pub struct DagEngineImpl {
    config: ExecutorConfig,
    #[allow(dead_code)] // DI 装配：引擎持有执行器引用，当前由调度循环参数传递，预留直接访问入口
    node_executor: Arc<dyn NodeExecutor>,
    /// 任务执行状态（task_id -> state）
    states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>,
    /// 执行控制通道（task_id -> ControlCommand）
    control_tx: mpsc::UnboundedSender<ControlCommand>,
}

/// 控制命令
#[derive(Debug)]
pub(crate) enum ControlCommand {
    Start {
        task: Box<Task>,
        plan: Box<CollaborationPlan>,
        options: Box<ExecutionOptions>,
    },
    Pause {
        task_id: Uuid,
    },
    Resume {
        task_id: Uuid,
    },
    Cancel {
        task_id: Uuid,
        reason: Option<String>,
    },
    SkipNode {
        task_id: Uuid,
        node_id: String,
        reason: Option<String>,
    },
}

impl DagEngineImpl {
    pub(crate) fn new(
        config: ExecutorConfig,
        node_executor: Arc<dyn NodeExecutor>,
    ) -> (Self, mpsc::UnboundedReceiver<ControlCommand>) {
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        let engine = Self {
            config,
            node_executor,
            states: Arc::new(RwLock::new(HashMap::new())),
            control_tx,
        };

        (engine, control_rx)
    }

    /// 创建引擎并启动调度循环（便捷方法）
    pub fn spawn(config: ExecutorConfig, node_executor: Arc<dyn NodeExecutor>) -> Arc<Self> {
        let (engine, control_rx) = Self::new(config.clone(), node_executor.clone());
        let engine = Arc::new(engine);

        let states_clone = engine.states.clone();
        let executor_clone = node_executor;
        let poll_interval = config.poll_interval_ms;

        tokio::spawn(async move {
            Self::run_scheduler_loop(states_clone, executor_clone, control_rx, poll_interval).await;
        });

        engine
    }

    /// 调度循环（在独立任务中运行）
    pub(crate) async fn run_scheduler_loop(
        states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>,
        node_executor: Arc<dyn NodeExecutor>,
        mut control_rx: mpsc::UnboundedReceiver<ControlCommand>,
        poll_interval_ms: u64,
    ) {
        loop {
            tokio::select! {
                // 处理控制命令
                Some(cmd) = control_rx.recv() => {
                    Self::handle_control_command(&states, cmd);
                }

                // 定期调度就绪节点
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)) => {
                    Self::schedule_ready_nodes(states.clone(), node_executor.clone()).await;
                }
            }
        }
    }

    /// 处理控制命令
    fn handle_control_command(
        states: &RwLock<HashMap<Uuid, TaskExecutionState>>,
        cmd: ControlCommand,
    ) {
        match cmd {
            ControlCommand::Start { task, plan, options } => {
                let mut task = *task;
                let plan = *plan;
                let options = *options;

                let mut nodes_map = HashMap::new();
                for node in &plan.nodes {
                    nodes_map.insert(node.node_id.clone(), node.clone());
                }

                let task_id = task.task_id;
                let node_count = plan.nodes.len();

                // 启动时将任务状态设为 Running
                task.status = TaskStatus::Running;
                task.started_at = Some(chrono::Utc::now());

                let state = TaskExecutionState {
                    task,
                    plan,
                    nodes: nodes_map,
                    options,
                    outputs: HashMap::new(),
                    fusion_output: None,
                };

                let mut states = states.write();
                states.insert(task_id, state);
                info!(
                    "Task execution started: {} ({} nodes)",
                    task_id,
                    node_count
                );
            }
            ControlCommand::Pause { task_id } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    state.task.status = TaskStatus::Paused;
                    info!("Task execution paused: {}", task_id);
                }
            }
            ControlCommand::Resume { task_id } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    state.task.status = TaskStatus::Running;
                    info!("Task execution resumed: {}", task_id);
                }
            }
            ControlCommand::Cancel { task_id, reason } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    state.task.status = TaskStatus::Cancelled;
                    // 取消所有未完成的节点
                    for node in state.nodes.values_mut() {
                        if !node.status.is_terminal() {
                            node.status = NodeStatus::Cancelled;
                        }
                    }
                    info!("Task execution cancelled: {}, reason: {:?}", task_id, reason);
                }
            }
            ControlCommand::SkipNode { task_id, node_id, reason } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    if let Some(node) = state.nodes.get_mut(&node_id) {
                        if !node.status.is_terminal() {
                            node.status = NodeStatus::Skipped;
                            info!(
                                "Node skipped: {}/{}, reason: {:?}",
                                task_id, node_id, reason
                            );
                        }
                    }
                }
            }
        }
    }

    /// 调度就绪节点
    async fn schedule_ready_nodes(
        states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>,
        node_executor: Arc<dyn NodeExecutor>,
    ) {
        // 收集所有就绪的节点
        let mut ready_nodes: Vec<(Uuid, String, Node, String)> = Vec::new();

        {
            let states = states.read();
            for (task_id, state) in states.iter() {
                if state.task.status != TaskStatus::Running {
                    continue;
                }

                let nodes_list: Vec<Node> = state.nodes.values().cloned().collect();
                let ready = dag::find_ready_nodes(&nodes_list);

                for node_id in ready {
                    if let Some(node) = state.nodes.get(&node_id) {
                        ready_nodes.push((
                            *task_id,
                            node_id,
                            node.clone(),
                            state.task.tenant_id.to_string(),
                        ));
                    }
                }
            }
        }

        // 并发执行就绪节点
        let mut handles = Vec::new();
        for (task_id, node_id, node, tenant_id) in ready_nodes {
            // 标记为 Running
            {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    if let Some(n) = state.nodes.get_mut(&node_id) {
                        if n.status == NodeStatus::Pending {
                            n.status = NodeStatus::Running;
                            n.started_at = Some(chrono::Utc::now());
                        } else {
                            continue; // 已经被调度了
                        }
                    }
                }
            }

            let executor = node_executor.clone();
            let states_clone = states.clone();

            let handle = tokio::spawn(async move {
                let request = NodeExecutionRequest {
                    task_id,
                    node: node.clone(),
                    input_data: None,
                    context: None,
                    tenant_id,
                };

                debug!("Executing node: {}/{}", task_id, node_id);
                let result = executor.execute_node(request).await;

                // 更新节点状态
                let mut states = states_clone.write();
                if let Some(state) = states.get_mut(&task_id) {
                    // 持久化节点执行结果（供 DAG 尾部融合与结果获取）
                    if let Ok(exec_result) = &result {
                        state.outputs.insert(node_id.clone(), exec_result.clone());
                    }
                    if let Some(n) = state.nodes.get_mut(&node_id) {
                        match result {
                            Ok(exec_result) => {
                                if exec_result.success {
                                    n.status = NodeStatus::Completed;
                                    n.output_ref = exec_result
                                        .output
                                        .as_ref()
                                        .map(|_| format!("output-{}", node_id));
                                } else {
                                    n.status = NodeStatus::Failed;
                                    n.error_message = exec_result.error_message;
                                }
                                n.completed_at = Some(chrono::Utc::now());
                                n.duration_ms = Some(exec_result.duration_ms as i64);
                                n.retry_count = exec_result.retry_count;

                                debug!(
                                    "Node {}/{} completed: status={:?}, duration={}ms",
                                    task_id, node_id, n.status, exec_result.duration_ms
                                );
                            }
                            Err(e) => {
                                n.status = NodeStatus::Failed;
                                n.error_message = Some(e.to_string());
                                n.completed_at = Some(chrono::Utc::now());
                                error!("Node {}/{} failed with error: {}", task_id, node_id, e);
                            }
                        }
                    }

                    // 检查任务是否完成
                    Self::check_task_completion(state);
                }
            });

            handles.push(handle);
        }

        // 等待所有当前批次的节点完成
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// 检查任务是否完成
    fn check_task_completion(state: &mut TaskExecutionState) {        let all_terminal = state.nodes.values().all(|n| n.status.is_terminal());
        let any_failed = state
            .nodes
            .values()
            .any(|n| n.status == NodeStatus::Failed);

        if all_terminal {
            if any_failed {
                state.task.status = TaskStatus::Failed;
                state.task.progress = 1.0;
                warn!("Task {} completed with failures", state.task.task_id);
            } else {
                // DAG 尾部融合：全部节点成功后，按 plan.fusion_strategy 执行融合
                Self::run_fusion(state);
                state.task.status = TaskStatus::Completed;
                state.task.progress = 1.0;
                info!("Task {} completed successfully", state.task.task_id);
            }
            state.task.completed_at = Some(chrono::Utc::now());
            if let Some(started) = state.task.started_at {
                let duration = chrono::Utc::now() - started;
                state.task.duration_ms = Some(duration.num_milliseconds());
            }
        } else {
            // 更新进度
            let total = state.nodes.len() as f32;
            let completed = state
                .nodes
                .values()
                .filter(|n| n.status.is_terminal())
                .count() as f32;
            state.task.progress = completed / total;
        }
    }

    /// 在 DAG 尾部执行结果融合，兑现 `plan.fusion_strategy`
    ///
    /// 收集全部成功节点的输出，构造 `FusionInput` 并调用融合引擎，
    /// 结果写入 `state.fusion_output`（任务结构不含专有结果字段）。
    fn run_fusion(state: &mut TaskExecutionState) {
        let strategy = state.plan.fusion_strategy;
        let mut items: Vec<FusionItem> = Vec::new();
        for (node_id, result) in &state.outputs {
            if let Some(node) = state.nodes.get(node_id) {
                if let Some(item) = FusionItem::from_execution(node, result) {
                    items.push(item);
                }
            }
        }

        let input = FusionInput {
            items,
            expert_weights: HashMap::new(),
            strategy,
            task_description: state.task.description.clone(),
        };

        match FusionEngine::new().fuse(input) {
            Ok(output) => {
                info!(
                    "Fusion completed for task {} with strategy {:?}: {} experts, confidence {:.2}",
                    state.task.task_id,
                    strategy,
                    output.expert_count,
                    output.confidence
                );
                // 融合结果写入执行状态（任务结构不含专有结果字段）
                state.fusion_output = Some(output);
            }
            Err(e) => {
                warn!("Fusion failed for task {}: {}", state.task.task_id, e);
            }
        }
    }

    /// 计算执行状态
    fn compute_execution_status(state: &TaskExecutionState) -> ExecutionStatus {        let nodes: Vec<&Node> = state.nodes.values().collect();
        let total = nodes.len();
        let completed = nodes.iter().filter(|n| n.status == NodeStatus::Completed).count();
        let running = nodes.iter().filter(|n| n.status == NodeStatus::Running).count();
        let failed = nodes.iter().filter(|n| n.status == NodeStatus::Failed).count();
        let pending = nodes.iter().filter(|n| n.status == NodeStatus::Pending || n.status == NodeStatus::Ready).count();
        let skipped = nodes.iter().filter(|n| n.status == NodeStatus::Skipped).count();
        let cancelled = nodes.iter().filter(|n| n.status == NodeStatus::Cancelled).count();

        ExecutionStatus {
            task_id: state.task.task_id,
            total_nodes: total,
            completed_nodes: completed,
            running_nodes: running,
            failed_nodes: failed,
            pending_nodes: pending,
            skipped_nodes: skipped,
            cancelled_nodes: cancelled,
            progress: state.task.progress,
            started_at: state.task.started_at,
            estimated_remaining_ms: None,
        }
    }

    /// 获取任务的融合结果（DAG 尾部融合产出；未完成/无结果返回 Ok(None)）
    pub fn get_fusion_output(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
    ) -> AllianceResult<Option<FusionOutput>> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }
        Ok(state.fusion_output.clone())
    }
}

#[async_trait]
impl DagEngine for DagEngineImpl {
    async fn start_execution(
        &self,
        task: &Task,
        plan: CollaborationPlan,
        options: ExecutionOptions,
    ) -> AllianceResult<()> {
        // 验证计划
        plan.validate().map_err(|e| {
            AllianceError::new(AllianceErrorCode::InvalidPlan, e)
        })?;

        // 发送启动命令
        self.control_tx
            .send(ControlCommand::Start {
                task: Box::new(task.clone()),
                plan: Box::new(plan),
                options: Box::new(options),
            })
            .map_err(|e| {
                AllianceError::internal(format!("Failed to send start command: {}", e))
            })?;

        Ok(())
    }

    async fn pause_execution(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        // 验证租户
        {
            let states = self.states.read();
            let state = states
                .get(&task_id)
                .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
            if state.task.tenant_id != tenant_id {
                return Err(AllianceError::new(
                    AllianceErrorCode::TenantMismatch,
                    "Task does not belong to this tenant",
                ));
            }
        }

        self.control_tx
            .send(ControlCommand::Pause { task_id })
            .map_err(|e| {
                AllianceError::internal(format!("Failed to send pause command: {}", e))
            })?;

        Ok(())
    }

    async fn resume_execution(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }
        if state.task.status != TaskStatus::Paused {
            return Err(AllianceError::new(
                AllianceErrorCode::InvalidTaskStatus,
                "Can only resume paused task",
            ));
        }
        drop(states);

        self.control_tx
            .send(ControlCommand::Resume { task_id })
            .map_err(|e| {
                AllianceError::internal(format!("Failed to send resume command: {}", e))
            })?;

        Ok(())
    }

    async fn cancel_execution(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;
        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }
        drop(states);

        self.control_tx
            .send(ControlCommand::Cancel { task_id, reason })
            .map_err(|e| {
                AllianceError::internal(format!("Failed to send cancel command: {}", e))
            })?;

        Ok(())
    }

    async fn get_execution_status(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<ExecutionStatus> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;

        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }

        Ok(Self::compute_execution_status(state))
    }

    async fn get_nodes(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Vec<Node>> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;

        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }

        let mut nodes: Vec<Node> = state.nodes.values().cloned().collect();
        nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        Ok(nodes)
    }

    async fn get_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        tenant_id: Uuid,
    ) -> AllianceResult<Node> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;

        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }

        state
            .nodes
            .get(node_id)
            .cloned()
            .ok_or_else(|| AllianceError::not_found("Node", node_id))
    }

    async fn skip_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()> {
        let states = self.states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| AllianceError::not_found("Task", &task_id.to_string()))?;

        if state.task.tenant_id != tenant_id {
            return Err(AllianceError::new(
                AllianceErrorCode::TenantMismatch,
                "Task does not belong to this tenant",
            ));
        }
        drop(states);

        self.control_tx
            .send(ControlCommand::SkipNode {
                task_id,
                node_id: node_id.to_string(),
                reason,
            })
            .map_err(|e| {
                AllianceError::internal(format!("Failed to send skip command: {}", e))
            })?;

        Ok(())
    }

    fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}


