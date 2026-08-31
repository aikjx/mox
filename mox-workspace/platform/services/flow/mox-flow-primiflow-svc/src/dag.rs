//! DAG（有向无环图）定义
//!
//! 流程的有向无环图表示，支持拓扑排序与环检测

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use crate::error::{FlowError, FlowResult};

/// DAG 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// 节点 ID
    pub id: String,
    /// 节点名称
    pub name: String,
    /// 关联的算子类型
    pub operator_type: String,
    /// 节点配置参数
    pub config: Option<serde_json::Value>,
    /// 输入节点 ID 列表
    pub inputs: Vec<String>,
    /// 输出节点 ID 列表
    pub outputs: Vec<String>,
}

/// DAG 定义
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dag {
    /// DAG ID
    pub id: String,
    /// DAG 名称
    pub name: String,
    /// 节点映射
    nodes: HashMap<String, DagNode>,
    /// 邻接表（节点 -> 后继节点）
    adjacency: HashMap<String, Vec<String>>,
    /// 入度表
    in_degree: HashMap<String, usize>,
}

impl Dag {
    /// 创建空 DAG
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: HashMap::new(),
            adjacency: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: DagNode) -> FlowResult<()> {
        let node_id = node.id.clone();
        if self.nodes.contains_key(&node_id) {
            return Err(FlowError::DagBuildError(format!("节点已存在: {}", node_id)));
        }
        self.adjacency.insert(node_id.clone(), Vec::new());
        self.in_degree.insert(node_id.clone(), 0);
        self.nodes.insert(node_id, node);
        Ok(())
    }

    /// 添加边（from -> to）
    pub fn add_edge(&mut self, from: &str, to: &str) -> FlowResult<()> {
        if !self.nodes.contains_key(from) {
            return Err(FlowError::NodeNotFound(from.to_string()));
        }
        if !self.nodes.contains_key(to) {
            return Err(FlowError::NodeNotFound(to.to_string()));
        }

        self.adjacency
            .get_mut(from)
            .unwrap()
            .push(to.to_string());
        *self.in_degree.get_mut(to).unwrap() += 1;

        Ok(())
    }

    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.get(id)
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }

    /// 获取入度为零的节点（起点）
    pub fn source_nodes(&self) -> Vec<&String> {
        self.in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id)
            .collect()
    }

    /// 获取出度为零的节点（终点）
    pub fn sink_nodes(&self) -> Vec<&String> {
        self.adjacency
            .iter()
            .filter(|(_, neighbors)| neighbors.is_empty())
            .map(|(id, _)| id)
            .collect()
    }

    /// 拓扑排序（Kahn 算法）
    ///
    /// 如果检测到环，返回错误
    pub fn topological_sort(&self) -> FlowResult<Vec<String>> {
        let mut in_degree = self.in_degree.clone();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // 初始化：所有入度为 0 的节点入队
        for (node_id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(node_id.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(neighbors) = self.adjacency.get(&node) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(FlowError::CycleDetected(
                "DAG 中存在环，无法完成拓扑排序".to_string(),
            ));
        }

        Ok(result)
    }

    /// 检测是否有环
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    /// 获取节点的前驱节点
    pub fn predecessors(&self, node_id: &str) -> Vec<&String> {
        let mut preds = Vec::new();
        for (id, neighbors) in &self.adjacency {
            if neighbors.contains(&node_id.to_string()) {
                preds.push(id);
            }
        }
        preds
    }

    /// 获取节点的后继节点
    pub fn successors(&self, node_id: &str) -> Vec<&String> {
        self.adjacency
            .get(node_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_dag_add_node() {
        let mut dag = Dag::new("test", "Test DAG");
        dag.add_node(make_node("a")).unwrap();
        assert_eq!(dag.node_count(), 1);
    }

    #[test]
    fn test_dag_topological_sort() {
        let mut dag = Dag::new("test", "Test DAG");
        dag.add_node(make_node("a")).unwrap();
        dag.add_node(make_node("b")).unwrap();
        dag.add_node(make_node("c")).unwrap();
        dag.add_edge("a", "b").unwrap();
        dag.add_edge("b", "c").unwrap();

        let order = dag.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        // a 必须在 b 前面，b 必须在 c 前面
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_dag_cycle_detection() {
        let mut dag = Dag::new("test", "Test DAG");
        dag.add_node(make_node("a")).unwrap();
        dag.add_node(make_node("b")).unwrap();
        dag.add_edge("a", "b").unwrap();
        dag.add_edge("b", "a").unwrap(); // 形成环

        assert!(dag.has_cycle());
        assert!(dag.topological_sort().is_err());
    }

    #[test]
    fn test_dag_source_sink() {
        let mut dag = Dag::new("test", "Test DAG");
        dag.add_node(make_node("a")).unwrap();
        dag.add_node(make_node("b")).unwrap();
        dag.add_node(make_node("c")).unwrap();
        dag.add_edge("a", "b").unwrap();
        dag.add_edge("b", "c").unwrap();

        let sources = dag.source_nodes();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&&"a".to_string()));

        let sinks = dag.sink_nodes();
        assert_eq!(sinks.len(), 1);
        assert!(sinks.contains(&&"c".to_string()));
    }
}
