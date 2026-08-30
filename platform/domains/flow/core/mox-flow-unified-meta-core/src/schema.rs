// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! Schema 管理
//!
//! 统一的 Schema 定义，用于验证实体属性结构。
//! 支持 KG Schema、Cloud Object Schema 等多种类型。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{MetaError, MetaResult};
use crate::types::{EntityKind, VersionInfo, new_id};

/// Schema 字段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaFieldType {
    /// 字符串
    String,
    /// 整数
    Integer,
    /// 浮点数
    Float,
    /// 布尔值
    Boolean,
    /// 时间戳
    Timestamp,
    /// 二进制
    Bytes,
    /// 引用（指向另一个实体）
    Reference,
    /// 列表
    List,
    /// 对象/Map
    Object,
    /// 枚举
    Enum,
}

impl SchemaFieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SchemaFieldType::String => "string",
            SchemaFieldType::Integer => "integer",
            SchemaFieldType::Float => "float",
            SchemaFieldType::Boolean => "boolean",
            SchemaFieldType::Timestamp => "timestamp",
            SchemaFieldType::Bytes => "bytes",
            SchemaFieldType::Reference => "reference",
            SchemaFieldType::List => "list",
            SchemaFieldType::Object => "object",
            SchemaFieldType::Enum => "enum",
        }
    }
}

/// Schema 字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    /// 字段名
    pub name: String,
    /// 字段类型
    pub field_type: SchemaFieldType,
    /// 是否必填
    pub required: bool,
    /// 默认值（JSON 字符串）
    pub default_value: Option<String>,
    /// 描述
    pub description: String,
    /// 是否可索引
    pub indexable: bool,
    /// 最小约束（字符串长度、数字大小）
    pub min_value: Option<f64>,
    /// 最大约束
    pub max_value: Option<f64>,
    /// 枚举可选值
    pub enum_values: Vec<String>,
    /// 引用的 Schema ID（Reference 类型）
    pub ref_schema: Option<String>,
}

impl SchemaField {
    /// 创建新字段
    pub fn new(name: &str, field_type: SchemaFieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            required: false,
            default_value: None,
            description: String::new(),
            indexable: false,
            min_value: None,
            max_value: None,
            enum_values: Vec::new(),
            ref_schema: None,
        }
    }

    /// 设置必填
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置默认值
    pub fn with_default(mut self, default: &str) -> Self {
        self.default_value = Some(default.to_string());
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// 设置可索引
    pub fn indexable(mut self) -> Self {
        self.indexable = true;
        self
    }

    /// 设置范围约束
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// 设置枚举值
    pub fn with_enum_values(mut self, values: &[&str]) -> Self {
        self.enum_values = values.iter().map(|v| v.to_string()).collect();
        self
    }
}

/// Schema 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema ID
    pub id: String,
    /// Schema 名称
    pub name: String,
    /// 适用的实体种类
    pub applies_to: EntityKind,
    /// 字段定义
    pub fields: HashMap<String, SchemaField>,
    /// 版本信息
    pub version: VersionInfo,
    /// 描述
    pub description: String,
    /// 父 Schema ID（用于继承）
    pub parent_schema: Option<String>,
    /// 是否为系统 Schema
    pub is_system: bool,
}

impl Schema {
    /// 创建新 Schema
    pub fn new(name: &str, applies_to: EntityKind) -> Self {
        Self {
            id: new_id(),
            name: name.to_string(),
            applies_to,
            fields: HashMap::new(),
            version: VersionInfo::new(),
            description: String::new(),
            parent_schema: None,
            is_system: false,
        }
    }

    /// 使用指定 ID
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// 设置父 Schema
    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_schema = Some(parent_id.to_string());
        self
    }

    /// 标记为系统 Schema
    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// 添加字段
    pub fn add_field(&mut self, field: SchemaField) -> MetaResult<()> {
        if field.name.is_empty() {
            return Err(MetaError::InvalidParameter {
                param: "field.name".to_string(),
                reason: "field name cannot be empty".to_string(),
            });
        }
        if self.fields.contains_key(&field.name) {
            return Err(MetaError::InvalidParameter {
                param: "field.name".to_string(),
                reason: format!("field '{}' already exists", field.name),
            });
        }

        // 验证枚举类型有枚举值
        if field.field_type == SchemaFieldType::Enum && field.enum_values.is_empty() {
            return Err(MetaError::InvalidParameter {
                param: "field.enum_values".to_string(),
                reason: "enum type requires enum_values".to_string(),
            });
        }

        // 验证引用类型有 ref_schema
        if field.field_type == SchemaFieldType::Reference && field.ref_schema.is_none() {
            return Err(MetaError::InvalidParameter {
                param: "field.ref_schema".to_string(),
                reason: "reference type requires ref_schema".to_string(),
            });
        }

        self.fields.insert(field.name.clone(), field);
        self.version.bump();
        Ok(())
    }

    /// 获取字段
    pub fn get_field(&self, name: &str) -> Option<&SchemaField> {
        self.fields.get(name)
    }

    /// 移除字段
    pub fn remove_field(&mut self, name: &str) -> MetaResult<bool> {
        let removed = self.fields.remove(name).is_some();
        if removed {
            self.version.bump();
        }
        Ok(removed)
    }

    /// 必填字段列表
    pub fn required_fields(&self) -> Vec<&SchemaField> {
        self.fields.values().filter(|f| f.required).collect()
    }

    /// 可索引字段列表
    pub fn indexable_fields(&self) -> Vec<&SchemaField> {
        self.fields.values().filter(|f| f.indexable).collect()
    }

    /// 验证实体数据（简化验证）
    pub fn validate(&self, data: &HashMap<String, String>) -> MetaResult<()> {
        // 检查必填字段
        for field in self.required_fields() {
            if !data.contains_key(&field.name) && field.default_value.is_none() {
                return Err(MetaError::ValidationError(format!(
                    "required field '{}' is missing",
                    field.name
                )));
            }
        }

        // 检查枚举值
        for (key, value) in data {
            if let Some(field) = self.fields.get(key) {
                if field.field_type == SchemaFieldType::Enum
                    && !field.enum_values.is_empty()
                    && !field.enum_values.iter().any(|v| v == value)
                {
                    return Err(MetaError::ValidationError(format!(
                        "field '{}' has invalid value '{}', expected one of: {:?}",
                        field.name, value, field.enum_values
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Schema 构建器
pub struct SchemaBuilder {
    name: String,
    applies_to: EntityKind,
    id: Option<String>,
    description: String,
    fields: Vec<SchemaField>,
    parent_schema: Option<String>,
    is_system: bool,
}

impl SchemaBuilder {
    pub fn new(name: &str, applies_to: EntityKind) -> Self {
        Self {
            name: name.to_string(),
            applies_to,
            id: None,
            description: String::new(),
            fields: Vec::new(),
            parent_schema: None,
            is_system: false,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn field(mut self, field: SchemaField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn parent(mut self, parent_id: &str) -> Self {
        self.parent_schema = Some(parent_id.to_string());
        self
    }

    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    pub fn build(self) -> MetaResult<Schema> {
        let mut schema = Schema::new(&self.name, self.applies_to);
        if let Some(id) = self.id {
            schema.id = id;
        }
        schema.description = self.description;
        schema.parent_schema = self.parent_schema;
        schema.is_system = self.is_system;

        for field in self.fields {
            schema.add_field(field)?;
        }

        // 重置版本
        schema.version.version = 1;

        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_new() {
        let schema = Schema::new("Person", EntityKind::GraphNode);
        assert_eq!(schema.name, "Person");
        assert_eq!(schema.applies_to, EntityKind::GraphNode);
        assert!(schema.fields.is_empty());
    }

    #[test]
    fn test_add_field() {
        let mut schema = Schema::new("Person", EntityKind::GraphNode);

        schema
            .add_field(SchemaField::new("name", SchemaFieldType::String).required())
            .unwrap();

        assert_eq!(schema.fields.len(), 1);
        assert!(schema.get_field("name").unwrap().required);
    }

    #[test]
    fn test_duplicate_field_error() {
        let mut schema = Schema::new("Test", EntityKind::Generic);
        schema
            .add_field(SchemaField::new("f1", SchemaFieldType::String))
            .unwrap();

        let result = schema.add_field(SchemaField::new("f1", SchemaFieldType::Integer));
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_field_validation() {
        let mut schema = Schema::new("Test", EntityKind::Generic);

        // 枚举字段必须有枚举值
        let result = schema.add_field(SchemaField::new("status", SchemaFieldType::Enum));
        assert!(result.is_err());

        schema
            .add_field(
                SchemaField::new("status", SchemaFieldType::Enum)
                    .with_enum_values(&["active", "inactive"]),
            )
            .unwrap();
    }

    #[test]
    fn test_schema_validate_required() {
        let schema = SchemaBuilder::new("Test", EntityKind::Generic)
            .field(SchemaField::new("name", SchemaFieldType::String).required())
            .build()
            .unwrap();

        let mut data = HashMap::new();
        assert!(schema.validate(&data).is_err());

        data.insert("name".to_string(), "test".to_string());
        assert!(schema.validate(&data).is_ok());
    }

    #[test]
    fn test_schema_validate_enum() {
        let schema = SchemaBuilder::new("Test", EntityKind::Generic)
            .field(
                SchemaField::new("color", SchemaFieldType::Enum)
                    .with_enum_values(&["red", "green", "blue"]),
            )
            .build()
            .unwrap();

        let mut data = HashMap::new();
        data.insert("color".to_string(), "red".to_string());
        assert!(schema.validate(&data).is_ok());

        data.insert("color".to_string(), "yellow".to_string());
        assert!(schema.validate(&data).is_err());
    }

    #[test]
    fn test_required_fields() {
        let schema = SchemaBuilder::new("Test", EntityKind::Generic)
            .field(SchemaField::new("a", SchemaFieldType::String).required())
            .field(SchemaField::new("b", SchemaFieldType::String))
            .field(SchemaField::new("c", SchemaFieldType::String).required())
            .build()
            .unwrap();

        assert_eq!(schema.required_fields().len(), 2);
    }

    #[test]
    fn test_indexable_fields() {
        let schema = SchemaBuilder::new("Test", EntityKind::Generic)
            .field(SchemaField::new("a", SchemaFieldType::String).indexable())
            .field(SchemaField::new("b", SchemaFieldType::String))
            .build()
            .unwrap();

        assert_eq!(schema.indexable_fields().len(), 1);
    }

    #[test]
    fn test_remove_field() {
        let mut schema = Schema::new("Test", EntityKind::Generic);
        schema
            .add_field(SchemaField::new("f1", SchemaFieldType::String))
            .unwrap();

        assert!(schema.remove_field("f1").unwrap());
        assert!(!schema.remove_field("f1").unwrap());
    }

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new("User", EntityKind::GraphNode)
            .id("user_schema")
            .description("User schema")
            .field(SchemaField::new("name", SchemaFieldType::String).required())
            .field(SchemaField::new("age", SchemaFieldType::Integer).with_range(0.0, 150.0))
            .build()
            .unwrap();

        assert_eq!(schema.id, "user_schema");
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.description, "User schema");
    }
}
