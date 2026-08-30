// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use crate::types::{KnowledgeEdge, KnowledgeNode};
use std::collections::HashMap;

/// 知识图谱构建算子
pub struct KnowledgeGraphBuilder {
    graph: KnowledgeGraph,
}

impl KnowledgeGraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: KnowledgeGraph::new(),
        }
    }

    pub fn add_node(mut self, id: &str, label: &str, node_type: &str) -> Self {
        self.graph.add_node(KnowledgeNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: node_type.to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        self
    }

    pub fn add_node_with_embedding(
        mut self,
        id: &str,
        label: &str,
        node_type: &str,
        embedding: Vec<f64>,
    ) -> Self {
        self.graph.add_node(KnowledgeNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: node_type.to_string(),
            properties: serde_json::json!({}),
            embedding: Some(embedding),
            activation: 0.0,
            metadata: HashMap::new(),
        });
        self
    }

    pub fn add_edge(mut self, source: &str, target: &str, weight: f64) -> Self {
        let _ = self.graph.add_edge(KnowledgeEdge {
            source: source.to_string(),
            target: target.to_string(),
            weight,
            relation_type: "related".to_string(),
            properties: serde_json::json!({}),
        });
        self
    }

    pub fn add_edge_typed(
        mut self,
        source: &str,
        target: &str,
        weight: f64,
        relation: &str,
    ) -> Self {
        let _ = self.graph.add_edge(KnowledgeEdge {
            source: source.to_string(),
            target: target.to_string(),
            weight,
            relation_type: relation.to_string(),
            properties: serde_json::json!({}),
        });
        self
    }

    pub fn build(self) -> KnowledgeGraph {
        self.graph
    }
}

impl Default for KnowledgeGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
