// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 对象存储接口
//!
//! 提供面向对象数据的高级存储接口，兼容 S3 风格操作。

use std::sync::Arc;

use crate::error::{StorageError, StorageResult};
use crate::storage_trait::StorageBackend;
use crate::types::{ListObjectsOptions, ListObjectsResult, ObjectMeta};

/// 对象存储
pub struct ObjectStore {
    backend: Arc<dyn StorageBackend>,
}

impl ObjectStore {
    /// 创建新的对象存储
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    /// 上传对象
    pub async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> StorageResult<ObjectMeta> {
        if key.is_empty() {
            return Err(StorageError::InvalidParameter {
                param: "key".to_string(),
                reason: "object key cannot be empty".to_string(),
            });
        }
        self.backend.object_put(key, data, content_type).await
    }

    /// 获取对象
    pub async fn get_object(&self, key: &str) -> StorageResult<(ObjectMeta, Vec<u8>)> {
        self.backend
            .object_get(key)
            .await?
            .ok_or_else(|| StorageError::ObjectNotFound(key.to_string()))
    }

    /// 获取对象范围
    pub async fn get_object_range(
        &self,
        key: &str,
        offset: u64,
        length: Option<u64>,
    ) -> StorageResult<Vec<u8>> {
        self.backend
            .object_get_range(key, offset, length)
            .await?
            .ok_or_else(|| StorageError::ObjectNotFound(key.to_string()))
    }

    /// 获取对象元数据
    pub async fn head_object(&self, key: &str) -> StorageResult<ObjectMeta> {
        self.backend
            .object_head(key)
            .await?
            .ok_or_else(|| StorageError::ObjectNotFound(key.to_string()))
    }

    /// 检查对象是否存在
    pub async fn object_exists(&self, key: &str) -> StorageResult<bool> {
        self.backend.object_exists(key).await
    }

    /// 删除对象
    pub async fn delete_object(&self, key: &str) -> StorageResult<bool> {
        self.backend.object_delete(key).await
    }

    /// 列出对象
    pub async fn list_objects(
        &self,
        options: ListObjectsOptions,
    ) -> StorageResult<ListObjectsResult> {
        self.backend.object_list(options).await
    }

    /// 复制对象
    pub async fn copy_object(&self, source_key: &str, dest_key: &str) -> StorageResult<ObjectMeta> {
        let (meta, data) = self.get_object(source_key).await?;
        let mut new_meta = self
            .backend
            .object_put(dest_key, data, Some(&meta.content_type))
            .await?;
        // 保留原元数据
        new_meta.metadata = meta.metadata;
        Ok(new_meta)
    }

    /// 批量删除
    pub async fn delete_objects(&self, keys: &[&str]) -> StorageResult<Vec<String>> {
        let mut deleted = Vec::new();
        for key in keys {
            if self.backend.object_delete(key).await? {
                deleted.push(key.to_string());
            }
        }
        Ok(deleted)
    }

    /// 对象总数
    pub async fn object_count(&self) -> StorageResult<u64> {
        Ok(self.backend.stats().await?.total_objects)
    }

    /// 已用存储空间
    pub async fn used_bytes(&self) -> StorageResult<u64> {
        Ok(self.backend.stats().await?.used_bytes)
    }

    /// 列出目录（使用分隔符）
    pub async fn list_directory(
        &self,
        prefix: &str,
        delimiter: &str,
    ) -> StorageResult<(Vec<ObjectMeta>, Vec<String>)> {
        let result = self
            .list_objects(ListObjectsOptions {
                prefix: Some(prefix.to_string()),
                delimiter: Some(delimiter.to_string()),
                ..Default::default()
            })
            .await?;
        Ok((result.objects, result.common_prefixes))
    }

    /// 获取对象大小
    pub async fn object_size(&self, key: &str) -> StorageResult<u64> {
        Ok(self.head_object(key).await?.size)
    }
}
