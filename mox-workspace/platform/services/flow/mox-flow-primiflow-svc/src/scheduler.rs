//! DAG 调度器
//!
//! 根据拓扑顺序和依赖关系调度节点执行

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::dag::Dag;
use crate::error::FlowResult;

/// 调度策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulingStrategy {
    /// 串行执行（按拓扑顺序）
    Sequential,
    /// 并行执行（最大并行度）
    Parallel,
    /// 受资源约束的并行
    ResourceConstrained,
    /// 基于优先级的调度
    PriorityBased,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        SchedulingStrategy::Parallel
    }
}

/// 调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// 调度策略
    pub strategy: SchedulingStrategy,
    /// 最大并行度
    pub max_parallelism: usize,
    /// 单个节点超时时间（秒）
    pub node_timeout_secs: u64,
    /// 整体流程超时时间（秒）
    pub flow_timeout_secs: u64,
    /// 失败重试次数
    pub max_retries: u32,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            strategy: SchedulingStrategy::Parallel,
            max_parallelism: 10,
            node_timeout_secs: 300,
            flow_timeout_secs: 3600,
            max_retries: 0,
        }
    }
}

/// 调度结果：执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 执行阶段列表，每个阶段内的节点可并行执行
    pub stages: Vec<Vec<String>>,
    /// 总节点数
    pub total_nodes: usize,
    /// 总阶段数（关键路径长度）
    pub total_stages: usize,
}

/// DAG 调度器接口
#[async_trait]
pub trait DagScheduler: Send + Sync {
    /// 调度器名称
    fn name(&self) -> &str;

    /// 生成执行计划
    fn schedule(&self, dag: &Dag, config: &ScheduleConfig) -> FlowResult<ExecutionPlan>;
}

/// 默认调度器实现
#[derive(Debug, Default)]
pub struct DefaultScheduler;

impl DefaultScheduler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DagScheduler for DefaultScheduler {
    fn name(&self) -> &str {
        "default-scheduler"
    }

    fn schedule(&self, dag: &Dag, _config: &ScheduleConfig) -> FlowResult<ExecutionPlan> {
        // 使用分层方法生成执行计划
        // 每一层包含所有入度已满足的节点
        use std::collections::HashMap;

        let mut in_degree: HashMap<String, usize> = dag
            .topological_sort()?
            .into_iter()
            .map(|id| (id.clone(), dag.predecessors(&id).len()))
            .collect();

        let mut stages: Vec<Vec<String>> = Vec::new();
        let mut remaining: std::collections::HashSet<String> =
            in_degree.keys().cloned().collect();

        while !remaining.is_empty() {
            // 找出所有入度为 0 的节点作为当前阶段
            let current_stage: Vec<String> = remaining
                .iter()
                .filter(|id| *in_degree.get(*id).unwrap_or(&0) == 0)
                .cloned()
                .collect();

            if current_stage.is_empty() {
                // 不应该发生，因为拓扑排序已经验证无环
                break;
            }

            // 从 remaining 中移除当前阶段的节点
            for node_id in &current_stage {
                remaining.remove(node_id);
                // 更新后继节点的入度
                for succ in dag.successors(node_id) {
                    if let Some(deg) = in_degree.get_mut(succ) {
                        if *deg > 0 {
                            *deg -= 1;
                        }
                    }
                }
            }

            stages.push(current_stage);
        }

        let total_nodes = dag.node_count();
        let total_stages = stages.len();

        Ok(ExecutionPlan {
            stages,
            total_nodes,
            total_stages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{Dag, DagNode};

    fn make_node(id: &str) -> DagNode {
        DagNode {
            id: id.to_string(),
            name: id.to_string(),
            operator_type: "noop".to_string(),
            config: None,
            inputs: vec![],
            outputs: vec![],
        }
    }

    #[test]
    fn test_schedule_linear_dag() {
        let mut dag = Dag::new("test", "Test");
        dag.add_node(make_node("a")).unwrap();
        dag.add_node(make_node("b")).unwrap();
        dag.add_node(make_node("c")).unwrap();
        dag.add_edge("a", "b").unwrap();
        dag.add_edge("b", "c").unwrap();

        let scheduler = DefaultScheduler::new();
        let config = ScheduleConfig::default();
        let plan = scheduler.schedule(&dag, &config).unwrap();

        assert_eq!(plan.total_nodes, 3);
        assert_eq!(plan.total_stages, 3); // 线性 DAG 有 3 个阶段
        assert_eq!(plan.stages[0], vec!["a"]);
        assert_eq!(plan.stages[1], vec!["b"]);
        assert_eq!(plan.stages[2], vec!["c"]);
    }

    #[test]
    fn test_schedule_parallel_dag() {
        let mut dag = Dag::new("test", "Test");
        dag.add_node(make_node("start")).unwrap();
        dag.add_node(make_node("a")).unwrap();
        dag.add_node(make_node("b")).unwrap();
        dag.add_node(make_node("end")).unwrap();
        dag.add_edge("start", "a").unwrap();
        dag.add_edge("start", "b").unwrap();
        dag.add_edge("a", "end").unwrap();
        dag.add_edge("b", "end").unwrap();

        let scheduler = DefaultScheduler::new();
        let config = ScheduleConfig::default();
        let plan = scheduler.schedule(&dag, &config).unwrap();

        assert_eq!(plan.total_nodes, 4);
        assert_eq!(plan.total_stages, 3);
        // 第 0 阶段: start
        assert_eq!(plan.stages[0], vec!["start"]);
        // 第 1 阶段: a 和 b（顺序可能不定）
        assert_eq!(plan.stages[1].len(), 2);
        // 第 2 阶段: end
        assert_eq!(plan.stages[2], vec!["end"]);
    }
}
