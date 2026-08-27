//! 动态字段 — 政企自定义业务字段扩展
//!
//! 允许政企在不修改代码的情况下，为业务实体添加自定义字段。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// 字段类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DynamicFieldType {
    /// 短文本
    String,
    /// 长文本
    Text,
    /// 整数
    Integer,
    /// 浮点数
    Float,
    /// 布尔
    Boolean,
    /// 日期
    Date,
    /// 日期时间
    DateTime,
    /// 单选
    Select,
    /// 多选
    MultiSelect,
    /// 附件
    Attachment,
    /// 图片
    Image,
    /// 用户选择器
    User,
    /// 部门选择器
    Department,
    /// JSON
    Json,
}

impl DynamicFieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DynamicFieldType::String => "string",
            DynamicFieldType::Text => "text",
            DynamicFieldType::Integer => "integer",
            DynamicFieldType::Float => "float",
            DynamicFieldType::Boolean => "boolean",
            DynamicFieldType::Date => "date",
            DynamicFieldType::DateTime => "datetime",
            DynamicFieldType::Select => "select",
            DynamicFieldType::MultiSelect => "multi_select",
            DynamicFieldType::Attachment => "attachment",
            DynamicFieldType::Image => "image",
            DynamicFieldType::User => "user",
            DynamicFieldType::Department => "department",
            DynamicFieldType::Json => "json",
        }
    }
}

/// 字段选项（用于select/multi_select）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// 动态字段Schema定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicFieldSchema {
    /// 字段唯一标识（英文，用于代码引用）
    pub key: String,
    /// 字段显示名称
    pub label: String,
    /// 字段类型
    pub field_type: DynamicFieldType,
    /// 是否必填
    #[serde(default)]
    pub required: bool,
    /// 是否唯一
    #[serde(default)]
    pub unique: bool,
    /// 默认值
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    /// 占位提示
    #[serde(default)]
    pub placeholder: Option<String>,
    /// 帮助文本
    #[serde(default)]
    pub help_text: Option<String>,
    /// 选项列表（select/multi_select）
    #[serde(default)]
    pub options: Vec<FieldOption>,
    /// 最小值（数字/日期）
    #[serde(default)]
    pub min: Option<f64>,
    /// 最大值（数字/日期）
    #[serde(default)]
    pub max: Option<f64>,
    /// 最大长度（文本）
    #[serde(default)]
    pub max_length: Option<usize>,
    /// 正则校验
    #[serde(default)]
    pub pattern: Option<String>,
    /// 是否在列表中显示
    #[serde(default = "default_true")]
    pub show_in_list: bool,
    /// 是否在表单中显示
    #[serde(default = "default_true")]
    pub show_in_form: bool,
    /// 是否在详情中显示
    #[serde(default = "default_true")]
    pub show_in_detail: bool,
    /// 排序权重（越小越靠前）
    #[serde(default)]
    pub sort_order: u32,
    /// 字段分组
    #[serde(default)]
    pub group: Option<String>,
}

fn default_true() -> bool { true }

/// 动态字段值（运行时数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicFieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<serde_json::Value>),
    Object(serde_json::Value),
    Null,
}

impl DynamicFieldValue {
    pub fn is_null(&self) -> bool {
        matches!(self, DynamicFieldValue::Null)
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            DynamicFieldValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DynamicFieldValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DynamicFieldValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DynamicFieldValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl Default for DynamicFieldValue {
    fn default() -> Self { DynamicFieldValue::Null }
}

/// 动态字段Schema管理器 — 按租户+实体类型管理字段定义
pub struct DynamicFieldManager {
    /// (tenant_id, entity_type) -> Vec<schema>
    schemas: RwLock<HashMap<(String, String), Vec<DynamicFieldSchema>>>,
}

impl DynamicFieldManager {
    pub fn new() -> Self {
        Self { schemas: RwLock::new(HashMap::new()) }
    }

    /// 注册字段
    pub fn register(&self, tenant_id: &str, entity_type: &str, schema: DynamicFieldSchema) {
        let key = (tenant_id.into(), entity_type.into());
        let mut schemas = self.schemas.write();
        let list = schemas.entry(key).or_default();
        // 同key覆盖
        if let Some(pos) = list.iter().position(|s| s.key == schema.key) {
            list[pos] = schema;
        } else {
            list.push(schema);
        }
    }

    /// 批量注册
    pub fn register_batch(&self, tenant_id: &str, entity_type: &str, schemas: Vec<DynamicFieldSchema>) {
        for schema in schemas {
            self.register(tenant_id, entity_type, schema);
        }
    }

    /// 获取实体的所有字段Schema
    pub fn get_schemas(&self, tenant_id: &str, entity_type: &str) -> Vec<DynamicFieldSchema> {
        let key = (tenant_id.into(), entity_type.into());
        self.schemas.read().get(&key).cloned().unwrap_or_default()
    }

    /// 获取单个字段Schema
    pub fn get_schema(&self, tenant_id: &str, entity_type: &str, key: &str) -> Option<DynamicFieldSchema> {
        self.get_schemas(tenant_id, entity_type).into_iter().find(|s| s.key == key)
    }

    /// 删除字段
    pub fn remove(&self, tenant_id: &str, entity_type: &str, key: &str) -> bool {
        let key_tuple = (tenant_id.into(), entity_type.into());
        let mut schemas = self.schemas.write();
        if let Some(list) = schemas.get_mut(&key_tuple) {
            if let Some(pos) = list.iter().position(|s| s.key == key) {
                list.remove(pos);
                return true;
            }
        }
        false
    }

    /// 校验字段值
    pub fn validate_value(
        &self,
        tenant_id: &str,
        entity_type: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let schema = self.get_schema(tenant_id, entity_type, key)
            .ok_or_else(|| format!("field not found: {}", key))?;

        // 必填校验
        if schema.required && value.is_null() {
            return Err(format!("field {} is required", key));
        }

        // 类型校验（简化）
        match schema.field_type {
            DynamicFieldType::String | DynamicFieldType::Text => {
                if !value.is_null() && !value.is_string() {
                    return Err(format!("field {} must be string", key));
                }
                if let (Some(max), Some(s)) = (schema.max_length, value.as_str()) {
                    if s.chars().count() > max {
                        return Err(format!("field {} exceeds max length {}", key, max));
                    }
                }
            }
            DynamicFieldType::Integer => {
                if !value.is_null() && !value.is_i64() {
                    return Err(format!("field {} must be integer", key));
                }
            }
            DynamicFieldType::Float => {
                if !value.is_null() && !value.is_number() {
                    return Err(format!("field {} must be number", key));
                }
            }
            DynamicFieldType::Boolean => {
                if !value.is_null() && !value.is_boolean() {
                    return Err(format!("field {} must be boolean", key));
                }
            }
            _ => {} // 其他类型简化处理
        }

        Ok(())
    }

    /// 已注册字段总数
    pub fn total_fields(&self) -> usize {
        self.schemas.read().values().map(|v| v.len()).sum()
    }
}

impl Default for DynamicFieldManager {
    fn default() -> Self { Self::new() }
}
