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
    DagEngine, ExecutionOptions, ExecutionStatus, FusionOutput, NodeExecutor, NodeExecutionRequest,
    NodeExecutionResult,
};
use mox_alliance_core::dag;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use mox_alliance_executor_proto::types::ExecutorConfig;

use crate::condition::{CompareOp, Condition, Operand, Operator};
use crate::fusion::{FusionEngine, FusionInput, FusionItem};
use crate::state_sink::ExecutionStateSink;

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
    /// 动态路由表（决策节点 -> 路由规则），Dynamic 模式专用
    dynamic_routes: Option<HashMap<String, DynamicRouteRule>>,
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
    /// 执行状态持久化端口（None = 纯内存执行，行为与未接线前一致）
    state_sink: Option<Arc<dyn ExecutionStateSink>>,
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
    /// 创建引擎并注入执行状态持久化端口（svc 装配 SQLite/DB 适配器时使用）
    pub(crate) fn with_state_sink(
        config: ExecutorConfig,
        node_executor: Arc<dyn NodeExecutor>,
        state_sink: Option<Arc<dyn ExecutionStateSink>>,
    ) -> (Self, mpsc::UnboundedReceiver<ControlCommand>) {
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        let engine = Self {
            config,
            node_executor,
            states: Arc::new(RwLock::new(HashMap::new())),
            control_tx,
            state_sink,
        };

        (engine, control_rx)
    }

    /// 创建引擎并启动调度循环（便捷方法）
    pub fn spawn(config: ExecutorConfig, node_executor: Arc<dyn NodeExecutor>) -> Arc<Self> {
        Self::spawn_with_state_sink(config, node_executor, None)
    }

    /// 创建引擎并启动调度循环（带执行状态持久化端口）
    pub fn spawn_with_state_sink(
        config: ExecutorConfig,
        node_executor: Arc<dyn NodeExecutor>,
        state_sink: Option<Arc<dyn ExecutionStateSink>>,
    ) -> Arc<Self> {
        let (engine, control_rx) = Self::with_state_sink(config.clone(), node_executor.clone(), state_sink);
        let engine = Arc::new(engine);

        let states_clone = engine.states.clone();
        let executor_clone = node_executor;
        let sink_clone = engine.state_sink.clone();
        let poll_interval = config.poll_interval_ms;

        tokio::spawn(async move {
            Self::run_scheduler_loop(
                states_clone,
                executor_clone,
                control_rx,
                poll_interval,
                sink_clone,
            )
            .await;
        });

        engine
    }

    /// 调度循环（在独立任务中运行）
    pub(crate) async fn run_scheduler_loop(
        states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>,
        node_executor: Arc<dyn NodeExecutor>,
        mut control_rx: mpsc::UnboundedReceiver<ControlCommand>,
        poll_interval_ms: u64,
        state_sink: Option<Arc<dyn ExecutionStateSink>>,
    ) {
        loop {
            tokio::select! {
                // 处理控制命令
                Some(cmd) = control_rx.recv() => {
                    Self::handle_control_command(&states, cmd, state_sink.as_ref());
                }

                // 定期调度就绪节点
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)) => {
                    Self::schedule_ready_nodes(states.clone(), node_executor.clone(), state_sink.clone()).await;
                }
            }
        }
    }

    /// 持久化任务状态（失败仅告警，不阻断执行——可用性优先，见 state_sink 模块文档）
    fn persist_task_state(
        sink: Option<&Arc<dyn ExecutionStateSink>>,
        task: &Task,
    ) {
        if let Some(s) = sink {
            if let Err(e) = s.persist_task(task) {
                warn!("持久化任务状态失败 {}: {}", task.task_id, e);
            }
        }
    }

    /// 处理控制命令
    fn handle_control_command(
        states: &RwLock<HashMap<Uuid, TaskExecutionState>>,
        cmd: ControlCommand,
        state_sink: Option<&Arc<dyn ExecutionStateSink>>,
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
                // 先构建路由表再转移 plan 所有权：借用必须发生在 move 之前
                let dynamic_routes = Self::build_dynamic_routes(&plan);

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
                    // 由计划携带的路由规则构建执行期路由表（非 Dynamic 模式为 None）
                    dynamic_routes,
                };

                let mut states = states.write();
                states.insert(task_id, state);
                info!(
                    "Task execution started: {} ({} nodes)",
                    task_id,
                    node_count
                );
                // 场景②出口：任务进入执行期即落库，崩溃后恢复扫描可见
                if let Some(st) = states.get(&task_id) {
                    Self::persist_task_state(state_sink, &st.task);
                }
            }
            ControlCommand::Pause { task_id } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    state.task.status = TaskStatus::Paused;
                    info!("Task execution paused: {}", task_id);
                    Self::persist_task_state(state_sink, &state.task);
                }
            }
            ControlCommand::Resume { task_id } => {
                let mut states = states.write();
                if let Some(state) = states.get_mut(&task_id) {
                    state.task.status = TaskStatus::Running;
                    info!("Task execution resumed: {}", task_id);
                    Self::persist_task_state(state_sink, &state.task);
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
                    // 场景④出口：终态立即落库（存储层恢复解析与调度器同步都以本状态为准）
                    Self::persist_task_state(state_sink, &state.task);
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
                    Self::persist_task_state(state_sink, &state.task);
                }
            }
        }
    }

    /// 调度就绪节点
    async fn schedule_ready_nodes(
        states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>,
        node_executor: Arc<dyn NodeExecutor>,
        state_sink: Option<Arc<dyn ExecutionStateSink>>,
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
            let sink = state_sink.clone();

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

                    // Dynamic 模式：决策节点完成后执行动态路由
                    Self::apply_dynamic_routes(state, &node_id);

                    // 检查任务是否完成
                    Self::check_task_completion(state);

                    // 场景③出口：节点终态增量落库 + 任务进度/终态落库。
                    // 持久化失败仅告警（见 state_sink 模块文档），内存态仍是执行期权威。
                    if let Some(s) = sink.as_ref() {
                        let status_str = node_status_str(
                            state
                                .nodes
                                .get(&node_id)
                                .map(|n| n.status)
                                .unwrap_or(NodeStatus::Pending),
                        );
                        let result_json =
                            state.outputs.get(&node_id).and_then(|r| r.output.clone());
                        if let Err(e) =
                            s.persist_node(task_id, &node_id, status_str, result_json.as_ref())
                        {
                            warn!("持久化节点状态失败 {}/{}: {}", task_id, node_id, e);
                        }
                        Self::persist_task_state(Some(s), &state.task);
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有当前批次的节点完成
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// 由计划携带的路由规则构建执行期动态路由表
    ///
    /// 计划不含规则（非 Dynamic 模式，或旧版本计划序列化而来）时返回 `None`，
    /// `apply_dynamic_routes` 会直接跳过，与改动前行为完全一致。
    fn build_dynamic_routes(plan: &CollaborationPlan) -> Option<HashMap<String, DynamicRouteRule>> {
        if plan.dynamic_routes.is_empty() {
            return None;
        }

        let mut map: HashMap<String, DynamicRouteRule> = HashMap::new();
        for r in &plan.dynamic_routes {
            let operator = match r.operator.as_str() {
                "eq" => Operator::Eq,
                "neq" => Operator::NotEq,
                "gt" => Operator::GreaterThan,
                "gte" => Operator::GreaterThanOrEq,
                "lt" => Operator::LessThan,
                "lte" => Operator::LessThanOrEq,
                other => {
                    warn!(
                        "忽略未知动态路由运算符: {} (决策节点 {}), 该规则不生效",
                        other, r.decision_node
                    );
                    continue;
                }
            };
            map.insert(
                r.decision_node.clone(),
                DynamicRouteRule {
                    decision_node: r.decision_node.clone(),
                    condition: Condition::Compare(CompareOp {
                        left: Operand::Field(r.field.clone()),
                        operator,
                        right: Operand::Literal(r.value.clone()),
                    }),
                    true_branch: r.true_branch.clone(),
                    false_branch: r.false_branch.clone(),
                },
            );
        }

        if map.is_empty() {
            None
        } else {
            info!("Dynamic routes loaded: {} rule(s)", map.len());
            Some(map)
        }
    }

    /// 应用动态路由规则（Dynamic 模式）
    ///
    /// 当决策节点完成后，根据其输出结果选择激活 true_branch 或 false_branch 节点，
    /// 未选择分支的节点标记为 Skipped。
    fn apply_dynamic_routes(state: &mut TaskExecutionState, completed_node_id: &str) {
        let Some(routes) = &state.dynamic_routes else { return };
        let Some(rule) = routes.get(completed_node_id) else { return };

        // 读取决策节点输出，用条件表达式引擎评估
        let output = state.outputs.get(completed_node_id);
        let context = serde_json::json!({
            "success": output.map(|o| o.success).unwrap_or(false),
            "output": output.and_then(|o| o.output.clone()),
        });
        let decision = rule.condition.evaluate(&context);

        let chosen: &Vec<String> = if decision { &rule.true_branch } else { &rule.false_branch };
        let skipped: Vec<String> = (if decision { &rule.false_branch } else { &rule.true_branch }).to_vec();

        // 标记未选择分支的节点为 Skipped
        for node_id in &skipped {
            if let Some(n) = state.nodes.get_mut(node_id) {
                if n.status == NodeStatus::Pending {
                    n.status = NodeStatus::Skipped;
                }
            }
        }

        debug!(
            "Dynamic route applied: node={}, decision={}, chosen={}, skipped={}",
            completed_node_id, decision, chosen.len(), skipped.len()
        );
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
            expert_weights: state.plan.expert_weights.clone(),
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

    /// 获取任务的融合结果（DAG 尾部融合产出；未完成/无结果返回 Ok(None)）
    async fn get_fusion_output(
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

    fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}




/// 节点状态 → 存储层 snake_case 字符串（与 TaskStatus serde 口径一致，供恢复解析对齐）
fn node_status_str(status: NodeStatus) -> &'static str {
    match status {
        NodeStatus::Running => "running",
        NodeStatus::Completed => "completed",
        NodeStatus::Failed => "failed",
        NodeStatus::Skipped => "skipped",
        NodeStatus::Cancelled => "cancelled",
        _ => "pending",
    }
}

/// 动态路由规则（Dynamic 模式专用）///
/// 描述一个决策节点如何根据中间结果选择后续路径。
/// 当前为框架定义，具体路由逻辑由执行器侧实现。
#[derive(Debug, Clone)]
pub struct DynamicRouteRule {
    /// 决策节点 ID
    pub decision_node: String,
    /// 路由条件表达式（预留：当前未实现）
    pub condition: crate::condition::Condition,
    /// 条件为真时激活的节点列表
    pub true_branch: Vec<String>,
    /// 条件为假时激活的节点列表
    pub false_branch: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{AllianceMode, AllianceResult, FusionStrategy, PlanDynamicRoute};

    fn plan_with_routes(routes: Vec<PlanDynamicRoute>) -> CollaborationPlan {
        CollaborationPlan {
            task_id: Uuid::new_v4(),
            mode: AllianceMode::Dynamic,
            fusion_strategy: FusionStrategy::Weighted,
            nodes: vec![],
            dynamic_routes: routes,
            expert_weights: HashMap::new(),
            version: 1,
            created_at: chrono::Utc::now(),
        }
    }

    fn route(operator: &str) -> PlanDynamicRoute {
        PlanDynamicRoute {
            decision_node: "node-decision".to_string(),
            field: "success".to_string(),
            operator: operator.to_string(),
            value: serde_json::Value::Bool(true),
            true_branch: vec!["node-main".to_string()],
            false_branch: vec!["node-fallback".to_string()],
        }
    }

    #[test]
    fn empty_routes_yield_none() {
        // 计划不含规则（非 Dynamic 模式 / 旧版本序列化而来）→ 不启用动态路由，行为同前
        assert!(DagEngineImpl::build_dynamic_routes(&plan_with_routes(vec![])).is_none());
    }

    #[test]
    fn valid_route_is_loaded_and_evaluable() {
        let routes = DagEngineImpl::build_dynamic_routes(&plan_with_routes(vec![route("eq")]))
            .expect("合法规则应构建出路由表");
        let rule = routes.get("node-decision").expect("应按决策节点 ID 索引");

        // 决策成功 → true_branch
        let ok = serde_json::json!({ "success": true, "output": null });
        assert!(rule.condition.evaluate(&ok));
        assert_eq!(rule.true_branch, vec!["node-main".to_string()]);

        // 决策失败 → false_branch
        let fail = serde_json::json!({ "success": false, "output": null });
        assert!(!rule.condition.evaluate(&fail));
        assert_eq!(rule.false_branch, vec!["node-fallback".to_string()]);
    }

    #[test]
    fn unknown_operator_is_skipped_without_affecting_others() {
        // 未知运算符只跳过该条规则并告警，不 panic，也不影响其余规则
        assert!(DagEngineImpl::build_dynamic_routes(&plan_with_routes(vec![route("bogus")])).is_none());

        let routes = DagEngineImpl::build_dynamic_routes(&plan_with_routes(vec![route("bogus"), route("eq")]))
            .expect("剩余合法规则应继续生效");
        assert_eq!(routes.len(), 1);
    }

    #[test]
    fn numeric_operator_maps_correctly() {
        // 分数型条件：output.score >= 0.8
        let r = PlanDynamicRoute {
            decision_node: "node-decision".to_string(),
            field: "output.score".to_string(),
            operator: "gte".to_string(),
            value: serde_json::json!(0.8),
            true_branch: vec!["node-main".to_string()],
            false_branch: vec![],
        };
        let routes = DagEngineImpl::build_dynamic_routes(&plan_with_routes(vec![r]))
            .expect("gte 应被识别");
        let rule = routes.get("node-decision").unwrap();
        assert!(rule.condition.evaluate(&serde_json::json!({"output": {"score": 0.9}})));
        assert!(!rule.condition.evaluate(&serde_json::json!({"output": {"score": 0.5}})));
    }

    // ═══ 执行状态持久化端口（场景②/③/④ 引擎侧出口）═══

    #[derive(Default)]
    struct RecordingSink {
        tasks: std::sync::Mutex<Vec<Task>>,
        nodes: std::sync::Mutex<Vec<(Uuid, String, String)>>,
    }

    impl ExecutionStateSink for RecordingSink {
        fn persist_task(&self, task: &Task) -> AllianceResult<()> {
            self.tasks.lock().unwrap().push(task.clone());
            Ok(())
        }

        fn persist_node(
            &self,
            task_id: Uuid,
            node_id: &str,
            status: &str,
            _result: Option<&serde_json::Value>,
        ) -> AllianceResult<()> {
            self.nodes
                .lock()
                .unwrap()
                .push((task_id, node_id.to_string(), status.to_string()));
            Ok(())
        }
    }

    fn test_options() -> ExecutionOptions {
        ExecutionOptions {
            max_retries: 1,
            node_timeout_ms: 30_000,
            fail_fast: false,
        }
    }

    #[test]
    fn start_command_persists_running_task_via_sink() {
        let states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let sink = Arc::new(RecordingSink::default());
        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;
        let sink_dyn: Arc<dyn ExecutionStateSink> = sink.clone();

        DagEngineImpl::handle_control_command(
            &states,
            ControlCommand::Start {
                task: Box::new(task),
                plan: Box::new(plan_with_routes(vec![])),
                options: Box::new(test_options()),
            },
            Some(&sink_dyn),
        );

        // 内存态已建立（执行不依赖持久化成败）
        assert!(states.read().contains_key(&task_id));
        // 场景②出口：任务进入执行期即以 Running 落库，崩溃后恢复扫描可见
        let persisted = sink.tasks.lock().unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].task_id, task_id);
        assert_eq!(persisted[0].status, TaskStatus::Running);
    }

    #[test]
    fn cancel_command_persists_terminal_state_via_sink() {
        let states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let sink = Arc::new(RecordingSink::default());
        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;
        let sink_dyn: Arc<dyn ExecutionStateSink> = sink.clone();

        DagEngineImpl::handle_control_command(
            &states,
            ControlCommand::Start {
                task: Box::new(task),
                plan: Box::new(plan_with_routes(vec![])),
                options: Box::new(test_options()),
            },
            Some(&sink_dyn),
        );
        DagEngineImpl::handle_control_command(
            &states,
            ControlCommand::Cancel { task_id, reason: None },
            Some(&sink_dyn),
        );

        // 场景④出口：终态立即落库（调度器同步与恢复解析都以本状态为准）
        let persisted = sink.tasks.lock().unwrap();
        assert_eq!(persisted.len(), 2, "Start 与 Cancel 各落库一次");
        assert_eq!(persisted[1].status, TaskStatus::Cancelled);
        // 终态一致性：取消后内存任务状态与落库状态一致
        assert_eq!(
            states.read().get(&task_id).unwrap().task.status,
            TaskStatus::Cancelled
        );
    }

    #[test]
    fn without_sink_execution_stays_pure_in_memory() {
        // 未注入端口（None）时行为与改动前完全一致：纯内存，无任何持久化调用点
        let states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;

        DagEngineImpl::handle_control_command(
            &states,
            ControlCommand::Start {
                task: Box::new(task),
                plan: Box::new(plan_with_routes(vec![])),
                options: Box::new(test_options()),
            },
            None,
        );

        assert!(states.read().contains_key(&task_id));
    }
}
