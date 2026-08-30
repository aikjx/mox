// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 图存储接口
//!
//! 提供面向图数据的高级存储接口，底层通过 StorageBackend 实现。

use std::sync::Arc;

use crate::error::{StorageError, StorageResult};
use crate::storage_trait::StorageBackend;
use crate::types::{EdgeDirection, GraphEdge, GraphNode, RangeOptions, Value};

/// 图存储
pub struct GraphStore {
    backend: Arc<dyn StorageBackend>,
}

impl GraphStore {
    /// 创建新的图存储
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    /// 添加节点
    pub async fn add_node(&self, node: GraphNode) -> StorageResult<()> {
        if node.id.is_empty() {
            return Err(StorageError::InvalidParameter {
                param: "id".to_string(),
                reason: "node id cannot be empty".to_string(),
            });
        }
        self.backend.graph_put_node(node).await
    }

    /// 获取节点
    pub async fn get_node(&self, node_id: &str) -> StorageResult<GraphNode> {
        self.backend
            .graph_get_node(node_id)
            .await?
            .ok_or_else(|| StorageError::NodeNotFound(node_id.to_string()))
    }

    /// 获取节点（可选）
    pub async fn try_get_node(&self, node_id: &str) -> StorageResult<Option<GraphNode>> {
        self.backend.graph_get_node(node_id).await
    }

    /// 检查节点是否存在
    pub async fn node_exists(&self, node_id: &str) -> StorageResult<bool> {
        self.backend.graph_node_exists(node_id).await
    }

    /// 删除节点
    pub async fn delete_node(&self, node_id: &str) -> StorageResult<bool> {
        self.backend.graph_delete_node(node_id).await
    }

    /// 更新节点属性
    pub async fn update_node_property(
        &self,
        node_id: &str,
        key: &str,
        value: Value,
    ) -> StorageResult<()> {
        let mut node = self.get_node(node_id).await?;
        node.properties.insert(key.to_string(), value);
        node.updated_at = now_ms();
        self.backend.graph_put_node(node).await
    }

    /// 添加边
    pub async fn add_edge(&self, edge: GraphEdge) -> StorageResult<()> {
        if edge.src_id.is_empty() {
            return Err(StorageError::InvalidParameter {
                param: "src_id".to_string(),
                reason: "edge source id cannot be empty".to_string(),
            });
        }
        if edge.dst_id.is_empty() {
            return Err(StorageError::InvalidParameter {
                param: "dst_id".to_string(),
                reason: "edge destination id cannot be empty".to_string(),
            });
        }
        self.backend.graph_put_edge(edge).await
    }

    /// 获取边
    pub async fn get_edge(&self, edge_id: &str) -> StorageResult<GraphEdge> {
        self.backend
            .graph_get_edge(edge_id)
            .await?
            .ok_or_else(|| StorageError::EdgeNotFound(edge_id.to_string()))
    }

    /// 删除边
    pub async fn delete_edge(&self, edge_id: &str) -> StorageResult<bool> {
        self.backend.graph_delete_edge(edge_id).await
    }

    /// 获取节点的出边
    pub async fn get_out_edges(
        &self,
        node_id: &str,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>> {
        self.backend
            .graph_get_edges(node_id, EdgeDirection::Out, edge_type)
            .await
    }

    /// 获取节点的入边
    pub async fn get_in_edges(
        &self,
        node_id: &str,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>> {
        self.backend
            .graph_get_edges(node_id, EdgeDirection::In, edge_type)
            .await
    }

    /// 获取节点的所有边
    pub async fn get_all_edges(
        &self,
        node_id: &str,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>> {
        self.backend
            .graph_get_edges(node_id, EdgeDirection::Both, edge_type)
            .await
    }

    /// 获取邻居节点 ID
    pub async fn get_neighbors(
        &self,
        node_id: &str,
        direction: EdgeDirection,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<String>> {
        let edges = self
            .backend
            .graph_get_edges(node_id, direction, edge_type)
            .await?;

        let neighbors: Vec<String> = edges
            .iter()
            .map(|e| {
                if e.src_id == node_id {
                    e.dst_id.clone()
                } else {
                    e.src_id.clone()
                }
            })
            .collect();

        Ok(neighbors)
    }

    /// 列出节点
    pub async fn list_nodes(&self, options: RangeOptions) -> StorageResult<Vec<GraphNode>> {
        self.backend.graph_list_nodes(options).await
    }

    /// 列出边
    pub async fn list_edges(&self, options: RangeOptions) -> StorageResult<Vec<GraphEdge>> {
        self.backend.graph_list_edges(options).await
    }

    /// 节点计数
    pub async fn node_count(&self) -> StorageResult<u64> {
        Ok(self.backend.stats().await?.total_nodes)
    }

    /// 边计数
    pub async fn edge_count(&self) -> StorageResult<u64> {
        Ok(self.backend.stats().await?.total_edges)
    }

    /// 批量添加节点
    pub async fn batch_add_nodes(&self, nodes: Vec<GraphNode>) -> StorageResult<usize> {
        let mut count = 0;
        for node in nodes {
            self.add_node(node).await?;
            count += 1;
        }
        Ok(count)
    }

    /// 批量添加边
    pub async fn batch_add_edges(&self, edges: Vec<GraphEdge>) -> StorageResult<usize> {
        let mut count = 0;
        for edge in edges {
            self.add_edge(edge).await?;
            count += 1;
        }
        Ok(count)
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
