// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 内存存储后端
//!
//! 基于内存的存储实现，用于测试和缓存场景。
//! 支持 KV、图、对象三种数据模型。

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::StorageResult;
use crate::storage_trait::StorageBackend;
use crate::types::{
    EdgeDirection, GraphEdge, GraphNode, ListObjectsOptions, ListObjectsResult, ObjectMeta,
    RangeOptions, StorageStats, Value,
};

/// 内存存储后端
pub struct MemoryBackend {
    /// KV 存储
    kv_store: RwLock<BTreeMap<String, Value>>,
    /// 图节点存储
    nodes: RwLock<BTreeMap<String, GraphNode>>,
    /// 图边存储
    edges: RwLock<BTreeMap<String, GraphEdge>>,
    /// 出边索引：node_id -> Vec<edge_id>
    out_edges: RwLock<BTreeMap<String, Vec<String>>>,
    /// 入边索引：node_id -> Vec<edge_id>
    in_edges: RwLock<BTreeMap<String, Vec<String>>>,
    /// 对象存储
    objects: RwLock<BTreeMap<String, (ObjectMeta, Vec<u8>)>>,

    // 统计
    total_reads: AtomicU64,
    total_writes: AtomicU64,
    total_deletes: AtomicU64,
    capacity: u64,
}

impl MemoryBackend {
    /// 创建新的内存存储后端
    pub fn new() -> Self {
        Self {
            kv_store: RwLock::new(BTreeMap::new()),
            nodes: RwLock::new(BTreeMap::new()),
            edges: RwLock::new(BTreeMap::new()),
            out_edges: RwLock::new(BTreeMap::new()),
            in_edges: RwLock::new(BTreeMap::new()),
            objects: RwLock::new(BTreeMap::new()),
            total_reads: AtomicU64::new(0),
            total_writes: AtomicU64::new(0),
            total_deletes: AtomicU64::new(0),
            capacity: u64::MAX,
        }
    }

    /// 设置容量限制
    pub fn with_capacity(mut self, capacity_bytes: u64) -> Self {
        self.capacity = capacity_bytes;
        self
    }

    /// 估算已用空间
    fn used_bytes(&self) -> u64 {
        let kv_size: u64 = self
            .kv_store
            .read()
            .iter()
            .map(|(k, v)| (k.len() + v.estimated_size()) as u64)
            .sum();
        let node_size: u64 = self
            .nodes
            .read()
            .values()
            .map(|n| n.estimated_size() as u64)
            .sum();
        let edge_size: u64 = self
            .edges
            .read()
            .values()
            .map(|e| e.estimated_size() as u64)
            .sum();
        let obj_size: u64 = self
            .objects
            .read()
            .values()
            .map(|(meta, data)| (meta.size + meta.key.len() as u64) + data.len() as u64)
            .sum();
        kv_size + node_size + edge_size + obj_size
    }

    fn apply_range_filter(
        entries: Vec<(String, Value)>,
        options: &RangeOptions,
    ) -> Vec<(String, Value)> {
        let mut result = entries;

        // 前缀过滤
        if let Some(prefix) = &options.prefix {
            result.retain(|(k, _)| k.starts_with(prefix));
        }

        // 起始键过滤
        if let Some(start) = &options.start {
            result.retain(|(k, _)| k >= start);
        }

        // 结束键过滤
        if let Some(end) = &options.end {
            result.retain(|(k, _)| k <= end);
        }

        // 反向
        if options.reverse {
            result.reverse();
        }

        // 限制数量
        if let Some(limit) = options.limit {
            result.truncate(limit);
        }

        result
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    // === KV 操作 ===

    async fn kv_get(&self, key: &str) -> StorageResult<Option<Value>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.kv_store.read().get(key).cloned())
    }

    async fn kv_put(&self, key: &str, value: Value) -> StorageResult<()> {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        self.kv_store
            .write()
            .insert(key.to_string(), value);
        Ok(())
    }

    async fn kv_delete(&self, key: &str) -> StorageResult<bool> {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);
        Ok(self.kv_store.write().remove(key).is_some())
    }

    async fn kv_scan(&self, options: RangeOptions) -> StorageResult<Vec<(String, Value)>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        let store = self.kv_store.read();
        let entries: Vec<(String, Value)> = store
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Self::apply_range_filter(entries, &options))
    }

    // === 图操作 ===

    async fn graph_get_node(&self, node_id: &str) -> StorageResult<Option<GraphNode>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.nodes.read().get(node_id).cloned())
    }

    async fn graph_put_node(&self, node: GraphNode) -> StorageResult<()> {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        self.nodes.write().insert(node.id.clone(), node);
        Ok(())
    }

    async fn graph_delete_node(&self, node_id: &str) -> StorageResult<bool> {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);

        // 删除关联的边
        let mut edges_to_delete = Vec::new();

        if let Some(out_edges) = self.out_edges.read().get(node_id) {
            edges_to_delete.extend(out_edges.clone());
        }
        if let Some(in_edges) = self.in_edges.read().get(node_id) {
            edges_to_delete.extend(in_edges.clone());
        }

        for edge_id in &edges_to_delete {
            self.edges.write().remove(edge_id);
        }

        self.out_edges.write().remove(node_id);
        self.in_edges.write().remove(node_id);

        Ok(self.nodes.write().remove(node_id).is_some())
    }

    async fn graph_get_edge(&self, edge_id: &str) -> StorageResult<Option<GraphEdge>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.edges.read().get(edge_id).cloned())
    }

    async fn graph_put_edge(&self, edge: GraphEdge) -> StorageResult<()> {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        let edge_id = edge.id.clone();
        let src_id = edge.src_id.clone();
        let dst_id = edge.dst_id.clone();

        self.edges.write().insert(edge_id.clone(), edge);

        // 更新索引
        self.out_edges
            .write()
            .entry(src_id)
            .or_default()
            .push(edge_id.clone());
        self.in_edges
            .write()
            .entry(dst_id)
            .or_default()
            .push(edge_id);

        Ok(())
    }

    async fn graph_delete_edge(&self, edge_id: &str) -> StorageResult<bool> {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);

        if let Some(edge) = self.edges.write().remove(edge_id) {
            // 从索引中移除
            if let Some(out_list) = self.out_edges.write().get_mut(&edge.src_id) {
                out_list.retain(|e| e != edge_id);
            }
            if let Some(in_list) = self.in_edges.write().get_mut(&edge.dst_id) {
                in_list.retain(|e| e != edge_id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn graph_get_edges(
        &self,
        node_id: &str,
        direction: EdgeDirection,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        let edges_store = self.edges.read();
        let mut result = Vec::new();

        let collect_edges = |edge_ids: &[String]| -> Vec<GraphEdge> {
            edge_ids
                .iter()
                .filter_map(|id| edges_store.get(id).cloned())
                .filter(|e| {
                    if let Some(et) = edge_type {
                        e.edge_type == et
                    } else {
                        true
                    }
                })
                .collect()
        };

        match direction {
            EdgeDirection::Out => {
                if let Some(ids) = self.out_edges.read().get(node_id) {
                    result = collect_edges(ids);
                }
            }
            EdgeDirection::In => {
                if let Some(ids) = self.in_edges.read().get(node_id) {
                    result = collect_edges(ids);
                }
            }
            EdgeDirection::Both => {
                if let Some(out_ids) = self.out_edges.read().get(node_id) {
                    result.extend(collect_edges(out_ids));
                }
                if let Some(in_ids) = self.in_edges.read().get(node_id) {
                    result.extend(collect_edges(in_ids));
                }
            }
        }

        Ok(result)
    }

    async fn graph_list_nodes(&self, options: RangeOptions) -> StorageResult<Vec<GraphNode>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        let store = self.nodes.read();
        let mut nodes: Vec<GraphNode> = store
            .iter()
            .filter(|(id, _)| {
                if let Some(prefix) = &options.prefix {
                    id.starts_with(prefix)
                } else {
                    true
                }
            })
            .filter(|(id, _)| {
                if let Some(start) = &options.start {
                    id.as_str() >= start.as_str()
                } else {
                    true
                }
            })
            .filter(|(id, _)| {
                if let Some(end) = &options.end {
                    id.as_str() <= end.as_str()
                } else {
                    true
                }
            })
            .map(|(_, node)| node.clone())
            .collect();

        if options.reverse {
            nodes.reverse();
        }
        if let Some(limit) = options.limit {
            nodes.truncate(limit);
        }

        Ok(nodes)
    }

    async fn graph_list_edges(&self, options: RangeOptions) -> StorageResult<Vec<GraphEdge>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        let store = self.edges.read();
        let mut edges: Vec<GraphEdge> = store
            .iter()
            .filter(|(id, _)| {
                if let Some(prefix) = &options.prefix {
                    id.starts_with(prefix)
                } else {
                    true
                }
            })
            .map(|(_, edge)| edge.clone())
            .collect();

        if options.reverse {
            edges.reverse();
        }
        if let Some(limit) = options.limit {
            edges.truncate(limit);
        }

        Ok(edges)
    }

    // === 对象操作 ===

    async fn object_get(&self, key: &str) -> StorageResult<Option<(ObjectMeta, Vec<u8>)>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self
            .objects
            .read()
            .get(key)
            .map(|(meta, data)| (meta.clone(), data.clone())))
    }

    async fn object_head(&self, key: &str) -> StorageResult<Option<ObjectMeta>> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.objects.read().get(key).map(|(meta, _)| meta.clone()))
    }

    async fn object_put(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> StorageResult<ObjectMeta> {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        let mut meta = ObjectMeta::new(key, data.len() as u64);
        if let Some(ct) = content_type {
            meta.content_type = ct.to_string();
        }
        meta.etag = format!("{:x}", md5_hash(&data));
        self.objects
            .write()
            .insert(key.to_string(), (meta.clone(), data));
        Ok(meta)
    }

    async fn object_delete(&self, key: &str) -> StorageResult<bool> {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);
        Ok(self.objects.write().remove(key).is_some())
    }

    async fn object_list(&self, options: ListObjectsOptions) -> StorageResult<ListObjectsResult> {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        let store = self.objects.read();
        let mut objects: Vec<ObjectMeta> = Vec::new();
        let mut common_prefixes: Vec<String> = Vec::new();
        let mut added_prefixes = std::collections::HashSet::new();

        for (key, (meta, _)) in store.iter() {
            // 前缀过滤
            if let Some(prefix) = &options.prefix {
                if !key.starts_with(prefix) {
                    continue;
                }
            }

            // marker 过滤
            if let Some(marker) = &options.marker {
                if key <= marker {
                    continue;
                }
            }

            // 分隔符处理（模拟目录）
            if let Some(delimiter) = &options.delimiter {
                let prefix_str = options.prefix.as_deref().unwrap_or("");
                let rest = &key[prefix_str.len()..];
                if let Some(pos) = rest.find(delimiter) {
                    let common_prefix = format!("{}{}", prefix_str, &rest[..=pos]);
                    if added_prefixes.insert(common_prefix.clone()) {
                        common_prefixes.push(common_prefix);
                    }
                    continue;
                }
            }

            objects.push(meta.clone());

            if objects.len() >= options.max_keys {
                break;
            }
        }

        let is_truncated = objects.len() >= options.max_keys;
        let next_marker = if is_truncated {
            objects.last().map(|o| o.key.clone())
        } else {
            None
        };

        Ok(ListObjectsResult {
            objects,
            common_prefixes,
            is_truncated,
            next_marker,
        })
    }

    // === 通用操作 ===

    async fn stats(&self) -> StorageResult<StorageStats> {
        Ok(StorageStats {
            total_keys: self.kv_store.read().len() as u64,
            total_nodes: self.nodes.read().len() as u64,
            total_edges: self.edges.read().len() as u64,
            total_objects: self.objects.read().len() as u64,
            used_bytes: self.used_bytes(),
            capacity_bytes: self.capacity,
            read_ops: self.total_reads.load(Ordering::Relaxed),
            write_ops: self.total_writes.load(Ordering::Relaxed),
            delete_ops: self.total_deletes.load(Ordering::Relaxed),
            cache_hits: 0,
            cache_misses: 0,
        })
    }

    async fn clear(&self) -> StorageResult<()> {
        self.kv_store.write().clear();
        self.nodes.write().clear();
        self.edges.write().clear();
        self.out_edges.write().clear();
        self.in_edges.write().clear();
        self.objects.write().clear();
        self.total_reads.store(0, Ordering::Relaxed);
        self.total_writes.store(0, Ordering::Relaxed);
        self.total_deletes.store(0, Ordering::Relaxed);
        Ok(())
    }
}

/// 简单的 MD5 哈希（用于生成 ETag）
fn md5_hash(data: &[u8]) -> u128 {
    // 使用简单的 FNV 哈希替代 MD5，避免额外依赖
    let mut hash: u128 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kv_operations() {
        let store = MemoryBackend::new();

        assert!(store.kv_get("key1").await.unwrap().is_none());
        assert!(!store.kv_exists("key1").await.unwrap());

        store.kv_put("key1", Value::from("value1")).await.unwrap();
        assert_eq!(
            store.kv_get("key1").await.unwrap().unwrap().as_str(),
            Some("value1")
        );
        assert!(store.kv_exists("key1").await.unwrap());

        store.kv_put("key2", Value::from(42i64)).await.unwrap();
        assert_eq!(store.kv_get("key2").await.unwrap().unwrap().as_int(), Some(42));

        assert!(store.kv_delete("key1").await.unwrap());
        assert!(!store.kv_exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_kv_scan() {
        let store = MemoryBackend::new();

        for i in 0..10 {
            store
                .kv_put(&format!("key_{:02}", i), Value::from(i as i64))
                .await
                .unwrap();
        }

        // 前缀扫描 - key_0 匹配 key_00 ~ key_09 共10个
        let result = store
            .kv_scan(RangeOptions::with_prefix("key_0"))
            .await
            .unwrap();
        assert_eq!(result.len(), 10);

        // 更精确的前缀
        let result = store
            .kv_scan(RangeOptions::with_prefix("key_00"))
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        // limit
        let result = store
            .kv_scan(RangeOptions::default().with_limit(5))
            .await
            .unwrap();
        assert_eq!(result.len(), 5);

        // 反向
        let result = store
            .kv_scan(RangeOptions {
                reverse: true,
                limit: Some(3),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "key_09");
    }

    #[tokio::test]
    async fn test_graph_operations() {
        let store = MemoryBackend::new();

        let node1 = GraphNode::new("node1").with_label("Person");
        let node2 = GraphNode::new("node2").with_label("Person");
        store.graph_put_node(node1).await.unwrap();
        store.graph_put_node(node2).await.unwrap();

        assert_eq!(store.graph_list_nodes(RangeOptions::default()).await.unwrap().len(), 2);

        let edge = GraphEdge::new("node1", "KNOWS", "node2").with_weight(0.8);
        store.graph_put_edge(edge).await.unwrap();

        let out_edges = store
            .graph_get_edges("node1", EdgeDirection::Out, None)
            .await
            .unwrap();
        assert_eq!(out_edges.len(), 1);
        assert_eq!(out_edges[0].edge_type, "KNOWS");

        let in_edges = store
            .graph_get_edges("node2", EdgeDirection::In, None)
            .await
            .unwrap();
        assert_eq!(in_edges.len(), 1);

        // 删除节点级联删除边
        store.graph_delete_node("node1").await.unwrap();
        assert!(!store.graph_node_exists("node1").await.unwrap());
        assert_eq!(store.graph_list_edges(RangeOptions::default()).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_object_operations() {
        let store = MemoryBackend::new();

        let data = b"hello world".to_vec();
        let meta = store
            .object_put("test.txt", data.clone(), Some("text/plain"))
            .await
            .unwrap();

        assert_eq!(meta.key, "test.txt");
        assert_eq!(meta.size, 11);
        assert_eq!(meta.content_type, "text/plain");

        let (got_meta, got_data) = store.object_get("test.txt").await.unwrap().unwrap();
        assert_eq!(got_data, data);
        assert_eq!(got_meta.size, 11);

        assert!(store.object_exists("test.txt").await.unwrap());
        assert!(!store.object_exists("nonexist.txt").await.unwrap());

        store.object_delete("test.txt").await.unwrap();
        assert!(!store.object_exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_object_list() {
        let store = MemoryBackend::new();

        store
            .object_put("dir1/file1.txt", vec![1, 2, 3], None)
            .await
            .unwrap();
        store
            .object_put("dir1/file2.txt", vec![4, 5], None)
            .await
            .unwrap();
        store
            .object_put("dir2/file3.txt", vec![6], None)
            .await
            .unwrap();

        let result = store
            .object_list(ListObjectsOptions {
                prefix: Some("dir1/".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(result.objects.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let store = MemoryBackend::new();

        store.kv_put("a", Value::from(1i64)).await.unwrap();
        store.kv_put("b", Value::from(2i64)).await.unwrap();

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.write_ops, 2);
        assert!(stats.used_bytes > 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let store = MemoryBackend::new();
        store.kv_put("a", Value::from(1i64)).await.unwrap();
        store.graph_put_node(GraphNode::new("n1")).await.unwrap();

        store.clear().await.unwrap();

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.total_nodes, 0);
    }
}
