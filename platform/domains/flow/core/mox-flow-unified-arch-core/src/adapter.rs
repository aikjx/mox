// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 适配器模式 — 资源模型转换
//!
//! 将第三方系统的数据模型转换为统一资源模型，
//! 以及将统一模型转换为第三方系统的格式。

use crate::error::{ArchError, ArchResult};
use crate::types::UnifiedResource;
use serde_json::Value;
use std::collections::HashMap;

/// 字段映射类型
#[derive(Debug, Clone)]
pub enum FieldMappingType {
    /// 直接映射：source_field -> target_field
    Direct { source: String, target: String },
    /// 常量映射：target_field = constant_value
    Constant { target: String, value: String },
    /// 表达式映射（简单字符串模板）
    Template { target: String, template: String },
    /// 转换映射：通过函数转换
    Transform { source: String, target: String, transform: TransformFn },
}

/// 转换函数枚举
#[derive(Debug, Clone, Copy)]
pub enum TransformFn {
    /// 转大写
    ToUpperCase,
    /// 转小写
    ToLowerCase,
    /// 去除两端空白
    Trim,
    /// 字符串长度
    Length,
    /// 取绝对值
    Abs,
    /// 转 JSON 字符串
    ToJsonString,
}

/// 资源适配器
///
/// 负责第三方系统数据模型与统一资源模型之间的双向转换。
pub struct ResourceAdapter {
    /// 适配器名称
    pub name: String,
    /// 源系统名称
    pub source_system: String,
    /// 字段映射（源 -> 统一模型）
    pub to_unified_mappings: Vec<FieldMappingType>,
    /// 字段映射（统一模型 -> 源）
    pub from_unified_mappings: Vec<FieldMappingType>,
    /// 默认资源类型
    pub default_resource_type: String,
    /// ID 字段名（源系统中的唯一标识）
    pub id_field: String,
    /// 名称字段名
    pub name_field: String,
    /// 状态字段名
    pub status_field: Option<String>,
}

impl ResourceAdapter {
    /// 创建新的资源适配器
    pub fn new(
        name: &str,
        source_system: &str,
        default_resource_type: &str,
        id_field: &str,
        name_field: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            source_system: source_system.to_string(),
            to_unified_mappings: Vec::new(),
            from_unified_mappings: Vec::new(),
            default_resource_type: default_resource_type.to_string(),
            id_field: id_field.to_string(),
            name_field: name_field.to_string(),
            status_field: None,
        }
    }

    /// 添加到统一模型的映射
    pub fn add_to_unified_mapping(&mut self, mapping: FieldMappingType) {
        self.to_unified_mappings.push(mapping);
    }

    /// 添加从统一模型的映射
    pub fn add_from_unified_mapping(&mut self, mapping: FieldMappingType) {
        self.from_unified_mappings.push(mapping);
    }

    /// 设置状态字段
    pub fn with_status_field(mut self, field: &str) -> Self {
        self.status_field = Some(field.to_string());
        self
    }

    /// 将源系统数据转换为统一资源
    pub fn to_unified(
        &self,
        source_data: &Value,
        connector_id: &str,
    ) -> ArchResult<UnifiedResource> {
        let id = source_data
            .get(&self.id_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArchError::AdapterError(format!(
                "missing id field '{}'",
                self.id_field
            )))?
            .to_string();

        let name = source_data
            .get(&self.name_field)
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let status = if let Some(status_field) = &self.status_field {
            source_data
                .get(status_field)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string()
        } else {
            "active".to_string()
        };

        let mut properties = HashMap::new();

        // 应用映射
        for mapping in &self.to_unified_mappings {
            match mapping {
                FieldMappingType::Direct { source, target } => {
                    if let Some(val) = source_data.get(source) {
                        properties.insert(target.clone(), val.clone());
                    }
                }
                FieldMappingType::Constant { target, value } => {
                    properties.insert(target.clone(), Value::String(value.clone()));
                }
                FieldMappingType::Template { target, template } => {
                    let mut result = template.clone();
                    // 简单的模板替换：{field_name}
                    if let Some(obj) = source_data.as_object() {
                        for (key, val) in obj {
                            let placeholder = format!("{{{}}}", key);
                            if let Some(s) = val.as_str() {
                                result = result.replace(&placeholder, s);
                            }
                        }
                    }
                    properties.insert(target.clone(), Value::String(result));
                }
                FieldMappingType::Transform { source, target, transform } => {
                    if let Some(val) = source_data.get(source) {
                        let transformed = apply_transform(val, *transform);
                        properties.insert(target.clone(), transformed);
                    }
                }
            }
        }

        // 如果没有配置映射，直接把所有字段都放进去
        if self.to_unified_mappings.is_empty() {
            if let Some(obj) = source_data.as_object() {
                for (k, v) in obj {
                    if k != &self.id_field && k != &self.name_field {
                        properties.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let now = crate::types::now_ms();

        Ok(UnifiedResource {
            id: format!("{}:{}", self.source_system, id),
            resource_type: self.default_resource_type.clone(),
            name,
            connector_id: connector_id.to_string(),
            external_id: id,
            properties,
            status,
            created_at: now,
            updated_at: now,
            supported_operations: vec![
                "get".to_string(),
                "list".to_string(),
                "create".to_string(),
                "update".to_string(),
                "delete".to_string(),
            ],
        })
    }

    /// 将统一资源转换为源系统格式
    pub fn from_unified(&self, resource: &UnifiedResource) -> ArchResult<Value> {
        let mut result = serde_json::Map::new();

        // 基础字段
        result.insert(self.id_field.clone(), Value::String(resource.external_id.clone()));
        result.insert(self.name_field.clone(), Value::String(resource.name.clone()));

        if let Some(status_field) = &self.status_field {
            result.insert(status_field.clone(), Value::String(resource.status.clone()));
        }

        // 应用反向映射
        for mapping in &self.from_unified_mappings {
            match mapping {
                FieldMappingType::Direct { source, target } => {
                    if let Some(val) = resource.properties.get(source) {
                        result.insert(target.clone(), val.clone());
                    }
                }
                FieldMappingType::Constant { target, value } => {
                    result.insert(target.clone(), Value::String(value.clone()));
                }
                FieldMappingType::Transform { source, target, transform } => {
                    if let Some(val) = resource.properties.get(source) {
                        let transformed = apply_transform(val, *transform);
                        result.insert(target.clone(), transformed);
                    }
                }
                FieldMappingType::Template { .. } => {
                    // 反向模板暂不支持
                }
            }
        }

        // 如果没有配置反向映射，把所有属性都放进去
        if self.from_unified_mappings.is_empty() {
            for (k, v) in &resource.properties {
                if !result.contains_key(k) {
                    result.insert(k.clone(), v.clone());
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// 批量转换为统一资源
    pub fn to_unified_batch(
        &self,
        source_items: &[Value],
        connector_id: &str,
    ) -> ArchResult<Vec<UnifiedResource>> {
        let mut results = Vec::with_capacity(source_items.len());
        for item in source_items {
            results.push(self.to_unified(item, connector_id)?);
        }
        Ok(results)
    }
}

/// 应用转换函数
fn apply_transform(value: &Value, transform: TransformFn) -> Value {
    match transform {
        TransformFn::ToUpperCase => {
            Value::String(value.as_str().unwrap_or("").to_uppercase())
        }
        TransformFn::ToLowerCase => {
            Value::String(value.as_str().unwrap_or("").to_lowercase())
        }
        TransformFn::Trim => {
            Value::String(value.as_str().unwrap_or("").trim().to_string())
        }
        TransformFn::Length => {
            Value::Number(serde_json::Number::from(
                value.as_str().map(|s| s.len()).unwrap_or(0) as i64
            ))
        }
        TransformFn::Abs => {
            if let Some(n) = value.as_i64() {
                Value::Number(serde_json::Number::from(n.abs()))
            } else if let Some(n) = value.as_f64() {
                Value::Number(
                    serde_json::Number::from_f64(n.abs()).unwrap_or_else(|| serde_json::Number::from(0)),
                )
            } else {
                value.clone()
            }
        }
        TransformFn::ToJsonString => {
            Value::String(serde_json::to_string(value).unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_direct_mapping() {
        let mut adapter = ResourceAdapter::new(
            "test_adapter",
            "test_sys",
            "user",
            "userId",
            "userName",
        );

        adapter.add_to_unified_mapping(FieldMappingType::Direct {
            source: "email".to_string(),
            target: "email".to_string(),
        });

        let source = json!({
            "userId": "u123",
            "userName": "Alice",
            "email": "alice@example.com"
        });

        let resource = adapter.to_unified(&source, "conn-1").unwrap();
        assert_eq!(resource.external_id, "u123");
        assert_eq!(resource.name, "Alice");
        assert_eq!(resource.properties.get("email").unwrap().as_str().unwrap(), "alice@example.com");
        assert!(resource.id.starts_with("test_sys:"));
    }

    #[test]
    fn test_constant_mapping() {
        let mut adapter = ResourceAdapter::new(
            "const_test",
            "sys",
            "item",
            "id",
            "name",
        );

        adapter.add_to_unified_mapping(FieldMappingType::Constant {
            target: "source".to_string(),
            value: "external".to_string(),
        });

        let source = json!({ "id": "1", "name": "Item 1" });
        let resource = adapter.to_unified(&source, "conn").unwrap();
        assert_eq!(
            resource.properties.get("source").unwrap().as_str().unwrap(),
            "external"
        );
    }

    #[test]
    fn test_template_mapping() {
        let mut adapter = ResourceAdapter::new(
            "tpl_test",
            "sys",
            "doc",
            "id",
            "title",
        );

        adapter.add_to_unified_mapping(FieldMappingType::Template {
            target: "full_name".to_string(),
            template: "{first_name} {last_name}".to_string(),
        });

        let source = json!({
            "id": "1",
            "title": "Doc",
            "first_name": "John",
            "last_name": "Doe"
        });

        let resource = adapter.to_unified(&source, "conn").unwrap();
        assert_eq!(
            resource.properties.get("full_name").unwrap().as_str().unwrap(),
            "John Doe"
        );
    }

    #[test]
    fn test_transform_mapping() {
        let mut adapter = ResourceAdapter::new(
            "transform_test",
            "sys",
            "item",
            "id",
            "name",
        );

        adapter.add_to_unified_mapping(FieldMappingType::Transform {
            source: "name".to_string(),
            target: "name_upper".to_string(),
            transform: TransformFn::ToUpperCase,
        });

        let source = json!({ "id": "1", "name": "hello" });
        let resource = adapter.to_unified(&source, "conn").unwrap();
        assert_eq!(
            resource.properties.get("name_upper").unwrap().as_str().unwrap(),
            "HELLO"
        );
    }

    #[test]
    fn test_from_unified() {
        let mut adapter = ResourceAdapter::new(
            "reverse_test",
            "sys",
            "user",
            "external_id",
            "display_name",
        );

        adapter.add_from_unified_mapping(FieldMappingType::Direct {
            source: "email".to_string(),
            target: "user_email".to_string(),
        });

        let resource = UnifiedResource {
            id: "sys:123".to_string(),
            resource_type: "user".to_string(),
            name: "Alice".to_string(),
            connector_id: "conn-1".to_string(),
            external_id: "123".to_string(),
            properties: {
                let mut p = HashMap::new();
                p.insert("email".to_string(), Value::String("a@b.com".to_string()));
                p
            },
            status: "active".to_string(),
            created_at: 0,
            updated_at: 0,
            supported_operations: vec![],
        };

        let result = adapter.from_unified(&resource).unwrap();
        assert_eq!(result["external_id"], "123");
        assert_eq!(result["display_name"], "Alice");
        assert_eq!(result["user_email"], "a@b.com");
    }

    #[test]
    fn test_batch_conversion() {
        let adapter = ResourceAdapter::new("batch", "sys", "item", "id", "name");

        let items = vec![
            json!({ "id": "1", "name": "One" }),
            json!({ "id": "2", "name": "Two" }),
            json!({ "id": "3", "name": "Three" }),
        ];

        let results = adapter.to_unified_batch(&items, "conn").unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "One");
        assert_eq!(results[1].external_id, "2");
    }

    #[test]
    fn test_missing_id_field() {
        let adapter = ResourceAdapter::new("missing", "sys", "item", "id", "name");
        let source = json!({ "name": "No ID" });
        let result = adapter.to_unified(&source, "conn");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_field() {
        let adapter = ResourceAdapter::new(
            "status_test",
            "sys",
            "item",
            "id",
            "name",
        )
        .with_status_field("state");

        let source = json!({ "id": "1", "name": "Test", "state": "inactive" });
        let resource = adapter.to_unified(&source, "conn").unwrap();
        assert_eq!(resource.status, "inactive");
    }
}
