// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 表单引擎
//!
//! 动态表单生成、验证、联动逻辑。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{LowcodeError, LowcodeResult};
use crate::expression::ExpressionEvaluator;
use crate::types::{DataType, FieldDef, FieldType, ValidationRule, ValidationType, now_ms};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 表单字段组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormWidgetType {
    /// 文本输入
    Input,
    /// 文本域
    Textarea,
    /// 数字输入
    Number,
    /// 开关
    Switch,
    /// 下拉选择
    Select,
    /// 单选框组
    Radio,
    /// 多选框组
    Checkbox,
    /// 日期选择
    DatePicker,
    /// 日期时间选择
    DateTimePicker,
    /// 文件上传
    Upload,
    /// 图片上传
    ImageUpload,
    /// 富文本
    RichText,
    /// 级联选择
    Cascader,
    /// 关联选择
    ReferenceSelect,
    /// 自动完成
    AutoComplete,
    /// 评分
    Rate,
    /// 滑块
    Slider,
    /// 颜色选择
    ColorPicker,
    /// 只读展示
    Display,
}

/// 表单字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// 字段 ID
    pub id: String,
    /// 字段名（对应数据字段）
    pub name: String,
    /// 显示标签
    pub label: String,
    /// 组件类型
    pub widget_type: FormWidgetType,
    /// 字段类型
    pub field_type: FieldType,
    /// 占位符
    pub placeholder: Option<String>,
    /// 默认值表达式
    pub default_value: Option<String>,
    /// 是否必填
    pub required: bool,
    /// 是否禁用
    pub disabled: bool,
    /// 是否只读
    pub readonly: bool,
    /// 是否隐藏
    pub hidden: bool,
    /// 显示条件表达式
    pub visible_on: Option<String>,
    /// 禁用条件表达式
    pub disable_on: Option<String>,
    /// 必填条件表达式
    pub required_on: Option<String>,
    /// 验证规则
    pub validations: Vec<ValidationRule>,
    /// 联动规则（字段变化时触发的动作）
    pub on_change_actions: Vec<FieldAction>,
    /// 组件配置
    pub widget_config: HashMap<String, serde_json::Value>,
    /// 栅格宽度（1-24）
    pub span: u8,
    /// 排序号
    pub sort_order: u32,
    /// 提示信息
    pub tooltip: Option<String>,
    /// 帮助文本
    pub help_text: Option<String>,
}

/// 字段动作（联动）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAction {
    /// 动作类型
    pub action_type: FieldActionType,
    /// 目标字段
    pub target_field: String,
    /// 值表达式
    pub value_expression: Option<String>,
    /// 条件表达式
    pub condition: Option<String>,
}

/// 字段动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldActionType {
    /// 设置值
    SetValue,
    /// 设置可见
    SetVisible,
    /// 设置禁用
    SetDisabled,
    /// 设置必填
    SetRequired,
    /// 触发验证
    Validate,
    /// 清空值
    ClearValue,
}

/// 表单布局类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormLayout {
    /// 水平布局
    Horizontal,
    /// 垂直布局
    Vertical,
    /// 内联布局
    Inline,
}

/// 表单 Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    /// 表单 ID
    pub id: String,
    /// 表单名称
    pub name: String,
    /// 关联实体 ID
    pub entity_id: Option<String>,
    /// 布局
    pub layout: FormLayout,
    /// 列数
    pub columns: u8,
    /// 标签宽度
    pub label_width: u16,
    /// 字段列表
    pub fields: Vec<FormField>,
    /// 表单级验证规则
    pub form_validations: Vec<String>,
    /// 提交按钮文本
    pub submit_text: String,
    /// 重置按钮文本
    pub reset_text: String,
    /// 是否显示重置按钮
    pub show_reset: bool,
    /// 描述
    pub description: Option<String>,
}

impl FormSchema {
    /// 创建表单 Schema
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            entity_id: None,
            layout: FormLayout::Horizontal,
            columns: 2,
            label_width: 100,
            fields: Vec::new(),
            form_validations: Vec::new(),
            submit_text: "提交".to_string(),
            reset_text: "重置".to_string(),
            show_reset: true,
            description: None,
        }
    }

    /// 从实体定义生成表单
    pub fn from_entity(entity: &crate::types::EntityDef) -> Self {
        let mut form = Self::new(&entity.label);
        form.entity_id = Some(entity.id.clone());
        form.description = entity.description.clone();

        for field_def in &entity.fields {
            if field_def.show_in_form && !field_def.is_system {
                form.fields.push(FormField::from_field_def(field_def));
            }
        }

        // 按 sort_order 排序
        form.fields.sort_by_key(|f| f.sort_order);
        form
    }
}

impl FormField {
    /// 从字段定义创建表单字段
    pub fn from_field_def(field_def: &FieldDef) -> Self {
        let widget_type = match field_def.field_type {
            FieldType::String => FormWidgetType::Input,
            FieldType::Text => FormWidgetType::Textarea,
            FieldType::Integer | FieldType::Float => FormWidgetType::Number,
            FieldType::Boolean => FormWidgetType::Switch,
            FieldType::Enum => FormWidgetType::Select,
            FieldType::Date => FormWidgetType::DatePicker,
            FieldType::DateTime => FormWidgetType::DateTimePicker,
            FieldType::Reference => FormWidgetType::ReferenceSelect,
            FieldType::File => FormWidgetType::Upload,
            FieldType::Image => FormWidgetType::ImageUpload,
            _ => FormWidgetType::Input,
        };

        Self {
            id: field_def.id.clone(),
            name: field_def.name.clone(),
            label: field_def.label.clone(),
            widget_type,
            field_type: field_def.field_type,
            placeholder: None,
            default_value: field_def.default_value.clone(),
            required: field_def.required,
            disabled: false,
            readonly: false,
            hidden: false,
            visible_on: None,
            disable_on: None,
            required_on: None,
            validations: field_def.validations.clone(),
            on_change_actions: Vec::new(),
            widget_config: HashMap::new(),
            span: 12,
            sort_order: field_def.sort_order,
            tooltip: None,
            help_text: field_def.description.clone(),
        }
    }
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormValidationResult {
    /// 是否通过
    pub valid: bool,
    /// 字段错误：字段名 -> 错误消息列表
    pub field_errors: HashMap<String, Vec<String>>,
    /// 表单级错误
    pub form_errors: Vec<String>,
    /// 验证时间
    pub validated_at: u64,
}

impl FormValidationResult {
    /// 创建通过结果
    pub fn success() -> Self {
        Self {
            valid: true,
            field_errors: HashMap::new(),
            form_errors: Vec::new(),
            validated_at: now_ms(),
        }
    }

    /// 添加字段错误
    pub fn add_field_error(&mut self, field: &str, message: &str) {
        self.valid = false;
        self.field_errors
            .entry(field.to_string())
            .or_default()
            .push(message.to_string());
    }

    /// 添加表单错误
    pub fn add_form_error(&mut self, message: &str) {
        self.valid = false;
        self.form_errors.push(message.to_string());
    }

    /// 获取错误总数
    pub fn error_count(&self) -> usize {
        let field_count: usize = self.field_errors.values().map(|v| v.len()).sum();
        field_count + self.form_errors.len()
    }
}

/// 表单引擎
pub struct FormEngine {
    /// 表单 Schema 缓存
    schemas: RwLock<HashMap<String, FormSchema>>,
}

impl FormEngine {
    /// 创建表单引擎
    pub fn new() -> Self {
        Self {
            schemas: RwLock::new(HashMap::new()),
        }
    }

    /// 注册表单 Schema
    pub fn register_schema(&self, schema: FormSchema) {
        self.schemas
            .write()
            .insert(schema.id.clone(), schema);
    }

    /// 获取表单 Schema
    pub fn get_schema(&self, schema_id: &str) -> LowcodeResult<FormSchema> {
        self.schemas
            .read()
            .get(schema_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("form schema '{}' not found", schema_id)))
    }

    /// 验证表单数据
    pub fn validate(
        &self,
        schema: &FormSchema,
        data: &HashMap<String, DataType>,
    ) -> FormValidationResult {
        let mut result = FormValidationResult::success();

        for field in &schema.fields {
            if field.hidden {
                continue; // 隐藏字段不验证
            }

            let value = data.get(&field.name);

            // 必填验证
            if field.required {
                let is_empty = match value {
                    Some(DataType::Null) | None => true,
                    Some(DataType::String(s)) => s.is_empty(),
                    Some(DataType::Array(arr)) => arr.is_empty(),
                    _ => false,
                };
                if is_empty {
                    result.add_field_error(
                        &field.name,
                        &format!("{}是必填项", field.label),
                    );
                    continue;
                }
            }

            // 如果值为空且非必填，跳过其他验证
            let value = match value {
                Some(v) if !v.is_null() => v,
                _ => continue,
            };

            // 运行验证规则
            for rule in &field.validations {
                if !rule.enabled {
                    continue;
                }
                if let Some(err) = validate_field(field, value, rule, data) {
                    result.add_field_error(
                        &field.name,
                        &rule.message.clone().unwrap_or(err),
                    );
                }
            }
        }

        // 表单级验证
        for expr in &schema.form_validations {
            match ExpressionEvaluator::evaluate_bool(expr, data) {
                Ok(false) => {
                    result.add_form_error(expr);
                }
                Err(e) => {
                    result.add_form_error(&format!("表达式错误: {}", e));
                }
                _ => {}
            }
        }

        result
    }

    /// 计算默认值
    pub fn compute_defaults(
        &self,
        schema: &FormSchema,
        context: &HashMap<String, DataType>,
    ) -> HashMap<String, DataType> {
        let mut defaults = HashMap::new();

        for field in &schema.fields {
            if let Some(expr) = &field.default_value {
                // 先尝试作为表达式求值
                if let Ok(value) = ExpressionEvaluator::evaluate(expr, context) {
                    defaults.insert(field.name.clone(), value);
                }
            }
        }

        defaults
    }

    /// 应用字段联动
    pub fn apply_field_changes(
        &self,
        schema: &FormSchema,
        changed_field: &str,
        data: &mut HashMap<String, DataType>,
    ) -> Vec<String> {
        let mut changed = Vec::new();

        for field in &schema.fields {
            if field.name == changed_field {
                for action in &field.on_change_actions {
                    // 检查条件
                    if let Some(cond) = &action.condition {
                        match ExpressionEvaluator::evaluate_bool(cond, data) {
                            Ok(true) => {}
                            _ => continue,
                        }
                    }

                    match action.action_type {
                        FieldActionType::SetValue => {
                            if let Some(expr) = &action.value_expression {
                                if let Ok(new_val) =
                                    ExpressionEvaluator::evaluate(expr, data)
                                {
                                    data.insert(action.target_field.clone(), new_val);
                                    changed.push(action.target_field.clone());
                                }
                            }
                        }
                        FieldActionType::ClearValue => {
                            data.insert(action.target_field.clone(), DataType::Null);
                            changed.push(action.target_field.clone());
                        }
                        _ => {
                            // 其他动作由前端处理
                        }
                    }
                }
            }
        }

        changed
    }

    /// 计算字段可见性
    pub fn compute_visibility(
        &self,
        schema: &FormSchema,
        data: &HashMap<String, DataType>,
    ) -> HashMap<String, bool> {
        let mut visibility = HashMap::new();

        for field in &schema.fields {
            let visible = if let Some(expr) = &field.visible_on {
                ExpressionEvaluator::evaluate_bool(expr, data).unwrap_or(true)
            } else {
                !field.hidden
            };
            visibility.insert(field.name.clone(), visible);
        }

        visibility
    }

    /// 表单总数
    pub fn schema_count(&self) -> usize {
        self.schemas.read().len()
    }
}

impl Default for FormEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证单个字段
fn validate_field(
    field: &FormField,
    value: &DataType,
    rule: &ValidationRule,
    _context: &HashMap<String, DataType>,
) -> Option<String> {
    match rule.rule_type {
        ValidationType::Required => None, // 已单独处理
        ValidationType::MinLength => {
            let min: usize = rule
                .params
                .get("min")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let len = value.as_str().map(|s| s.chars().count()).unwrap_or(0);
            if len < min {
                Some(format!("最少需要 {} 个字符", min))
            } else {
                None
            }
        }
        ValidationType::MaxLength => {
            let max: usize = rule
                .params
                .get("max")
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            let len = value.as_str().map(|s| s.chars().count()).unwrap_or(0);
            if len > max {
                Some(format!("最多允许 {} 个字符", max))
            } else {
                None
            }
        }
        ValidationType::MinValue => {
            let min = rule
                .params
                .get("min")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(f64::NEG_INFINITY);
            let val = value.as_float().unwrap_or(0.0);
            if val < min {
                Some(format!("不能小于 {}", min))
            } else {
                None
            }
        }
        ValidationType::MaxValue => {
            let max = rule
                .params
                .get("max")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(f64::INFINITY);
            let val = value.as_float().unwrap_or(0.0);
            if val > max {
                Some(format!("不能大于 {}", max))
            } else {
                None
            }
        }
        ValidationType::Pattern => {
            if let (Some(s), Some(pattern)) = (value.as_str(), rule.params.get("pattern")) {
                // 简单的正则匹配
                let re = regex_like_match(s, pattern);
                if !re {
                    Some("格式不正确".to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        ValidationType::Email => {
            if let Some(s) = value.as_str() {
                if is_email(s) {
                    None
                } else {
                    Some("请输入有效的邮箱地址".to_string())
                }
            } else {
                None
            }
        }
        ValidationType::Phone => {
            if let Some(s) = value.as_str() {
                if is_phone(s) {
                    None
                } else {
                    Some("请输入有效的手机号码".to_string())
                }
            } else {
                None
            }
        }
        ValidationType::Url => {
            if let Some(s) = value.as_str() {
                if is_url(s) {
                    None
                } else {
                    Some("请输入有效的URL".to_string())
                }
            } else {
                None
            }
        }
        ValidationType::Unique => None, // 需要后端验证
        ValidationType::Custom => {
            // 自定义表达式验证
            if let Some(expr) = rule.params.get("expression") {
                // 创建上下文，把当前值作为变量
                let mut ctx = HashMap::new();
                ctx.insert("value".to_string(), value.clone());
                ctx.insert(field.name.clone(), value.clone());

                match ExpressionEvaluator::evaluate_bool(expr, &ctx) {
                    Ok(true) => None,
                    Ok(false) => Some("验证失败".to_string()),
                    Err(e) => Some(format!("表达式错误: {}", e)),
                }
            } else {
                None
            }
        }
    }
}

/// 简单的邮箱验证
fn is_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    parts.len() == 2
        && !parts[0].is_empty()
        && !parts[1].is_empty()
        && parts[1].contains('.')
}

/// 简单的手机号验证（中国大陆）
fn is_phone(s: &str) -> bool {
    s.len() == 11 && s.starts_with('1') && s.chars().all(|c| c.is_ascii_digit())
}

/// 简单的 URL 验证
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// 简单的类正则匹配（只支持 * 通配符）
fn regex_like_match(s: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        let s_chars: Vec<char> = s.chars().collect();
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                if !s.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 {
                if !s.ends_with(part) {
                    return false;
                }
            } else {
                let rest = &s_chars[pos..];
                let part_chars: Vec<char> = part.chars().collect();
                if let Some(idx) = rest.windows(part_chars.len()).position(|w| w == part_chars.as_slice())
                {
                    pos += idx + part.len();
                } else {
                    return false;
                }
            }
        }
        true
    } else {
        s == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldType;

    fn create_test_form() -> FormSchema {
        let mut form = FormSchema::new("Test Form");
        form.layout = FormLayout::Vertical;
        form.columns = 1;

        let name_field = FormField {
            id: "f1".to_string(),
            name: "name".to_string(),
            label: "姓名".to_string(),
            widget_type: FormWidgetType::Input,
            field_type: FieldType::String,
            placeholder: Some("请输入姓名".to_string()),
            default_value: None,
            required: true,
            disabled: false,
            readonly: false,
            hidden: false,
            visible_on: None,
            disable_on: None,
            required_on: None,
            validations: vec![
                ValidationRule::min_length(2),
                ValidationRule::max_length(50),
            ],
            on_change_actions: Vec::new(),
            widget_config: HashMap::new(),
            span: 24,
            sort_order: 1,
            tooltip: None,
            help_text: None,
        };

        let age_field = FormField {
            id: "f2".to_string(),
            name: "age".to_string(),
            label: "年龄".to_string(),
            widget_type: FormWidgetType::Number,
            field_type: FieldType::Integer,
            placeholder: Some("请输入年龄".to_string()),
            default_value: None,
            required: true,
            disabled: false,
            readonly: false,
            hidden: false,
            visible_on: None,
            disable_on: None,
            required_on: None,
            validations: vec![
                ValidationRule {
                    rule_type: ValidationType::MinValue,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("min".to_string(), "0".to_string());
                        m
                    },
                    message: Some("年龄不能小于0".to_string()),
                    enabled: true,
                },
                ValidationRule {
                    rule_type: ValidationType::MaxValue,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("max".to_string(), "150".to_string());
                        m
                    },
                    message: Some("年龄不能大于150".to_string()),
                    enabled: true,
                },
            ],
            on_change_actions: Vec::new(),
            widget_config: HashMap::new(),
            span: 24,
            sort_order: 2,
            tooltip: None,
            help_text: None,
        };

        let email_field = FormField {
            id: "f3".to_string(),
            name: "email".to_string(),
            label: "邮箱".to_string(),
            widget_type: FormWidgetType::Input,
            field_type: FieldType::String,
            placeholder: Some("请输入邮箱".to_string()),
            default_value: None,
            required: false,
            disabled: false,
            readonly: false,
            hidden: false,
            visible_on: None,
            disable_on: None,
            required_on: None,
            validations: vec![ValidationRule {
                rule_type: ValidationType::Email,
                params: HashMap::new(),
                message: None,
                enabled: true,
            }],
            on_change_actions: Vec::new(),
            widget_config: HashMap::new(),
            span: 24,
            sort_order: 3,
            tooltip: None,
            help_text: None,
        };

        form.fields.push(name_field);
        form.fields.push(age_field);
        form.fields.push(email_field);
        form
    }

    #[test]
    fn test_form_schema_creation() {
        let form = create_test_form();
        assert_eq!(form.name, "Test Form");
        assert_eq!(form.fields.len(), 3);
    }

    #[test]
    fn test_validate_success() {
        let engine = FormEngine::new();
        let form = create_test_form();

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("张三".to_string()));
        data.insert("age".to_string(), DataType::Integer(25));
        data.insert("email".to_string(), DataType::String("test@example.com".to_string()));

        let result = engine.validate(&form, &data);
        assert!(result.valid);
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_validate_required_missing() {
        let engine = FormEngine::new();
        let form = create_test_form();

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("".to_string()));
        data.insert("age".to_string(), DataType::Integer(25));

        let result = engine.validate(&form, &data);
        assert!(!result.valid);
        assert!(result.field_errors.contains_key("name"));
    }

    #[test]
    fn test_validate_min_length() {
        let engine = FormEngine::new();
        let form = create_test_form();

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("A".to_string()));
        data.insert("age".to_string(), DataType::Integer(25));

        let result = engine.validate(&form, &data);
        assert!(!result.valid);
        assert!(result.field_errors.contains_key("name"));
    }

    #[test]
    fn test_validate_min_value() {
        let engine = FormEngine::new();
        let form = create_test_form();

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("张三".to_string()));
        data.insert("age".to_string(), DataType::Integer(-5));

        let result = engine.validate(&form, &data);
        assert!(!result.valid);
        assert!(result.field_errors.contains_key("age"));
    }

    #[test]
    fn test_validate_email() {
        let engine = FormEngine::new();
        let form = create_test_form();

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("张三".to_string()));
        data.insert("age".to_string(), DataType::Integer(25));
        data.insert("email".to_string(), DataType::String("invalid-email".to_string()));

        let result = engine.validate(&form, &data);
        assert!(!result.valid);
        assert!(result.field_errors.contains_key("email"));
    }

    #[test]
    fn test_validate_custom_expression() {
        let engine = FormEngine::new();
        let mut form = create_test_form();

        // 添加自定义验证：年龄必须大于18才能选"成人"类型
        let rule = ValidationRule::custom("value >= 18", "年龄必须满18岁");
        form.fields[1].validations.push(rule);

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("张三".to_string()));
        data.insert("age".to_string(), DataType::Integer(16));

        let result = engine.validate(&form, &data);
        assert!(!result.valid);
    }

    #[test]
    fn test_compute_visibility() {
        let engine = FormEngine::new();
        let mut form = create_test_form();

        // 邮箱字段只在年龄 >= 18 时显示
        form.fields[2].visible_on = Some("age >= 18".to_string());

        let mut data = HashMap::new();
        data.insert("age".to_string(), DataType::Integer(20));

        let visibility = engine.compute_visibility(&form, &data);
        assert_eq!(visibility.get("email"), Some(&true));

        let mut data2 = HashMap::new();
        data2.insert("age".to_string(), DataType::Integer(15));

        let visibility2 = engine.compute_visibility(&form, &data2);
        assert_eq!(visibility2.get("email"), Some(&false));
    }

    #[test]
    fn test_register_and_get_schema() {
        let engine = FormEngine::new();
        let form = create_test_form();
        let id = form.id.clone();

        engine.register_schema(form);
        assert_eq!(engine.schema_count(), 1);

        let got = engine.get_schema(&id).unwrap();
        assert_eq!(got.name, "Test Form");
    }

    #[test]
    fn test_field_changes() {
        let engine = FormEngine::new();
        let mut form = create_test_form();

        // 添加联动：年龄变化时，如果>60，name字段追加"（老年）"
        let action = FieldAction {
            action_type: FieldActionType::SetValue,
            target_field: "display_label".to_string(),
            value_expression: Some("name + ' (senior)'".to_string()),
            condition: Some("age > 60".to_string()),
        };
        form.fields[1].on_change_actions.push(action);

        // 添加 display_label 字段
        form.fields.push(FormField {
            id: "f4".to_string(),
            name: "display_label".to_string(),
            label: "显示标签".to_string(),
            widget_type: FormWidgetType::Display,
            field_type: FieldType::String,
            placeholder: None,
            default_value: None,
            required: false,
            disabled: false,
            readonly: true,
            hidden: false,
            visible_on: None,
            disable_on: None,
            required_on: None,
            validations: vec![],
            on_change_actions: vec![],
            widget_config: HashMap::new(),
            span: 24,
            sort_order: 4,
            tooltip: None,
            help_text: None,
        });

        let mut data = HashMap::new();
        data.insert("name".to_string(), DataType::String("张三".to_string()));
        data.insert("age".to_string(), DataType::Integer(65));

        let changed = engine.apply_field_changes(&form, "age", &mut data);
        assert!(changed.iter().any(|f| f == "display_label"));
    }

    #[test]
    fn test_is_email() {
        assert!(is_email("test@example.com"));
        assert!(!is_email("invalid"));
        assert!(!is_email("@example.com"));
    }

    #[test]
    fn test_is_phone() {
        assert!(is_phone("13800138000"));
        assert!(!is_phone("12345"));
        assert!(!is_phone("abcdefghijk"));
    }
}
