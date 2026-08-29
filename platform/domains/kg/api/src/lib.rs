// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Knowledge Graph Domain API — trait contracts for graph storage, analytics, fusion, streams.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// —— 兼容旧错误类型（逐步迁移到 mox-error）——
#[derive(Debug, Error)]
pub enum KgApiError {
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("edge not found: {0}")]
    EdgeNotFound(String),
    #[error("graph error: {0}")]
    GraphError(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type KgApiResult<T> = Result<T, KgApiError>;

/// 知识图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub properties: serde_json::Value,
}

/// 知识图谱边（关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub weight: f64,
    pub properties: serde_json::Value,
}

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn add_node(&self, node: GraphNode) -> KgApiResult<()>;
    async fn get_node(&self, id: &str) -> KgApiResult<Option<GraphNode>>;
    async fn delete_node(&self, id: &str) -> KgApiResult<bool>;
    async fn add_edge(&self, edge: GraphEdge) -> KgApiResult<()>;
    async fn get_edges(&self, node_id: &str, direction: EdgeDirection) -> KgApiResult<Vec<GraphEdge>>;
    async fn delete_edge(&self, edge_id: &str) -> KgApiResult<bool>;
    async fn query(&self, cypher: &str) -> KgApiResult<Vec<serde_json::Value>>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDirection { Incoming, Outgoing, Both }

pub trait GraphAnalytics: Send + Sync {
    fn pagerank(&self, damping: f64, max_iter: usize) -> HashMap<String, f64>;
    fn betweenness_centrality(&self) -> HashMap<String, f64>;
    fn connected_components(&self) -> Vec<HashSet<String>>;
    fn shortest_path(&self, source: &str, target: &str) -> Option<Vec<String>>;
    fn community_detection(&self, max_iter: usize) -> HashMap<String, usize>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResult {
    pub entity_id: String,
    pub score: f64,
    pub sources: Vec<String>,
    pub merged_properties: serde_json::Value,
}

pub trait GraphFusion: Send + Sync {
    fn fuse(&self, results: &[Vec<FusionResult>], k: f64) -> Vec<FusionResult>;
    fn align_entities(&self, nodes: &[GraphNode], threshold: f64) -> Vec<Vec<String>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEvent {
    pub event_type: String,
    pub node_id: Option<String>,
    pub edge_id: Option<String>,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait GraphStream: Send + Sync {
    async fn publish(&self, event: GraphEvent) -> KgApiResult<()>;
    async fn subscribe(&self, event_type: &str) -> KgApiResult<tokio::sync::mpsc::Receiver<GraphEvent>>;
}
