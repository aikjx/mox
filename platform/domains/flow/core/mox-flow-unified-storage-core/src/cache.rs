// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 缓存层 — LRU 缓存装饰器
//!
//! 在存储后端之上增加缓存层，提升热点数据的访问速度。
//! 采用 LRU 淘汰策略，支持 KV、图节点、对象元数据的缓存。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::error::StorageResult;
use crate::storage_trait::StorageBackend;
use crate::types::{
    EdgeDirection, GraphEdge, GraphNode, ListObjectsOptions, ListObjectsResult, ObjectMeta,
    RangeOptions, StorageStats, Value,
};

/// LRU 缓存条目
struct CacheEntry<V> {
    value: V,
    access_time: u64,
    size_bytes: usize,
}

/// 简单 LRU 缓存
struct LruCache<K, V> {
    data: HashMap<K, CacheEntry<V>>,
    max_entries: usize,
    max_bytes: usize,
    used_bytes: usize,
    access_counter: u64,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            data: HashMap::with_capacity(max_entries.min(256)),
            max_entries,
            max_bytes,
            used_bytes: 0,
            access_counter: 0,
        }
    }

    fn get<Q: ?Sized>(&mut self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        if let Some(entry) = self.data.get_mut(key) {
            self.access_counter += 1;
            entry.access_time = self.access_counter;
            Some(&entry.value)
        } else {
            None
        }
    }

    fn insert(&mut self, key: K, value: V, size_bytes: usize) {
        self.access_counter += 1;

        // 如果键已存在，先减去旧的大小
        if let Some(old) = self.data.remove(&key) {
            self.used_bytes -= old.size_bytes;
        }

        // 检查容量并驱逐
        while self.data.len() >= self.max_entries
            || (self.max_bytes > 0 && self.used_bytes + size_bytes > self.max_bytes)
        {
            if let Some((evict_key, _)) = self
                .data
                .iter()
                .min_by_key(|(_, v)| v.access_time)
                .map(|(k, v)| (k.clone(), v.access_time))
            {
                if let Some(evicted) = self.data.remove(&evict_key) {
                    self.used_bytes -= evicted.size_bytes;
                }
            } else {
                break;
            }
        }

        self.used_bytes += size_bytes;
        self.data.insert(
            key,
            CacheEntry {
                value,
                access_time: self.access_counter,
                size_bytes,
            },
        );
    }

    fn invalidate(&mut self, key: &K) -> bool {
        if let Some(entry) = self.data.remove(key) {
            self.used_bytes -= entry.size_bytes;
            true
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.used_bytes = 0;
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

/// 缓存存储后端
///
/// 在底层存储后端之上增加缓存层，加速热点数据访问。
pub struct CachedBackend {
    /// 底层存储
    inner: Arc<dyn StorageBackend>,
    /// KV 缓存
    kv_cache: Mutex<LruCache<String, Value>>,
    /// 节点缓存
    node_cache: Mutex<LruCache<String, GraphNode>>,
    /// 边缓存
    edge_cache: Mutex<LruCache<String, GraphEdge>>,
    /// 对象元数据缓存
    object_meta_cache: Mutex<LruCache<String, ObjectMeta>>,

    /// 缓存命中
    cache_hits: AtomicU64,
    /// 缓存未命中
    cache_misses: AtomicU64,
}

impl CachedBackend {
    /// 创建缓存后端
    pub fn new(inner: Arc<dyn StorageBackend>, max_entries: usize, max_bytes: usize) -> Self {
        Self {
            inner,
            kv_cache: Mutex::new(LruCache::new(max_entries, max_bytes / 3)),
            node_cache: Mutex::new(LruCache::new(max_entries, max_bytes / 3)),
            edge_cache: Mutex::new(LruCache::new(max_entries, max_bytes / 6)),
            object_meta_cache: Mutex::new(LruCache::new(max_entries, max_bytes / 6)),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    /// 创建默认配置的缓存后端
    pub fn with_default_capacity(inner: Arc<dyn StorageBackend>) -> Self {
        Self::new(inner, 10000, 100 * 1024 * 1024) // 10000 条目，100MB
    }

    fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// 缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 清除所有缓存
    pub fn clear_cache(&self) {
        self.kv_cache.lock().clear();
        self.node_cache.lock().clear();
        self.edge_cache.lock().clear();
        self.object_meta_cache.lock().clear();
    }
}

#[async_trait]
impl StorageBackend for CachedBackend {
    // === KV 操作（带缓存）===

    async fn kv_get(&self, key: &str) -> StorageResult<Option<Value>> {
        // 先查缓存
        let cached = self.kv_cache.lock().get(key).cloned();
        if let Some(val) = cached {
            self.record_hit();
            return Ok(Some(val));
        }

        self.record_miss();
        let result = self.inner.kv_get(key).await?;

        if let Some(val) = &result {
            let size = key.len() + val.estimated_size();
            self.kv_cache.lock().insert(key.to_string(), val.clone(), size);
        }

        Ok(result)
    }

    async fn kv_put(&self, key: &str, value: Value) -> StorageResult<()> {
        // 写穿透：先写底层，再更新缓存
        self.inner.kv_put(key, value.clone()).await?;
        let size = key.len() + value.estimated_size();
        self.kv_cache.lock().insert(key.to_string(), value, size);
        Ok(())
    }

    async fn kv_delete(&self, key: &str) -> StorageResult<bool> {
        let result = self.inner.kv_delete(key).await?;
        self.kv_cache.lock().invalidate(&key.to_string());
        Ok(result)
    }

    async fn kv_scan(&self, options: RangeOptions) -> StorageResult<Vec<(String, Value)>> {
        // 范围查询不走缓存，直接查底层
        self.inner.kv_scan(options).await
    }

    // === 图操作（节点/边带缓存）===

    async fn graph_get_node(&self, node_id: &str) -> StorageResult<Option<GraphNode>> {
        let cached = self.node_cache.lock().get(node_id).cloned();
        if let Some(node) = cached {
            self.record_hit();
            return Ok(Some(node));
        }

        self.record_miss();
        let result = self.inner.graph_get_node(node_id).await?;

        if let Some(node) = &result {
            let size = node.estimated_size();
            self.node_cache
                .lock()
                .insert(node_id.to_string(), node.clone(), size);
        }

        Ok(result)
    }

    async fn graph_put_node(&self, node: GraphNode) -> StorageResult<()> {
        let node_id = node.id.clone();
        let size = node.estimated_size();
        self.inner.graph_put_node(node.clone()).await?;
        self.node_cache.lock().insert(node_id, node, size);
        Ok(())
    }

    async fn graph_delete_node(&self, node_id: &str) -> StorageResult<bool> {
        let result = self.inner.graph_delete_node(node_id).await?;
        self.node_cache.lock().invalidate(&node_id.to_string());
        // 节点删除会影响边，但边缓存失效较复杂，这里简化为清空边缓存
        // 实际生产中应该精确失效
        self.edge_cache.lock().clear();
        Ok(result)
    }

    async fn graph_get_edge(&self, edge_id: &str) -> StorageResult<Option<GraphEdge>> {
        let cached = self.edge_cache.lock().get(edge_id).cloned();
        if let Some(edge) = cached {
            self.record_hit();
            return Ok(Some(edge));
        }

        self.record_miss();
        let result = self.inner.graph_get_edge(edge_id).await?;

        if let Some(edge) = &result {
            let size = edge.estimated_size();
            self.edge_cache
                .lock()
                .insert(edge_id.to_string(), edge.clone(), size);
        }

        Ok(result)
    }

    async fn graph_put_edge(&self, edge: GraphEdge) -> StorageResult<()> {
        let edge_id = edge.id.clone();
        let size = edge.estimated_size();
        self.inner.graph_put_edge(edge.clone()).await?;
        self.edge_cache.lock().insert(edge_id, edge, size);
        Ok(())
    }

    async fn graph_delete_edge(&self, edge_id: &str) -> StorageResult<bool> {
        let result = self.inner.graph_delete_edge(edge_id).await?;
        self.edge_cache.lock().invalidate(&edge_id.to_string());
        Ok(result)
    }

    async fn graph_get_edges(
        &self,
        node_id: &str,
        direction: EdgeDirection,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>> {
        // 邻边查询不走单条缓存
        self.inner.graph_get_edges(node_id, direction, edge_type).await
    }

    async fn graph_list_nodes(&self, options: RangeOptions) -> StorageResult<Vec<GraphNode>> {
        self.inner.graph_list_nodes(options).await
    }

    async fn graph_list_edges(&self, options: RangeOptions) -> StorageResult<Vec<GraphEdge>> {
        self.inner.graph_list_edges(options).await
    }

    // === 对象操作（元数据带缓存）===

    async fn object_get(&self, key: &str) -> StorageResult<Option<(ObjectMeta, Vec<u8>)>> {
        // 对象数据太大，不缓存数据本身，只缓存元数据
        let result = self.inner.object_get(key).await?;

        if let Some((meta, data)) = &result {
            self.object_meta_cache
                .lock()
                .insert(key.to_string(), meta.clone(), meta.key.len() + 256);
            Ok(Some((meta.clone(), data.clone())))
        } else {
            Ok(None)
        }
    }

    async fn object_head(&self, key: &str) -> StorageResult<Option<ObjectMeta>> {
        let cached = self.object_meta_cache.lock().get(key).cloned();
        if let Some(meta) = cached {
            self.record_hit();
            return Ok(Some(meta));
        }

        self.record_miss();
        let result = self.inner.object_head(key).await?;

        if let Some(meta) = &result {
            self.object_meta_cache
                .lock()
                .insert(key.to_string(), meta.clone(), meta.key.len() + 256);
        }

        Ok(result)
    }

    async fn object_put(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> StorageResult<ObjectMeta> {
        let meta = self.inner.object_put(key, data, content_type).await?;
        self.object_meta_cache
            .lock()
            .insert(key.to_string(), meta.clone(), meta.key.len() + 256);
        Ok(meta)
    }

    async fn object_delete(&self, key: &str) -> StorageResult<bool> {
        let result = self.inner.object_delete(key).await?;
        self.object_meta_cache.lock().invalidate(&key.to_string());
        Ok(result)
    }

    async fn object_list(&self, options: ListObjectsOptions) -> StorageResult<ListObjectsResult> {
        self.inner.object_list(options).await
    }

    // === 通用操作 ===

    async fn stats(&self) -> StorageResult<StorageStats> {
        let mut stats = self.inner.stats().await?;
        stats.cache_hits = self.cache_hits.load(Ordering::Relaxed);
        stats.cache_misses = self.cache_misses.load(Ordering::Relaxed);
        Ok(stats)
    }

    async fn clear(&self) -> StorageResult<()> {
        self.clear_cache();
        self.inner.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_backend::MemoryBackend;

    #[tokio::test]
    async fn test_kv_cache() {
        let memory = Arc::new(MemoryBackend::new());
        let cached = CachedBackend::with_default_capacity(memory.clone());

        // 第一次读：miss
        assert!(cached.kv_get("key1").await.unwrap().is_none());

        // 写入
        cached.kv_put("key1", Value::from("value1")).await.unwrap();

        // 第二次读：hit
        let val = cached.kv_get("key1").await.unwrap().unwrap();
        assert_eq!(val.as_str(), Some("value1"));

        let stats = cached.stats().await.unwrap();
        assert!(stats.cache_hits >= 1);
        assert!(stats.cache_misses >= 1);
    }

    #[tokio::test]
    async fn test_node_cache() {
        let memory = Arc::new(MemoryBackend::new());
        let cached = CachedBackend::with_default_capacity(memory.clone());

        let node = GraphNode::new("n1").with_label("Test");
        cached.graph_put_node(node).await.unwrap();

        // 从缓存读
        let got = cached.graph_get_node("n1").await.unwrap().unwrap();
        assert_eq!(got.id, "n1");

        let rate = cached.hit_rate();
        assert!(rate >= 0.0);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let memory = Arc::new(MemoryBackend::new());
        let cached = CachedBackend::with_default_capacity(memory.clone());

        cached.kv_put("a", Value::from(1i64)).await.unwrap();
        cached.kv_put("b", Value::from(2i64)).await.unwrap();

        cached.clear_cache();

        // 缓存清空后，数据应该还在底层
        assert_eq!(
            memory.kv_get("a").await.unwrap().unwrap().as_int(),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_delete() {
        let memory = Arc::new(MemoryBackend::new());
        let cached = CachedBackend::with_default_capacity(memory.clone());

        cached.kv_put("key", Value::from("val")).await.unwrap();
        assert!(cached.kv_exists("key").await.unwrap());

        cached.kv_delete("key").await.unwrap();
        assert!(!cached.kv_exists("key").await.unwrap());
    }
}
