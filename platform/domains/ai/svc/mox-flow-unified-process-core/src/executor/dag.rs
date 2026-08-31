// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG 流程执行器 —— 内置统一执行引擎
//!
//! 实现了 `FlowExecutor` trait，提供：
//! - DAG 结构校验（节点引用、循环检测）
//! - 拓扑排序分层执行
//! - 条件分支求值
//! - 变量传递与模板替换
//! - 节点处理器注册与调度
//!
//! 核心算法：
//! 1. 拓扑排序确定执行层次
//! 2. 按层遍历（同层节点可并行）
//! 3. Decision 节点：计算条件 → 选择分支 → 标记跳过节点
//! 4. Guard 节点：失败则阻断后续路径
//! 5. 收集全部节点结果

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::error::{FlowResult, UnifiedFlowError};
use crate::executor::context::ExecutionContext;
use crate::executor::r#trait::{FlowExecutor, FlowHook, NodeHandler};
use crate::extension::ExtensionRegistry;
use crate::types::*;
use crate::utils::condition::evaluate_condition;
use crate::utils::dag::{detect_cycle, topo_sort};

/// DAG 流程执行器
pub struct DagFlowExecutor {
    /// 节点处理器注册表
    handlers: HashMap<UnifiedNodeKind, Box<dyn NodeHandler>>,
    /// 执行钩子
    hooks: Vec<Box<dyn FlowHook>>,
    /// 扩展注册表
    extensions: Arc<ExtensionRegistry>,
    /// 最大执行步数（防无限循环）
    max_execution_steps: usize,
    /// 是否启用并行执行（同层节点并发）
    parallel_execution: bool,
}

impl DagFlowExecutor {
    /// 创建新的执行器
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            hooks: Vec::new(),
            extensions: Arc::new(ExtensionRegistry::new()),
            max_execution_steps: 1000,
            parallel_execution: true,
        }
    }

    /// 设置最大执行步数
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_execution_steps = max_steps;
        self
    }

    /// 设置是否并行执行
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel_execution = parallel;
        self
    }

    /// 设置扩展注册表
    pub fn with_extensions(mut self, extensions: Arc<ExtensionRegistry>) -> Self {
        self.extensions = extensions;
        self
    }

    /// 添加执行钩子
    pub fn add_hook(&mut self, hook: Box<dyn FlowHook>) {
        self.hooks.push(hook);
    }

    /// 注册多个内置处理器（控制节点 + 数据节点等）
    pub fn register_builtin_handlers(&mut self) {
        use crate::handlers::control::*;
        use crate::handlers::data::*;
        use crate::handlers::script::*;

        self.register_handler(UnifiedNodeKind::Start, Box::new(StartHandler));
        self.register_handler(UnifiedNodeKind::End, Box::new(EndHandler));
        self.register_handler(UnifiedNodeKind::Decision, Box::new(DecisionHandler));
        self.register_handler(UnifiedNodeKind::DataInput, Box::new(DataInputHandler));
        self.register_handler(UnifiedNodeKind::DataOutput, Box::new(DataOutputHandler));
        self.register_handler(UnifiedNodeKind::Transform, Box::new(TransformHandler));
        self.register_handler(UnifiedNodeKind::Delay, Box::new(DelayHandler));
        self.register_handler(UnifiedNodeKind::Script, Box::new(ScriptHandler));
    }

    // === 内部执行逻辑 ===

    /// 执行 before_execute 钩子
    async fn run_before_execute_hooks(&self, graph: &UnifiedFlowGraph) -> FlowResult<()> {
        for hook in &self.hooks {
            hook.before_execute(graph).await?;
        }
        Ok(())
    }

    /// 执行 after_execute 钩子
    async fn run_after_execute_hooks(
        &self,
        result: &mut UnifiedExecutionResult,
    ) -> FlowResult<()> {
        for hook in &self.hooks {
            hook.after_execute(result).await?;
        }
        Ok(())
    }

    /// 执行单个节点（含 before/after 钩子）
    async fn execute_node_with_hooks(
        &self,
        node: &UnifiedFlowNode,
        variables: &HashMap<String, serde_json::Value>,
        previous_outputs: &HashMap<String, serde_json::Value>,
        trace_id: &str,
    ) -> FlowResult<UnifiedNodeResult> {
        // 占位图：execute_node 是内部方法，实际应从外部传入 graph
        let placeholder_graph = UnifiedFlowGraph::new("internal", "internal");
        let ctx = ExecutionContext::new(
            &placeholder_graph,
            variables,
            previous_outputs,
            trace_id,
            &self.extensions,
        );

        // before_node 钩子
        for hook in &self.hooks {
            hook.before_node(node, &ctx).await?;
        }

        // 查找处理器并执行
        let handler = self.get_handler(&node.kind).ok_or_else(|| {
            UnifiedFlowError::HandlerNotFound {
                node_id: node.id.clone(),
                kind: format!("{:?}", node.kind),
            }
        })?;

        let mut result = handler.execute(node, &ctx).await?;

        // after_node 钩子
        for hook in &self.hooks {
            hook.after_node(node, &mut result).await?;
        }

        Ok(result)
    }
}

impl Default for DagFlowExecutor {
    fn default() -> Self {
        let mut executor = Self::new();
        executor.register_builtin_handlers();
        executor
    }
}

#[async_trait]
impl FlowExecutor for DagFlowExecutor {
    async fn execute(
        &self,
        graph: &UnifiedFlowGraph,
        input: HashMap<String, serde_json::Value>,
    ) -> FlowResult<UnifiedExecutionResult> {
        let start_time = Instant::now();
        let trace_id = Uuid::new_v4().to_string();

        // 0. 结构校验
        self.validate(graph)?;

        // 1. 执行前钩子
        self.run_before_execute_hooks(graph).await?;

        // 2. 初始化变量（合并 graph.variables + input）
        let mut variables = graph.variables.clone();
        variables.extend(input.clone());

        let mut node_results: Vec<UnifiedNodeResult> = Vec::new();
        let mut previous_outputs: HashMap<String, serde_json::Value> = HashMap::new();
        let mut skipped_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 3. 拓扑排序
        let layers = topo_sort(graph)?;
        let mut exec_steps = 0usize;

        // 4. 按层执行
        for layer in &layers {
            // 收集本层需要执行的节点（排除被跳过的）
            let active_nodes: Vec<&UnifiedFlowNode> = layer
                .iter()
                .filter_map(|id| {
                    if skipped_nodes.contains(id) {
                        None
                    } else {
                        graph.node(id)
                    }
                })
                .collect();

            if active_nodes.is_empty() {
                continue;
            }

            // 步数检查
            exec_steps += active_nodes.len();
            if exec_steps > self.max_execution_steps {
                return Err(UnifiedFlowError::ExecutionStepsExceeded {
                    max_steps: self.max_execution_steps,
                });
            }

            // 执行本层节点
            // TODO: 并行执行需要 graph 是 Arc 或类似结构，当前简化为串行
            for node in &active_nodes {
                let result = self
                    .execute_node_with_hooks(
                        node,
                        &variables,
                        &previous_outputs,
                        &trace_id,
                    )
                    .await?;

                // 更新变量和输出
                if let Some(ref output) = result.output {
                    previous_outputs.insert(node.id.clone(), output.clone());
                    variables.insert(format!("node_{}", node.id), output.clone());
                    variables.insert("last_output".to_string(), output.clone());
                }

                // Decision 节点：处理条件分支
                if matches!(node.kind, UnifiedNodeKind::Decision) {
                    if let UnifiedNodeConfig::Decision { expression } = &node.config {
                        let condition_result =
                            evaluate_condition(expression, &variables)
                                .map_err(|e| UnifiedFlowError::ConditionError(e))?;

                        // 标记被跳过的分支
                        let outgoing = graph.outgoing_edges(&node.id);
                        for edge in &outgoing {
                            if edge.kind == UnifiedEdgeKind::Conditional {
                                let edge_condition = edge.condition.as_deref().unwrap_or("");
                                let is_true_branch =
                                    edge_condition == "true" || edge_condition == "1";
                                let is_false_branch =
                                    edge_condition == "false" || edge_condition == "0";

                                if (condition_result && is_false_branch)
                                    || (!condition_result && is_true_branch)
                                {
                                    // 标记整个下游分支为跳过
                                    mark_branch_skipped(
                                        graph,
                                        &edge.target,
                                        &mut skipped_nodes,
                                    );
                                }
                            }
                        }
                    }
                }

                // Guard 节点：如果失败，阻断后续路径
                if matches!(node.kind, UnifiedNodeKind::Guard)
                    && result.status == UnifiedNodeStatus::Blocked
                {
                    let outgoing = graph.outgoing_edges(&node.id);
                    for edge in &outgoing {
                        mark_branch_skipped(graph, &edge.target, &mut skipped_nodes);
                    }
                }

                let is_failed = result.status == UnifiedNodeStatus::Failed;
                node_results.push(result);

                if is_failed {
                    // 节点失败，终止执行
                    let duration = start_time.elapsed().as_millis() as u64;
                    let mut result = UnifiedExecutionResult::err(
                        graph,
                        node_results,
                        format!("节点执行失败"),
                        variables,
                        duration,
                    );
                    self.run_after_execute_hooks(&mut result).await?;
                    return Ok(result);
                }
            }
        }

        // 5. 收集最终输出
        let final_output = variables.get("last_output").cloned();
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let mut result = UnifiedExecutionResult::ok(
            graph,
            node_results,
            final_output,
            variables,
            execution_time_ms,
        );

        // 6. 执行后钩子
        self.run_after_execute_hooks(&mut result).await?;

        Ok(result)
    }

    fn validate(&self, graph: &UnifiedFlowGraph) -> FlowResult<()> {
        // 检查 Start 节点
        if graph.start_node().is_none() {
            return Err(UnifiedFlowError::MissingStartNode);
        }

        // 检查 End 节点
        if graph.end_nodes().is_empty() {
            return Err(UnifiedFlowError::MissingEndNode);
        }

        // 检查所有边的节点引用
        let node_ids: std::collections::HashSet<&str> =
            graph.nodes.iter().map(|n| n.id.as_str()).collect();

        for edge in &graph.edges {
            if !node_ids.contains(edge.source.as_str()) {
                return Err(UnifiedFlowError::EdgeRefNotFound {
                    edge: edge.id.clone(),
                    node: edge.source.clone(),
                });
            }
            if !node_ids.contains(edge.target.as_str()) {
                return Err(UnifiedFlowError::EdgeRefNotFound {
                    edge: edge.id.clone(),
                    node: edge.target.clone(),
                });
            }
        }

        // 循环检测
        if detect_cycle(graph)? {
            return Err(UnifiedFlowError::CycleDetected(
                "流程图存在循环依赖".into(),
            ));
        }

        Ok(())
    }

    fn register_handler(&mut self, kind: UnifiedNodeKind, handler: Box<dyn NodeHandler>) {
        self.handlers.insert(kind, handler);
    }

    fn get_handler(&self, kind: &UnifiedNodeKind) -> Option<&dyn NodeHandler> {
        self.handlers.get(kind).map(|h| h.as_ref())
    }
}

/// 标记某个分支下游的所有节点为跳过
fn mark_branch_skipped(
    graph: &UnifiedFlowGraph,
    start_node: &str,
    skipped: &mut std::collections::HashSet<String>,
) {
    let mut stack = vec![start_node.to_string()];
    while let Some(node_id) = stack.pop() {
        if !skipped.insert(node_id.clone()) {
            continue;
        }
        for edge in graph.outgoing_edges(&node_id) {
            stack.push(edge.target.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn simple_graph() -> UnifiedFlowGraph {
        let mut g = UnifiedFlowGraph::new("test", "测试流程");
        g.add_node(UnifiedFlowNode::new("start", "开始", UnifiedNodeKind::Start));
        g.add_node(UnifiedFlowNode::new("end", "结束", UnifiedNodeKind::End));
        g.add_edge(UnifiedFlowEdge::seq("e1", "start", "end"));
        g
    }

    #[test]
    fn test_validate_simple_graph() {
        let executor = DagFlowExecutor::default();
        let g = simple_graph();
        assert!(executor.validate(&g).is_ok());
    }

    #[test]
    fn test_validate_missing_start() {
        let executor = DagFlowExecutor::default();
        let mut g = UnifiedFlowGraph::new("test", "测试");
        g.add_node(UnifiedFlowNode::new("end", "结束", UnifiedNodeKind::End));
        assert!(matches!(
            executor.validate(&g),
            Err(UnifiedFlowError::MissingStartNode)
        ));
    }

    #[test]
    fn test_validate_missing_end() {
        let executor = DagFlowExecutor::default();
        let mut g = UnifiedFlowGraph::new("test", "测试");
        g.add_node(UnifiedFlowNode::new("start", "开始", UnifiedNodeKind::Start));
        assert!(matches!(
            executor.validate(&g),
            Err(UnifiedFlowError::MissingEndNode)
        ));
    }

    #[test]
    fn test_validate_cycle() {
        let executor = DagFlowExecutor::default();
        let mut g = UnifiedFlowGraph::new("test", "循环测试");
        g.add_node(UnifiedFlowNode::new("a", "A", UnifiedNodeKind::Start));
        g.add_node(UnifiedFlowNode::new("b", "B", UnifiedNodeKind::Task));
        g.add_node(UnifiedFlowNode::new("c", "C", UnifiedNodeKind::End));
        g.add_edge(UnifiedFlowEdge::seq("e1", "a", "b"));
        g.add_edge(UnifiedFlowEdge::seq("e2", "b", "c"));
        g.add_edge(UnifiedFlowEdge::seq("e3", "c", "a")); // 循环
        assert!(matches!(
            executor.validate(&g),
            Err(UnifiedFlowError::CycleDetected(_))
        ));
    }

    #[tokio::test]
    async fn test_execute_simple_flow() {
        let executor = DagFlowExecutor::default();
        let g = simple_graph();
        let result = executor.execute(&g, HashMap::new()).await.unwrap();
        assert!(result.success);
        assert_eq!(result.node_results.len(), 2); // start + end
    }
}
