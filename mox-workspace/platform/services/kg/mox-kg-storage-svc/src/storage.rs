//! 图数据库存储抽象
//!
//! 定义统一的图存储接口，支持多种图数据库后端
//! （Neo4j、NebulaGraph、JanusGraph、自研存储等）

use async_trait::async_trait;
use mox_kg_meta_core::{GraphNode, GraphEdge, GraphSchema, NodeId, EdgeId};
use crate::error::StorageResult;

/// 分页参数
#[derive(Debug, Clone)]
pub struct PageParams {
    /// 页码，从 0 开始
    pub offset: usize,
    /// 每页大小
    pub limit: usize,
}

impl Default for PageParams {
    fn default() -> Self {
        Self { offset: 0, limit: 20 }
    }
}

/// 分页结果
#[derive(Debug, Clone)]
pub struct PagedResult<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 总数
    pub total: usize,
    /// 是否有更多
    pub has_more: bool,
}

/// 图存储接口
///
/// 定义图数据库的标准 CRUD 与查询操作
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// 存储后端名称
    fn name(&self) -> &str;

    // --- Schema 操作 ---

    /// 获取图谱 Schema
    async fn get_schema(&self, graph_id: &str) -> StorageResult<GraphSchema>;

    /// 更新图谱 Schema
    async fn update_schema(&self, graph_id: &str, schema: &GraphSchema) -> StorageResult<()>;

    // --- 节点操作 ---

    /// 创建节点
    async fn create_node(&self, graph_id: &str, node: &GraphNode) -> StorageResult<GraphNode>;

    /// 批量创建节点
    async fn create_nodes(&self, graph_id: &str, nodes: &[GraphNode]) -> StorageResult<Vec<GraphNode>>;

    /// 获取节点
    async fn get_node(&self, graph_id: &str, node_id: &NodeId) -> StorageResult<Option<GraphNode>>;

    /// 更新节点
    async fn update_node(&self, graph_id: &str, node: &GraphNode) -> StorageResult<GraphNode>;

    /// 删除节点
    async fn delete_node(&self, graph_id: &str, node_id: &NodeId) -> StorageResult<bool>;

    /// 按标签查询节点（分页）
    async fn query_nodes_by_label(
        &self,
        graph_id: &str,
        label: &str,
        page: &PageParams,
    ) -> StorageResult<PagedResult<GraphNode>>;

    // --- 边操作 ---

    /// 创建边
    async fn create_edge(&self, graph_id: &str, edge: &GraphEdge) -> StorageResult<GraphEdge>;

    /// 批量创建边
    async fn create_edges(&self, graph_id: &str, edges: &[GraphEdge]) -> StorageResult<Vec<GraphEdge>>;

    /// 获取边
    async fn get_edge(&self, graph_id: &str, edge_id: &EdgeId) -> StorageResult<Option<GraphEdge>>;

    /// 更新边
    async fn update_edge(&self, graph_id: &str, edge: &GraphEdge) -> StorageResult<GraphEdge>;

    /// 删除边
    async fn delete_edge(&self, graph_id: &str, edge_id: &EdgeId) -> StorageResult<bool>;

    /// 查询节点的出边
    async fn get_out_edges(
        &self,
        graph_id: &str,
        node_id: &NodeId,
        page: &PageParams,
    ) -> StorageResult<PagedResult<GraphEdge>>;

    /// 查询节点的入边
    async fn get_in_edges(
        &self,
        graph_id: &str,
        node_id: &NodeId,
        page: &PageParams,
    ) -> StorageResult<PagedResult<GraphEdge>>;

    // --- 事务 ---

    /// 执行事务
    async fn execute_transaction<F, T>(&self, f: F) -> StorageResult<T>
    where
        F: FnOnce() -> T + Send;

    // --- 统计 ---

    /// 获取节点总数
    async fn count_nodes(&self, graph_id: &str) -> StorageResult<usize>;

    /// 获取边总数
    async fn count_edges(&self, graph_id: &str) -> StorageResult<usize>;
}
