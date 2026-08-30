// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 分布式存储引擎门面
//!
//! 对外提供统一的知识图谱存储接口，内部基于分片 Raft + RocksDB 实现分布式存储。
//!
//! ## 架构层次
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                  StorageEngine (本模块)                 │
//! │  - 统一 API 门面                                        │
//! │  - 分片路由 (VID hash → shard_id)                       │
//! │  - 多副本读写调度 (quorum)                               │
//! │  - CDC 事件发布                                         │
//! └──────────────┬──────────────────────┬───────────────────┘
//!                │                      │
//!    ┌───────────▼─────────┐  ┌────────▼──────────┐
//!    │   ShardRaft (分片)  │  │  CdcPublisher     │
//!    │   - Raft 共识       │  │  - 事件分发        │
//!    │   - 状态机应用      │  │  - offset 管理     │
//!    └───────────┬─────────┘  └───────────────────┘
//!                │
//!    ┌───────────▼─────────┐
//!    │   RocksDBStore      │
//!    │   - KV 存储引擎     │
//!    │   - 列族索引        │
//!    └─────────────────────┘
//! ```
//!
//! ## 分片路由策略
//!
//! 使用 SHA256(VID) 取低 k 位作为分片 ID，其中 2^k = 分片数量。
//! 这种方式的优点：
//! - 分布均匀：SHA256 哈希特性保证数据均匀分布
//! - 计算高效：位运算替代取模
//! - 易于扩展：分片数翻倍时，每个分片分裂为两个
//!
//! ## 读写一致性
//!
//! - **写入**：必须写入 Leader，经过 Raft 共识（多数派确认）后返回成功
//! - **读取**：
//!   - 强一致读：从 Leader 读取，保证读到最新数据
//!   - 最终一致读：从任意副本读取，可能有延迟但吞吐更高
//!
//! ## CDC 事件
//!
//! 所有写入操作都会产生 CDC 事件，通过 CdcPublisher 发布给订阅者。
//! 事件类型包括：VertexCreated、VertexUpdated、VertexDeleted、EdgeCreated、EdgeDeleted

use crate::cdc_publisher::{CdcEvent, CdcEventType, CdcPublisher};
use crate::error::{StorageError, StorageResult};
use crate::kv_rocksdb::{StoredEdge, StoredNode};
use crate::shard_raft::{RaftLogEntry, ShardRaft};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// 数据结构定义
// ============================================================================

/// 顶点数据结构（对外 API 使用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vertex {
    pub vid: String,
    pub tag: String,
    pub label: String,
    pub properties: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<StoredNode> for Vertex {
    fn from(node: StoredNode) -> Self {
        Self {
            vid: node.vid,
            tag: node.node_type,
            label: node.label,
            properties: node.properties,
            created_at: node.created_at,
            updated_at: node.updated_at,
        }
    }
}

/// 边数据结构（对外 API 使用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub src_vid: String,
    pub dst_vid: String,
    pub edge_type: String,
    pub rank: i64,
    pub weight: f64,
    pub properties: serde_json::Value,
    pub created_at: i64,
}

impl From<StoredEdge> for Edge {
    fn from(edge: StoredEdge) -> Self {
        Self {
            src_vid: edge.src_vid,
            dst_vid: edge.dst_vid,
            edge_type: edge.edge_type,
            rank: edge.rank,
            weight: edge.weight,
            properties: edge.properties,
            created_at: edge.created_at,
        }
    }
}

/// 邻居信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neighbor {
    pub vid: String,
    pub direction: Direction,
    pub edge_type: String,
    pub rank: i64,
    pub weight: f64,
    pub edge_properties: serde_json::Value,
    pub vertex: Option<Vertex>,
}

/// 边方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    /// 出边（从当前顶点出发）
    Out,
    /// 入边（指向当前顶点）
    In,
    /// 双向（出边 + 入边）
    Both,
}

/// 读取一致性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadConsistency {
    /// 强一致：必须从 Leader 读取
    Strong,
    /// 最终一致：可以从任意副本读取
    Eventual,
}

/// 写入确认级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteConsistency {
    /// 写入 Leader 内存即返回（最低延迟，可能丢数据）
    One,
    /// 多数派确认后返回（默认，保证数据安全）
    Quorum,
    /// 所有副本确认后返回（最高安全，延迟最高）
    All,
}

/// 批量写入结果
#[derive(Debug, Clone, Default)]
pub struct BatchResult {
    pub vertices_inserted: usize,
    pub edges_inserted: usize,
    pub vertices_deleted: usize,
    pub edges_deleted: usize,
    pub applied_index: u64,
}

/// 图遍历选项
#[derive(Debug, Clone)]
pub struct TraversalOptions {
    /// 边类型过滤（None 表示所有类型）
    pub edge_types: Option<Vec<String>>,
    /// 遍历方向
    pub direction: Direction,
    /// 最大跳数
    pub max_hops: usize,
    /// 每跳最大邻居数
    pub max_neighbors_per_hop: usize,
    /// 顶点标签过滤
    pub vertex_tag_filter: Option<Vec<String>>,
}

impl Default for TraversalOptions {
    fn default() -> Self {
        Self {
            edge_types: None,
            direction: Direction::Out,
            max_hops: 1,
            max_neighbors_per_hop: 1000,
            vertex_tag_filter: None,
        }
    }
}

// ============================================================================
// StorageEngine trait
// ============================================================================

/// 分布式存储引擎 trait
///
/// 定义知识图谱存储的核心接口。所有实现必须保证：
/// - 原子性：单个操作要么全部成功，要么全部失败
/// - 一致性：写入对所有读操作可见（取决于一致性级别）
/// - 持久性：已确认的数据不会丢失
pub trait StorageEngine: Send + Sync {
    /// 插入顶点
    fn insert_vertex(&self, space_id: i32, vertex: &Vertex) -> StorageResult<u64>;

    /// 插入边
    fn insert_edge(&self, space_id: i32, edge: &Edge) -> StorageResult<u64>;

    /// 查找顶点
    fn lookup_vertex(&self, space_id: i32, vid: &str) -> StorageResult<Option<Vertex>>;

    /// 获取邻居
    fn get_neighbors(
        &self,
        space_id: i32,
        vid: &str,
        direction: Direction,
        edge_types: Option<&[String]>,
        limit: usize,
    ) -> StorageResult<Vec<Neighbor>>;

    /// 图遍历（多跳邻居查询）
    fn go(
        &self,
        space_id: i32,
        start_vids: &[String],
        options: &TraversalOptions,
    ) -> StorageResult<Vec<Neighbor>>;

    /// 删除顶点
    fn delete_vertex(&self, space_id: i32, vid: &str) -> StorageResult<u64>;

    /// 删除边
    fn delete_edge(
        &self,
        space_id: i32,
        src: &str,
        dst: &str,
        edge_type: &str,
        rank: i64,
    ) -> StorageResult<u64>;

    /// 批量写入
    fn batch_write(
        &self,
        space_id: i32,
        vertices: &[Vertex],
        edges: &[Edge],
    ) -> StorageResult<BatchResult>;

    /// 按类型列出顶点
    fn list_vertices_by_tag(
        &self,
        space_id: i32,
        tag: &str,
        limit: usize,
        offset: usize,
    ) -> StorageResult<Vec<Vertex>>;

    /// 获取顶点数量（近似值）
    fn vertex_count(&self, space_id: i32) -> StorageResult<u64>;

    /// 获取边数量（近似值）
    fn edge_count(&self, space_id: i32) -> StorageResult<u64>;

    /// 健康检查
    fn health_check(&self) -> StorageResult<bool>;
}

// ============================================================================
// DistributedStorageEngine 实现
// ============================================================================

/// 分布式存储引擎实现
///
/// 基于分片 Raft + RocksDB 的分布式知识图谱存储引擎。
/// 支持千亿级数据规模，通过水平扩展提升容量和吞吐。
pub struct DistributedStorageEngine {
    /// 分片 Raft 层
    pub shard_raft: Arc<ShardRaft>,
    /// CDC 发布者
    pub cdc: Arc<CdcPublisher>,
    /// 写入一致性级别
    write_consistency: Mutex<WriteConsistency>,
    /// 读取一致性级别
    read_consistency: Mutex<ReadConsistency>,
    /// 分片节点映射（shard_id -> 节点地址列表）
    shard_nodes: Mutex<HashMap<u16, Vec<String>>>,
}

impl DistributedStorageEngine {
    /// 创建新的分布式存储引擎
    ///
    /// # Arguments
    /// * `shard_raft` - 分片 Raft 层实例
    /// * `cdc` - CDC 发布者实例
    pub fn new(shard_raft: Arc<ShardRaft>, cdc: Arc<CdcPublisher>) -> Self {
        let mut shard_nodes = HashMap::new();
        for shard_id in shard_raft.all_shard_ids() {
            // 初始化为空，实际集群中由服务发现填充
            shard_nodes.insert(shard_id, Vec::new());
        }

        Self {
            shard_raft,
            cdc,
            write_consistency: Mutex::new(WriteConsistency::Quorum),
            read_consistency: Mutex::new(ReadConsistency::Strong),
            shard_nodes: Mutex::new(shard_nodes),
        }
    }

    /// 设置写入一致性级别
    pub fn set_write_consistency(&self, level: WriteConsistency) {
        *self.write_consistency.lock() = level;
    }

    /// 设置读取一致性级别
    pub fn set_read_consistency(&self, level: ReadConsistency) {
        *self.read_consistency.lock() = level;
    }

    /// 获取分片数量
    pub fn shard_count(&self) -> u16 {
        self.shard_raft.shard_count()
    }

    /// 计算 VID 所属分片
    pub fn shard_for_vid(&self, vid: &str) -> u16 {
        self.shard_raft.shard_for_vid(vid)
    }

    /// 获取指定分片的 Leader 地址
    pub fn get_shard_leader(&self, shard_id: u16) -> Option<String> {
        // 简化实现：从本地 Raft 组获取
        // 实际集群中需要通过元数据服务获取
        None
    }

    /// 内部写入：构造 Raft 日志并应用
    fn apply_write(&self, entry: RaftLogEntry) -> StorageResult<u64> {
        // 提取 CDC 事件信息（在 apply 之前）
        let cdc_event = self.build_cdc_event(&entry);

        // 应用 Raft 日志
        let index = self.shard_raft.apply(&entry)?;

        // 发布 CDC 事件
        if let Some(event) = cdc_event {
            self.cdc.publish(event);
        }

        Ok(index)
    }

    /// 构建 CDC 事件
    fn build_cdc_event(&self, entry: &RaftLogEntry) -> Option<CdcEvent> {
        match entry {
            RaftLogEntry::InsertVertex {
                space_id, vid, ..
            } => Some(CdcEvent {
                event_type: CdcEventType::VertexCreated,
                space_id: *space_id,
                entity_id: vid.clone(),
                payload: serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raft_index: 0, // 由 publisher 填充
            }),
            RaftLogEntry::DeleteVertex {
                space_id, vid, ..
            } => Some(CdcEvent {
                event_type: CdcEventType::VertexDeleted,
                space_id: *space_id,
                entity_id: vid.clone(),
                payload: serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raft_index: 0,
            }),
            RaftLogEntry::UpdateVertexProps {
                space_id, vid, ..
            } => Some(CdcEvent {
                event_type: CdcEventType::VertexUpdated,
                space_id: *space_id,
                entity_id: vid.clone(),
                payload: serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raft_index: 0,
            }),
            RaftLogEntry::InsertEdge {
                space_id,
                src_vid,
                dst_vid,
                ..
            } => Some(CdcEvent {
                event_type: CdcEventType::EdgeCreated,
                space_id: *space_id,
                entity_id: format!("{}-{}", src_vid, dst_vid),
                payload: serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raft_index: 0,
            }),
            RaftLogEntry::DeleteEdge {
                space_id,
                src_vid,
                dst_vid,
                ..
            } => Some(CdcEvent {
                event_type: CdcEventType::EdgeDeleted,
                space_id: *space_id,
                entity_id: format!("{}-{}", src_vid, dst_vid),
                payload: serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                timestamp: chrono::Utc::now().timestamp_millis(),
                raft_index: 0,
            }),
            RaftLogEntry::Noop => None,
        }
    }

    // ---- 内部查询方法 ----

    /// 内部获取出边邻居
    fn get_out_neighbors_internal(
        &self,
        space_id: i32,
        vid: &str,
        edge_types: Option<&[String]>,
        limit: usize,
    ) -> StorageResult<Vec<Neighbor>> {
        let edges = self
            .shard_raft
            .store
            .get_out_neighbors(space_id, vid, None, limit)?;

        let mut neighbors = Vec::with_capacity(edges.len());
        for edge in edges {
            // 边类型过滤
            if let Some(types) = edge_types {
                if !types.contains(&edge.edge_type) {
                    continue;
                }
            }
            neighbors.push(Neighbor {
                vid: edge.dst_vid.clone(),
                direction: Direction::Out,
                edge_type: edge.edge_type,
                rank: edge.rank,
                weight: edge.weight,
                edge_properties: edge.properties,
                vertex: None, // 可选：查询顶点属性
            });
        }
        Ok(neighbors)
    }

    /// 内部获取入边邻居
    fn get_in_neighbors_internal(
        &self,
        space_id: i32,
        vid: &str,
        edge_types: Option<&[String]>,
        limit: usize,
    ) -> StorageResult<Vec<Neighbor>> {
        let edges = self
            .shard_raft
            .store
            .get_in_neighbors(space_id, vid, None, limit)?;

        let mut neighbors = Vec::with_capacity(edges.len());
        for edge in edges {
            if let Some(types) = edge_types {
                if !types.contains(&edge.edge_type) {
                    continue;
                }
            }
            neighbors.push(Neighbor {
                vid: edge.src_vid.clone(),
                direction: Direction::In,
                edge_type: edge.edge_type,
                rank: edge.rank,
                weight: edge.weight,
                edge_properties: edge.properties,
                vertex: None,
            });
        }
        Ok(neighbors)
    }
}

// ============================================================================
// StorageEngine trait 实现
// ============================================================================

impl StorageEngine for DistributedStorageEngine {
    fn insert_vertex(&self, space_id: i32, vertex: &Vertex) -> StorageResult<u64> {
        let shard_id = self.shard_for_vid(&vertex.vid);

        let entry = RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: vertex.vid.clone(),
            node_type: vertex.tag.clone(),
            label: vertex.label.clone(),
            properties: vertex.properties.clone(),
        };

        self.apply_write(entry)
    }

    fn insert_edge(&self, space_id: i32, edge: &Edge) -> StorageResult<u64> {
        // 边存储在源顶点所在的分片
        let shard_id = self.shard_for_vid(&edge.src_vid);

        let entry = RaftLogEntry::InsertEdge {
            shard_id,
            space_id,
            src_vid: edge.src_vid.clone(),
            dst_vid: edge.dst_vid.clone(),
            edge_type: edge.edge_type.clone(),
            rank: edge.rank,
            weight: edge.weight,
            properties: edge.properties.clone(),
        };

        self.apply_write(entry)
    }

    fn lookup_vertex(&self, space_id: i32, vid: &str) -> StorageResult<Option<Vertex>> {
        // 强一致读需要走 Leader，这里简化为直接从本地存储读取
        // 实际集群中需要根据一致性级别路由到对应节点
        let node = self.shard_raft.store.get_node(space_id, vid)?;
        Ok(node.map(Vertex::from))
    }

    fn get_neighbors(
        &self,
        space_id: i32,
        vid: &str,
        direction: Direction,
        edge_types: Option<&[String]>,
        limit: usize,
    ) -> StorageResult<Vec<Neighbor>> {
        match direction {
            Direction::Out => {
                self.get_out_neighbors_internal(space_id, vid, edge_types, limit)
            }
            Direction::In => {
                self.get_in_neighbors_internal(space_id, vid, edge_types, limit)
            }
            Direction::Both => {
                let per_limit = (limit + 1) / 2; // 均分 limit
                let mut out =
                    self.get_out_neighbors_internal(space_id, vid, edge_types, per_limit)?;
                let mut inn =
                    self.get_in_neighbors_internal(space_id, vid, edge_types, per_limit)?;
                out.append(&mut inn);
                if out.len() > limit {
                    out.truncate(limit);
                }
                Ok(out)
            }
        }
    }

    fn go(
        &self,
        space_id: i32,
        start_vids: &[String],
        options: &TraversalOptions,
    ) -> StorageResult<Vec<Neighbor>> {
        if options.max_hops == 0 || start_vids.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<Neighbor> = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut current_frontier: Vec<String> = start_vids.to_vec();

        for _hop in 0..options.max_hops {
            let mut next_frontier: Vec<String> = Vec::new();

            for vid in &current_frontier {
                if visited.contains(vid) {
                    continue;
                }
                visited.insert(vid.clone());

                let neighbors = self.get_neighbors(
                    space_id,
                    vid,
                    options.direction,
                    options.edge_types.as_deref(),
                    options.max_neighbors_per_hop,
                )?;

                for neighbor in &neighbors {
                    // 顶点标签过滤
                    if let Some(ref tags) = options.vertex_tag_filter {
                        // 如果有顶点信息则过滤，否则保留
                        if let Some(ref v) = neighbor.vertex {
                            if !tags.contains(&v.tag) {
                                continue;
                            }
                        }
                    }

                    if !visited.contains(&neighbor.vid) {
                        next_frontier.push(neighbor.vid.clone());
                    }
                    results.push(neighbor.clone());
                }
            }

            current_frontier = next_frontier;
            if current_frontier.is_empty() {
                break;
            }
        }

        Ok(results)
    }

    fn delete_vertex(&self, space_id: i32, vid: &str) -> StorageResult<u64> {
        let shard_id = self.shard_for_vid(vid);

        let entry = RaftLogEntry::DeleteVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
        };

        self.apply_write(entry)
    }

    fn delete_edge(
        &self,
        space_id: i32,
        src: &str,
        dst: &str,
        edge_type: &str,
        rank: i64,
    ) -> StorageResult<u64> {
        let shard_id = self.shard_for_vid(src);

        let entry = RaftLogEntry::DeleteEdge {
            shard_id,
            space_id,
            src_vid: src.to_string(),
            dst_vid: dst.to_string(),
            edge_type: edge_type.to_string(),
            rank,
        };

        self.apply_write(entry)
    }

    fn batch_write(
        &self,
        space_id: i32,
        vertices: &[Vertex],
        edges: &[Edge],
    ) -> StorageResult<BatchResult> {
        let mut result = BatchResult::default();

        // 按分片分组
        let mut vertex_by_shard: HashMap<u16, Vec<&Vertex>> = HashMap::new();
        for v in vertices {
            let shard_id = self.shard_for_vid(&v.vid);
            vertex_by_shard.entry(shard_id).or_default().push(v);
        }

        let mut edge_by_shard: HashMap<u16, Vec<&Edge>> = HashMap::new();
        for e in edges {
            let shard_id = self.shard_for_vid(&e.src_vid);
            edge_by_shard.entry(shard_id).or_default().push(e);
        }

        // 逐个分片应用
        let mut last_index = 0;
        for (shard_id, shard_vertices) in &vertex_by_shard {
            for v in shard_vertices {
                let entry = RaftLogEntry::InsertVertex {
                    shard_id: *shard_id,
                    space_id,
                    vid: v.vid.clone(),
                    node_type: v.tag.clone(),
                    label: v.label.clone(),
                    properties: v.properties.clone(),
                };
                last_index = self.apply_write(entry)?;
                result.vertices_inserted += 1;
            }
        }

        for (shard_id, shard_edges) in &edge_by_shard {
            for e in shard_edges {
                let entry = RaftLogEntry::InsertEdge {
                    shard_id: *shard_id,
                    space_id,
                    src_vid: e.src_vid.clone(),
                    dst_vid: e.dst_vid.clone(),
                    edge_type: e.edge_type.clone(),
                    rank: e.rank,
                    weight: e.weight,
                    properties: e.properties.clone(),
                };
                last_index = self.apply_write(entry)?;
                result.edges_inserted += 1;
            }
        }

        result.applied_index = last_index;
        Ok(result)
    }

    fn list_vertices_by_tag(
        &self,
        space_id: i32,
        tag: &str,
        limit: usize,
        offset: usize,
    ) -> StorageResult<Vec<Vertex>> {
        let nodes = self
            .shard_raft
            .store
            .list_nodes_by_type(space_id, tag, limit + offset)?;

        let vertices: Vec<Vertex> = nodes
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(Vertex::from)
            .collect();

        Ok(vertices)
    }

    fn vertex_count(&self, _space_id: i32) -> StorageResult<u64> {
        // 简化实现：使用 nodes CF 的近似数量
        // 实际生产中需要按 space_id 统计
        self.shard_raft.store.approx_count("nodes")
    }

    fn edge_count(&self, _space_id: i32) -> StorageResult<u64> {
        self.shard_raft.store.approx_count("edges")
    }

    fn health_check(&self) -> StorageResult<bool> {
        Ok(self.shard_raft.store.health_check().unwrap_or(false))
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdc_publisher::CdcPublisher;
    use crate::kv_rocksdb::RocksDBStore;
    use crate::shard_raft::ShardRaft;

    fn create_engine() -> DistributedStorageEngine {
        let store = RocksDBStore::open_mem().unwrap();
        let peers = vec!["node1:8000".to_string()];
        let shard_raft = Arc::new(ShardRaft::new(store, 4, &peers));
        let cdc = Arc::new(CdcPublisher::new("default_topic"));
        DistributedStorageEngine::new(shard_raft, cdc)
    }

    fn test_vertex(vid: &str, tag: &str, label: &str) -> Vertex {
        Vertex {
            vid: vid.to_string(),
            tag: tag.to_string(),
            label: label.to_string(),
            properties: serde_json::json!({}),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn test_edge(src: &str, dst: &str, etype: &str, rank: i64) -> Edge {
        Edge {
            src_vid: src.to_string(),
            dst_vid: dst.to_string(),
            edge_type: etype.to_string(),
            rank,
            weight: 1.0,
            properties: serde_json::json!({}),
            created_at: 0,
        }
    }

    #[test]
    fn test_insert_and_lookup_vertex() {
        let engine = create_engine();
        let space_id = 1;

        let v = test_vertex("alice", "Person", "Alice");
        let idx = engine.insert_vertex(space_id, &v).unwrap();
        assert!(idx > 0);

        let got = engine.lookup_vertex(space_id, "alice").unwrap();
        assert!(got.is_some());
        let got = got.unwrap();
        assert_eq!(got.vid, "alice");
        assert_eq!(got.tag, "Person");
        assert_eq!(got.label, "Alice");
    }

    #[test]
    fn test_lookup_nonexistent_vertex() {
        let engine = create_engine();
        let got = engine.lookup_vertex(1, "nonexistent").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn test_insert_and_get_neighbors() {
        let engine = create_engine();
        let space_id = 1;

        engine.insert_vertex(space_id, &test_vertex("a", "T", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("b", "T", "B")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("c", "T", "C")).unwrap();

        engine.insert_edge(space_id, &test_edge("a", "b", "knows", 0)).unwrap();
        engine.insert_edge(space_id, &test_edge("a", "c", "knows", 1)).unwrap();
        engine.insert_edge(space_id, &test_edge("a", "b", "likes", 0)).unwrap();

        // 出边邻居
        let out_neighbors = engine
            .get_neighbors(space_id, "a", Direction::Out, None, 10)
            .unwrap();
        assert_eq!(out_neighbors.len(), 3);

        // 按类型过滤
        let knows = engine
            .get_neighbors(
                space_id,
                "a",
                Direction::Out,
                Some(&["knows".to_string()]),
                10,
            )
            .unwrap();
        assert_eq!(knows.len(), 2);

        // 入边邻居
        let in_neighbors = engine
            .get_neighbors(space_id, "b", Direction::In, None, 10)
            .unwrap();
        assert_eq!(in_neighbors.len(), 2); // knows + likes
    }

    #[test]
    fn test_both_direction() {
        let engine = create_engine();
        let space_id = 1;

        engine.insert_vertex(space_id, &test_vertex("a", "T", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("b", "T", "B")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("c", "T", "C")).unwrap();

        engine.insert_edge(space_id, &test_edge("a", "b", "r", 0)).unwrap();
        engine.insert_edge(space_id, &test_edge("c", "b", "r", 0)).unwrap();

        let both = engine
            .get_neighbors(space_id, "b", Direction::Both, None, 10)
            .unwrap();
        // a 出边到 b（b 的入边），c 出边到 b（b 的入边）
        // 所以 b 的 Both 方向应该是 2 个入边
        assert_eq!(both.len(), 2);
    }

    #[test]
    fn test_delete_vertex() {
        let engine = create_engine();
        let space_id = 1;

        engine.insert_vertex(space_id, &test_vertex("v1", "T", "V1")).unwrap();
        assert!(engine.lookup_vertex(space_id, "v1").unwrap().is_some());

        let idx = engine.delete_vertex(space_id, "v1").unwrap();
        assert!(idx > 0);
        assert!(engine.lookup_vertex(space_id, "v1").unwrap().is_none());
    }

    #[test]
    fn test_delete_edge() {
        let engine = create_engine();
        let space_id = 1;

        engine.insert_vertex(space_id, &test_vertex("a", "T", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("b", "T", "B")).unwrap();
        engine.insert_edge(space_id, &test_edge("a", "b", "r", 0)).unwrap();

        let neighbors = engine
            .get_neighbors(space_id, "a", Direction::Out, None, 10)
            .unwrap();
        assert_eq!(neighbors.len(), 1);

        let idx = engine.delete_edge(space_id, "a", "b", "r", 0).unwrap();
        assert!(idx > 0);

        let neighbors = engine
            .get_neighbors(space_id, "a", Direction::Out, None, 10)
            .unwrap();
        assert_eq!(neighbors.len(), 0);
    }

    #[test]
    fn test_batch_write() {
        let engine = create_engine();
        let space_id = 1;

        let vertices = vec![
            test_vertex("a", "T", "A"),
            test_vertex("b", "T", "B"),
            test_vertex("c", "T", "C"),
        ];
        let edges = vec![
            test_edge("a", "b", "r", 0),
            test_edge("b", "c", "r", 0),
        ];

        let result = engine.batch_write(space_id, &vertices, &edges).unwrap();
        assert_eq!(result.vertices_inserted, 3);
        assert_eq!(result.edges_inserted, 2);
        assert!(result.applied_index > 0);
    }

    #[test]
    fn test_go_traversal() {
        let engine = create_engine();
        let space_id = 1;

        // 创建链: a -> b -> c -> d
        engine.insert_vertex(space_id, &test_vertex("a", "T", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("b", "T", "B")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("c", "T", "C")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("d", "T", "D")).unwrap();

        engine.insert_edge(space_id, &test_edge("a", "b", "r", 0)).unwrap();
        engine.insert_edge(space_id, &test_edge("b", "c", "r", 0)).unwrap();
        engine.insert_edge(space_id, &test_edge("c", "d", "r", 0)).unwrap();

        // 1 跳
        let opts = TraversalOptions {
            max_hops: 1,
            ..Default::default()
        };
        let result = engine.go(space_id, &["a".to_string()], &opts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vid, "b");

        // 2 跳
        let opts = TraversalOptions {
            max_hops: 2,
            ..Default::default()
        };
        let result = engine.go(space_id, &["a".to_string()], &opts).unwrap();
        assert_eq!(result.len(), 2); // b + c

        // 3 跳
        let opts = TraversalOptions {
            max_hops: 3,
            ..Default::default()
        };
        let result = engine.go(space_id, &["a".to_string()], &opts).unwrap();
        assert_eq!(result.len(), 3); // b + c + d
    }

    #[test]
    fn test_go_zero_hops() {
        let engine = create_engine();
        let opts = TraversalOptions {
            max_hops: 0,
            ..Default::default()
        };
        let result = engine.go(1, &["a".to_string()], &opts).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_vertices_by_tag() {
        let engine = create_engine();
        let space_id = 1;

        engine.insert_vertex(space_id, &test_vertex("v1", "Person", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("v2", "Person", "B")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("v3", "Company", "C")).unwrap();

        let persons = engine.list_vertices_by_tag(space_id, "Person", 10, 0).unwrap();
        assert_eq!(persons.len(), 2);

        let companies = engine.list_vertices_by_tag(space_id, "Company", 10, 0).unwrap();
        assert_eq!(companies.len(), 1);
    }

    #[test]
    fn test_vertex_and_edge_count() {
        let engine = create_engine();
        let space_id = 1;

        // 初始计数
        let v_count = engine.vertex_count(space_id).unwrap();
        let e_count = engine.edge_count(space_id).unwrap();
        assert!(v_count == 0);
        assert!(e_count == 0);

        // 插入数据
        engine.insert_vertex(space_id, &test_vertex("a", "T", "A")).unwrap();
        engine.insert_vertex(space_id, &test_vertex("b", "T", "B")).unwrap();
        engine.insert_edge(space_id, &test_edge("a", "b", "r", 0)).unwrap();

        // 注意：approx_count 可能不是精确值，但应该大于 0
        // 内存模式下是精确的
        let v_count = engine.vertex_count(space_id).unwrap();
        let e_count = engine.edge_count(space_id).unwrap();
        assert!(v_count >= 2);
        assert!(e_count >= 1);
    }

    #[test]
    fn test_health_check() {
        let engine = create_engine();
        assert!(engine.health_check().unwrap());
    }

    #[test]
    fn test_shard_for_vid() {
        let engine = create_engine();
        let shard = engine.shard_for_vid("test-vid");
        assert!(shard < engine.shard_count());
    }

    #[test]
    fn test_set_consistency() {
        let engine = create_engine();

        engine.set_write_consistency(WriteConsistency::One);
        assert_eq!(*engine.write_consistency.lock(), WriteConsistency::One);

        engine.set_read_consistency(ReadConsistency::Eventual);
        assert_eq!(
            *engine.read_consistency.lock(),
            ReadConsistency::Eventual
        );
    }

    #[test]
    fn test_direction_display() {
        assert_eq!(format!("{:?}", Direction::Out), "Out");
        assert_eq!(format!("{:?}", Direction::In), "In");
        assert_eq!(format!("{:?}", Direction::Both), "Both");
    }

    #[test]
    fn test_traversal_options_default() {
        let opts = TraversalOptions::default();
        assert_eq!(opts.max_hops, 1);
        assert_eq!(opts.direction, Direction::Out);
        assert_eq!(opts.max_neighbors_per_hop, 1000);
        assert!(opts.edge_types.is_none());
        assert!(opts.vertex_tag_filter.is_none());
    }

    #[test]
    fn test_batch_result_default() {
        let result = BatchResult::default();
        assert_eq!(result.vertices_inserted, 0);
        assert_eq!(result.edges_inserted, 0);
        assert_eq!(result.vertices_deleted, 0);
        assert_eq!(result.edges_deleted, 0);
        assert_eq!(result.applied_index, 0);
    }

    #[test]
    fn test_vertex_from_stored_node() {
        let node = StoredNode::new("v1", "Person", "Alice");
        let vertex = Vertex::from(node.clone());
        assert_eq!(vertex.vid, node.vid);
        assert_eq!(vertex.tag, node.node_type);
        assert_eq!(vertex.label, node.label);
    }

    #[test]
    fn test_edge_from_stored_edge() {
        let edge = StoredEdge::new("a", "b", "knows", 0);
        let e = Edge::from(edge.clone());
        assert_eq!(e.src_vid, edge.src_vid);
        assert_eq!(e.dst_vid, edge.dst_vid);
        assert_eq!(e.edge_type, edge.edge_type);
        assert_eq!(e.rank, edge.rank);
    }
}
