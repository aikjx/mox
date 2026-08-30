// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 统一存储引擎
//!
//! 将 KV、图、对象三种数据模型统一到一个引擎中，
//! 共享底层存储后端和缓存层。

use std::sync::Arc;

use crate::cache::CachedBackend;
use crate::error::StorageResult;
use crate::graph_store::GraphStore;
use crate::kv_store::KvStore;
use crate::memory_backend::MemoryBackend;
use crate::object_store::ObjectStore;
use crate::storage_trait::StorageBackend;
use crate::types::{StorageBackend as BackendType, StorageStats};

/// 统一存储引擎
///
/// 融合知识图谱存储与云盘对象存储，提供统一的访问接口。
/// 支持多种后端（内存、RocksDB、对象存储、混合）。
pub struct UnifiedStorageEngine {
    /// 后端存储（可能带缓存）
    backend: Arc<dyn StorageBackend>,
    /// 后端类型
    backend_type: BackendType,
    /// KV 存储接口
    pub kv: KvStore,
    /// 图存储接口
    pub graph: GraphStore,
    /// 对象存储接口
    pub object: ObjectStore,
    /// 是否启用了缓存
    cache_enabled: bool,
}

impl UnifiedStorageEngine {
    /// 创建内存存储引擎
    pub fn memory() -> Self {
        let backend = Arc::new(MemoryBackend::new());
        Self::with_backend(backend, BackendType::Memory)
    }

    /// 创建带缓存的内存存储引擎
    pub fn memory_with_cache() -> Self {
        let memory = Arc::new(MemoryBackend::new());
        let cached = Arc::new(CachedBackend::with_default_capacity(memory));
        Self {
            backend: cached.clone(),
            backend_type: BackendType::Memory,
            kv: KvStore::new(cached.clone()),
            graph: GraphStore::new(cached.clone()),
            object: ObjectStore::new(cached.clone()),
            cache_enabled: true,
        }
    }

    /// 使用自定义后端创建引擎
    pub fn with_backend(backend: Arc<dyn StorageBackend>, backend_type: BackendType) -> Self {
        Self {
            backend: backend.clone(),
            backend_type,
            kv: KvStore::new(backend.clone()),
            graph: GraphStore::new(backend.clone()),
            object: ObjectStore::new(backend),
            cache_enabled: false,
        }
    }

    /// 启用缓存
    pub fn with_cache(mut self, max_entries: usize, max_bytes: usize) -> Self {
        let cached = Arc::new(CachedBackend::new(self.backend.clone(), max_entries, max_bytes));
        self.backend = cached.clone();
        self.kv = KvStore::new(cached.clone());
        self.graph = GraphStore::new(cached.clone());
        self.object = ObjectStore::new(cached.clone());
        self.cache_enabled = true;
        self
    }

    /// 获取后端类型
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// 检查缓存是否启用
    pub fn is_cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    /// 获取统计信息
    pub async fn stats(&self) -> StorageResult<StorageStats> {
        self.backend.stats().await
    }

    /// 刷新数据到持久化存储
    pub async fn flush(&self) -> StorageResult<()> {
        self.backend.flush().await
    }

    /// 清空所有数据
    pub async fn clear(&self) -> StorageResult<()> {
        self.backend.clear().await
    }

    /// 关闭引擎
    pub async fn close(&self) -> StorageResult<()> {
        self.backend.close().await
    }
}

impl Default for UnifiedStorageEngine {
    fn default() -> Self {
        Self::memory_with_cache()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraphEdge, GraphNode, Value};

    #[tokio::test]
    async fn test_unified_engine_kv() {
        let engine = UnifiedStorageEngine::memory();

        engine.kv.put("key1", Value::from("value1")).await.unwrap();
        assert_eq!(
            engine.kv.get("key1").await.unwrap().as_str(),
            Some("value1")
        );
    }

    #[tokio::test]
    async fn test_unified_engine_graph() {
        let engine = UnifiedStorageEngine::memory();

        let node1 = GraphNode::new("alice").with_label("Person");
        let node2 = GraphNode::new("bob").with_label("Person");
        engine.graph.add_node(node1).await.unwrap();
        engine.graph.add_node(node2).await.unwrap();

        let edge = GraphEdge::new("alice", "KNOWS", "bob");
        engine.graph.add_edge(edge).await.unwrap();

        assert_eq!(engine.graph.node_count().await.unwrap(), 2);
        assert_eq!(engine.graph.edge_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_unified_engine_object() {
        let engine = UnifiedStorageEngine::memory();

        let data = b"unified storage test".to_vec();
        engine
            .object
            .put_object("test/file.txt", data.clone(), Some("text/plain"))
            .await
            .unwrap();

        let (meta, got_data) = engine.object.get_object("test/file.txt").await.unwrap();
        assert_eq!(got_data, data);
        assert_eq!(meta.content_type, "text/plain");
    }

    #[tokio::test]
    async fn test_unified_engine_cross_model() {
        // 测试三种数据模型在同一个后端上共存
        let engine = UnifiedStorageEngine::memory();

        // KV
        engine.kv.put("config/version", Value::from("1.0")).await.unwrap();

        // Graph
        engine
            .graph
            .add_node(GraphNode::new("node1"))
            .await
            .unwrap();

        // Object
        engine
            .object
            .put_object("file.bin", vec![1, 2, 3], None)
            .await
            .unwrap();

        let stats = engine.stats().await.unwrap();
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.total_objects, 1);
    }

    #[tokio::test]
    async fn test_memory_with_cache() {
        let engine = UnifiedStorageEngine::memory_with_cache();
        assert!(engine.is_cache_enabled());

        engine.kv.put("a", Value::from(1i64)).await.unwrap();

        // 第一次读：缓存命中（因为写入时已经缓存）
        let _ = engine.kv.get("a").await.unwrap();
        let stats = engine.stats().await.unwrap();
        assert!(stats.cache_hits >= 1);
    }

    #[tokio::test]
    async fn test_with_cache() {
        let engine = UnifiedStorageEngine::memory().with_cache(100, 1024 * 1024);
        assert!(engine.is_cache_enabled());
        assert_eq!(engine.backend_type(), BackendType::Memory);
    }
}
