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
use crate::state_sink::{ExecutionStateSink, ExecutionView, synthesize_nodes};

/// 任务执行状态（内部完整状态）
pub(crate) struct TaskExecutionState {
    task: Task,
    plan: CollaborationPlan,
    nodes: HashMap<String, Node>,
    /// 任务级执行选项（重试预算 / 超时 / fail-fast，场景⑤ SSOT）
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
    /// 启动恢复：重建执行状态，`completed` 中的节点直接置 Completed（不重复执行）
    Restore {
        task: Box<Task>,
        plan: Box<CollaborationPlan>,
        options: Box<ExecutionOptions>,
        /// 已完成节点（node_id, result）
        completed: Vec<(String, Option<serde_json::Value>)>,
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

    /// 启动恢复扫描（场景②）：从持久化端口读出未完成任务，重建执行状态并继续派发。
    ///
    /// - 已完成节点直接置 Completed（**跳过**：不重复执行、不重复外部副作用）；
    /// - 已在执行中的任务跳过（防重复注入）；
    /// - 未注入 sink，或 file 底座（无 plan / 节点表）→ 返回空列表。
    pub fn restore(&self, sink: &Arc<dyn ExecutionStateSink>) -> AllianceResult<Vec<Uuid>> {
        let restorable = sink.restore_pending()?;
        let mut restored = Vec::new();
        for r in restorable {
            let task_id = r.task.task_id;
            if self.states.read().contains_key(&task_id) {
                debug!("Task {} 已在执行中，跳过恢复", task_id);
                continue;
            }
            let cmd = ControlCommand::Restore {
                task: Box::new(r.task),
                plan: Box::new(r.plan),
                options: Box::new(ExecutionOptions {
                    max_retries: self.config.default_max_retries,
                    node_timeout_ms: self.config.default_node_timeout_ms,
                    fail_fast: false,
                }),
                completed: r.completed_nodes,
            };
            if self.control_tx.send(cmd).is_err() {
                warn!("恢复任务 {}：控制通道已不可用", task_id);
                continue;
            }
            restored.push(task_id);
        }
        Ok(restored)
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

    /// 融合输出持久化出口（任务 Completed 且已产出融合结果时写入；sqlite 底座持久化，
    /// 其余底座默认 no-op——重启后 `/result` 不再丢失已产出结论）
    fn persist_fusion_output(
        sink: Option<&Arc<dyn ExecutionStateSink>>,
        state: &TaskExecutionState,
    ) {
        let Some(s) = sink else { return };
        if state.task.status != TaskStatus::Completed {
            return;
        }
        let Some(output) = state.fusion_output.as_ref() else {
            return;
        };
        match serde_json::to_value(output) {
            Ok(json) => {
                if let Err(e) = s.persist_fusion_output(state.task.task_id, &json) {
                    warn!("持久化融合输出失败 {}: {}", state.task.task_id, e);
                }
            }
            Err(e) => warn!("融合输出序列化失败 {}: {}", state.task.task_id, e),
        }
    }

    /// 内存 miss 时的存储层回读（多实例 / 重启后可用）：
    /// 任务存在性 + 租户校验 + 融合输出读回；底座无能力 → 语义同旧版。
    fn fusion_from_sink(
        sink: &Option<Arc<dyn ExecutionStateSink>>,
        task_id: Uuid,
        tenant_id: Uuid,
    ) -> AllianceResult<Option<FusionOutput>> {
        let not_found =
            || AllianceError::not_found("Task", &task_id.to_string());
        let Some(s) = sink.as_ref() else {
            return Err(not_found());
        };
        match s.read_back(task_id)? {
            Some(view) => {
                view_tenant_check(&view, tenant_id)?;
                match s.read_fusion_output(task_id)? {
                    Some(json) => match serde_json::from_value::<FusionOutput>(json) {
                        Ok(output) => Ok(Some(output)),
                        Err(e) => {
                            warn!("融合输出反序列化失败 {}: {}", task_id, e);
                            Ok(None)
                        }
                    },
                    None => Ok(None),
                }
            }
            None => Err(not_found()),
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
                    // 计划同样落库：恢复期需要原始 DAG 才能重建执行状态（场景②前提）
                    if let Some(s) = state_sink {
                        if let Err(e) = s.persist_plan(task_id, &st.plan) {
                            warn!("持久化协作计划失败 {}: {}", task_id, e);
                        }
                    }
                }
            }
            ControlCommand::Restore {
                task,
                plan,
                options,
                completed,
            } => {
                let mut task = *task;
                let plan = *plan;
                let options = *options;

                let mut nodes_map = HashMap::new();
                for node in &plan.nodes {
                    nodes_map.insert(node.node_id.clone(), node.clone());
                }
                let task_id = task.task_id;
                let dynamic_routes = Self::build_dynamic_routes(&plan);

                // 已完成节点：置 Completed 并回填输出，避免重复执行与重复副作用
                let mut outputs: HashMap<String, NodeExecutionResult> = HashMap::new();
                for (node_id, result) in &completed {
                    if let Some(n) = nodes_map.get_mut(node_id) {
                        n.status = NodeStatus::Completed;
                        n.completed_at = Some(chrono::Utc::now());
                        n.output_ref = Some(format!("output-{}", node_id));
                        outputs.insert(
                            node_id.clone(),
                            NodeExecutionResult {
                                node_id: node_id.clone(),
                                task_id,
                                success: true,
                                output: result.clone(),
                                error_message: None,
                                duration_ms: 0,
                                retry_count: 0,
                            },
                        );
                    }
                }
                // 其余节点保持 Pending（存储层已把 interrupted 解析为 pending），等待调度器继续派发
                task.status = TaskStatus::Running;
                if task.started_at.is_none() {
                    task.started_at = Some(chrono::Utc::now());
                }

                let state = TaskExecutionState {
                    task,
                    plan,
                    nodes: nodes_map,
                    options,
                    outputs,
                    fusion_output: None,
                    dynamic_routes,
                };
                let mut states = states.write();
                states.insert(task_id, state);
                info!(
                    "Task execution restored: {} ({} completed node(s) skipped)",
                    task_id,
                    completed.len()
                );
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
        // 收集所有就绪的节点（附带任务级重试预算，场景⑤ SSOT）
        let mut ready_nodes: Vec<(Uuid, String, Node, String, u32)> = Vec::new();

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
                            state.options.max_retries,
                        ));
                    }
                }
            }
        }

        // 并发执行就绪节点
        let mut handles = Vec::new();
        for (task_id, node_id, node, tenant_id, max_retries) in ready_nodes {
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
                    // 场景⑤ SSOT：任务级重试预算从 ExecutionOptions 注入（覆盖执行器默认）
                    max_retries: Some(max_retries),
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
                        Self::persist_fusion_output(Some(s), state);
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
    fn check_task_completion(state: &mut TaskExecutionState) {
        // 场景④仲裁：任务已到终态（如取消后残留的 Running 节点晚到完成）→
        // 终态不可离开，完成度推进与融合一律不执行（否则 Cancelled 会被覆盖成 Completed）
        if state.task.status.is_terminal() {
            debug!(
                "Task {} 已处终态 {:?}，忽略节点完成度推进",
                state.task.task_id, state.task.status
            );
            return;
        }
        let all_terminal = state.nodes.values().all(|n| n.status.is_terminal());
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
        {
            let states = self.states.read();
            if let Some(state) = states.get(&task_id) {
                if state.task.tenant_id != tenant_id {
                    return Err(AllianceError::new(
                        AllianceErrorCode::TenantMismatch,
                        "Task does not belong to this tenant",
                    ));
                }
                return Ok(Self::compute_execution_status(state));
            }
        }
        // 内存 miss：存储层回读（多实例——任务可能在另一实例执行，或本进程重启前完成）
        if let Some(sink) = self.state_sink.as_ref() {
            if let Some(view) = sink.read_back(task_id)? {
                view_tenant_check(&view, tenant_id)?;
                return Ok(status_from_view(&view));
            }
        }
        Err(AllianceError::not_found("Task", &task_id.to_string()))
    }

    async fn get_nodes(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Vec<Node>> {
        {
            let states = self.states.read();
            if let Some(state) = states.get(&task_id) {
                if state.task.tenant_id != tenant_id {
                    return Err(AllianceError::new(
                        AllianceErrorCode::TenantMismatch,
                        "Task does not belong to this tenant",
                    ));
                }
                let mut nodes: Vec<Node> = state.nodes.values().cloned().collect();
                nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
                return Ok(nodes);
            }
        }
        if let Some(sink) = self.state_sink.as_ref() {
            if let Some(view) = sink.read_back(task_id)? {
                view_tenant_check(&view, tenant_id)?;
                let mut nodes = nodes_from_view(&view);
                nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
                return Ok(nodes);
            }
        }
        Err(AllianceError::not_found("Task", &task_id.to_string()))
    }

    async fn get_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        tenant_id: Uuid,
    ) -> AllianceResult<Node> {
        {
            let states = self.states.read();
            if let Some(state) = states.get(&task_id) {
                if state.task.tenant_id != tenant_id {
                    return Err(AllianceError::new(
                        AllianceErrorCode::TenantMismatch,
                        "Task does not belong to this tenant",
                    ));
                }
                return state
                    .nodes
                    .get(node_id)
                    .cloned()
                    .ok_or_else(|| AllianceError::not_found("Node", node_id));
            }
        }
        if let Some(sink) = self.state_sink.as_ref() {
            if let Some(view) = sink.read_back(task_id)? {
                view_tenant_check(&view, tenant_id)?;
                return nodes_from_view(&view)
                    .into_iter()
                    .find(|n| n.node_id == node_id)
                    .ok_or_else(|| AllianceError::not_found("Node", node_id));
            }
        }
        Err(AllianceError::not_found("Task", &task_id.to_string()))
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
    ///
    /// 诚实边界：融合输出目前仅存在于执行内存，**未持久化**——重启后或跨实例查询
    /// 已完成任务时返回 Ok(None)（任务存在性仍可经存储层回读确认）。
    async fn get_fusion_output(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
    ) -> AllianceResult<Option<FusionOutput>> {
        {
            let states = self.states.read();
            if let Some(state) = states.get(&task_id) {
                if state.task.tenant_id != tenant_id {
                    return Err(AllianceError::new(
                        AllianceErrorCode::TenantMismatch,
                        "Task does not belong to this tenant",
                    ));
                }
                return Ok(state.fusion_output.clone());
            }
        }
        // 内存 miss：回落到存储层（任务存在性 + 租户校验 + 融合输出读回）
        Self::fusion_from_sink(&self.state_sink, task_id, tenant_id)
    }

    fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}




/// 由回读视图合成执行状态（节点计数与进度，与内存态同口径）
fn status_from_view(view: &ExecutionView) -> ExecutionStatus {
    let rows: HashMap<&str, &str> = view
        .nodes
        .iter()
        .map(|(id, s, _)| (id.as_str(), s.as_str()))
        .collect();
    let statuses: Vec<&str> = match &view.plan {
        Some(plan) => plan
            .nodes
            .iter()
            .map(|n| rows.get(n.node_id.as_str()).copied().unwrap_or("pending"))
            .collect(),
        None => view.nodes.iter().map(|(_, s, _)| s.as_str()).collect(),
    };
    let (mut completed, mut running, mut failed, mut pending, mut skipped, mut cancelled) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for s in &statuses {
        match *s {
            "completed" => completed += 1,
            "running" => running += 1,
            "failed" => failed += 1,
            "skipped" => skipped += 1,
            "cancelled" => cancelled += 1,
            _ => pending += 1,
        }
    }
    let total = statuses.len();
    ExecutionStatus {
        task_id: view.task.task_id,
        total_nodes: total,
        completed_nodes: completed,
        running_nodes: running,
        failed_nodes: failed,
        pending_nodes: pending,
        skipped_nodes: skipped,
        cancelled_nodes: cancelled,
        progress: if total > 0 {
            completed as f32 / total as f32
        } else {
            0.0
        },
        started_at: view.task.started_at,
        estimated_remaining_ms: None,
    }
}

/// 由回读视图合成节点集合（无计划 → 空集，诚实降级）
fn nodes_from_view(view: &ExecutionView) -> Vec<Node> {
    match &view.plan {
        Some(plan) => synthesize_nodes(plan, &view.nodes),
        None => Vec::new(),
    }
}

/// 回读视图的租户校验
fn view_tenant_check(view: &ExecutionView, tenant_id: Uuid) -> AllianceResult<()> {
    if view.task.tenant_id != tenant_id {
        return Err(AllianceError::new(
            AllianceErrorCode::TenantMismatch,
            "Task does not belong to this tenant",
        ));
    }
    Ok(())
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
    use crate::state_sink::RestorableTask;
    use mox_alliance_common_proto::{
        AllianceMode, AllianceResult, CollaborationPlan, FusionStrategy, PlanDynamicRoute,
    };

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
        plans: std::sync::Mutex<Vec<Uuid>>,
        fusion: std::sync::Mutex<HashMap<Uuid, serde_json::Value>>,
    }

    impl ExecutionStateSink for RecordingSink {
        fn persist_task(&self, task: &Task) -> AllianceResult<()> {
            self.tasks.lock().unwrap().push(task.clone());
            Ok(())
        }

        fn persist_plan(&self, task_id: Uuid, _plan: &CollaborationPlan) -> AllianceResult<()> {
            self.plans.lock().unwrap().push(task_id);
            Ok(())
        }

        fn restore_pending(&self) -> AllianceResult<Vec<RestorableTask>> {
            Ok(Vec::new())
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

        fn read_back(&self, task_id: Uuid) -> AllianceResult<Option<ExecutionView>> {
            let task = self
                .tasks
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|t| t.task_id == task_id)
                .cloned();
            Ok(task.map(|task| ExecutionView {
                task,
                plan: None,
                nodes: Vec::new(),
            }))
        }

        fn persist_fusion_output(
            &self,
            task_id: Uuid,
            output: &serde_json::Value,
        ) -> AllianceResult<()> {
            self.fusion.lock().unwrap().insert(task_id, output.clone());
            Ok(())
        }

        fn read_fusion_output(&self, task_id: Uuid) -> AllianceResult<Option<serde_json::Value>> {
            Ok(self.fusion.lock().unwrap().get(&task_id).cloned())
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
    fn cancel_wins_over_late_node_completion_scenario4() {
        // 场景④：取消后，残留 Running 节点晚到完成 → 任务必须保持 Cancelled，
        // 不得被 check_task_completion 覆盖成 Completed
        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;
        let mut state = TaskExecutionState {
            task,
            plan: plan_with_routes(vec![]),
            nodes: HashMap::new(),
            options: test_options(),
            outputs: HashMap::new(),
            fusion_output: None,
            dynamic_routes: None,
        };
        // 模拟取消后：任务 Cancelled，节点全部终态（Cancelled）
        state.task.status = TaskStatus::Cancelled;
        let mut node = mox_alliance_common_proto::Node {
            node_id: "n1".to_string(),
            task_id,
            expert_id: "e".to_string(),
            module_id: None,
            name: "n".to_string(),
            description: None,
            status: NodeStatus::Completed,
            retry_count: 0,
            dependencies: vec![],
            input_refs: vec![],
            started_at: None,
            completed_at: Some(chrono::Utc::now()),
            duration_ms: Some(1),
            output_ref: None,
            error_message: None,
        };
        state.nodes.insert("n1".to_string(), node.clone());
        node.status = NodeStatus::Cancelled;
        let _ = node; // 第二个节点保持 Cancelled（用两个节点模拟竞争）
        state.nodes.insert(
            "n2".to_string(),
            mox_alliance_common_proto::Node {
                node_id: "n2".to_string(),
                task_id,
                expert_id: "e".to_string(),
                module_id: None,
                name: "n2".to_string(),
                description: None,
                status: NodeStatus::Cancelled,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                started_at: None,
                completed_at: Some(chrono::Utc::now()),
                duration_ms: Some(1),
                output_ref: None,
                error_message: None,
            },
        );

        DagEngineImpl::check_task_completion(&mut state);

        assert_eq!(
            state.task.status,
            TaskStatus::Cancelled,
            "终态不可被晚到的节点完成覆盖（场景④）"
        );
        assert!(state.fusion_output.is_none(), "已取消任务不得执行融合");
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

    #[test]
    fn fusion_output_persist_and_read_back_via_sink() {
        let sink = Arc::new(RecordingSink::default());
        let sink_dyn: Arc<dyn ExecutionStateSink> = sink.clone();
        let tenant = Uuid::new_v4();
        let task = Task::new(tenant, Uuid::new_v4(), "f".to_string(), "d".to_string());
        let task_id = task.task_id;

        let mut state = TaskExecutionState {
            task,
            plan: plan_with_routes(vec![]),
            nodes: HashMap::new(),
            options: test_options(),
            outputs: HashMap::new(),
            fusion_output: Some(FusionOutput {
                content: serde_json::json!({"winner": "w", "steps": ["a", "b"]}),
                confidence: 0.9,
                expert_count: 2,
                strategy: FusionStrategy::Weighted,
                contributions: HashMap::new(),
                summary: "s".to_string(),
                participating_nodes: 2,
            }),
            dynamic_routes: None,
        };

        // 未达 Completed：不落库（避免中间态/终态前覆盖）
        state.task.status = TaskStatus::Running;
        DagEngineImpl::persist_fusion_output(Some(&sink_dyn), &state);
        assert!(sink.fusion.lock().unwrap().is_empty(), "非完成态不得持久化融合输出");

        // Completed：先落任务行（供 read_back 确认存在）再落融合输出
        state.task.status = TaskStatus::Completed;
        sink.persist_task(&state.task).unwrap();
        DagEngineImpl::persist_fusion_output(Some(&sink_dyn), &state);
        assert!(sink.fusion.lock().unwrap().contains_key(&task_id));

        // 内存 miss → 经存储层读回完整融合输出
        let out = DagEngineImpl::fusion_from_sink(&Some(sink_dyn.clone()), task_id, tenant)
            .unwrap()
            .expect("应从存储层读回融合输出");
        assert_eq!(out.confidence, 0.9);
        assert_eq!(out.content["winner"], "w");
        assert_eq!(out.participating_nodes, 2);

        // 错误租户 → TenantMismatch
        let err = DagEngineImpl::fusion_from_sink(&Some(sink_dyn), task_id, Uuid::new_v4())
            .unwrap_err();
        assert_eq!(err.code(), Some(AllianceErrorCode::TenantMismatch));

        // 未注入 sink / 存储层无此任务 → NotFound（与改动前一致）
        assert!(DagEngineImpl::fusion_from_sink(&None, task_id, tenant).is_err());
        let orphan = Uuid::new_v4();
        assert!(matches!(
            DagEngineImpl::fusion_from_sink(&Some(sink.clone() as Arc<dyn ExecutionStateSink>), orphan, tenant),
            Err(e) if e.code() == Some(AllianceErrorCode::NotFound)
        ));
    }
}
