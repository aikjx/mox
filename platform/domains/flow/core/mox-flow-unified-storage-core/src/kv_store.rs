// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! KV 存储接口
//!
//! 提供面向键值数据的高级存储接口。

use std::sync::Arc;

use crate::error::{StorageError, StorageResult};
use crate::storage_trait::StorageBackend;
use crate::types::{RangeOptions, Value};

/// KV 存储
pub struct KvStore {
    backend: Arc<dyn StorageBackend>,
}

impl KvStore {
    /// 创建新的 KV 存储
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    /// 获取值
    pub async fn get(&self, key: &str) -> StorageResult<Value> {
        self.backend
            .kv_get(key)
            .await?
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }

    /// 获取值（可选）
    pub async fn try_get(&self, key: &str) -> StorageResult<Option<Value>> {
        self.backend.kv_get(key).await
    }

    /// 设置值
    pub async fn put(&self, key: &str, value: Value) -> StorageResult<()> {
        if key.is_empty() {
            return Err(StorageError::InvalidParameter {
                param: "key".to_string(),
                reason: "key cannot be empty".to_string(),
            });
        }
        self.backend.kv_put(key, value).await
    }

    /// 设置值（如果不存在）
    pub async fn put_if_absent(&self, key: &str, value: Value) -> StorageResult<bool> {
        if self.backend.kv_exists(key).await? {
            return Ok(false);
        }
        self.backend.kv_put(key, value).await?;
        Ok(true)
    }

    /// 删除键
    pub async fn delete(&self, key: &str) -> StorageResult<bool> {
        self.backend.kv_delete(key).await
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> StorageResult<bool> {
        self.backend.kv_exists(key).await
    }

    /// 范围扫描
    pub async fn scan(&self, options: RangeOptions) -> StorageResult<Vec<(String, Value)>> {
        self.backend.kv_scan(options).await
    }

    /// 前缀扫描
    pub async fn scan_prefix(&self, prefix: &str) -> StorageResult<Vec<(String, Value)>> {
        self.backend
            .kv_scan(RangeOptions::with_prefix(prefix))
            .await
    }

    /// 批量获取
    pub async fn batch_get(&self, keys: &[&str]) -> StorageResult<Vec<(String, Option<Value>)>> {
        self.backend.kv_batch_get(keys).await
    }

    /// 批量写入
    pub async fn batch_put(&self, pairs: &[(&str, Value)]) -> StorageResult<()> {
        self.backend.kv_batch_put(pairs).await
    }

    /// 批量删除
    pub async fn batch_delete(&self, keys: &[&str]) -> StorageResult<usize> {
        self.backend.kv_batch_delete(keys).await
    }

    /// 获取并设置（原子操作，简化实现）
    pub async fn get_and_set(&self, key: &str, value: Value) -> StorageResult<Option<Value>> {
        let old = self.backend.kv_get(key).await?;
        self.backend.kv_put(key, value).await?;
        Ok(old)
    }

    /// 自增（整数）
    pub async fn increment(&self, key: &str, delta: i64) -> StorageResult<i64> {
        let current = self
            .backend
            .kv_get(key)
            .await?
            .and_then(|v| v.as_int())
            .unwrap_or(0);
        let new_val = current + delta;
        self.backend.kv_put(key, Value::Int(new_val)).await?;
        Ok(new_val)
    }

    /// 键总数
    pub async fn count(&self) -> StorageResult<u64> {
        Ok(self.backend.stats().await?.total_keys)
    }
}
