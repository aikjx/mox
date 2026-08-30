// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 知识图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub properties: serde_json::Value,
    pub embedding: Option<Vec<f64>>,
    pub activation: f64,
    pub metadata: HashMap<String, String>,
}

/// 知识图谱边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub relation_type: String,
    pub properties: serde_json::Value,
}

/// 中心性指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralityMetrics {
    pub degree_centrality: HashMap<String, f64>,
    pub betweenness_centrality: HashMap<String, f64>,
    pub pagerank: HashMap<String, f64>,
    pub closeness_centrality: HashMap<String, f64>,
}

/// 社区结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: usize,
    pub nodes: Vec<String>,
    pub density: f64,
    pub label: String,
}

/// 路径分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub path: Vec<String>,
    pub total_weight: f64,
    pub length: usize,
}

/// 节点推荐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecommendation {
    pub node_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// 图统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub average_degree: f64,
    pub strongly_connected_components: usize,
    pub diameter: Option<usize>,
    pub clustering_coefficient: f64,
}
