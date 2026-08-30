// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 元数据索引
//!
//! 提供多种索引结构加速元数据查询：
//! - 标签索引
//! - 属性索引
//! - 全文索引（简化版）
//! - 时间范围索引

use std::collections::{BTreeMap, HashMap, HashSet};

use parking_lot::RwLock;

use crate::error::{MetaError, MetaResult};
use crate::types::EntityKind;

/// 索引类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// 标签索引
    Tag,
    /// 属性值索引
    Property,
    /// 全文索引
    FullText,
    /// 时间范围索引
    TimeRange,
}

impl IndexType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexType::Tag => "tag",
            IndexType::Property => "property",
            IndexType::FullText => "fulltext",
            IndexType::TimeRange => "time_range",
        }
    }
}

/// 索引描述
#[derive(Debug, Clone)]
pub struct IndexDescriptor {
    /// 索引名称
    pub name: String,
    /// 索引类型
    pub index_type: IndexType,
    /// 适用的实体种类
    pub entity_kind: Option<EntityKind>,
    /// 属性名（Property 索引）
    pub property: Option<String>,
    /// 是否唯一索引
    pub unique: bool,
}

/// 倒排列表条目
#[derive(Debug, Clone)]
struct PostingList {
    /// 实体 ID 集合
    entity_ids: HashSet<String>,
}

impl PostingList {
    fn new() -> Self {
        Self {
            entity_ids: HashSet::new(),
        }
    }

    fn add(&mut self, entity_id: &str) -> bool {
        self.entity_ids.insert(entity_id.to_string())
    }

    fn remove(&mut self, entity_id: &str) -> bool {
        self.entity_ids.remove(entity_id)
    }

    fn contains(&self, entity_id: &str) -> bool {
        self.entity_ids.contains(entity_id)
    }

    fn len(&self) -> usize {
        self.entity_ids.len()
    }
}

/// 元数据索引引擎
pub struct MetadataIndex {
    /// 标签索引：tag -> entity_ids
    tag_index: RwLock<BTreeMap<String, PostingList>>,
    /// 属性索引：(property_name, value) -> entity_ids
    property_index: RwLock<HashMap<String, BTreeMap<String, PostingList>>>,
    /// 索引描述
    indexes: RwLock<HashMap<String, IndexDescriptor>>,
}

impl MetadataIndex {
    /// 创建索引引擎
    pub fn new() -> Self {
        Self {
            tag_index: RwLock::new(BTreeMap::new()),
            property_index: RwLock::new(HashMap::new()),
            indexes: RwLock::new(HashMap::new()),
        }
    }

    /// 创建索引
    pub fn create_index(&self, descriptor: IndexDescriptor) -> MetaResult<()> {
        let mut indexes = self.indexes.write();

        if indexes.contains_key(&descriptor.name) {
            return Err(MetaError::IndexError(format!(
                "index '{}' already exists",
                descriptor.name
            )));
        }

        // 如果是属性索引，初始化索引结构
        if descriptor.index_type == IndexType::Property {
            if let Some(prop) = &descriptor.property {
                self.property_index
                    .write()
                    .insert(prop.clone(), BTreeMap::new());
            } else {
                return Err(MetaError::InvalidParameter {
                    param: "property".to_string(),
                    reason: "property index requires property name".to_string(),
                });
            }
        }

        indexes.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// 删除索引
    pub fn drop_index(&self, name: &str) -> MetaResult<bool> {
        let descriptor = {
            let mut indexes = self.indexes.write();
            indexes.remove(name)
        };

        if let Some(desc) = descriptor {
            if desc.index_type == IndexType::Property {
                if let Some(prop) = &desc.property {
                    self.property_index.write().remove(prop);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 检查索引是否存在
    pub fn index_exists(&self, name: &str) -> bool {
        self.indexes.read().contains_key(name)
    }

    /// 获取所有索引
    pub fn list_indexes(&self) -> Vec<IndexDescriptor> {
        self.indexes.read().values().cloned().collect()
    }

    // === 标签索引 ===

    /// 添加标签索引
    pub fn add_tag(&self, entity_id: &str, tag: &str) -> bool {
        let mut index = self.tag_index.write();
        index
            .entry(tag.to_string())
            .or_insert_with(PostingList::new)
            .add(entity_id)
    }

    /// 移除标签索引
    pub fn remove_tag(&self, entity_id: &str, tag: &str) -> bool {
        let mut index = self.tag_index.write();
        if let Some(posting) = index.get_mut(tag) {
            let removed = posting.remove(entity_id);
            if posting.len() == 0 {
                index.remove(tag);
            }
            removed
        } else {
            false
        }
    }

    /// 按标签查找
    pub fn find_by_tag(&self, tag: &str) -> Vec<String> {
        self.tag_index
            .read()
            .get(tag)
            .map(|p| p.entity_ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 多标签交集查询
    pub fn find_by_tags_intersection(&self, tags: &[String]) -> Vec<String> {
        if tags.is_empty() {
            return Vec::new();
        }

        let index = self.tag_index.read();

        // 获取第一个标签的结果
        let mut result: HashSet<String> = match index.get(&tags[0]) {
            Some(p) => p.entity_ids.clone(),
            None => return Vec::new(),
        };

        // 依次与其他标签取交集
        for tag in &tags[1..] {
            if let Some(p) = index.get(tag) {
                result = result.intersection(&p.entity_ids).cloned().collect();
            } else {
                return Vec::new();
            }
        }

        result.into_iter().collect()
    }

    /// 多标签并集查询
    pub fn find_by_tags_union(&self, tags: &[String]) -> Vec<String> {
        let index = self.tag_index.read();
        let mut result = HashSet::new();

        for tag in tags {
            if let Some(p) = index.get(tag) {
                result.extend(p.entity_ids.iter().cloned());
            }
        }

        result.into_iter().collect()
    }

    /// 前缀匹配标签
    pub fn find_by_tag_prefix(&self, prefix: &str) -> Vec<(String, Vec<String>)> {
        let index = self.tag_index.read();
        let mut results = Vec::new();

        for (tag, posting) in index.range(prefix.to_string()..) {
            if !tag.starts_with(prefix) {
                break;
            }
            results.push((tag.clone(), posting.entity_ids.iter().cloned().collect()));
        }

        results
    }

    /// 所有标签及数量
    pub fn all_tags_with_counts(&self) -> Vec<(String, usize)> {
        self.tag_index
            .read()
            .iter()
            .map(|(tag, posting)| (tag.clone(), posting.len()))
            .collect()
    }

    // === 属性索引 ===

    /// 添加属性索引
    pub fn add_property(&self, property: &str, value: &str, entity_id: &str) -> MetaResult<bool> {
        let mut index = self.property_index.write();
        let prop_index = index
            .get_mut(property)
            .ok_or_else(|| MetaError::IndexError(format!("property index '{}' not found", property)))?;

        Ok(prop_index
            .entry(value.to_string())
            .or_insert_with(PostingList::new)
            .add(entity_id))
    }

    /// 移除属性索引
    pub fn remove_property(&self, property: &str, value: &str, entity_id: &str) -> MetaResult<bool> {
        let mut index = self.property_index.write();
        let prop_index = index
            .get_mut(property)
            .ok_or_else(|| MetaError::IndexError(format!("property index '{}' not found", property)))?;

        if let Some(posting) = prop_index.get_mut(value) {
            let removed = posting.remove(entity_id);
            if posting.len() == 0 {
                prop_index.remove(value);
            }
            Ok(removed)
        } else {
            Ok(false)
        }
    }

    /// 按属性值查找
    pub fn find_by_property(&self, property: &str, value: &str) -> MetaResult<Vec<String>> {
        let index = self.property_index.read();
        let prop_index = index
            .get(property)
            .ok_or_else(|| MetaError::IndexError(format!("property index '{}' not found", property)))?;

        Ok(prop_index
            .get(value)
            .map(|p| p.entity_ids.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// 属性值范围查询
    pub fn find_by_property_range(
        &self,
        property: &str,
        start: &str,
        end: &str,
    ) -> MetaResult<Vec<String>> {
        let index = self.property_index.read();
        let prop_index = index
            .get(property)
            .ok_or_else(|| MetaError::IndexError(format!("property index '{}' not found", property)))?;

        let mut result = HashSet::new();
        for (_value, posting) in prop_index.range(start.to_string()..=end.to_string()) {
            result.extend(posting.entity_ids.iter().cloned());
        }

        Ok(result.into_iter().collect())
    }

    /// 索引统计
    pub fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        let tag_index = self.tag_index.read();
        stats.insert("tag_indexes".to_string(), tag_index.len());
        let total_tag_entries: usize = tag_index.values().map(|p| p.len()).sum();
        stats.insert("tag_index_entries".to_string(), total_tag_entries);

        let prop_index = self.property_index.read();
        stats.insert("property_indexes".to_string(), prop_index.len());
        let total_prop_entries: usize = prop_index
            .values()
            .map(|m| m.values().map(|p| p.len()).sum::<usize>())
            .sum();
        stats.insert("property_index_entries".to_string(), total_prop_entries);

        stats.insert("total_indexes".to_string(), self.indexes.read().len());

        stats
    }
}

impl Default for MetadataIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_index() {
        let index = MetadataIndex::new();

        index.add_tag("entity1", "tag1");
        index.add_tag("entity2", "tag1");
        index.add_tag("entity1", "tag2");

        let result = index.find_by_tag("tag1");
        assert_eq!(result.len(), 2);

        let result = index.find_by_tag("tag2");
        assert_eq!(result.len(), 1);
        assert!(result.contains(&"entity1".to_string()));
    }

    #[test]
    fn test_tag_index_remove() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "t1");
        index.add_tag("e2", "t1");

        assert!(index.remove_tag("e1", "t1"));

        let result = index.find_by_tag("t1");
        assert_eq!(result.len(), 1);
        assert!(result.contains(&"e2".to_string()));
    }

    #[test]
    fn test_tags_intersection() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "red");
        index.add_tag("e1", "big");
        index.add_tag("e2", "red");
        index.add_tag("e2", "small");
        index.add_tag("e3", "red");
        index.add_tag("e3", "big");

        let result = index.find_by_tags_intersection(&["red".to_string(), "big".to_string()]);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"e1".to_string()));
        assert!(result.contains(&"e3".to_string()));
    }

    #[test]
    fn test_tags_union() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "a");
        index.add_tag("e2", "b");
        index.add_tag("e3", "a");

        let result = index.find_by_tags_union(&["a".to_string(), "b".to_string()]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_tag_prefix() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "user/admin");
        index.add_tag("e2", "user/guest");
        index.add_tag("e3", "system/root");

        let result = index.find_by_tag_prefix("user/");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_property_index() {
        let index = MetadataIndex::new();

        index
            .create_index(IndexDescriptor {
                name: "idx_status".to_string(),
                index_type: IndexType::Property,
                entity_kind: None,
                property: Some("status".to_string()),
                unique: false,
            })
            .unwrap();

        index.add_property("status", "active", "e1").unwrap();
        index.add_property("status", "active", "e2").unwrap();
        index.add_property("status", "inactive", "e3").unwrap();

        let result = index.find_by_property("status", "active").unwrap();
        assert_eq!(result.len(), 2);

        let result = index.find_by_property("status", "inactive").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_property_range() {
        let index = MetadataIndex::new();

        index
            .create_index(IndexDescriptor {
                name: "idx_age".to_string(),
                index_type: IndexType::Property,
                entity_kind: None,
                property: Some("age".to_string()),
                unique: false,
            })
            .unwrap();

        index.add_property("age", "020", "e1").unwrap();
        index.add_property("age", "030", "e2").unwrap();
        index.add_property("age", "040", "e3").unwrap();

        let result = index.find_by_property_range("age", "020", "030").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_create_drop_index() {
        let index = MetadataIndex::new();

        assert!(!index.index_exists("idx1"));

        index
            .create_index(IndexDescriptor {
                name: "idx1".to_string(),
                index_type: IndexType::Tag,
                entity_kind: None,
                property: None,
                unique: false,
            })
            .unwrap();

        assert!(index.index_exists("idx1"));
        assert_eq!(index.list_indexes().len(), 1);

        assert!(index.drop_index("idx1").unwrap());
        assert!(!index.index_exists("idx1"));
    }

    #[test]
    fn test_stats() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "t1");
        index.add_tag("e2", "t1");
        index.add_tag("e1", "t2");

        let stats = index.stats();
        assert_eq!(stats["tag_indexes"], 2);
        assert_eq!(stats["tag_index_entries"], 3);
    }

    #[test]
    fn test_all_tags_with_counts() {
        let index = MetadataIndex::new();

        index.add_tag("e1", "popular");
        index.add_tag("e2", "popular");
        index.add_tag("e3", "popular");
        index.add_tag("e1", "rare");

        let tags = index.all_tags_with_counts();
        assert_eq!(tags.len(), 2);
    }
}
