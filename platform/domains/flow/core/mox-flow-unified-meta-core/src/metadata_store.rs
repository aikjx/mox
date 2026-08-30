// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 元数据存储
//!
//! 基于统一存储引擎实现元数据的持久化存储。
//! 支持实体 CRUD、Schema 管理、版本控制、标签索引。

use std::sync::Arc;

use mox_flow_unified_storage_core::types::{RangeOptions, Value};
use mox_flow_unified_storage_core::UnifiedStorageEngine;

use crate::error::{MetaError, MetaResult};
use crate::entity::Entity;
use crate::schema::Schema;
use crate::types::{EntityKind, ResourceStatus};

/// 元数据存储
///
/// 提供实体和 Schema 的持久化存储能力。
/// 底层使用统一存储引擎的 KV 和 Graph 接口。
pub struct MetadataStore {
    /// 底层存储引擎
    storage: Arc<UnifiedStorageEngine>,
    /// 实体前缀
    entity_prefix: String,
    /// Schema 前缀
    schema_prefix: String,
    /// 标签索引前缀
    tag_index_prefix: String,
}

impl MetadataStore {
    /// 创建元数据存储
    pub fn new(storage: Arc<UnifiedStorageEngine>) -> Self {
        Self {
            storage,
            entity_prefix: "meta:entity:".to_string(),
            schema_prefix: "meta:schema:".to_string(),
            tag_index_prefix: "meta:tag_idx:".to_string(),
        }
    }

    fn entity_key(&self, id: &str) -> String {
        format!("{}{}", self.entity_prefix, id)
    }

    fn schema_key(&self, id: &str) -> String {
        format!("{}{}", self.schema_prefix, id)
    }

    fn tag_index_key(&self, tag: &str) -> String {
        format!("{}{}", self.tag_index_prefix, tag)
    }

    // === 实体操作 ===

    /// 创建实体
    pub async fn create_entity(&self, entity: Entity) -> MetaResult<Entity> {
        let key = self.entity_key(&entity.id);

        if self.storage.kv.exists(&key).await? {
            return Err(MetaError::EntityAlreadyExists(entity.id));
        }

        let data = serde_json::to_string(&entity).map_err(|e| MetaError::StorageError(e.to_string()))?;
        self.storage.kv.put(&key, Value::String(data)).await?;

        // 更新标签索引
        self.update_tag_index(&entity, &[]).await?;

        Ok(entity)
    }

    /// 获取实体
    pub async fn get_entity(&self, id: &str) -> MetaResult<Entity> {
        let key = self.entity_key(id);
        let value = self.storage.kv.get(&key).await?;

        match value {
            Value::String(s) => {
                let entity: Entity = serde_json::from_str(&s)
                    .map_err(|e| MetaError::StorageError(e.to_string()))?;
                Ok(entity)
            }
            _ => Err(MetaError::EntityNotFound(id.to_string())),
        }
    }

    /// 尝试获取实体
    pub async fn try_get_entity(&self, id: &str) -> MetaResult<Option<Entity>> {
        match self.get_entity(id).await {
            Ok(entity) => Ok(Some(entity)),
            Err(MetaError::EntityNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 检查实体是否存在
    pub async fn entity_exists(&self, id: &str) -> MetaResult<bool> {
        Ok(self.storage.kv.exists(&self.entity_key(id)).await?)
    }

    /// 更新实体
    pub async fn update_entity(&self, entity: &Entity) -> MetaResult<()> {
        let key = self.entity_key(&entity.id);

        if !self.storage.kv.exists(&key).await? {
            return Err(MetaError::EntityNotFound(entity.id.clone()));
        }

        // 获取旧实体用于标签索引更新
        let old_entity = self.get_entity(&entity.id).await?;
        let old_tags: Vec<String> = old_entity.tags.iter().cloned().collect();

        let data = serde_json::to_string(entity)
            .map_err(|e| MetaError::StorageError(e.to_string()))?;
        self.storage.kv.put(&key, Value::String(data)).await?;

        // 更新标签索引
        let new_tags: Vec<String> = entity.tags.iter().cloned().collect();
        self.update_tag_index(entity, &old_tags).await?;

        // 清理不再使用的标签
        for tag in &old_tags {
            if !new_tags.iter().any(|t| t == tag) {
                self.remove_from_tag_index(tag, &entity.id).await?;
            }
        }

        Ok(())
    }

    /// 删除实体
    pub async fn delete_entity(&self, id: &str) -> MetaResult<bool> {
        let entity = match self.try_get_entity(id).await? {
            Some(e) => e,
            None => return Ok(false),
        };

        // 清理标签索引
        for tag in &entity.tags {
            self.remove_from_tag_index(tag, id).await?;
        }

        // 软删除
        let mut entity = entity;
        entity.soft_delete();
        self.update_entity(&entity).await?;

        Ok(true)
    }

    /// 硬删除（物理删除）
    pub async fn hard_delete_entity(&self, id: &str) -> MetaResult<bool> {
        let entity = match self.try_get_entity(id).await? {
            Some(e) => e,
            None => return Ok(false),
        };

        // 清理标签索引
        for tag in &entity.tags {
            self.remove_from_tag_index(tag, id).await?;
        }

        Ok(self.storage.kv.delete(&self.entity_key(id)).await?)
    }

    /// 按种类列出实体
    pub async fn list_entities_by_kind(
        &self,
        kind: EntityKind,
        limit: Option<usize>,
    ) -> MetaResult<Vec<Entity>> {
        let options = RangeOptions {
            prefix: Some(self.entity_prefix.clone()),
            limit,
            ..Default::default()
        };

        let kvs = self.storage.kv.scan(options).await?;
        let mut entities = Vec::new();

        for (_, value) in kvs {
            if let Value::String(s) = value {
                if let Ok(entity) = serde_json::from_str::<Entity>(&s) {
                    if entity.kind == kind && entity.status != ResourceStatus::Deleted {
                        entities.push(entity);
                    }
                }
            }
        }

        Ok(entities)
    }

    /// 按标签查找实体
    pub async fn find_entities_by_tag(
        &self,
        tag: &str,
        limit: Option<usize>,
    ) -> MetaResult<Vec<Entity>> {
        let index_key = self.tag_index_key(tag);
        let value = self.storage.kv.try_get(&index_key).await?;

        let ids: Vec<String> = match value {
            Some(Value::String(s)) => {
                serde_json::from_str(&s).unwrap_or_default()
            }
            _ => Vec::new(),
        };

        let limit = limit.unwrap_or(usize::MAX);
        let mut entities = Vec::new();

        for id in ids.iter().take(limit) {
            if let Ok(entity) = self.get_entity(id).await {
                if entity.status != ResourceStatus::Deleted {
                    entities.push(entity);
                }
            }
        }

        Ok(entities)
    }

    /// 更新标签索引
    async fn update_tag_index(&self, entity: &Entity, _old_tags: &[String]) -> MetaResult<()> {
        for tag in &entity.tags {
            let index_key = self.tag_index_key(tag);
            let value = self.storage.kv.try_get(&index_key).await?;

            let mut ids: Vec<String> = match value {
                Some(Value::String(s)) => {
                    serde_json::from_str(&s).unwrap_or_default()
                }
                _ => Vec::new(),
            };

            if !ids.iter().any(|id| id == &entity.id) {
                ids.push(entity.id.clone());
                let data = serde_json::to_string(&ids)
                    .map_err(|e| MetaError::StorageError(e.to_string()))?;
                self.storage.kv.put(&index_key, Value::String(data)).await?;
            }
        }

        Ok(())
    }

    /// 从标签索引中移除
    async fn remove_from_tag_index(&self, tag: &str, entity_id: &str) -> MetaResult<()> {
        let index_key = self.tag_index_key(tag);
        let value = self.storage.kv.try_get(&index_key).await?;

        if let Some(Value::String(s)) = value {
            let mut ids: Vec<String> = serde_json::from_str(&s).unwrap_or_default();
            ids.retain(|id| id != entity_id);

            if ids.is_empty() {
                self.storage.kv.delete(&index_key).await?;
            } else {
                let data = serde_json::to_string(&ids)
                    .map_err(|e| MetaError::StorageError(e.to_string()))?;
                self.storage.kv.put(&index_key, Value::String(data)).await?;
            }
        }

        Ok(())
    }

    // === Schema 操作 ===

    /// 创建 Schema
    pub async fn create_schema(&self, schema: Schema) -> MetaResult<Schema> {
        let key = self.schema_key(&schema.id);

        if self.storage.kv.exists(&key).await? {
            return Err(MetaError::SchemaAlreadyExists(schema.id));
        }

        let data = serde_json::to_string(&schema)
            .map_err(|e| MetaError::StorageError(e.to_string()))?;
        self.storage.kv.put(&key, Value::String(data)).await?;

        Ok(schema)
    }

    /// 获取 Schema
    pub async fn get_schema(&self, id: &str) -> MetaResult<Schema> {
        let key = self.schema_key(id);
        let value = self.storage.kv.get(&key).await?;

        match value {
            Value::String(s) => {
                let schema: Schema = serde_json::from_str(&s)
                    .map_err(|e| MetaError::StorageError(e.to_string()))?;
                Ok(schema)
            }
            _ => Err(MetaError::SchemaNotFound(id.to_string())),
        }
    }

    /// 检查 Schema 是否存在
    pub async fn schema_exists(&self, id: &str) -> MetaResult<bool> {
        Ok(self.storage.kv.exists(&self.schema_key(id)).await?)
    }

    /// 更新 Schema
    pub async fn update_schema(&self, schema: &Schema) -> MetaResult<()> {
        let key = self.schema_key(&schema.id);

        if !self.storage.kv.exists(&key).await? {
            return Err(MetaError::SchemaNotFound(schema.id.clone()));
        }

        let data = serde_json::to_string(schema)
            .map_err(|e| MetaError::StorageError(e.to_string()))?;
        self.storage.kv.put(&key, Value::String(data)).await?;

        Ok(())
    }

    /// 删除 Schema
    pub async fn delete_schema(&self, id: &str) -> MetaResult<bool> {
        Ok(self.storage.kv.delete(&self.schema_key(id)).await?)
    }

    /// 列出所有 Schema
    pub async fn list_schemas(&self) -> MetaResult<Vec<Schema>> {
        let options = RangeOptions::with_prefix(&self.schema_prefix);
        let kvs = self.storage.kv.scan(options).await?;

        let mut schemas = Vec::new();
        for (_, value) in kvs {
            if let Value::String(s) = value {
                if let Ok(schema) = serde_json::from_str::<Schema>(&s) {
                    schemas.push(schema);
                }
            }
        }

        Ok(schemas)
    }

    /// 按种类获取 Schema
    pub async fn get_schemas_by_kind(&self, kind: EntityKind) -> MetaResult<Vec<Schema>> {
        let all = self.list_schemas().await?;
        Ok(all.into_iter().filter(|s| s.applies_to == kind).collect())
    }

    /// 实体总数
    pub async fn entity_count(&self) -> MetaResult<u64> {
        Ok(self.storage.stats().await?.total_keys)
    }
}
