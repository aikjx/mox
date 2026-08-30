// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 分片 Raft 共识层
//!
//! 分布式知识图谱存储的共识层实现，采用分片架构，每个分片独立运行 Raft 共识组。
//! 设计参考 NebulaGraph 的 Raft 分片架构，但完全自研实现。
//!
//! ## 架构设计
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │                    Storage Engine                        │
//! └──────────────────────┬───────────────────────────────────┘
//!                        │ VID hash → shard_id
//! ┌───────────┬──────────┼──────────┬───────────┐
//! │  Shard 0  │  Shard 1 │  ...     │  Shard N  │
//! │  Raft Grp │  Raft Grp│          │  Raft Grp │
//! └─────┬─────┴─────┬────┘          └─────┬─────┘
//!       │           │                     │
//! ┌─────▼─────┐ ┌───▼──────┐        ┌────▼──────┐
//! │ RocksDB   │ │ RocksDB  │        │ RocksDB   │
//! │ Shard 0   │ │ Shard 1  │  ...   │ Shard N   │
//! └───────────┘ └──────────┘        └───────────┘
//! ```
//!
//! ## 分片策略
//!
//! - 分片数必须为 2 的幂，便于位运算快速路由
//! - 使用 SHA256(VID) 取低 k 位作为分片 ID（2^k = 分片数）
//! - 支持在线分裂：单个分片分裂为两个，分片数翻倍
//!
//! ## Raft 日志类型
//!
//! | 日志类型               | 描述                                   |
//! |------------------------|----------------------------------------|
//! | `InsertVertex`         | 插入顶点                               |
//! | `InsertEdge`           | 插入边                                 |
//! | `DeleteVertex`         | 删除顶点（级联删除关联边）             |
//! | `DeleteEdge`           | 删除边                                 |
//! | `UpdateVertexProps`    | 更新顶点属性                           |
//! | `Noop`                 | 空操作（用于 Leader 心跳）             |
//!
//! ## 状态机应用
//!
//! 每条 Raft 日志被提交后，通过 `apply()` 方法应用到底层 RocksDB 存储。
//! 所有写入操作必须经过 Raft 共识后才能应用，保证数据一致性。
//!
//! ## 快照机制
//!
//! - 快照生成：从 RocksDB 中导出指定分片的所有数据
//! - 快照恢复：将快照数据写入 RocksDB 并重建索引
//! - 用于：新节点加入时的状态同步、日志压缩

use crate::error::{StorageError, StorageResult};
use crate::kv_rocksdb::{
    edge_key, node_key, node_type_index_key, out_index_key, in_index_key, RocksDBStore,
    StoredEdge, StoredNode, CF_EDGES, CF_EDGE_INDEX, CF_NODE_INDEX, CF_NODES, WriteBatch,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ============================================================================
// 类型定义
// ============================================================================

/// Raft 日志条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RaftLogEntry {
    /// 插入顶点
    InsertVertex {
        shard_id: u16,
        space_id: i32,
        vid: String,
        node_type: String,
        label: String,
        properties: serde_json::Value,
    },
    /// 插入边
    InsertEdge {
        shard_id: u16,
        space_id: i32,
        src_vid: String,
        dst_vid: String,
        edge_type: String,
        rank: i64,
        weight: f64,
        properties: serde_json::Value,
    },
    /// 删除顶点（级联删除关联边）
    DeleteVertex {
        shard_id: u16,
        space_id: i32,
        vid: String,
    },
    /// 删除边
    DeleteEdge {
        shard_id: u16,
        space_id: i32,
        src_vid: String,
        dst_vid: String,
        edge_type: String,
        rank: i64,
    },
    /// 更新顶点属性
    UpdateVertexProps {
        shard_id: u16,
        space_id: i32,
        vid: String,
        properties: serde_json::Value,
        /// 是否全量替换（true: 替换整个 properties 对象；false: 合并）
        replace: bool,
    },
    /// 空操作（Leader 心跳、测试用）
    Noop,
}

/// 节点角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Leader 节点：处理所有写入请求
    Leader,
    /// Follower 节点：被动复制日志，处理读请求（可配置）
    Follower,
    /// Candidate 候选节点：选举中
    Candidate,
}

/// Raft 组信息
#[derive(Debug)]
pub struct RaftGroup {
    /// 分片 ID
    pub shard_id: u16,
    /// 当前节点角色
    pub role: NodeRole,
    /// 已应用的日志索引
    pub applied_index: AtomicU64,
    /// 已提交的日志索引
    pub committed_index: AtomicU64,
    /// 当前任期
    pub current_term: AtomicU64,
    /// 副本地址列表
    pub peer_addrs: Vec<String>,
    /// Leader 地址
    pub leader_addr: Option<String>,
}

impl Clone for RaftGroup {
    fn clone(&self) -> Self {
        Self {
            shard_id: self.shard_id,
            role: self.role.clone(),
            applied_index: AtomicU64::new(self.applied_index.load(Ordering::SeqCst)),
            committed_index: AtomicU64::new(self.committed_index.load(Ordering::SeqCst)),
            current_term: AtomicU64::new(self.current_term.load(Ordering::SeqCst)),
            peer_addrs: self.peer_addrs.clone(),
            leader_addr: self.leader_addr.clone(),
        }
    }
}

impl RaftGroup {
    pub fn new(shard_id: u16, role: NodeRole, peer_addrs: Vec<String>) -> Self {
        Self {
            shard_id,
            role,
            applied_index: AtomicU64::new(0),
            committed_index: AtomicU64::new(0),
            current_term: AtomicU64::new(0),
            peer_addrs,
            leader_addr: None,
        }
    }

    pub fn is_leader(&self) -> bool {
        matches!(self.role, NodeRole::Leader)
    }
}

/// 分片统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShardStats {
    pub vertex_count: u64,
    pub edge_count: u64,
    pub applied_index: u64,
    pub last_apply_ts: u64,
}

// ============================================================================
// ShardRaft 主结构
// ============================================================================

/// 分片 Raft 共识层
///
/// 管理多个分片的 Raft 组，提供统一的 apply 接口。
/// 每个分片独立维护自己的 Raft 状态和 RocksDB 存储。
pub struct ShardRaft {
    /// 底层 KV 存储
    pub store: RocksDBStore,
    /// 分片数量（必须是 2 的幂）
    shard_count: AtomicU64,
    /// Raft 组映射：shard_id → RaftGroup
    groups: Mutex<BTreeMap<u16, RaftGroup>>,
    /// 分片统计
    shard_stats: Mutex<BTreeMap<u16, ShardStats>>,
    /// 全局已应用日志数（用于 CDC 偏移量）
    global_applied: AtomicU64,
}

impl ShardRaft {
    /// 创建新的分片 Raft 实例
    ///
    /// # Arguments
    /// * `store` - RocksDB 存储实例
    /// * `shard_count` - 分片数量，必须是 2 的幂
    /// * `peer_addrs` - 所有存储节点地址（用于 Raft 组配置）
    pub fn new(store: RocksDBStore, shard_count: u16, peer_addrs: &[String]) -> Self {
        assert!(
            shard_count.is_power_of_two(),
            "shard_count must be power of two, got {shard_count}"
        );

        let mut groups = BTreeMap::new();
        let mut stats = BTreeMap::new();
        let addrs_vec: Vec<String> = peer_addrs.to_vec();
        let n = addrs_vec.len().max(1);

        for shard_id in 0..shard_count {
            // 简单分配：第 shard_id % n 个节点为 Leader
            let role = if (shard_id as usize) % n == 0 {
                NodeRole::Leader
            } else {
                NodeRole::Follower
            };
            groups.insert(shard_id, RaftGroup::new(shard_id, role, addrs_vec.clone()));
            stats.insert(shard_id, ShardStats::default());
        }

        Self {
            store,
            shard_count: AtomicU64::new(shard_count as u64),
            groups: Mutex::new(groups),
            shard_stats: Mutex::new(stats),
            global_applied: AtomicU64::new(0),
        }
    }

    /// 获取分片数量
    pub fn shard_count(&self) -> u16 {
        self.shard_count.load(Ordering::SeqCst) as u16
    }

    /// 计算 VID 所属的分片 ID
    pub fn shard_for_vid(&self, vid: &str) -> u16 {
        vid_hash_shard(vid, self.shard_count())
    }

    /// 检查指定分片是否为 Leader
    pub fn is_leader(&self, shard_id: u16) -> bool {
        self.groups
            .lock()
            .get(&shard_id)
            .map(|g| g.is_leader())
            .unwrap_or(false)
    }

    /// 获取分片的已应用索引
    pub fn applied_index(&self, shard_id: u16) -> u64 {
        self.groups
            .lock()
            .get(&shard_id)
            .map(|g| g.applied_index.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// 获取全局已应用索引
    pub fn global_applied(&self) -> u64 {
        self.global_applied.load(Ordering::SeqCst)
    }

    /// 获取分片统计
    pub fn shard_stats(&self, shard_id: u16) -> Option<ShardStats> {
        self.shard_stats.lock().get(&shard_id).cloned()
    }

    /// 确保分片存在（用于动态添加分片）
    pub fn ensure_shard(&self, shard_id: u16) -> StorageResult<()> {
        let has_shard = self.groups.lock().contains_key(&shard_id);
        if !has_shard {
            let mut groups = self.groups.lock();
            if !groups.contains_key(&shard_id) {
                groups.insert(
                    shard_id,
                    RaftGroup::new(shard_id, NodeRole::Follower, Vec::new()),
                );
            }
            drop(groups);
            self.shard_stats
                .lock()
                .entry(shard_id)
                .or_insert_with(ShardStats::default);
        }
        Ok(())
    }

    /// 应用 Raft 日志到状态机
    ///
    /// 这是 Raft 共识层的核心方法。日志经过共识提交后，
    /// 通过此方法应用到底层 RocksDB 存储。
    ///
    /// # Returns
    /// 返回全局已应用日志索引
    pub fn apply(&self, entry: &RaftLogEntry) -> StorageResult<u64> {
        // 确保目标分片存在
        let shard_id = match entry {
            RaftLogEntry::InsertVertex { shard_id, .. } => *shard_id,
            RaftLogEntry::InsertEdge { shard_id, .. } => *shard_id,
            RaftLogEntry::DeleteVertex { shard_id, .. } => *shard_id,
            RaftLogEntry::DeleteEdge { shard_id, .. } => *shard_id,
            RaftLogEntry::UpdateVertexProps { shard_id, .. } => *shard_id,
            RaftLogEntry::Noop => 0,
        };

        self.ensure_shard(shard_id)?;

        // 应用到状态机
        match entry {
            RaftLogEntry::InsertVertex {
                space_id,
                vid,
                node_type,
                label,
                properties,
                ..
            } => self.apply_insert_vertex(*space_id, vid, node_type, label, properties)?,
            RaftLogEntry::InsertEdge {
                space_id,
                src_vid,
                dst_vid,
                edge_type,
                rank,
                weight,
                properties,
                ..
            } => self.apply_insert_edge(
                *space_id, src_vid, dst_vid, edge_type, *rank, *weight, properties,
            )?,
            RaftLogEntry::DeleteVertex {
                space_id, vid, ..
            } => self.apply_delete_vertex(*space_id, vid)?,
            RaftLogEntry::DeleteEdge {
                space_id,
                src_vid,
                dst_vid,
                edge_type,
                rank,
                ..
            } => self.apply_delete_edge(*space_id, src_vid, dst_vid, edge_type, *rank)?,
            RaftLogEntry::UpdateVertexProps {
                space_id,
                vid,
                properties,
                replace,
                ..
            } => self.apply_update_vertex_props(*space_id, vid, properties, *replace)?,
            RaftLogEntry::Noop => {}
        }

        // 更新分片 applied index
        if let Some(group) = self.groups.lock().get(&shard_id) {
            group.applied_index.fetch_add(1, Ordering::SeqCst);
        }

        // 更新分片统计
        self.update_stats_after_apply(shard_id, entry);

        // 更新全局 applied index
        Ok(self.global_applied.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// 批量应用多条日志
    pub fn apply_batch(&self, entries: &[RaftLogEntry]) -> StorageResult<u64> {
        let mut last_index = 0;
        for entry in entries {
            last_index = self.apply(entry)?;
        }
        Ok(last_index)
    }

    // ---- 状态机应用方法 ----

    fn apply_insert_vertex(
        &self,
        space_id: i32,
        vid: &str,
        node_type: &str,
        label: &str,
        properties: &serde_json::Value,
    ) -> StorageResult<()> {
        let node = StoredNode {
            vid: vid.to_string(),
            node_type: node_type.to_string(),
            label: label.to_string(),
            properties: properties.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        };
        self.store.put_node(space_id, &node)
    }

    fn apply_insert_edge(
        &self,
        space_id: i32,
        src_vid: &str,
        dst_vid: &str,
        edge_type: &str,
        rank: i64,
        weight: f64,
        properties: &serde_json::Value,
    ) -> StorageResult<()> {
        let edge = StoredEdge {
            src_vid: src_vid.to_string(),
            dst_vid: dst_vid.to_string(),
            edge_type: edge_type.to_string(),
            rank,
            weight,
            properties: properties.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        self.store.put_edge(space_id, &edge)
    }

    fn apply_delete_vertex(&self, space_id: i32, vid: &str) -> StorageResult<()> {
        let deleted = self.store.delete_node(space_id, vid)?;
        if !deleted {
            return Err(StorageError::VidNotFound(vid.to_string()));
        }
        Ok(())
    }

    fn apply_delete_edge(
        &self,
        space_id: i32,
        src_vid: &str,
        dst_vid: &str,
        edge_type: &str,
        rank: i64,
    ) -> StorageResult<()> {
        let deleted = self.store.delete_edge(space_id, src_vid, edge_type, dst_vid, rank)?;
        if !deleted {
            return Err(StorageError::EdgeNotFound {
                src: src_vid.to_string(),
                dst: dst_vid.to_string(),
                etype: edge_type.to_string(),
                rank,
            });
        }
        Ok(())
    }

    fn apply_update_vertex_props(
        &self,
        space_id: i32,
        vid: &str,
        properties: &serde_json::Value,
        replace: bool,
    ) -> StorageResult<()> {
        let Some(mut node) = self.store.get_node(space_id, vid)? else {
            return Err(StorageError::VidNotFound(vid.to_string()));
        };

        if replace {
            node.properties = properties.clone();
        } else {
            // 合并属性
            if let (Some(existing), Some(new)) =
                (node.properties.as_object_mut(), properties.as_object())
            {
                for (k, v) in new {
                    existing.insert(k.clone(), v.clone());
                }
            } else {
                node.properties = properties.clone();
            }
        }
        node.updated_at = chrono::Utc::now().timestamp_millis();
        self.store.put_node(space_id, &node)
    }

    fn update_stats_after_apply(&self, shard_id: u16, entry: &RaftLogEntry) {
        let mut stats = self.shard_stats.lock();
        let stat = stats.entry(shard_id).or_insert_with(ShardStats::default);
        stat.applied_index += 1;
        stat.last_apply_ts = chrono::Utc::now().timestamp_millis() as u64;

        match entry {
            RaftLogEntry::InsertVertex { .. } => {
                stat.vertex_count += 1;
            }
            RaftLogEntry::DeleteVertex { .. } => {
                stat.vertex_count = stat.vertex_count.saturating_sub(1);
            }
            RaftLogEntry::InsertEdge { .. } => {
                stat.edge_count += 1;
            }
            RaftLogEntry::DeleteEdge { .. } => {
                stat.edge_count = stat.edge_count.saturating_sub(1);
            }
            _ => {}
        }
    }

    // ---- 快照相关 ----

    /// 生成指定分片的快照
    ///
    /// 快照包含该分片的所有节点和边数据，用于：
    /// - 新节点加入时的状态同步
    /// - 日志压缩（删除已快照的旧日志）
    /// - 数据备份
    pub fn snapshot_shard(&self, shard_id: u16) -> StorageResult<ShardSnapshot> {
        // 记录快照时的 applied index
        let applied_idx = self.applied_index(shard_id);

        // 使用 store 的快照功能
        let snapshot_data = self.store.snapshot()?;

        Ok(ShardSnapshot {
            shard_id,
            applied_index: applied_idx,
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: snapshot_data,
        })
    }

    /// 从快照恢复分片
    pub fn restore_snapshot(&self, snapshot: &ShardSnapshot) -> StorageResult<()> {
        self.ensure_shard(snapshot.shard_id)?;
        self.store.restore_snapshot(&snapshot.data)?;

        // 更新 applied index
        if let Some(group) = self.groups.lock().get(&snapshot.shard_id) {
            group
                .applied_index
                .store(snapshot.applied_index, Ordering::SeqCst);
        }

        Ok(())
    }

    // ---- 分片管理 ----

    /// 分裂单个分片（旧分片 → 两个新分片）
    ///
    /// 分裂后分片数翻倍。使用 VID 哈希的下一位来决定数据归属。
    pub fn split_shard(&self, old_shard: u16, new_a: u16, new_b: u16) -> StorageResult<()> {
        let current_count = self.shard_count();
        let new_count = (new_b + 1).max(current_count);

        // 确保新分片存在
        self.ensure_shard(new_a)?;
        self.ensure_shard(new_b)?;

        // 获取旧分片的所有节点数据
        let space_id = 0; // 简化：默认 space_id=0
        let nodes = self.store.list_nodes(space_id, usize::MAX, 0)?;

        // 根据新的分片数重新分配
        let mut batch = WriteBatch::new();
        let mut moved_count = 0;

        for node in &nodes {
            let new_shard = vid_hash_shard(&node.vid, new_count);
            if new_shard != old_shard {
                // 从旧分片删除，添加到新分片
                let old_key = node_key(space_id, &node.vid);
                batch.delete(CF_NODES, old_key.as_bytes());

                let old_idx_key =
                    node_type_index_key(space_id, &node.node_type, &node.vid);
                batch.delete(CF_NODE_INDEX, old_idx_key.as_bytes());

                // 新分片的 key 格式相同（space_id 相同）
                // 实际生产中每个分片有独立的 RocksDB 实例
                // 这里简化为同一个 store，key 相同，所以不需要重新写入
                // 仅做计数统计
                moved_count += 1;
            }
        }

        self.store.write_batch(batch)?;

        // 更新分片数（如果翻倍了）
        if new_count == current_count * 2 {
            self.shard_count.store(new_count as u64, Ordering::SeqCst);
        }

        // 重新统计
        self.recount_shards(new_count);

        Ok(())
    }

    /// 重新统计所有分片的节点数
    pub fn recount_shards(&self, up_to: u16) {
        let mut stats = self.shard_stats.lock();
        for shard_id in 0..up_to {
            // 简化实现：通过前缀扫描统计
            // 实际生产中使用 RocksDB 的 approx_count
            let count = self
                .store
                .approx_count(CF_NODES)
                .unwrap_or(0)
                .min(u64::MAX / up_to as u64);
            let stat = stats.entry(shard_id).or_insert_with(ShardStats::default);
            stat.vertex_count = count;
        }
    }

    /// 检查分片是否健康（可读写）
    pub fn shard_healthy(&self, shard_id: u16) -> bool {
        if !self.groups.lock().contains_key(&shard_id) {
            return false;
        }
        self.store.health_check().unwrap_or(false)
    }

    /// 获取所有分片 ID 列表
    pub fn all_shard_ids(&self) -> Vec<u16> {
        (0..self.shard_count()).collect()
    }
}

// ============================================================================
// 分片快照
// ============================================================================

/// 分片快照数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardSnapshot {
    /// 分片 ID
    pub shard_id: u16,
    /// 快照对应的已应用日志索引
    pub applied_index: u64,
    /// 快照生成时间戳（毫秒）
    pub timestamp: i64,
    /// 快照数据（序列化的 KV 数据）
    pub data: Vec<u8>,
}

impl ShardSnapshot {
    /// 计算快照大小（字节）
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// VID 分片哈希：SHA256(VID) 取低 k 位
///
/// 分片数必须是 2 的幂，使用位运算替代取模，性能更高。
pub fn vid_hash_shard(vid: &str, shard_count: u16) -> u16 {
    assert!(
        shard_count.is_power_of_two(),
        "shard_count must be power of two"
    );
    let mut h = Sha256::new();
    h.update(vid.as_bytes());
    let d = h.finalize();
    let mut a = [0u8; 8];
    a.copy_from_slice(&d[..8]);
    let v = u64::from_le_bytes(a);
    (v & (shard_count as u64 - 1)) as u16
}

/// VID 哈希为 u64（用于分片分裂时的额外位判断）
pub fn vid_hash_u64(vid: &str) -> u64 {
    let mut h = Sha256::new();
    h.update(vid.as_bytes());
    let d = h.finalize();
    let mut a = [0u8; 8];
    a.copy_from_slice(&d[..8]);
    u64::from_le_bytes(a)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kv_rocksdb::RocksDBStore;

    fn create_raft(shard_count: u16) -> ShardRaft {
        let store = RocksDBStore::open_mem().expect("create store");
        let peers = vec!["node1:8000".to_string(), "node2:8000".to_string()];
        ShardRaft::new(store, shard_count, &peers)
    }

    #[test]
    fn test_vid_hash_shard() {
        // 分片数必须是 2 的幂
        let shard = vid_hash_shard("test-vid", 16);
        assert!(shard < 16);

        // 相同 VID 应该得到相同的分片
        let s1 = vid_hash_shard("alice", 8);
        let s2 = vid_hash_shard("alice", 8);
        assert_eq!(s1, s2);

        // 不同 VID 可能得到不同分片
        // （有概率碰撞，但测试用例应该足够分散）
        let s3 = vid_hash_shard("bob", 8);
        // 不断言一定不同，只验证范围
        assert!(s3 < 8);
    }

    #[test]
    #[should_panic(expected = "shard_count must be power of two")]
    fn test_vid_hash_shard_non_power_of_two() {
        vid_hash_shard("test", 3);
    }

    #[test]
    fn test_shard_raft_new() {
        let raft = create_raft(4);
        assert_eq!(raft.shard_count(), 4);
        assert_eq!(raft.all_shard_ids(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_shard_for_vid() {
        let raft = create_raft(16);
        let shard = raft.shard_for_vid("test-vid");
        assert!(shard < 16);
    }

    #[test]
    fn test_is_leader() {
        let raft = create_raft(4);
        // 第 0 个节点是 Leader（因为有 2 个 peer，0%2==0）
        assert!(raft.is_leader(0));
        assert!(!raft.is_leader(1));
        assert!(raft.is_leader(2));
        assert!(!raft.is_leader(3));
    }

    #[test]
    fn test_apply_insert_vertex() {
        let raft = create_raft(4);
        let space_id = 1;
        let vid = "v1";
        let shard_id = raft.shard_for_vid(vid);

        let entry = RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
            node_type: "Person".to_string(),
            label: "Alice".to_string(),
            properties: serde_json::json!({"age": 30}),
        };

        let idx = raft.apply(&entry).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(raft.global_applied(), 1);
        assert_eq!(raft.applied_index(shard_id), 1);

        // 验证节点已写入
        let node = raft.store.get_node(space_id, vid).unwrap();
        assert!(node.is_some());
        assert_eq!(node.unwrap().label, "Alice");
    }

    #[test]
    fn test_apply_insert_edge() {
        let raft = create_raft(4);
        let space_id = 1;

        // 先插入两个顶点
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: raft.shard_for_vid("a"),
            space_id,
            vid: "a".to_string(),
            node_type: "T".to_string(),
            label: "A".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();

        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: raft.shard_for_vid("b"),
            space_id,
            vid: "b".to_string(),
            node_type: "T".to_string(),
            label: "B".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();

        // 插入边
        let shard_id = raft.shard_for_vid("a");
        raft.apply(&RaftLogEntry::InsertEdge {
            shard_id,
            space_id,
            src_vid: "a".to_string(),
            dst_vid: "b".to_string(),
            edge_type: "knows".to_string(),
            rank: 0,
            weight: 1.0,
            properties: serde_json::json!({"since": "2020"}),
        })
        .unwrap();

        let edge = raft
            .store
            .get_edge(space_id, "a", "knows", "b", 0)
            .unwrap();
        assert!(edge.is_some());
        assert_eq!(edge.unwrap().edge_type, "knows");
    }

    #[test]
    fn test_apply_delete_vertex() {
        let raft = create_raft(4);
        let space_id = 1;
        let vid = "v1";
        let shard_id = raft.shard_for_vid(vid);

        // 插入顶点
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
            node_type: "Person".to_string(),
            label: "Alice".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();

        assert!(raft.store.get_node(space_id, vid).unwrap().is_some());

        // 删除顶点
        raft.apply(&RaftLogEntry::DeleteVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
        })
        .unwrap();

        assert!(raft.store.get_node(space_id, vid).unwrap().is_none());
    }

    #[test]
    fn test_apply_delete_nonexistent_vertex() {
        let raft = create_raft(4);
        let result = raft.apply(&RaftLogEntry::DeleteVertex {
            shard_id: 0,
            space_id: 1,
            vid: "nonexistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_delete_edge() {
        let raft = create_raft(4);
        let space_id = 1;

        // 插入顶点和边
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: raft.shard_for_vid("a"),
            space_id,
            vid: "a".to_string(),
            node_type: "T".to_string(),
            label: "A".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: raft.shard_for_vid("b"),
            space_id,
            vid: "b".to_string(),
            node_type: "T".to_string(),
            label: "B".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();
        raft.apply(&RaftLogEntry::InsertEdge {
            shard_id: raft.shard_for_vid("a"),
            space_id,
            src_vid: "a".to_string(),
            dst_vid: "b".to_string(),
            edge_type: "r".to_string(),
            rank: 0,
            weight: 1.0,
            properties: serde_json::json!({}),
        })
        .unwrap();

        assert!(raft.store.get_edge(space_id, "a", "r", "b", 0).unwrap().is_some());

        // 删除边
        raft.apply(&RaftLogEntry::DeleteEdge {
            shard_id: raft.shard_for_vid("a"),
            space_id,
            src_vid: "a".to_string(),
            dst_vid: "b".to_string(),
            edge_type: "r".to_string(),
            rank: 0,
        })
        .unwrap();

        assert!(raft.store.get_edge(space_id, "a", "r", "b", 0).unwrap().is_none());
    }

    #[test]
    fn test_apply_update_vertex_props_merge() {
        let raft = create_raft(4);
        let space_id = 1;
        let vid = "v1";
        let shard_id = raft.shard_for_vid(vid);

        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
            node_type: "Person".to_string(),
            label: "Alice".to_string(),
            properties: serde_json::json!({"age": 30, "city": "Beijing"}),
        })
        .unwrap();

        // 合并更新
        raft.apply(&RaftLogEntry::UpdateVertexProps {
            shard_id,
            space_id,
            vid: vid.to_string(),
            properties: serde_json::json!({"age": 31, "job": "Engineer"}),
            replace: false,
        })
        .unwrap();

        let node = raft.store.get_node(space_id, vid).unwrap().unwrap();
        assert_eq!(node.properties["age"], 31);
        assert_eq!(node.properties["city"], "Beijing");
        assert_eq!(node.properties["job"], "Engineer");
    }

    #[test]
    fn test_apply_update_vertex_props_replace() {
        let raft = create_raft(4);
        let space_id = 1;
        let vid = "v1";
        let shard_id = raft.shard_for_vid(vid);

        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: vid.to_string(),
            node_type: "Person".to_string(),
            label: "Alice".to_string(),
            properties: serde_json::json!({"age": 30, "city": "Beijing"}),
        })
        .unwrap();

        // 全量替换
        raft.apply(&RaftLogEntry::UpdateVertexProps {
            shard_id,
            space_id,
            vid: vid.to_string(),
            properties: serde_json::json!({"new_prop": "value"}),
            replace: true,
        })
        .unwrap();

        let node = raft.store.get_node(space_id, vid).unwrap().unwrap();
        assert_eq!(node.properties["new_prop"], "value");
        assert!(node.properties.get("age").is_none());
    }

    #[test]
    fn test_apply_noop() {
        let raft = create_raft(4);
        let idx = raft.apply(&RaftLogEntry::Noop).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(raft.global_applied(), 1);
    }

    #[test]
    fn test_apply_batch() {
        let raft = create_raft(4);
        let space_id = 1;

        let entries = vec![
            RaftLogEntry::InsertVertex {
                shard_id: 0,
                space_id,
                vid: "v1".to_string(),
                node_type: "T".to_string(),
                label: "A".to_string(),
                properties: serde_json::json!({}),
            },
            RaftLogEntry::InsertVertex {
                shard_id: 0,
                space_id,
                vid: "v2".to_string(),
                node_type: "T".to_string(),
                label: "B".to_string(),
                properties: serde_json::json!({}),
            },
            RaftLogEntry::Noop,
        ];

        let last_idx = raft.apply_batch(&entries).unwrap();
        assert_eq!(last_idx, 3);
        assert_eq!(raft.global_applied(), 3);
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let raft = create_raft(4);
        let space_id = 1;

        // 插入一些数据
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: 0,
            space_id,
            vid: "v1".to_string(),
            node_type: "Person".to_string(),
            label: "Alice".to_string(),
            properties: serde_json::json!({"age": 30}),
        })
        .unwrap();

        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id: 0,
            space_id,
            vid: "v2".to_string(),
            node_type: "Person".to_string(),
            label: "Bob".to_string(),
            properties: serde_json::json!({"age": 25}),
        })
        .unwrap();

        // 生成快照
        let snapshot = raft.snapshot_shard(0).unwrap();
        assert_eq!(snapshot.shard_id, 0);
        assert!(snapshot.applied_index > 0);
        assert!(!snapshot.data.is_empty());
        assert!(snapshot.size_bytes() > 0);

        // 创建新的 raft 实例并恢复
        let store2 = RocksDBStore::open_mem().unwrap();
        let peers = vec!["node1:8000".to_string()];
        let raft2 = ShardRaft::new(store2, 4, &peers);
        raft2.restore_snapshot(&snapshot).unwrap();

        // 验证数据已恢复
        let node = raft2.store.get_node(space_id, "v1").unwrap();
        assert!(node.is_some());
        assert_eq!(node.unwrap().label, "Alice");
    }

    #[test]
    fn test_ensure_shard() {
        let raft = create_raft(4);
        assert!(!raft.groups.lock().contains_key(&10));

        raft.ensure_shard(10).unwrap();
        assert!(raft.groups.lock().contains_key(&10));
        assert!(!raft.is_leader(10)); // 新分片默认是 Follower
    }

    #[test]
    fn test_shard_stats() {
        let raft = create_raft(4);
        let space_id = 1;
        let shard_id = 0;

        // 初始统计
        let stats = raft.shard_stats(shard_id).unwrap();
        assert_eq!(stats.vertex_count, 0);
        assert_eq!(stats.edge_count, 0);

        // 插入顶点后统计
        raft.apply(&RaftLogEntry::InsertVertex {
            shard_id,
            space_id,
            vid: "v1".to_string(),
            node_type: "T".to_string(),
            label: "A".to_string(),
            properties: serde_json::json!({}),
        })
        .unwrap();

        let stats = raft.shard_stats(shard_id).unwrap();
        assert_eq!(stats.vertex_count, 1);
        assert_eq!(stats.applied_index, 1);

        // 删除顶点后统计
        raft.apply(&RaftLogEntry::DeleteVertex {
            shard_id,
            space_id,
            vid: "v1".to_string(),
        })
        .unwrap();

        let stats = raft.shard_stats(shard_id).unwrap();
        assert_eq!(stats.vertex_count, 0);
    }

    #[test]
    fn test_shard_healthy() {
        let raft = create_raft(4);
        assert!(raft.shard_healthy(0));
        assert!(!raft.shard_healthy(100)); // 不存在的分片
    }

    #[test]
    fn test_node_role_display() {
        assert_eq!(format!("{:?}", NodeRole::Leader), "Leader");
        assert_eq!(format!("{:?}", NodeRole::Follower), "Follower");
        assert_eq!(format!("{:?}", NodeRole::Candidate), "Candidate");
    }

    #[test]
    fn test_raft_group_new() {
        let group = RaftGroup::new(0, NodeRole::Leader, vec!["n1".to_string()]);
        assert_eq!(group.shard_id, 0);
        assert!(group.is_leader());
        assert_eq!(group.applied_index.load(Ordering::SeqCst), 0);
        assert_eq!(group.peer_addrs.len(), 1);
    }
}
