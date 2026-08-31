//! 图基础类型定义
//!
//! 所有图操作的基础数据结构，纯数据无 IO

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点 ID
pub type NodeId = String;

/// 边 ID
pub type EdgeId = String;

/// 属性值类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PropertyValue {
    /// 字符串
    String(String),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 列表
    List(Vec<PropertyValue>),
    /// 空值
    Null,
}

/// 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// 节点 ID
    pub id: NodeId,
    /// 节点类型
    pub label: String,
    /// 属性
    pub properties: HashMap<String, PropertyValue>,
}

/// 边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 边 ID
    pub id: EdgeId,
    /// 起点
    pub from: NodeId,
    /// 终点
    pub to: NodeId,
    /// 边类型
    pub label: String,
    /// 属性
    pub properties: HashMap<String, PropertyValue>,
    /// 是否有向
    pub directed: bool,
}

/// 图（内存表示）
#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: Vec<GraphEdge>,
    adjacency: HashMap<NodeId, Vec<EdgeId>>,
}

impl Graph {
    /// 创建空图
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加节点
    pub fn add_node(&mut self, node: GraphNode) {
        self.adjacency.entry(node.id.clone()).or_default();
        self.nodes.insert(node.id.clone(), node);
    }

    /// 添加边
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.id.clone());
        if !edge.directed {
            self.adjacency
                .entry(edge.to.clone())
                .or_default()
                .push(edge.id.clone());
        }
        self.edges.push(edge);
    }

    /// 获取节点
    pub fn get_node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 获取邻居节点 ID
    pub fn neighbors(&self, node_id: &NodeId) -> Vec<&NodeId> {
        self.adjacency
            .get(node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|eid| {
                        self.edges.iter().find(|e| &e.id == eid).map(|e| {
                            if &e.from == node_id { &e.to } else { &e.from }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有节点 ID
    pub fn node_ids(&self) -> Vec<&NodeId> {
        self.nodes.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_add_node_and_edge() {
        let mut g = Graph::new();
        g.add_node(GraphNode {
            id: "a".into(),
            label: "test".into(),
            properties: HashMap::new(),
        });
        g.add_node(GraphNode {
            id: "b".into(),
            label: "test".into(),
            properties: HashMap::new(),
        });
        g.add_edge(GraphEdge {
            id: "e1".into(),
            from: "a".into(),
            to: "b".into(),
            label: "link".into(),
            properties: HashMap::new(),
            directed: true,
        });

        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.neighbors(&"a".into()).len(), 1);
    }
}
