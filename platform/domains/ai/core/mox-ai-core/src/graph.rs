// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # 核心图谱数据结构
//!
//! 从 mox 单体原型整合的轻量图谱实现，为 AI 对话上下文注入和推理提供数据基础。
//!
//! 包含：节点/边 ID、关联类型、图谱节点/边、带邻接表的 MoxGraph、统计信息。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static TS_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 毫秒级时间戳（单调递增，避免同一毫秒内冲突）
pub fn current_ts() -> u64 {
    let base = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    TS_COUNTER.fetch_max(base, Ordering::SeqCst);
    TS_COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// ID 类型
// ============================================================================

/// 节点 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        NodeId(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        NodeId(s.to_string())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        NodeId(s)
    }
}

/// 关系 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationId(pub String);

impl std::fmt::Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// 关联类型
// ============================================================================

/// 关联类型（公理3图结构）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationType {
    Explicit,
    Implicit,
    Causal,
    Temporal,
    Counterfactual,
}

impl Default for AssociationType {
    fn default() -> Self {
        AssociationType::Explicit
    }
}

// ============================================================================
// 节点与边
// ============================================================================

/// 图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub label: String,
    pub properties: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl GraphNode {
    pub fn new(id: NodeId, label: impl Into<String>) -> Self {
        let now = current_ts();
        Self {
            id,
            label: label.into(),
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 图谱边（关联关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: RelationId,
    pub from: NodeId,
    pub to: NodeId,
    pub relation_type: String,
    pub weight: f64,
    pub assoc_type: AssociationType,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl GraphEdge {
    pub fn new(from: NodeId, to: NodeId, relation_type: impl Into<String>) -> Self {
        let now = current_ts();
        let rel_str = relation_type.into();
        let id_str = format!("{:?}-{:?}-{}", from, to, rel_str);
        Self {
            id: RelationId(id_str),
            from,
            to,
            relation_type: rel_str,
            weight: 0.5,
            assoc_type: AssociationType::Explicit,
            confidence: 1.0,
            evidence: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// 图谱统计
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_degree: f64,
}

// ============================================================================
// MoxGraph — 轻量内存图谱
// ============================================================================

#[derive(Clone, Default)]
pub struct MoxGraph {
    pub nodes: HashMap<NodeId, GraphNode>,
    pub edges: HashMap<RelationId, GraphEdge>,
    adj_out: HashMap<NodeId, Vec<RelationId>>,
    adj_in: HashMap<NodeId, Vec<RelationId>>,
}

impl MoxGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            nodes: HashMap::with_capacity(cap),
            edges: HashMap::with_capacity(cap * 2),
            adj_out: HashMap::with_capacity(cap),
            adj_in: HashMap::with_capacity(cap),
        }
    }

    pub fn add_node(&mut self, node: GraphNode) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        self.adj_out.entry(id.clone()).or_default();
        self.adj_in.entry(id).or_default();
        true
    }

    pub fn add_edge(&mut self, edge: GraphEdge) -> bool {
        if self.edges.contains_key(&edge.id) {
            return false;
        }
        if !self.nodes.contains_key(&edge.from) {
            self.add_node(GraphNode::new(edge.from.clone(), "auto"));
        }
        if !self.nodes.contains_key(&edge.to) {
            self.add_node(GraphNode::new(edge.to.clone(), "auto"));
        }
        self.edges.insert(edge.id.clone(), edge.clone());
        self.adj_out
            .entry(edge.from.clone())
            .or_default()
            .push(edge.id.clone());
        self.adj_in
            .entry(edge.to.clone())
            .or_default()
            .push(edge.id.clone());
        true
    }

    pub fn get_node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn get_edge(&self, id: &RelationId) -> Option<&GraphEdge> {
        self.edges.get(id)
    }

    pub fn remove_node(&mut self, id: &NodeId) -> bool {
        if !self.nodes.contains_key(id) {
            return false;
        }
        if let Some(out_rids) = self.adj_out.remove(id) {
            for rid in out_rids {
                self.edges.remove(&rid);
            }
        }
        if let Some(in_rids) = self.adj_in.remove(id) {
            for rid in in_rids {
                self.edges.remove(&rid);
            }
        }
        self.nodes.remove(id);
        true
    }

    pub fn remove_edge(&mut self, id: &RelationId) -> bool {
        if let Some(edge) = self.edges.remove(id) {
            if let Some(out) = self.adj_out.get_mut(&edge.from) {
                out.retain(|rid| rid != id);
            }
            if let Some(in_) = self.adj_in.get_mut(&edge.to) {
                in_.retain(|rid| rid != id);
            }
            true
        } else {
            false
        }
    }

    pub fn get_neighbors(&self, node_id: &NodeId, limit: usize) -> Vec<GraphNode> {
        let mut result = Vec::new();
        if let Some(out_edges) = self.adj_out.get(node_id) {
            for rid in out_edges.iter().take(limit) {
                if let Some(edge) = self.edges.get(rid) {
                    if let Some(node) = self.nodes.get(&edge.to) {
                        result.push(node.clone());
                    }
                }
            }
        }
        if let Some(in_edges) = self.adj_in.get(node_id) {
            for rid in in_edges.iter().take(limit.saturating_sub(result.len())) {
                if let Some(edge) = self.edges.get(rid) {
                    if let Some(node) = self.nodes.get(&edge.from) {
                        if result.len() >= limit {
                            break;
                        }
                        result.push(node.clone());
                    }
                }
            }
        }
        result
    }

    pub fn stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            avg_degree: if self.nodes.is_empty() {
                0.0
            } else {
                self.edges.len() as f64 / self.nodes.len() as f64
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node_and_query() {
        let mut graph = MoxGraph::new();
        let node = GraphNode::new(NodeId::new("user:001"), "User");
        assert!(graph.add_node(node.clone()));
        assert!(!graph.add_node(node.clone()));
        assert!(graph.get_node(&NodeId::new("user:001")).is_some());
        assert_eq!(graph.get_node(&NodeId::new("user:001")).unwrap().label, "User");
    }

    #[test]
    fn test_add_edge_and_neighbors() {
        let mut graph = MoxGraph::new();
        graph.add_node(GraphNode::new(NodeId::new("A"), "Person"));
        graph.add_node(GraphNode::new(NodeId::new("B"), "Company"));
        graph.add_edge(GraphEdge::new(NodeId::new("A"), NodeId::new("B"), "works_at"));
        let neighbors = graph.get_neighbors(&NodeId::new("A"), 10);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].label, "Company");
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = MoxGraph::with_capacity(100);
        for i in 0..10 {
            graph.add_node(GraphNode::new(NodeId::new(format!("n{}", i)), "Test"));
        }
        for i in 0..9 {
            graph.add_edge(GraphEdge::new(
                NodeId::new(format!("n{}", i)),
                NodeId::new(format!("n{}", i + 1)),
                "next",
            ));
        }
        let stats = graph.stats();
        assert_eq!(stats.node_count, 10);
        assert_eq!(stats.edge_count, 9);
        assert!((stats.avg_degree - 0.9).abs() < 0.001);
    }
}
