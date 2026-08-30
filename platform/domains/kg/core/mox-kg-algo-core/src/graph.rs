// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::types::{KnowledgeEdge, KnowledgeNode};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

use crate::Result;

/// 知识图谱 - AI驱动无限扩展关系网
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    pub(crate) graph: DiGraph<KnowledgeNode, f64>,
    pub(crate) node_map: HashMap<String, NodeIndex>,
    pub(crate) damping_factor: f64,
    pub(crate) learning_rate: f64,
    pub(crate) activation_history: Vec<HashMap<String, f64>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            damping_factor: 0.85,
            learning_rate: 0.01,
            activation_history: Vec::new(),
        }
    }

    pub fn with_damping(damping: f64) -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            damping_factor: damping,
            learning_rate: 0.01,
            activation_history: Vec::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: KnowledgeNode) -> NodeIndex {
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_map.insert(id, idx);
        idx
    }

    /// 添加边 - 支持权重自适应学习
    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> Result<()> {
        let source = self
            .node_map
            .get(&edge.source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", edge.source))?;
        let target = self
            .node_map
            .get(&edge.target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", edge.target))?;

        // 如果边已存在，强化权重（Hebbian学习）
        if let Some(existing_edge) = self.graph.find_edge(*source, *target) {
            let current_weight = *self.graph.edge_weight(existing_edge).unwrap();
            let new_weight = current_weight + edge.weight * self.learning_rate;
            *self.graph.edge_weight_mut(existing_edge).unwrap() = new_weight;
        } else {
            self.graph.add_edge(*source, *target, edge.weight);
        }
        Ok(())
    }

    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&KnowledgeNode> {
        self.node_map.get(id).map(|idx| &self.graph[*idx])
    }

    /// 获取节点（可变），用于回写布局优化结果（中心性/社区）
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut KnowledgeNode> {
        if let Some(idx) = self.node_map.get(id) {
            let idx = *idx;
            Some(&mut self.graph[idx])
        } else {
            None
        }
    }

    /// 获取节点数
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 获取边数
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// 邻居节点
    pub fn neighbors(&self, id: &str) -> Result<Vec<(String, f64, String)>> {
        let idx = self
            .node_map
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", id))?;
        let mut neighbors = Vec::new();

        for edge in self.graph.edges(*idx) {
            let target = &self.graph[edge.target()];
            neighbors.push((target.id.clone(), *edge.weight(), target.node_type.clone()));
        }
        for edge in self
            .graph
            .edges_directed(*idx, petgraph::Direction::Incoming)
        {
            let source = &self.graph[edge.source()];
            neighbors.push((source.id.clone(), *edge.weight(), source.node_type.clone()));
        }
        Ok(neighbors)
    }

    /// 获取所有节点
    pub fn nodes(&self) -> Vec<&KnowledgeNode> {
        self.graph.node_weights().collect()
    }

    /// 获取所有边
    pub fn edges(&self) -> Vec<KnowledgeEdge> {
        self.graph
            .edge_references()
            .map(|e| {
                let source = &self.graph[e.source()];
                let target = &self.graph[e.target()];
                KnowledgeEdge {
                    source: source.id.clone(),
                    target: target.id.clone(),
                    weight: *e.weight(),
                    relation_type: "related".to_string(),
                    properties: serde_json::json!({}),
                }
            })
            .collect()
    }

    /// 获取所有节点ID
    pub fn node_ids(&self) -> Vec<String> {
        self.node_map.keys().cloned().collect()
    }

    /// 余弦相似度计算（基于嵌入向量）
    pub fn cosine_similarity(&self, a: &str, b: &str) -> Result<f64> {
        let node_a = self
            .get_node(a)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", a))?;
        let node_b = self
            .get_node(b)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", b))?;

        if let (Some(emb_a), Some(emb_b)) = (&node_a.embedding, &node_b.embedding) {
            if emb_a.len() != emb_b.len() {
                return Ok(0.0);
            }
            let dot: f64 = emb_a.iter().zip(emb_b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f64 = emb_a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm_b: f64 = emb_b.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm_a > 1e-15 && norm_b > 1e-15 {
                Ok(dot / (norm_a * norm_b))
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}
