// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 低代码核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 字段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// 字符串
    String,
    /// 文本（长文本）
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
    /// 枚举
    Enum,
    /// 关联（引用其他实体）
    Reference,
    /// 文件
    File,
    /// 图片
    Image,
    /// JSON
    Json,
    /// 数组
    Array,
    /// 地理位置
    GeoPoint,
    /// 自动编号
    AutoNumber,
    /// 用户
    User,
    /// 部门
    Department,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Text => "text",
            FieldType::Integer => "integer",
            FieldType::Float => "float",
            FieldType::Boolean => "boolean",
            FieldType::Date => "date",
            FieldType::DateTime => "datetime",
            FieldType::Enum => "enum",
            FieldType::Reference => "reference",
            FieldType::File => "file",
            FieldType::Image => "image",
            FieldType::Json => "json",
            FieldType::Array => "array",
            FieldType::GeoPoint => "geo_point",
            FieldType::AutoNumber => "auto_number",
            FieldType::User => "user",
            FieldType::Department => "department",
        }
    }

    /// 是否是数值类型
    pub fn is_numeric(&self) -> bool {
        matches!(self, FieldType::Integer | FieldType::Float)
    }

    /// 是否是文本类型
    pub fn is_text(&self) -> bool {
        matches!(self, FieldType::String | FieldType::Text)
    }
}

/// 数据类型（运行时值的类型）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DataType {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<DataType>),
    Object(HashMap<String, DataType>),
}

impl DataType {
    /// 获取类型名
    pub fn type_name(&self) -> &'static str {
        match self {
            DataType::String(_) => "string",
            DataType::Integer(_) => "integer",
            DataType::Float(_) => "float",
            DataType::Boolean(_) => "boolean",
            DataType::Null => "null",
            DataType::Array(_) => "array",
            DataType::Object(_) => "object",
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataType::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DataType::Integer(i) => Some(*i),
            DataType::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// 转换为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            DataType::Float(f) => Some(*f),
            DataType::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转换为布尔
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataType::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// 是否为空
    pub fn is_null(&self) -> bool {
        matches!(self, DataType::Null)
    }
}

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        DataType::String(s.to_string())
    }
}

impl From<String> for DataType {
    fn from(s: String) -> Self {
        DataType::String(s)
    }
}

impl From<i64> for DataType {
    fn from(i: i64) -> Self {
        DataType::Integer(i)
    }
}

impl From<f64> for DataType {
    fn from(f: f64) -> Self {
        DataType::Float(f)
    }
}

impl From<bool> for DataType {
    fn from(b: bool) -> Self {
        DataType::Boolean(b)
    }
}

/// 验证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationType {
    /// 必填
    Required,
    /// 最小长度
    MinLength,
    /// 最大长度
    MaxLength,
    /// 最小值
    MinValue,
    /// 最大值
    MaxValue,
    /// 正则匹配
    Pattern,
    /// 邮箱格式
    Email,
    /// 手机号格式
    Phone,
    /// URL 格式
    Url,
    /// 唯一性
    Unique,
    /// 自定义表达式
    Custom,
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// 规则类型
    pub rule_type: ValidationType,
    /// 规则参数
    pub params: HashMap<String, String>,
    /// 错误消息
    pub message: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

impl ValidationRule {
    /// 创建必填规则
    pub fn required() -> Self {
        Self {
            rule_type: ValidationType::Required,
            params: HashMap::new(),
            message: Some("此字段为必填项".to_string()),
            enabled: true,
        }
    }

    /// 创建最小长度规则
    pub fn min_length(min: usize) -> Self {
        let mut params = HashMap::new();
        params.insert("min".to_string(), min.to_string());
        Self {
            rule_type: ValidationType::MinLength,
            params,
            message: Some(format!("最少需要 {} 个字符", min)),
            enabled: true,
        }
    }

    /// 创建最大长度规则
    pub fn max_length(max: usize) -> Self {
        let mut params = HashMap::new();
        params.insert("max".to_string(), max.to_string());
        Self {
            rule_type: ValidationType::MaxLength,
            params,
            message: Some(format!("最多允许 {} 个字符", max)),
            enabled: true,
        }
    }

    /// 创建正则规则
    pub fn pattern(pattern: &str, message: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("pattern".to_string(), pattern.to_string());
        Self {
            rule_type: ValidationType::Pattern,
            params,
            message: Some(message.to_string()),
            enabled: true,
        }
    }

    /// 创建自定义规则
    pub fn custom(expression: &str, message: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("expression".to_string(), expression.to_string());
        Self {
            rule_type: ValidationType::Custom,
            params,
            message: Some(message.to_string()),
            enabled: true,
        }
    }
}

/// 字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// 字段 ID
    pub id: String,
    /// 字段名（API 名称，英文）
    pub name: String,
    /// 字段显示名
    pub label: String,
    /// 字段类型
    pub field_type: FieldType,
    /// 是否必填
    pub required: bool,
    /// 是否唯一
    pub unique: bool,
    /// 默认值表达式
    pub default_value: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 验证规则
    pub validations: Vec<ValidationRule>,
    /// 额外配置
    pub config: HashMap<String, serde_json::Value>,
    /// 关联实体 ID（引用类型用）
    pub reference_entity_id: Option<String>,
    /// 枚举选项
    pub enum_options: Vec<EnumOption>,
    /// 是否在列表中显示
    pub show_in_list: bool,
    /// 是否在表单中显示
    pub show_in_form: bool,
    /// 是否可搜索
    pub searchable: bool,
    /// 排序号
    pub sort_order: u32,
    /// 是否系统字段
    pub is_system: bool,
}

/// 枚举选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumOption {
    pub value: String,
    pub label: String,
    pub color: Option<String>,
    pub disabled: bool,
}

impl FieldDef {
    /// 创建字段定义
    pub fn new(name: &str, label: &str, field_type: FieldType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            label: label.to_string(),
            field_type,
            required: false,
            unique: false,
            default_value: None,
            description: None,
            validations: Vec::new(),
            config: HashMap::new(),
            reference_entity_id: None,
            enum_options: Vec::new(),
            show_in_list: true,
            show_in_form: true,
            searchable: false,
            sort_order: 0,
            is_system: false,
        }
    }

    /// 设置必填
    pub fn required(mut self) -> Self {
        self.required = true;
        self.validations.push(ValidationRule::required());
        self
    }

    /// 添加验证规则
    pub fn with_validation(mut self, rule: ValidationRule) -> Self {
        self.validations.push(rule);
        self
    }

    /// 设置枚举选项
    pub fn with_options(mut self, options: Vec<(String, String)>) -> Self {
        self.enum_options = options
            .into_iter()
            .map(|(value, label)| EnumOption {
                value,
                label,
                color: None,
                disabled: false,
            })
            .collect();
        self
    }

    /// 设置为系统字段
    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }
}

/// 关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// 一对多
    OneToMany,
    /// 多对一
    ManyToOne,
    /// 一对一
    OneToOne,
    /// 多对多
    ManyToMany,
}

/// 关系定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDef {
    /// 关系 ID
    pub id: String,
    /// 关系名称
    pub name: String,
    /// 关系类型
    pub relation_type: RelationType,
    /// 源实体 ID
    pub source_entity_id: String,
    /// 目标实体 ID
    pub target_entity_id: String,
    /// 源字段名
    pub source_field: String,
    /// 目标字段名
    pub target_field: String,
    /// 外键字段
    pub foreign_key_field: Option<String>,
    /// 是否级联删除
    pub cascade_delete: bool,
    /// 描述
    pub description: Option<String>,
}

impl RelationDef {
    /// 创建关系定义
    pub fn new(
        name: &str,
        relation_type: RelationType,
        source_entity_id: &str,
        target_entity_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            relation_type,
            source_entity_id: source_entity_id.to_string(),
            target_entity_id: target_entity_id.to_string(),
            source_field: String::new(),
            target_field: String::new(),
            foreign_key_field: None,
            cascade_delete: false,
            description: None,
        }
    }
}

/// 实体定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    /// 实体 ID
    pub id: String,
    /// 实体名（API 名称，英文）
    pub name: String,
    /// 实体显示名
    pub label: String,
    /// 所属模块
    pub module: String,
    /// 描述
    pub description: Option<String>,
    /// 字段列表
    pub fields: Vec<FieldDef>,
    /// 主键字段名
    pub primary_key: String,
    /// 名称字段（用于显示记录名）
    pub name_field: String,
    /// 是否启用审计
    pub audit_enabled: bool,
    /// 是否启用软删除
    pub soft_delete: bool,
    /// 是否系统实体
    pub is_system: bool,
    /// 权限配置
    pub permission_config: HashMap<String, String>,
    /// 索引配置
    pub indexes: Vec<IndexDef>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 索引定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDef {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

impl EntityDef {
    /// 创建实体定义
    pub fn new(name: &str, label: &str, module: &str) -> Self {
        let now = now_ms();
        let mut entity = Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            label: label.to_string(),
            module: module.to_string(),
            description: None,
            fields: Vec::new(),
            primary_key: "id".to_string(),
            name_field: "name".to_string(),
            audit_enabled: true,
            soft_delete: true,
            is_system: false,
            permission_config: HashMap::new(),
            indexes: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        // 添加系统字段
        entity.add_system_field(FieldDef::new("id", "ID", FieldType::String).system());
        entity.add_system_field(FieldDef::new("name", "名称", FieldType::String).system());
        entity.add_system_field(FieldDef::new("created_at", "创建时间", FieldType::DateTime).system());
        entity.add_system_field(FieldDef::new("updated_at", "更新时间", FieldType::DateTime).system());

        entity
    }

    /// 添加字段
    pub fn add_field(&mut self, field: FieldDef) {
        self.fields.push(field);
        self.updated_at = now_ms();
    }

    /// 添加系统字段
    fn add_system_field(&mut self, mut field: FieldDef) {
        field.is_system = true;
        self.fields.push(field);
    }

    /// 按名称获取字段
    pub fn get_field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// 获取可见字段（列表用）
    pub fn list_visible_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.show_in_list).collect()
    }

    /// 获取表单字段
    pub fn form_visible_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.show_in_form).collect()
    }
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type() {
        assert!(FieldType::Integer.is_numeric());
        assert!(FieldType::String.is_text());
        assert!(!FieldType::Boolean.is_numeric());
        assert_eq!(FieldType::String.as_str(), "string");
    }

    #[test]
    fn test_data_type_conversions() {
        let s = DataType::from("hello");
        assert_eq!(s.as_str(), Some("hello"));
        assert_eq!(s.type_name(), "string");

        let i = DataType::from(42i64);
        assert_eq!(i.as_integer(), Some(42));
        assert_eq!(i.as_float(), Some(42.0));

        let b = DataType::from(true);
        assert_eq!(b.as_bool(), Some(true));

        assert!(DataType::Null.is_null());
    }

    #[test]
    fn test_validation_rules() {
        let required = ValidationRule::required();
        assert_eq!(required.rule_type, ValidationType::Required);
        assert!(required.enabled);

        let min_len = ValidationRule::min_length(5);
        assert_eq!(min_len.params.get("min").unwrap(), "5");

        let max_len = ValidationRule::max_length(100);
        assert_eq!(max_len.params.get("max").unwrap(), "100");
    }

    #[test]
    fn test_field_def_builder() {
        let field = FieldDef::new("username", "用户名", FieldType::String)
            .required()
            .with_validation(ValidationRule::min_length(3))
            .with_validation(ValidationRule::max_length(50));

        assert_eq!(field.name, "username");
        assert!(field.required);
        assert_eq!(field.validations.len(), 3); // required + min + max
    }

    #[test]
    fn test_entity_def() {
        let mut entity = EntityDef::new("user", "用户", "system");
        assert!(entity.audit_enabled);
        assert!(entity.soft_delete);
        assert_eq!(entity.fields.len(), 4); // 4 个系统字段

        let email = FieldDef::new("email", "邮箱", FieldType::String).required();
        entity.add_field(email);

        assert_eq!(entity.fields.len(), 5);
        assert!(entity.get_field("email").is_some());
        assert!(entity.get_field("nonexist").is_none());
    }

    #[test]
    fn test_entity_visible_fields() {
        let mut entity = EntityDef::new("test", "Test", "test");

        let mut hidden = FieldDef::new("secret", "Secret", FieldType::String);
        hidden.show_in_list = false;
        hidden.show_in_form = false;
        entity.add_field(hidden);

        assert!(entity.list_visible_fields().len() <= entity.fields.len());
        assert!(entity.form_visible_fields().len() <= entity.fields.len());
    }

    #[test]
    fn test_relation_def() {
        let rel = RelationDef::new(
            "user_orders",
            RelationType::OneToMany,
            "user-id",
            "order-id",
        );
        assert_eq!(rel.name, "user_orders");
        assert_eq!(rel.relation_type, RelationType::OneToMany);
    }

    #[test]
    fn test_enum_options() {
        let field = FieldDef::new("status", "状态", FieldType::Enum).with_options(vec![
            ("active".to_string(), "活跃".to_string()),
            ("inactive".to_string(), "停用".to_string()),
        ]);

        assert_eq!(field.enum_options.len(), 2);
        assert_eq!(field.enum_options[0].value, "active");
        assert_eq!(field.enum_options[1].label, "停用");
    }
}
