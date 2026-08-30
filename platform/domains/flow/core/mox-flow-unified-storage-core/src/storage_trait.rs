// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 存储后端 Trait 定义

use async_trait::async_trait;

use crate::error::StorageResult;
use crate::types::{
    EdgeDirection, GraphEdge, GraphNode, ListObjectsOptions, ListObjectsResult, ObjectMeta,
    RangeOptions, StorageStats, Value,
};

/// 底层存储后端 Trait
///
/// 所有存储后端（内存、RocksDB、对象存储等）都需要实现这个 trait。
/// 统一存储引擎通过这个 trait 操作底层存储。
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // === KV 操作 ===

    /// 获取键值
    async fn kv_get(&self, key: &str) -> StorageResult<Option<Value>>;

    /// 设置键值
    async fn kv_put(&self, key: &str, value: Value) -> StorageResult<()>;

    /// 删除键
    async fn kv_delete(&self, key: &str) -> StorageResult<bool>;

    /// 检查键是否存在
    async fn kv_exists(&self, key: &str) -> StorageResult<bool> {
        Ok(self.kv_get(key).await?.is_some())
    }

    /// 范围扫描
    async fn kv_scan(&self, options: RangeOptions) -> StorageResult<Vec<(String, Value)>>;

    /// 批量获取
    async fn kv_batch_get(&self, keys: &[&str]) -> StorageResult<Vec<(String, Option<Value>)>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let val = self.kv_get(key).await?;
            results.push((key.to_string(), val));
        }
        Ok(results)
    }

    /// 批量写入
    async fn kv_batch_put(&self, pairs: &[(&str, Value)]) -> StorageResult<()> {
        for (key, value) in pairs {
            self.kv_put(key, value.clone()).await?;
        }
        Ok(())
    }

    /// 批量删除
    async fn kv_batch_delete(&self, keys: &[&str]) -> StorageResult<usize> {
        let mut count = 0;
        for key in keys {
            if self.kv_delete(key).await? {
                count += 1;
            }
        }
        Ok(count)
    }

    // === 图操作 ===

    /// 获取节点
    async fn graph_get_node(&self, node_id: &str) -> StorageResult<Option<GraphNode>>;

    /// 添加/更新节点
    async fn graph_put_node(&self, node: GraphNode) -> StorageResult<()>;

    /// 删除节点
    async fn graph_delete_node(&self, node_id: &str) -> StorageResult<bool>;

    /// 检查节点是否存在
    async fn graph_node_exists(&self, node_id: &str) -> StorageResult<bool> {
        Ok(self.graph_get_node(node_id).await?.is_some())
    }

    /// 获取边
    async fn graph_get_edge(&self, edge_id: &str) -> StorageResult<Option<GraphEdge>>;

    /// 添加/更新边
    async fn graph_put_edge(&self, edge: GraphEdge) -> StorageResult<()>;

    /// 删除边
    async fn graph_delete_edge(&self, edge_id: &str) -> StorageResult<bool>;

    /// 获取节点的邻边
    async fn graph_get_edges(
        &self,
        node_id: &str,
        direction: EdgeDirection,
        edge_type: Option<&str>,
    ) -> StorageResult<Vec<GraphEdge>>;

    /// 列出所有节点（带分页）
    async fn graph_list_nodes(&self, options: RangeOptions) -> StorageResult<Vec<GraphNode>>;

    /// 列出所有边（带分页）
    async fn graph_list_edges(&self, options: RangeOptions) -> StorageResult<Vec<GraphEdge>>;

    // === 对象操作 ===

    /// 获取对象数据
    async fn object_get(&self, key: &str) -> StorageResult<Option<(ObjectMeta, Vec<u8>)>>;

    /// 获取对象元数据
    async fn object_head(&self, key: &str) -> StorageResult<Option<ObjectMeta>>;

    /// 上传对象
    async fn object_put(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> StorageResult<ObjectMeta>;

    /// 删除对象
    async fn object_delete(&self, key: &str) -> StorageResult<bool>;

    /// 检查对象是否存在
    async fn object_exists(&self, key: &str) -> StorageResult<bool> {
        Ok(self.object_head(key).await?.is_some())
    }

    /// 列出对象
    async fn object_list(&self, options: ListObjectsOptions) -> StorageResult<ListObjectsResult>;

    /// 获取对象范围数据
    async fn object_get_range(
        &self,
        key: &str,
        offset: u64,
        length: Option<u64>,
    ) -> StorageResult<Option<Vec<u8>>> {
        // 默认实现：读取全部再切片
        if let Some((_meta, data)) = self.object_get(key).await? {
            let end = match length {
                Some(len) => (offset + len).min(data.len() as u64) as usize,
                None => data.len(),
            };
            let start = offset.min(data.len() as u64) as usize;
            Ok(Some(data[start..end].to_vec()))
        } else {
            Ok(None)
        }
    }

    // === 通用操作 ===

    /// 获取统计信息
    async fn stats(&self) -> StorageResult<StorageStats>;

    /// 刷新/同步数据到持久化存储
    async fn flush(&self) -> StorageResult<()> {
        Ok(())
    }

    /// 关闭存储后端
    async fn close(&self) -> StorageResult<()> {
        self.flush().await
    }

    /// 清空所有数据（用于测试）
    async fn clear(&self) -> StorageResult<()>;
}
