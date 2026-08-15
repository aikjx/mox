//! # 算子优化器
//!
//! 实现公理5：资源约束优化
//! 基于DAG的算子调度，最小化资源消耗和执行时间

use operator_core::operator::Operator;
use operator_core::resource::ResourceCost;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::algo::toposort;
use std::sync::Arc;

/// 算子DAG节点
struct DagNode {
    operator: Arc<dyn Operator>,
    earliest_start: u64,
    latest_finish: u64,
}

/// 算子DAG
pub struct OperatorDag {
    graph: DiGraph<DagNode, ()>,
    node_map: std::collections::HashMap<String, NodeIndex>,
}

impl OperatorDag {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: std::collections::HashMap::new(),
        }
    }

    /// 添加算子节点
    pub fn add_operator(&mut self, id: &str, op: Arc<dyn Operator>) -> NodeIndex {
        let node = DagNode {
            operator: op,
            earliest_start: 0,
            latest_finish: u64::MAX,
        };
        let idx = self.graph.add_node(node);
        self.node_map.insert(id.to_string(), idx);
        idx
    }

    /// 添加依赖边：op2依赖op1完成
    pub fn add_dependency(&mut self, op1: &str, op2: &str) -> Result<(), String> {
        let idx1 = self.node_map.get(op1).ok_or_else(|| format!("算子不存在: {}", op1))?;
        let idx2 = self.node_map.get(op2).ok_or_else(|| format!("算子不存在: {}", op2))?;
        self.graph.add_edge(*idx1, *idx2, ());
        Ok(())
    }

    /// 拓扑排序
    pub fn topological_order(&self) -> Result<Vec<String>, String> {
        let sorted = toposort(&self.graph, None)
            .map_err(|_| "DAG中存在环")?;
        // 反向映射 O(n)，避免对排序结果逐元素反查 node_map 的 O(n²) 开销
        let idx_to_name: std::collections::HashMap<NodeIndex, &String> =
            self.node_map.iter().map(|(name, idx)| (*idx, name)).collect();
        Ok(sorted
            .iter()
            .map(|idx| idx_to_name.get(idx).cloned().cloned().unwrap_or_default())
            .collect())
    }

    /// 关键路径分析
    pub fn critical_path(&self) -> Vec<String> {
        // 简化实现：拓扑排序后计算最早开始时间
        let sorted = match toposort(&self.graph, None) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut earliest_finish = std::collections::HashMap::new();
        let mut predecessor = std::collections::HashMap::new();

        for &node in &sorted {
            let node_data = &self.graph[node];
            let cost = node_data.operator.resource_cost().cpu_cycles;
            
            let mut max_pred_finish = 0;
            let mut max_pred = None;
            for edge in self.graph.edges_directed(node, petgraph::Direction::Incoming) {
                let pred = edge.source();
                let pred_finish = *earliest_finish.get(&pred).unwrap_or(&0);
                if pred_finish > max_pred_finish {
                    max_pred_finish = pred_finish;
                    max_pred = Some(pred);
                }
            }

            earliest_finish.insert(node, max_pred_finish + cost);
            if let Some(pred) = max_pred {
                predecessor.insert(node, pred);
            }
        }

        // 找到最后完成的节点
        let mut current = sorted
            .iter()
            .max_by_key(|&&n| earliest_finish.get(&n).unwrap_or(&0))
            .copied();

        // 回溯关键路径
        let mut path = Vec::new();
        while let Some(node) = current {
            let name = self
                .node_map
                .iter()
                .find(|(_, i)| **i == node)
                .map(|(name, _)| name.clone())
                .unwrap();
            path.push(name);
            current = predecessor.get(&node).copied();
        }
        path.reverse();
        path
    }

    /// 估计总执行时间
    pub fn estimated_execution_time(&self) -> u64 {
        let sorted = match toposort(&self.graph, None) {
            Ok(s) => s,
            Err(_) => return u64::MAX,
        };

        let mut earliest_finish = std::collections::HashMap::new();
        for &node in &sorted {
            let node_data = &self.graph[node];
            let cost = node_data.operator.resource_cost().cpu_cycles;
            
            let mut max_pred_finish = 0;
            for edge in self.graph.edges_directed(node, petgraph::Direction::Incoming) {
                let pred = edge.source();
                let pred_finish = *earliest_finish.get(&pred).unwrap_or(&0);
                if pred_finish > max_pred_finish {
                    max_pred_finish = pred_finish;
                }
            }

            earliest_finish.insert(node, max_pred_finish + cost);
        }

        *earliest_finish.values().max().unwrap_or(&0)
    }

    /// 估计总资源消耗
    pub fn estimated_resource_cost(&self) -> ResourceCost {
        self.graph
            .node_weights()
            .map(|n| n.operator.resource_cost())
            .fold(ResourceCost::zero(), |a, b| a + b)
    }
}

impl Default for OperatorDag {
    fn default() -> Self {
        Self::new()
    }
}

/// 资源约束优化器
pub struct ResourceOptimizer {
    max_cpu: u64,
    max_memory: u64,
}

impl ResourceOptimizer {
    pub fn new(max_cpu: u64, max_memory: u64) -> Self {
        Self { max_cpu, max_memory }
    }

    /// 检查算子序列是否满足资源约束
    pub fn check_resources(&self, ops: &[Arc<dyn Operator>]) -> bool {
        let total_cost: ResourceCost = ops
            .iter()
            .map(|op| op.resource_cost())
            .fold(ResourceCost::zero(), |a, b| a + b);
        total_cost.cpu_cycles <= self.max_cpu && total_cost.memory_bytes <= self.max_memory
    }

    /// 贪心调度：按资源消耗排序
    pub fn greedy_schedule(&self, ops: Vec<Arc<dyn Operator>>) -> Vec<Arc<dyn Operator>> {
        let mut ops = ops;
        ops.sort_by_key(|op| op.resource_cost().cpu_cycles);
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_core::operator::IdentityOperator;

    #[test]
    fn test_dag_topological_sort() {
        let mut dag = OperatorDag::new();
        let op1 = Arc::new(IdentityOperator::new(10));
        let op2 = Arc::new(IdentityOperator::new(10));
        let op3 = Arc::new(IdentityOperator::new(10));

        dag.add_operator("a", op1);
        dag.add_operator("b", op2);
        dag.add_operator("c", op3);
        dag.add_dependency("a", "b").unwrap();
        dag.add_dependency("b", "c").unwrap();

        let order = dag.topological_order().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_critical_path() {
        let mut dag = OperatorDag::new();
        let op1 = Arc::new(IdentityOperator::new(10));
        let op2 = Arc::new(IdentityOperator::new(10));
        let op3 = Arc::new(IdentityOperator::new(10));

        dag.add_operator("a", op1);
        dag.add_operator("b", op2);
        dag.add_operator("c", op3);
        dag.add_dependency("a", "c").unwrap();
        dag.add_dependency("b", "c").unwrap();

        let path = dag.critical_path();
        assert!(path.contains(&"a".to_string()) || path.contains(&"b".to_string()));
        assert!(path.contains(&"c".to_string()));
    }

    #[test]
    fn test_cycle_detection_self_loop_fails_toposort() {
        let mut dag = OperatorDag::new();
        let op = Arc::new(IdentityOperator::new(10));
        dag.add_operator("a", op);
        // add_dependency 接受自环边（petgraph 允许），但拓扑排序会检测到环
        assert!(dag.add_dependency("a", "a").is_ok());
        assert!(dag.topological_order().is_err());
    }

    #[test]
    fn test_cycle_detection_back_edge_fails_toposort() {
        let mut dag = OperatorDag::new();
        let op1 = Arc::new(IdentityOperator::new(10));
        let op2 = Arc::new(IdentityOperator::new(10));
        dag.add_operator("a", op1);
        dag.add_operator("b", op2);
        dag.add_dependency("a", "b").unwrap();
        // 加 b -> a 形成环，add_dependency 接受，但拓扑排序应失败
        assert!(dag.add_dependency("b", "a").is_ok());
        assert!(dag.topological_order().is_err());
    }

    #[test]
    fn test_add_dependency_unknown_operator_errors() {
        let mut dag = OperatorDag::new();
        let op = Arc::new(IdentityOperator::new(10));
        dag.add_operator("a", op);
        assert!(dag.add_dependency("a", "ghost").is_err());
        assert!(dag.add_dependency("ghost", "a").is_err());
    }

    #[test]
    fn test_resource_cost_aggregation_and_within_limits() {
        use operator_core::resource::{ResourceCost, ResourceLimits, ResourceUsage};

        let c1 = ResourceCost::new(100, 1024);
        let c2 = ResourceCost::new(200, 2048);
        let total_cpu = c1.cpu_cycles + c2.cpu_cycles;
        let total_mem = c1.memory_bytes + c2.memory_bytes;
        assert_eq!(total_cpu, 300);
        assert_eq!(total_mem, 3072);

        let usage = ResourceUsage {
            cpu_time_ms: 50,
            memory_peak_bytes: 3000,
            disk_io_bytes: 0,
            network_bytes: 0,
        };
        let limits_ok = ResourceLimits {
            max_cpu_time_ms: 100,
            max_memory_bytes: 4096,
            max_disk_io_bytes: u64::MAX,
            max_network_bytes: u64::MAX,
        };
        assert!(usage.within_limits(&limits_ok));
        let limits_tight = ResourceLimits {
            max_cpu_time_ms: 10,
            max_memory_bytes: 1024,
            max_disk_io_bytes: u64::MAX,
            max_network_bytes: u64::MAX,
        };
        assert!(!usage.within_limits(&limits_tight));
    }

    #[test]
    fn test_operator_resource_cost_aggregation() {
        use operator_core::operator::Operator;
        use operator_core::resource::ResourceCost;
        let mut dag = OperatorDag::new();
        let op1 = Arc::new(IdentityOperator::new(10));
        let op2 = Arc::new(IdentityOperator::new(10));
        dag.add_operator("a", op1.clone());
        dag.add_operator("b", op2.clone());
        dag.add_dependency("a", "b").unwrap();
        // 资源成本可分别获取并聚合
        let total_cpu = op1.resource_cost().cpu_cycles + op2.resource_cost().cpu_cycles;
        let total_mem = op1.resource_cost().memory_bytes + op2.resource_cost().memory_bytes;
        assert!(total_cpu > 0);
        assert!(total_mem > 0);
        let _ = ResourceCost::new(total_cpu, total_mem);
        // 拓扑序仍成立
        assert_eq!(dag.topological_order().unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn test_multiple_dags_topological_order_with_diamond() {
        let mut dag = OperatorDag::new();
        let ops: Vec<Arc<IdentityOperator>> = (0..4).map(|_| Arc::new(IdentityOperator::new(4))).collect();
        dag.add_operator("top", ops[0].clone());
        dag.add_operator("left", ops[1].clone());
        dag.add_operator("right", ops[2].clone());
        dag.add_operator("bottom", ops[3].clone());
        dag.add_dependency("top", "left").unwrap();
        dag.add_dependency("top", "right").unwrap();
        dag.add_dependency("left", "bottom").unwrap();
        dag.add_dependency("right", "bottom").unwrap();
        let order = dag.topological_order().unwrap();
        // top 在前，bottom 在后
        let top_pos = order.iter().position(|x| x == "top").unwrap();
        let bottom_pos = order.iter().position(|x| x == "bottom").unwrap();
        assert!(top_pos < bottom_pos);
        assert!(order.len() == 4);
    }
}
