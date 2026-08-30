// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 统一实体模型
//!
//! 将知识图谱节点、云盘对象、算法等统一为 Entity 模型。
//! 支持版本控制、标签、元数据扩展。

use serde::{Deserialize, Serialize};

use crate::error::{MetaError, MetaResult};
use crate::types::{
    EntityKind, MetadataEntry, MetadataMap, ResourceStatus, TagSet, VersionInfo, new_id,
};

/// 统一实体
///
/// 所有受管资源（KG 节点、云盘对象、算法、流水线等）
/// 都抽象为 Entity，共享统一的元数据管理能力。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// 实体唯一 ID
    pub id: String,
    /// 实体名称
    pub name: String,
    /// 实体种类
    pub kind: EntityKind,
    /// 关联的 Schema ID（可选）
    pub schema_id: Option<String>,
    /// 资源状态
    pub status: ResourceStatus,
    /// 版本信息
    pub version: VersionInfo,
    /// 标签
    pub tags: TagSet,
    /// 元数据属性
    pub metadata: MetadataMap,
    /// 父实体 ID（用于层级结构）
    pub parent_id: Option<String>,
    /// 所有者 ID
    pub owner_id: Option<String>,
    /// 实体数据大小（字节，可选）
    pub size_bytes: Option<u64>,
}

impl Entity {
    /// 创建新实体
    pub fn new(name: &str, kind: EntityKind) -> Self {
        Self {
            id: new_id(),
            name: name.to_string(),
            kind,
            schema_id: None,
            status: ResourceStatus::default(),
            version: VersionInfo::new(),
            tags: TagSet::new(),
            metadata: MetadataMap::new(),
            parent_id: None,
            owner_id: None,
            size_bytes: None,
        }
    }

    /// 使用指定 ID 创建实体
    pub fn with_id(id: &str, name: &str, kind: EntityKind) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            ..Self::new(name, kind)
        }
    }

    /// 设置 Schema
    pub fn with_schema(mut self, schema_id: &str) -> Self {
        self.schema_id = Some(schema_id.to_string());
        self
    }

    /// 设置父实体
    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    /// 设置所有者
    pub fn with_owner(mut self, owner_id: &str) -> Self {
        self.owner_id = Some(owner_id.to_string());
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.insert(tag.to_string());
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .insert(key.to_string(), MetadataEntry::new(key, value));
        self
    }

    /// 设置系统元数据
    pub fn with_system_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .insert(key.to_string(), MetadataEntry::system(key, value));
        self
    }

    /// 设置大小
    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// 设置状态
    pub fn with_status(mut self, status: ResourceStatus) -> Self {
        self.status = status;
        self
    }

    /// 检查是否有指定标签
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// 获取元数据值
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|e| e.value.as_str())
    }

    /// 设置元数据
    pub fn set_metadata(&mut self, key: &str, value: &str) -> MetaResult<()> {
        // 检查是否为系统属性
        if let Some(existing) = self.metadata.get(key) {
            if existing.is_system {
                return Err(MetaError::InvalidParameter {
                    param: key.to_string(),
                    reason: "cannot modify system metadata".to_string(),
                });
            }
        }

        self.metadata
            .insert(key.to_string(), MetadataEntry::new(key, value));
        self.version.bump();
        Ok(())
    }

    /// 删除元数据
    pub fn remove_metadata(&mut self, key: &str) -> MetaResult<bool> {
        if let Some(existing) = self.metadata.get(key) {
            if existing.is_system {
                return Err(MetaError::InvalidParameter {
                    param: key.to_string(),
                    reason: "cannot remove system metadata".to_string(),
                });
            }
        }

        let removed = self.metadata.remove(key).is_some();
        if removed {
            self.version.bump();
        }
        Ok(removed)
    }

    /// 添加标签
    pub fn add_tag(&mut self, tag: &str) -> bool {
        let added = self.tags.insert(tag.to_string());
        if added {
            self.version.bump();
        }
        added
    }

    /// 移除标签
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let removed = self.tags.remove(tag);
        if removed {
            self.version.bump();
        }
        removed
    }

    /// 软删除
    pub fn soft_delete(&mut self) {
        self.status = ResourceStatus::Deleted;
        self.version.bump();
    }

    /// 归档
    pub fn archive(&mut self) {
        self.status = ResourceStatus::Archived;
        self.version.bump();
    }

    /// 激活
    pub fn activate(&mut self) {
        self.status = ResourceStatus::Active;
        self.version.bump();
    }

    /// 重命名
    pub fn rename(&mut self, new_name: &str) -> MetaResult<()> {
        if new_name.is_empty() {
            return Err(MetaError::InvalidParameter {
                param: "name".to_string(),
                reason: "name cannot be empty".to_string(),
            });
        }
        self.name = new_name.to_string();
        self.version.bump();
        Ok(())
    }

    /// 检查版本是否匹配（乐观锁）
    pub fn check_version(&self, expected_version: u64) -> MetaResult<()> {
        if self.version.version != expected_version {
            return Err(MetaError::VersionConflict {
                entity_id: self.id.clone(),
                expected: expected_version,
                actual: self.version.version,
            });
        }
        Ok(())
    }

    /// 估算大小（用于统计）
    pub fn estimated_size(&self) -> usize {
        self.id.len()
            + self.name.len()
            + self
                .metadata
                .iter()
                .map(|(k, v)| k.len() + v.value.len())
                .sum::<usize>()
            + self.tags.iter().map(|t| t.len()).sum::<usize>()
            + 100 // 固定开销
    }
}

/// 实体构建器
pub struct EntityBuilder {
    name: String,
    kind: EntityKind,
    id: Option<String>,
    schema_id: Option<String>,
    parent_id: Option<String>,
    owner_id: Option<String>,
    tags: TagSet,
    metadata: MetadataMap,
    status: Option<ResourceStatus>,
    size_bytes: Option<u64>,
}

impl EntityBuilder {
    /// 创建构建器
    pub fn new(name: &str, kind: EntityKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            id: None,
            schema_id: None,
            parent_id: None,
            owner_id: None,
            tags: TagSet::new(),
            metadata: MetadataMap::new(),
            status: None,
            size_bytes: None,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn schema(mut self, schema_id: &str) -> Self {
        self.schema_id = Some(schema_id.to_string());
        self
    }

    pub fn parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    pub fn owner(mut self, owner_id: &str) -> Self {
        self.owner_id = Some(owner_id.to_string());
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.insert(tag.to_string());
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .insert(key.to_string(), MetadataEntry::new(key, value));
        self
    }

    pub fn system_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .insert(key.to_string(), MetadataEntry::system(key, value));
        self
    }

    pub fn status(mut self, status: ResourceStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// 构建实体
    pub fn build(self) -> Entity {
        let mut entity = if let Some(id) = self.id {
            Entity::with_id(&id, &self.name, self.kind)
        } else {
            Entity::new(&self.name, self.kind)
        };

        entity.schema_id = self.schema_id;
        entity.parent_id = self.parent_id;
        entity.owner_id = self.owner_id;
        entity.tags = self.tags;
        entity.metadata = self.metadata;
        entity.size_bytes = self.size_bytes;

        if let Some(status) = self.status {
            entity.status = status;
        }

        // 重置版本（因为我们设置了很多属性，但只算一个版本）
        entity.version.version = 1;
        entity.version.updated_at = entity.version.created_at;

        entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_new() {
        let entity = Entity::new("test", EntityKind::Generic);
        assert_eq!(entity.name, "test");
        assert_eq!(entity.kind, EntityKind::Generic);
        assert_eq!(entity.status, ResourceStatus::Active);
        assert_eq!(entity.version.version, 1);
        assert!(!entity.id.is_empty());
    }

    #[test]
    fn test_entity_with_id() {
        let entity = Entity::with_id("my-id", "test", EntityKind::GraphNode);
        assert_eq!(entity.id, "my-id");
        assert_eq!(entity.kind, EntityKind::GraphNode);
        assert!(entity.kind.is_graph());
    }

    #[test]
    fn test_entity_metadata() {
        let mut entity = Entity::new("test", EntityKind::Generic);

        entity.set_metadata("key1", "value1").unwrap();
        assert_eq!(entity.get_metadata("key1"), Some("value1"));
        assert_eq!(entity.version.version, 2);

        entity.remove_metadata("key1").unwrap();
        assert_eq!(entity.get_metadata("key1"), None);
    }

    #[test]
    fn test_system_metadata_protected() {
        let mut entity = Entity::new("test", EntityKind::Generic);
        entity = entity.with_system_metadata("sys_key", "sys_value");

        let result = entity.set_metadata("sys_key", "new_value");
        assert!(result.is_err());

        let result = entity.remove_metadata("sys_key");
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_tags() {
        let mut entity = Entity::new("test", EntityKind::Generic);

        assert!(entity.add_tag("important"));
        assert!(entity.has_tag("important"));
        assert!(!entity.add_tag("important")); // 重复添加

        assert!(entity.remove_tag("important"));
        assert!(!entity.has_tag("important"));
    }

    #[test]
    fn test_entity_status_transitions() {
        let mut entity = Entity::new("test", EntityKind::Generic);
        assert_eq!(entity.status, ResourceStatus::Active);

        entity.archive();
        assert_eq!(entity.status, ResourceStatus::Archived);

        entity.activate();
        assert_eq!(entity.status, ResourceStatus::Active);

        entity.soft_delete();
        assert_eq!(entity.status, ResourceStatus::Deleted);
    }

    #[test]
    fn test_version_check() {
        let entity = Entity::new("test", EntityKind::Generic);
        assert!(entity.check_version(1).is_ok());
        assert!(entity.check_version(2).is_err());
    }

    #[test]
    fn test_entity_builder() {
        let entity = EntityBuilder::new("built", EntityKind::CloudObject)
            .id("obj-001")
            .tag("image")
            .metadata("format", "png")
            .size(1024)
            .build();

        assert_eq!(entity.id, "obj-001");
        assert_eq!(entity.name, "built");
        assert_eq!(entity.kind, EntityKind::CloudObject);
        assert!(entity.has_tag("image"));
        assert_eq!(entity.get_metadata("format"), Some("png"));
        assert_eq!(entity.size_bytes, Some(1024));
        assert_eq!(entity.version.version, 1);
    }

    #[test]
    fn test_entity_kind_classification() {
        assert!(EntityKind::GraphNode.is_graph());
        assert!(EntityKind::GraphSchema.is_graph());
        assert!(!EntityKind::GraphNode.is_cloud());

        assert!(EntityKind::CloudObject.is_cloud());
        assert!(EntityKind::CloudBucket.is_cloud());
        assert!(!EntityKind::CloudObject.is_graph());

        assert!(!EntityKind::Algorithm.is_graph());
        assert!(!EntityKind::Algorithm.is_cloud());
    }

    #[test]
    fn test_entity_rename() {
        let mut entity = Entity::new("old", EntityKind::Generic);
        entity.rename("new").unwrap();
        assert_eq!(entity.name, "new");
        assert_eq!(entity.version.version, 2);

        assert!(entity.rename("").is_err());
    }
}
