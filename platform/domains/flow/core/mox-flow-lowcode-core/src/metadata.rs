// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 元数据管理器
//!
//! 管理实体、字段、关系的元数据定义，提供元数据的 CRUD 和查询。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{LowcodeError, LowcodeResult};
use crate::types::{EntityDef, FieldDef, RelationDef, now_ms};

/// 元数据管理器
pub struct MetadataManager {
    /// 实体表
    entities: RwLock<HashMap<String, EntityDef>>,
    /// 实体名 -> ID 索引
    entity_names: RwLock<HashMap<String, String>>,
    /// 关系表
    relations: RwLock<HashMap<String, RelationDef>>,
    /// 实体的关系索引：entity_id -> Vec<relation_id>
    entity_relations: RwLock<HashMap<String, Vec<String>>>,
}

impl MetadataManager {
    /// 创建元数据管理器
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            entity_names: RwLock::new(HashMap::new()),
            relations: RwLock::new(HashMap::new()),
            entity_relations: RwLock::new(HashMap::new()),
        }
    }

    // ---------- 实体管理 ----------

    /// 创建实体
    pub fn create_entity(&self, entity: EntityDef) -> LowcodeResult<EntityDef> {
        // 检查名称唯一性
        if self.entity_names.read().contains_key(&entity.name) {
            return Err(LowcodeError::AlreadyExists(format!(
                "entity '{}' already exists",
                entity.name
            )));
        }

        self.entity_names
            .write()
            .insert(entity.name.clone(), entity.id.clone());
        self.entities
            .write()
            .insert(entity.id.clone(), entity.clone());
        Ok(entity)
    }

    /// 获取实体
    pub fn get_entity(&self, entity_id: &str) -> LowcodeResult<EntityDef> {
        self.entities
            .read()
            .get(entity_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", entity_id)))
    }

    /// 按名称获取实体
    pub fn get_entity_by_name(&self, name: &str) -> LowcodeResult<EntityDef> {
        let entity_id = self
            .entity_names
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", name)))?;
        self.get_entity(&entity_id)
    }

    /// 检查实体是否存在
    pub fn entity_exists(&self, entity_id: &str) -> bool {
        self.entities.read().contains_key(entity_id)
    }

    /// 更新实体
    pub fn update_entity(
        &self,
        entity_id: &str,
        mut update: EntityDef,
    ) -> LowcodeResult<EntityDef> {
        let mut entities = self.entities.write();
        let existing = entities
            .get_mut(entity_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", entity_id)))?;

        // 保留不可变字段
        update.id = entity_id.to_string();
        update.name = existing.name.clone(); // 名称不可改
        update.created_at = existing.created_at;
        update.updated_at = now_ms();

        *existing = update.clone();
        Ok(update)
    }

    /// 删除实体
    pub fn delete_entity(&self, entity_id: &str) -> LowcodeResult<bool> {
        let entity = self.get_entity(entity_id)?;

        if entity.is_system {
            return Err(LowcodeError::InvalidConfig(
                "cannot delete system entity".to_string(),
            ));
        }

        // 检查是否有关联关系
        if let Some(rels) = self.entity_relations.read().get(entity_id) {
            if !rels.is_empty() {
                return Err(LowcodeError::InvalidConfig(
                    "cannot delete entity with relations".to_string(),
                ));
            }
        }

        self.entity_names.write().remove(&entity.name);
        Ok(self.entities.write().remove(entity_id).is_some())
    }

    /// 列出所有实体
    pub fn list_entities(&self) -> Vec<EntityDef> {
        self.entities.read().values().cloned().collect()
    }

    /// 按模块列出实体
    pub fn list_entities_by_module(&self, module: &str) -> Vec<EntityDef> {
        self.entities
            .read()
            .values()
            .filter(|e| e.module == module)
            .cloned()
            .collect()
    }

    // ---------- 字段管理 ----------

    /// 添加字段
    pub fn add_field(&self, entity_id: &str, field: FieldDef) -> LowcodeResult<EntityDef> {
        let mut entities = self.entities.write();
        let entity = entities
            .get_mut(entity_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", entity_id)))?;

        // 检查字段名唯一性
        if entity.fields.iter().any(|f| f.name == field.name) {
            return Err(LowcodeError::AlreadyExists(format!(
                "field '{}' already exists in entity '{}'",
                field.name, entity.name
            )));
        }

        entity.add_field(field);
        Ok(entity.clone())
    }

    /// 更新字段
    pub fn update_field(
        &self,
        entity_id: &str,
        field_name: &str,
        field_update: FieldDef,
    ) -> LowcodeResult<EntityDef> {
        let mut entities = self.entities.write();
        let entity = entities
            .get_mut(entity_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", entity_id)))?;

        let field_idx = entity
            .fields
            .iter()
            .position(|f| f.name == field_name)
            .ok_or_else(|| {
                LowcodeError::NotFound(format!(
                    "field '{}' not found in entity '{}'",
                    field_name, entity.name
                ))
            })?;

        if entity.fields[field_idx].is_system {
            return Err(LowcodeError::InvalidConfig(
                "cannot modify system field".to_string(),
            ));
        }

        let mut updated = field_update;
        updated.name = field_name.to_string(); // 名称不可改
        updated.id = entity.fields[field_idx].id.clone();
        entity.fields[field_idx] = updated;
        entity.updated_at = now_ms();

        Ok(entity.clone())
    }

    /// 删除字段
    pub fn delete_field(&self, entity_id: &str, field_name: &str) -> LowcodeResult<bool> {
        let mut entities = self.entities.write();
        let entity = entities
            .get_mut(entity_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("entity '{}' not found", entity_id)))?;

        let field_idx = entity
            .fields
            .iter()
            .position(|f| f.name == field_name)
            .ok_or_else(|| {
                LowcodeError::NotFound(format!(
                    "field '{}' not found in entity '{}'",
                    field_name, entity.name
                ))
            })?;

        if entity.fields[field_idx].is_system {
            return Err(LowcodeError::InvalidConfig(
                "cannot delete system field".to_string(),
            ));
        }

        entity.fields.remove(field_idx);
        entity.updated_at = now_ms();
        Ok(true)
    }

    // ---------- 关系管理 ----------

    /// 创建关系
    pub fn create_relation(&self, relation: RelationDef) -> LowcodeResult<RelationDef> {
        // 验证源实体和目标实体存在
        if !self.entity_exists(&relation.source_entity_id) {
            return Err(LowcodeError::NotFound(format!(
                "source entity '{}' not found",
                relation.source_entity_id
            )));
        }
        if !self.entity_exists(&relation.target_entity_id) {
            return Err(LowcodeError::NotFound(format!(
                "target entity '{}' not found",
                relation.target_entity_id
            )));
        }

        // 添加到实体关系索引
        self.entity_relations
            .write()
            .entry(relation.source_entity_id.clone())
            .or_default()
            .push(relation.id.clone());
        self.entity_relations
            .write()
            .entry(relation.target_entity_id.clone())
            .or_default()
            .push(relation.id.clone());

        self.relations
            .write()
            .insert(relation.id.clone(), relation.clone());
        Ok(relation)
    }

    /// 获取关系
    pub fn get_relation(&self, relation_id: &str) -> LowcodeResult<RelationDef> {
        self.relations
            .read()
            .get(relation_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("relation '{}' not found", relation_id)))
    }

    /// 删除关系
    pub fn delete_relation(&self, relation_id: &str) -> LowcodeResult<bool> {
        let relation = self.get_relation(relation_id)?;

        // 从实体索引中移除
        if let Some(vec) = self
            .entity_relations
            .write()
            .get_mut(&relation.source_entity_id)
        {
            vec.retain(|id| id != relation_id);
        }
        if let Some(vec) = self
            .entity_relations
            .write()
            .get_mut(&relation.target_entity_id)
        {
            vec.retain(|id| id != relation_id);
        }

        Ok(self.relations.write().remove(relation_id).is_some())
    }

    /// 获取实体的所有关系
    pub fn get_entity_relations(&self, entity_id: &str) -> LowcodeResult<Vec<RelationDef>> {
        let rel_ids = self
            .entity_relations
            .read()
            .get(entity_id)
            .cloned()
            .unwrap_or_default();
        let relations = self.relations.read();
        Ok(rel_ids
            .into_iter()
            .filter_map(|id| relations.get(&id).cloned())
            .collect())
    }

    /// 实体总数
    pub fn entity_count(&self) -> usize {
        self.entities.read().len()
    }

    /// 关系总数
    pub fn relation_count(&self) -> usize {
        self.relations.read().len()
    }
}

impl Default for MetadataManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, RelationType};

    fn setup() -> MetadataManager {
        let mgr = MetadataManager::new();

        // 创建用户实体
        let user = EntityDef::new("user", "用户", "system");
        mgr.create_entity(user).unwrap();

        // 创建订单实体
        let order = EntityDef::new("order", "订单", "business");
        mgr.create_entity(order).unwrap();

        mgr
    }

    #[test]
    fn test_create_entity() {
        let mgr = setup();
        assert_eq!(mgr.entity_count(), 2);

        let user = mgr.get_entity_by_name("user").unwrap();
        assert_eq!(user.label, "用户");
        assert_eq!(user.module, "system");
    }

    #[test]
    fn test_duplicate_entity_name() {
        let mgr = setup();
        let user = EntityDef::new("user", "User 2", "test");
        assert!(mgr.create_entity(user).is_err());
    }

    #[test]
    fn test_add_field() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();

        let email = FieldDef::new("email", "邮箱", FieldType::String).required();
        let updated = mgr.add_field(&user.id, email).unwrap();

        assert!(updated.get_field("email").is_some());
        assert!(updated.get_field("email").unwrap().required);
    }

    #[test]
    fn test_duplicate_field_name() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();

        let field1 = FieldDef::new("email", "邮箱", FieldType::String);
        mgr.add_field(&user.id, field1).unwrap();

        let field2 = FieldDef::new("email", "邮箱2", FieldType::String);
        assert!(mgr.add_field(&user.id, field2).is_err());
    }

    #[test]
    fn test_delete_field() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();

        let email = FieldDef::new("email", "邮箱", FieldType::String);
        mgr.add_field(&user.id, email).unwrap();

        let result = mgr.delete_field(&user.id, "email").unwrap();
        assert!(result);

        let user = mgr.get_entity_by_name("user").unwrap();
        assert!(user.get_field("email").is_none());
    }

    #[test]
    fn test_cannot_delete_system_field() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();

        let result = mgr.delete_field(&user.id, "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_relation() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();
        let order = mgr.get_entity_by_name("order").unwrap();

        let rel = RelationDef::new(
            "user_orders",
            RelationType::OneToMany,
            &user.id,
            &order.id,
        );

        let created = mgr.create_relation(rel).unwrap();
        assert_eq!(created.name, "user_orders");
        assert_eq!(mgr.relation_count(), 1);

        let user_rels = mgr.get_entity_relations(&user.id).unwrap();
        assert_eq!(user_rels.len(), 1);
    }

    #[test]
    fn test_delete_relation() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();
        let order = mgr.get_entity_by_name("order").unwrap();

        let rel = RelationDef::new(
            "user_orders",
            RelationType::OneToMany,
            &user.id,
            &order.id,
        );
        let rel = mgr.create_relation(rel).unwrap();

        assert!(mgr.delete_relation(&rel.id).unwrap());
        assert_eq!(mgr.relation_count(), 0);

        let user_rels = mgr.get_entity_relations(&user.id).unwrap();
        assert_eq!(user_rels.len(), 0);
    }

    #[test]
    fn test_list_by_module() {
        let mgr = setup();
        let system_entities = mgr.list_entities_by_module("system");
        assert_eq!(system_entities.len(), 1);
        assert_eq!(system_entities[0].name, "user");
    }

    #[test]
    fn test_cannot_delete_entity_with_relations() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();
        let order = mgr.get_entity_by_name("order").unwrap();

        let rel = RelationDef::new(
            "r1",
            RelationType::OneToMany,
            &user.id,
            &order.id,
        );
        mgr.create_relation(rel).unwrap();

        let result = mgr.delete_entity(&user.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_field_preserves_name() {
        let mgr = setup();
        let user = mgr.get_entity_by_name("user").unwrap();

        let field = FieldDef::new("email", "邮箱", FieldType::String);
        mgr.add_field(&user.id, field).unwrap();

        let mut update = FieldDef::new("email_new", "新标签", FieldType::Text);
        update.required = true;

        let updated = mgr.update_field(&user.id, "email", update).unwrap();
        let f = updated.get_field("email").unwrap();
        assert_eq!(f.name, "email"); // 名称不变
        assert_eq!(f.label, "新标签"); // 标签更新
        assert!(f.required);
    }
}
