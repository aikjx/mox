// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG 执行引擎
//!
//! 负责协作计划（DAG）的实际执行，包括：
//! - 节点状态流转管理
//! - 依赖解析与就绪判断
//! - 并行/串行执行调度
//! - 模块化配置集成（每节点使用独立的 LLM/Graph 配置）
//! - 执行结果收集与融合
//! - 错误重试机制
//!
//! ## 设计原则
//! - 事件驱动：通过状态变更事件驱动执行流程
//! - 模块化：每个节点执行时使用对应的模块配置
//! - 可观测：完整的执行轨迹和状态追踪
//! - 容错：支持节点级重试和降级

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use mox_alliance_common_proto::{
    AllianceError, AllianceErrorCode, AllianceResult, CollaborationPlan, Node, NodeStatus, Task,
};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// 执行上下文
// ============================================================================

/// 节点执行上下文
///
/// 包含执行一个节点所需的所有信息，包括模块配置、输入数据等。
#[derive(Debug, Clone)]
pub struct NodeExecutionContext {
    /// 节点 ID
    pub node_id: String,
    /// 任务 ID
    pub task_id: Uuid,
    /// 专家 ID
    pub expert_id: String,
    /// 模块 ID（用于获取模块化配置）
    pub module_id: Option<String>,
    /// 节点名称
    pub node_name: String,
    /// 节点描述
    pub description: Option<String>,
    /// 输入数据引用（上游节点输出）
    pub input_refs: Vec<String>,
    /// 任务完整描述
    pub task_description: String,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
}

/// 节点执行结果
#[derive(Debug, Clone)]
pub struct NodeExecutionResult {
    /// 节点 ID
    pub node_id: String,
    /// 是否成功
    pub success: bool,
    /// 输出数据引用
    pub output_ref: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（失败时）
    pub error_message: Option<String>,
    /// 专家输出内容摘要
    pub output_summary: Option<String>,
    /// 置信度分数
    pub confidence: Option<f64>,
}

// ============================================================================
// 节点执行器 trait
// ============================================================================

/// 节点执行器 trait
///
/// 定义单个节点的执行逻辑，可由不同的后端实现（HTTP 调用、本地执行等）。
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// 执行单个节点
    async fn execute_node(&self, context: NodeExecutionContext) -> NodeExecutionResult;
}

/// Mock 节点执行器（用于测试）
pub struct MockNodeExecutor {
    /// 执行延迟（毫秒）
    pub delay_ms: u64,
    /// 是否模拟失败
    pub should_fail: bool,
}

impl Default for MockNodeExecutor {
    fn default() -> Self {
        Self {
            delay_ms: 10,
            should_fail: false,
        }
    }
}

#[async_trait]
impl NodeExecutor for MockNodeExecutor {
    async fn execute_node(&self, context: NodeExecutionContext) -> NodeExecutionResult {
        let start = std::time::Instant::now();

        // 模拟执行延迟
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;

        let success = !self.should_fail;
        let duration = start.elapsed().as_millis() as u64;

        NodeExecutionResult {
            node_id: context.node_id.clone(),
            success,
            output_ref: if success {
                Some(format!("output-{}", context.node_id))
            } else {
                None
            },
            duration_ms: duration,
            error_message: if success {
                None
            } else {
                Some("Mock execution failure".to_string())
            },
            output_summary: if success {
                Some(format!("{} executed successfully", context.node_name))
            } else {
                None
            },
            confidence: if success { Some(0.85) } else { None },
        }
    }
}

// ============================================================================
// DAG 执行引擎
// ============================================================================

/// DAG 执行引擎
///
/// 管理整个协作计划的执行生命周期，包括节点调度、状态追踪、结果收集。
pub struct DagExecutionEngine {
    /// 节点执行器
    executor: Arc<dyn NodeExecutor>,
    /// 执行中的任务状态 (task_id -> ExecutionState)
    executions: Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
    /// 最大并发节点数
    max_concurrent_nodes: usize,
    /// 默认最大重试次数
    default_max_retries: u32,
}

/// 单个任务的执行状态
struct ExecutionState {
    /// 任务信息
    task: Task,
    /// 协作计划
    plan: CollaborationPlan,
    /// 节点状态映射 (node_id -> Node)
    nodes: HashMap<String, Node>,
    /// 执行结果映射 (node_id -> NodeExecutionResult)
    results: HashMap<String, NodeExecutionResult>,
    /// 当前并发执行的节点数
    running_count: usize,
}

impl DagExecutionEngine {
    /// 创建 DAG 执行引擎
    pub fn new(executor: Arc<dyn NodeExecutor>) -> Self {
        Self {
            executor,
            executions: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_nodes: 10,
            default_max_retries: 2,
        }
    }

    /// 配置最大并发节点数
    pub fn with_max_concurrent_nodes(mut self, max: usize) -> Self {
        self.max_concurrent_nodes = max;
        self
    }

    /// 配置默认最大重试次数
    pub fn with_default_max_retries(mut self, retries: u32) -> Self {
        self.default_max_retries = retries;
        self
    }

    /// 开始执行一个协作计划
    ///
    /// 返回执行启动的确认，实际执行在后台异步进行。
    pub async fn start_execution(
        &self,
        task: Task,
        plan: CollaborationPlan,
    ) -> AllianceResult<()> {
        let task_id = task.task_id;

        // 验证计划
        plan.validate().map_err(|e| {
            AllianceError::new(AllianceErrorCode::PlanGenerationFailed, e)
        })?;

        // 初始化执行状态
        let mut nodes_map = HashMap::new();
        for node in &plan.nodes {
            nodes_map.insert(node.node_id.clone(), node.clone());
        }

        let state = ExecutionState {
            task,
            plan,
            nodes: nodes_map,
            results: HashMap::new(),
            running_count: 0,
        };

        self.executions.write().insert(task_id, state);

        info!(
            "Starting DAG execution for task {} with {} nodes",
            task_id,
            plan.nodes.len()
        );

        // 启动异步执行
        let executions = self.executions.clone();
        let executor = self.executor.clone();
        let max_concurrent = self.max_concurrent_nodes;
        let max_retries = self.default_max_retries;

        tokio::spawn(async move {
            if let Err(e) = Self::run_execution_loop(
                task_id,
                executions,
                executor,
                max_concurrent,
                max_retries,
            )
            .await
            {
                error!("DAG execution failed for task {}: {}", task_id, e);
            }
        });

        Ok(())
    }

    /// 执行主循环
    async fn run_execution_loop(
        task_id: Uuid,
        executions: Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
        executor: Arc<dyn NodeExecutor>,
        max_concurrent: usize,
        max_retries: u32,
    ) -> AllianceResult<()> {
        let (result_tx, mut result_rx) = mpsc::unbounded_channel::<NodeExecutionResult>();

        loop {
            // 找出所有就绪的节点
            let ready_nodes = {
                let state = executions.read();
                let state = state
                    .get(&task_id)
                    .ok_or_else(|| AllianceError::not_found("Execution", &task_id.to_string()))?;

                Self::find_ready_nodes(state)
            };

            // 启动就绪节点的执行（受并发限制）
            {
                let mut state = executions.write();
                let state = state
                    .get_mut(&task_id)
                    .ok_or_else(|| AllianceError::not_found("Execution", &task_id.to_string()))?;

                for node_id in &ready_nodes {
                    if state.running_count >= max_concurrent {
                        break;
                    }

                    let node = state.nodes.get_mut(node_id).unwrap();
                    if node.status == NodeStatus::Pending || node.status == NodeStatus::Ready {
                        node.status = NodeStatus::Running;
                        node.started_at = Some(Utc::now());
                        state.running_count += 1;

                        // 提交执行
                        let ctx = NodeExecutionContext {
                            node_id: node.node_id.clone(),
                            task_id,
                            expert_id: node.expert_id.clone(),
                            module_id: None, // TODO: 从模块配置中映射
                            node_name: node.name.clone(),
                            description: node.description.clone(),
                            input_refs: node.input_refs.clone(),
                            task_description: state.task.description.clone(),
                            retry_count: node.retry_count,
                            max_retries,
                        };

                        let exec = executor.clone();
                        let tx = result_tx.clone();

                        tokio::spawn(async move {
                            let result = exec.execute_node(ctx).await;
                            let _ = tx.send(result);
                        });
                    }
                }
            }

            // 检查是否已完成（没有运行中的节点且没有就绪节点）
            let is_complete = {
                let state = executions.read();
                let state = state.get(&task_id).unwrap();
                let all_terminal = state
                    .nodes
                    .values()
                    .all(|n| n.status.is_terminal());
                all_terminal || (ready_nodes.is_empty() && state.running_count == 0)
            };

            if is_complete {
                break;
            }

            // 等待执行结果
            match result_rx.recv().await {
                Some(result) => {
                    Self::handle_execution_result(task_id, &executions, result, max_retries).await;
                }
                None => {
                    warn!("Result channel closed unexpectedly for task {}", task_id);
                    break;
                }
            }
        }

        // 标记最终状态
        Self::finalize_execution(task_id, &executions).await?;

        Ok(())
    }

    /// 找出所有就绪（依赖已完成）的节点
    fn find_ready_nodes(state: &ExecutionState) -> Vec<String> {
        let mut ready = Vec::new();

        for node in state.plan.nodes.iter() {
            let current_node = state.nodes.get(&node.node_id).unwrap();

            // 只考虑 Pending 状态的节点
            if current_node.status != NodeStatus::Pending {
                continue;
            }

            // 检查所有依赖是否都已完成
            let all_deps_met = node.dependencies.iter().all(|dep_id| {
                if let Some(dep_node) = state.nodes.get(dep_id) {
                    dep_node.status == NodeStatus::Completed
                } else {
                    false // 依赖的节点不存在
                }
            });

            if all_deps_met {
                ready.push(node.node_id.clone());
            }
        }

        ready
    }

    /// 处理节点执行结果
    async fn handle_execution_result(
        task_id: Uuid,
        executions: &Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
        result: NodeExecutionResult,
        max_retries: u32,
    ) {
        let mut state = executions.write();
        let state = match state.get_mut(&task_id) {
            Some(s) => s,
            None => return,
        };

        state.running_count = state.running_count.saturating_sub(1);

        let node = match state.nodes.get_mut(&result.node_id) {
            Some(n) => n,
            None => return,
        };

        if result.success {
            // 成功
            node.status = NodeStatus::Completed;
            node.completed_at = Some(Utc::now());
            node.output_ref = result.output_ref.clone();
            node.duration_ms = Some(result.duration_ms as i64);

            debug!(
                "Node {} completed successfully in {}ms",
                result.node_id, result.duration_ms
            );
        } else {
            // 失败，检查是否可以重试
            if node.retry_count < max_retries {
                node.retry_count += 1;
                node.status = NodeStatus::Pending; // 重置为待执行，等待重新调度
                node.error_message = result.error_message.clone();

                warn!(
                    "Node {} failed, retrying ({}/{})",
                    result.node_id, node.retry_count, max_retries
                );
            } else {
                // 重试耗尽，标记失败
                node.status = NodeStatus::Failed;
                node.completed_at = Some(Utc::now());
                node.duration_ms = Some(result.duration_ms as i64);
                node.error_message = result.error_message.clone();

                error!(
                    "Node {} failed after {} retries: {:?}",
                    result.node_id, max_retries, result.error_message
                );

                // 级联失败：将所有依赖此节点的下游节点标记为 Skipped
                Self::cascade_failure(state, &result.node_id);
            }
        }

        // 保存结果
        state.results.insert(result.node_id.clone(), result);
    }

    /// 级联失败：将依赖失败节点的下游节点标记为跳过
    fn cascade_failure(state: &mut ExecutionState, failed_node_id: &str) {
        // 找出所有直接或间接依赖失败节点的节点
        let mut to_skip = HashSet::new();
        let mut queue = vec![failed_node_id.to_string()];

        while let Some(current) = queue.pop() {
            for node in &state.plan.nodes {
                if node.dependencies.contains(&current) && !to_skip.contains(&node.node_id) {
                    to_skip.insert(node.node_id.clone());
                    queue.push(node.node_id.clone());
                }
            }
        }

        for node_id in to_skip {
            if let Some(node) = state.nodes.get_mut(&node_id) {
                if !node.status.is_terminal() {
                    node.status = NodeStatus::Skipped;
                    node.error_message = Some("Upstream node failed".to_string());
                    debug!("Node {} skipped due to upstream failure", node_id);
                }
            }
        }
    }

    /// 完成执行，计算最终状态
    async fn finalize_execution(
        task_id: Uuid,
        executions: &Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
    ) -> AllianceResult<()> {
        let mut state = executions.write();
        let state = state
            .get_mut(&task_id)
            .ok_or_else(|| AllianceError::not_found("Execution", &task_id.to_string()))?;

        let total_nodes = state.plan.nodes.len();
        let completed = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Completed)
            .count();
        let failed = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Failed)
            .count();
        let skipped = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Skipped)
            .count();

        info!(
            "DAG execution finished for task {}: {} total, {} completed, {} failed, {} skipped",
            task_id, total_nodes, completed, failed, skipped
        );

        Ok(())
    }

    /// 获取执行状态
    pub fn get_execution_status(&self, task_id: Uuid) -> Option<ExecutionStatusView> {
        let state = self.executions.read();
        let state = state.get(&task_id)?;

        let total_nodes = state.plan.nodes.len();
        let completed = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Completed)
            .count();
        let failed = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Failed)
            .count();
        let running = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Running)
            .count();
        let pending = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Pending || n.status == NodeStatus::Ready)
            .count();
        let skipped = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Skipped)
            .count();

        let progress = if total_nodes > 0 {
            completed as f32 / total_nodes as f32
        } else {
            1.0
        };

        Some(ExecutionStatusView {
            task_id,
            total_nodes,
            completed_nodes: completed,
            running_nodes: running,
            failed_nodes: failed,
            pending_nodes: pending,
            skipped_nodes: skipped,
            progress,
            is_complete: failed > 0 || completed + failed + skipped == total_nodes,
            has_failure: failed > 0,
        })
    }

    /// 获取节点详情
    pub fn get_node_status(&self, task_id: Uuid, node_id: &str) -> Option<Node> {
        let state = self.executions.read();
        let state = state.get(&task_id)?;
        state.nodes.get(node_id).cloned()
    }

    /// 获取所有节点状态
    pub fn get_all_nodes(&self, task_id: Uuid) -> Option<Vec<Node>> {
        let state = self.executions.read();
        let state = state.get(&task_id)?;
        Some(state.nodes.values().cloned().collect())
    }

    /// 取消执行
    pub fn cancel_execution(&self, task_id: Uuid) -> bool {
        let mut state = self.executions.write();
        if let Some(exec_state) = state.get_mut(&task_id) {
            for node in exec_state.nodes.values_mut() {
                if !node.status.is_terminal() {
                    node.status = NodeStatus::Cancelled;
                }
            }
            info!("Execution cancelled for task {}", task_id);
            true
        } else {
            false
        }
    }
}

/// 执行状态视图（用于外部查询）
#[derive(Debug, Clone)]
pub struct ExecutionStatusView {
    pub task_id: Uuid,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub running_nodes: usize,
    pub failed_nodes: usize,
    pub pending_nodes: usize,
    pub skipped_nodes: usize,
    pub progress: f32,
    pub is_complete: bool,
    pub has_failure: bool,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{
        AllianceMode, CollaborationPlan, FusionStrategy, Node, NodeStatus, Task,
    };

    fn make_test_task() -> Task {
        Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            "Test description".to_string(),
        )
    }

    fn make_simple_dag(task_id: Uuid, node_count: usize) -> CollaborationPlan {
        let mut nodes = Vec::new();
        for i in 0..node_count {
            nodes.push(Node {
                node_id: format!("node-{}", i + 1),
                task_id,
                expert_id: format!("expert-{}", i + 1),
                name: format!("Node {}", i + 1),
                description: None,
                status: NodeStatus::Pending,
                retry_count: 0,
                dependencies: if i == 0 {
                    vec![]
                } else {
                    vec![format!("node-{}", i)]
                },
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            });
        }

        CollaborationPlan {
            task_id,
            mode: AllianceMode::Sequential,
            fusion_strategy: FusionStrategy::Weighted,
            nodes,
            version: 1,
            created_at: Utc::now(),
        }
    }

    fn make_parallel_dag(task_id: Uuid, node_count: usize) -> CollaborationPlan {
        let nodes = (0..node_count)
            .map(|i| Node {
                node_id: format!("node-{}", i + 1),
                task_id,
                expert_id: format!("expert-{}", i + 1),
                name: format!("Node {}", i + 1),
                description: None,
                status: NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            })
            .collect();

        CollaborationPlan {
            task_id,
            mode: AllianceMode::Parallel,
            fusion_strategy: FusionStrategy::Weighted,
            nodes,
            version: 1,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn parallel_execution_completes() {
        let executor = Arc::new(MockNodeExecutor::default());
        let engine = DagExecutionEngine::new(executor);

        let task = make_test_task();
        let task_id = task.task_id;
        let plan = make_parallel_dag(task_id, 3);

        engine.start_execution(task, plan).await.unwrap();

        // 等待执行完成
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let status = engine.get_execution_status(task_id).unwrap();
        assert!(status.is_complete);
        assert_eq!(status.completed_nodes, 3);
        assert_eq!(status.failed_nodes, 0);
        assert_eq!(status.progress, 1.0);
    }

    #[tokio::test]
    async fn sequential_execution_completes() {
        let executor = Arc::new(MockNodeExecutor::default());
        let engine = DagExecutionEngine::new(executor);

        let task = make_test_task();
        let task_id = task.task_id;
        let plan = make_simple_dag(task_id, 3);

        engine.start_execution(task, plan).await.unwrap();

        // 等待执行完成
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let status = engine.get_execution_status(task_id).unwrap();
        assert!(status.is_complete);
        assert_eq!(status.completed_nodes, 3);
    }

    #[tokio::test]
    async fn execution_failure_cascades() {
        let mut executor = MockNodeExecutor::default();
        executor.should_fail = true;
        executor.delay_ms = 5;
        let executor = Arc::new(executor);
        let engine = DagExecutionEngine::new(executor).with_default_max_retries(0);

        let task = make_test_task();
        let task_id = task.task_id;
        let plan = make_simple_dag(task_id, 3); // 串行：node-1 -> node-2 -> node-3

        engine.start_execution(task, plan).await.unwrap();

        // 等待执行完成
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let status = engine.get_execution_status(task_id).unwrap();
        assert!(status.is_complete);
        assert_eq!(status.failed_nodes, 1); // 第一个节点失败
        assert_eq!(status.skipped_nodes, 2); // 后续节点被跳过
        assert!(status.has_failure);
    }

    #[tokio::test]
    async fn retry_on_failure() {
        let mut executor = MockNodeExecutor::default();
        executor.should_fail = true;
        executor.delay_ms = 5;
        let executor = Arc::new(executor);
        let engine = DagExecutionEngine::new(executor).with_default_max_retries(2);

        let task = make_test_task();
        let task_id = task.task_id;
        let plan = make_parallel_dag(task_id, 1);

        engine.start_execution(task, plan).await.unwrap();

        // 等待执行完成（包括重试）
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let nodes = engine.get_all_nodes(task_id).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].status, NodeStatus::Failed);
        assert_eq!(nodes[0].retry_count, 2); // 重试了 2 次
    }

    #[tokio::test]
    async fn cancel_execution() {
        let mut executor = MockNodeExecutor::default();
        executor.delay_ms = 500; // 长延迟，确保有时间取消
        let executor = Arc::new(executor);
        let engine = DagExecutionEngine::new(executor);

        let task = make_test_task();
        let task_id = task.task_id;
        let plan = make_parallel_dag(task_id, 2);

        engine.start_execution(task, plan).await.unwrap();

        // 立即取消
        engine.cancel_execution(task_id);

        // 检查状态
        let status = engine.get_execution_status(task_id).unwrap();
        assert!(status.is_complete || status.running_nodes > 0);
    }

    #[test]
    fn execution_status_view_progress() {
        let executor = Arc::new(MockNodeExecutor::default());
        let engine = DagExecutionEngine::new(executor);

        let task = make_test_task();
        let task_id = task.task_id;

        // 手动构造部分完成的状态
        let mut state = ExecutionState {
            task: task.clone(),
            plan: make_parallel_dag(task_id, 5),
            nodes: HashMap::new(),
            results: HashMap::new(),
            running_count: 0,
        };

        for i in 0..5 {
            let mut node = Node {
                node_id: format!("node-{}", i + 1),
                task_id,
                expert_id: format!("expert-{}", i + 1),
                name: format!("Node {}", i + 1),
                description: None,
                status: if i < 2 {
                    NodeStatus::Completed
                } else {
                    NodeStatus::Pending
                },
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            };
            if i < 2 {
                node.completed_at = Some(Utc::now());
            }
            state.nodes.insert(node.node_id.clone(), node);
        }

        engine.executions.write().insert(task_id, state);

        let status = engine.get_execution_status(task_id).unwrap();
        assert_eq!(status.total_nodes, 5);
        assert_eq!(status.completed_nodes, 2);
        assert_eq!(status.pending_nodes, 3);
        assert!((status.progress - 0.4).abs() < 0.001);
        assert!(!status.is_complete);
    }
}
