// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SchemaStore：Space / Tag / EdgeType 定义。
//!
//! - Space：space_id(String) + partition_num(u16，默认 16，必须 2^n 且 ≥4）+ replica_factor(u8≤3)
//! - Tag 定义：tag_name + 字段定义列表 name/type(String/Int/Double/Bool/DateTime) + 索引类型 unique/ttl
//! - EdgeType：edge_name + from_tag + to_tag + rank/weight 属性
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::{MetaError, MetaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Int,
    Double,
    Bool,
    DateTime,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "String",
            FieldType::Int => "Int",
            FieldType::Double => "Double",
            FieldType::Bool => "Bool",
            FieldType::DateTime => "DateTime",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "String" | "string" => FieldType::String,
            "Int" | "int" | "integer" => FieldType::Int,
            "Double" | "double" | "float" => FieldType::Double,
            "Bool" | "bool" | "boolean" => FieldType::Bool,
            "DateTime" | "datetime" => FieldType::DateTime,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IndexKind {
    #[default]
    None,
    Unique,
    Ttl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub ftype: FieldType,
    pub index: IndexKind,
}

impl FieldDef {
    pub fn new(name: impl Into<String>, ftype: FieldType, index: IndexKind) -> Self {
        Self {
            name: name.into(),
            ftype,
            index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagDef {
    pub tag_name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDef {
    pub edge_name: String,
    pub from_tag: String,
    pub to_tag: String,
    pub has_rank: bool,
    pub has_weight: bool,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceDef {
    pub space_id: String,
    pub partition_num: u16,
    pub replica_factor: u8,
    pub created_at: u64,
}

impl SpaceDef {
    pub fn validate(&self) -> MetaResult<()> {
        if self.partition_num < 4 {
            return Err(MetaError::InvalidArgument(format!(
                "partition_num {} must be >= 4",
                self.partition_num
            )));
        }
        if self.partition_num.count_ones() != 1 {
            return Err(MetaError::InvalidArgument(format!(
                "partition_num {} must be a power of two",
                self.partition_num
            )));
        }
        if self.replica_factor == 0 || self.replica_factor > 3 {
            return Err(MetaError::InvalidArgument(format!(
                "replica_factor {} must be in [1,3]",
                self.replica_factor
            )));
        }
        if self.space_id.is_empty() {
            return Err(MetaError::InvalidArgument("space_id empty".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaStore {
    pub spaces: BTreeMap<String, SpaceDef>,
    pub tags: BTreeMap<String, BTreeMap<String, TagDef>>, // space -> tag_name -> def
    pub edges: BTreeMap<String, BTreeMap<String, EdgeDef>>, // space -> edge_name -> def
}

impl SchemaStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_space(&mut self, def: SpaceDef) -> MetaResult<()> {
        def.validate()?;
        if self.spaces.contains_key(&def.space_id) {
            return Err(MetaError::SpaceExists(def.space_id));
        }
        self.spaces.insert(def.space_id.clone(), def);
        Ok(())
    }

    pub fn drop_space(&mut self, name: &str) -> MetaResult<()> {
        self.spaces
            .remove(name)
            .ok_or_else(|| MetaError::SpaceNotFound(name.to_string()))?;
        self.tags.remove(name);
        self.edges.remove(name);
        Ok(())
    }

    pub fn list_spaces(&self) -> Vec<SpaceDef> {
        self.spaces.values().cloned().collect()
    }

    pub fn ensure_space(&self, space: &str) -> MetaResult<()> {
        if !self.spaces.contains_key(space) {
            return Err(MetaError::SpaceNotFound(space.to_string()));
        }
        Ok(())
    }

    pub fn create_tag(&mut self, space: &str, tag: TagDef) -> MetaResult<()> {
        self.ensure_space(space)?;
        let per_space = self.tags.entry(space.to_string()).or_default();
        if per_space.contains_key(&tag.tag_name) {
            return Err(MetaError::TagExists(tag.tag_name, space.to_string()));
        }
        per_space.insert(tag.tag_name.clone(), tag);
        Ok(())
    }

    pub fn alter_tag(
        &mut self,
        space: &str,
        tag_name: &str,
        add_fields: Vec<FieldDef>,
    ) -> MetaResult<()> {
        self.ensure_space(space)?;
        let per_space = self
            .tags
            .get_mut(space)
            .ok_or_else(|| MetaError::SpaceNotFound(space.to_string()))?;
        let tag = per_space
            .get_mut(tag_name)
            .ok_or_else(|| MetaError::TagNotFound(tag_name.to_string(), space.to_string()))?;
        tag.fields.extend(add_fields);
        Ok(())
    }

    pub fn drop_tag(&mut self, space: &str, tag_name: &str) -> MetaResult<()> {
        self.ensure_space(space)?;
        // 若该空间尚无任何 tags 登记，按 TagNotFound 处理而非 SpaceNotFound（空间已存在）。
        let per_space = match self.tags.get_mut(space) {
            Some(m) => m,
            None => {
                return Err(MetaError::TagNotFound(
                    tag_name.to_string(),
                    space.to_string(),
                ))
            }
        };
        per_space
            .remove(tag_name)
            .ok_or_else(|| MetaError::TagNotFound(tag_name.to_string(), space.to_string()))?;
        Ok(())
    }

    pub fn list_tags(&self, space: &str) -> MetaResult<Vec<TagDef>> {
        self.ensure_space(space)?;
        Ok(self
            .tags
            .get(space)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default())
    }

    pub fn create_edge_type(&mut self, space: &str, edge: EdgeDef) -> MetaResult<()> {
        self.ensure_space(space)?;
        let per_space = self.edges.entry(space.to_string()).or_default();
        if per_space.contains_key(&edge.edge_name) {
            return Err(MetaError::EdgeExists(edge.edge_name, space.to_string()));
        }
        per_space.insert(edge.edge_name.clone(), edge);
        Ok(())
    }

    pub fn drop_edge_type(&mut self, space: &str, edge_name: &str) -> MetaResult<()> {
        self.ensure_space(space)?;
        let per_space = match self.edges.get_mut(space) {
            Some(m) => m,
            None => {
                return Err(MetaError::EdgeNotFound(
                    edge_name.to_string(),
                    space.to_string(),
                ))
            }
        };
        per_space
            .remove(edge_name)
            .ok_or_else(|| MetaError::EdgeNotFound(edge_name.to_string(), space.to_string()))?;
        Ok(())
    }

    pub fn list_edge_types(&self, space: &str) -> MetaResult<Vec<EdgeDef>> {
        self.ensure_space(space)?;
        Ok(self
            .edges
            .get(space)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_partition_must_be_power_of_two() {
        let mut st = SchemaStore::new();
        let bad = SpaceDef {
            space_id: "bad".into(),
            partition_num: 7,
            replica_factor: 1,
            created_at: 0,
        };
        assert!(matches!(
            st.create_space(bad),
            Err(MetaError::InvalidArgument(_))
        ));
        let good = SpaceDef {
            space_id: "good".into(),
            partition_num: 16,
            replica_factor: 3,
            created_at: 0,
        };
        assert!(st.create_space(good).is_ok());
    }
    #[test]
    fn rf_must_be_le_3() {
        let mut st = SchemaStore::new();
        let bad = SpaceDef {
            space_id: "bad".into(),
            partition_num: 8,
            replica_factor: 4,
            created_at: 0,
        };
        assert!(matches!(
            st.create_space(bad),
            Err(MetaError::InvalidArgument(_))
        ));
    }
}
