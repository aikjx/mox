//! 表单设计器模型
//!
//! 支持低代码动态表单设计，包含多种表单字段类型、校验规则、联动逻辑

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 表单定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDefinition {
    /// 表单 ID
    pub form_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 表单名称
    pub name: String,
    /// 表单编码
    pub code: String,
    /// 表单分类：hr / finance / business / custom
    pub category: String,
    /// 表单描述
    pub description: Option<String>,
    /// 表单版本
    pub version: i32,
    /// 状态：draft / published / disabled
    pub status: String,
    /// 表单字段列表
    pub fields: Vec<FormField>,
    /// 表单布局配置
    pub layout: FormLayout,
    /// 表单联动规则
    pub linkages: Vec<FormLinkage>,
    /// 表单校验规则
    pub validations: Vec<FormValidation>,
    /// 关联的流程定义 ID
    pub process_id: Option<String>,
    /// 创建人
    pub created_by: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 发布时间
    pub published_at: Option<String>,
}

/// 表单字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// 字段 ID
    pub field_id: String,
    /// 字段编码（提交时的 key）
    pub field_code: String,
    /// 字段标签（显示名称）
    pub label: String,
    /// 字段类型
    pub field_type: FormFieldType,
    /// 占位提示
    pub placeholder: Option<String>,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
    /// 是否必填
    pub required: bool,
    /// 是否只读
    pub readonly: bool,
    /// 是否隐藏
    pub hidden: bool,
    /// 字段宽度（1-24栅格系统）
    pub width: i32,
    /// 字段位置（行/列）
    pub position: FieldPosition,
    /// 字段配置（根据类型不同）
    pub config: FieldConfig,
    /// 校验规则
    pub validation_rules: Vec<ValidationRule>,
    /// 描述/帮助文本
    pub help_text: Option<String>,
}

/// 字段类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FormFieldType {
    /// 单行文本
    Input,
    /// 多行文本
    Textarea,
    /// 数字
    Number,
    /// 金额
    Money,
    /// 百分比
    Percent,
    /// 日期
    Date,
    /// 日期时间
    DateTime,
    /// 时间
    Time,
    /// 单选下拉
    Select,
    /// 多选下拉
    MultiSelect,
    /// 单选框
    Radio,
    /// 多选框
    Checkbox,
    /// 开关
    Switch,
    /// 评分
    Rate,
    /// 滑块
    Slider,
    /// 颜色选择
    Color,
    /// 图片上传
    ImageUpload,
    /// 文件上传
    FileUpload,
    /// 富文本
    RichText,
    /// 签名
    Signature,
    /// 地址选择
    Address,
    /// 手机
    Phone,
    /// 邮箱
    Email,
    /// 身份证
    IdCard,
    /// 部门选择
    DeptSelect,
    /// 用户选择
    UserSelect,
    /// 关联表单
    RelationForm,
    /// 子表单（表格）
    SubForm,
    /// 分割线
    Divider,
    /// 标题
    Title,
    /// 描述文本
    Description,
    /// 计算字段
    Calculated,
}

impl FormFieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Textarea => "textarea",
            Self::Number => "number",
            Self::Money => "money",
            Self::Percent => "percent",
            Self::Date => "date",
            Self::DateTime => "datetime",
            Self::Time => "time",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Radio => "radio",
            Self::Checkbox => "checkbox",
            Self::Switch => "switch",
            Self::Rate => "rate",
            Self::Slider => "slider",
            Self::Color => "color",
            Self::ImageUpload => "image_upload",
            Self::FileUpload => "file_upload",
            Self::RichText => "rich_text",
            Self::Signature => "signature",
            Self::Address => "address",
            Self::Phone => "phone",
            Self::Email => "email",
            Self::IdCard => "id_card",
            Self::DeptSelect => "dept_select",
            Self::UserSelect => "user_select",
            Self::RelationForm => "relation_form",
            Self::SubForm => "sub_form",
            Self::Divider => "divider",
            Self::Title => "title",
            Self::Description => "description",
            Self::Calculated => "calculated",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::Input | Self::Textarea | Self::RichText => "文本",
            Self::Number | Self::Money | Self::Percent | Self::Calculated => "数字",
            Self::Date | Self::DateTime | Self::Time => "日期时间",
            Self::Select | Self::MultiSelect | Self::Radio | Self::Checkbox | Self::Switch | Self::Rate | Self::Slider | Self::Color => "选择",
            Self::ImageUpload | Self::FileUpload | Self::Signature => "上传",
            Self::Address | Self::Phone | Self::Email | Self::IdCard | Self::DeptSelect | Self::UserSelect | Self::RelationForm => "关联",
            Self::SubForm => "高级",
            Self::Divider | Self::Title | Self::Description => "布局",
        }
    }
}

/// 字段位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPosition {
    pub row: i32,
    pub col: i32,
}

/// 字段配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldConfig {
    /// 选项列表（select/radio/checkbox 使用）
    pub options: Option<Vec<FieldOption>>,
    /// 最小值（number/slider 使用）
    pub min: Option<f64>,
    /// 最大值
    pub max: Option<f64>,
    /// 步长
    pub step: Option<f64>,
    /// 小数位数
    pub precision: Option<i32>,
    /// 货币单位
    pub currency: Option<String>,
    /// 日期格式
    pub date_format: Option<String>,
    /// 最大长度
    pub max_length: Option<i32>,
    /// 最小长度
    pub min_length: Option<i32>,
    /// 最大文件数
    pub max_files: Option<i32>,
    /// 允许的文件类型
    pub allowed_file_types: Option<Vec<String>>,
    /// 最大文件大小（MB）
    pub max_file_size: Option<f64>,
    /// 是否多选（用户/部门选择）
    pub multiple: Option<bool>,
    /// 关联表单 ID
    pub relation_form_id: Option<String>,
    /// 关联显示字段
    pub relation_display_field: Option<String>,
    /// 子表单字段
    pub sub_fields: Option<Vec<FormField>>,
    /// 计算表达式
    pub calc_expression: Option<String>,
    /// 自动填充配置
    pub auto_fill: Option<AutoFillConfig>,
}

/// 字段选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub label: String,
    pub value: String,
    pub color: Option<String>,
    pub disabled: Option<bool>,
}

/// 自动填充配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoFillConfig {
    /// 数据源：user_info / dept_info / form_field
    pub source: String,
    /// 源字段
    pub source_field: String,
}

/// 表单布局
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormLayout {
    /// 布局类型：grid / flex / table
    pub layout_type: String,
    /// 列数（grid 布局使用）
    pub columns: i32,
    /// 字段间距
    pub gutter: i32,
    /// 标签位置：left / top / right
    pub label_position: String,
    /// 标签宽度
    pub label_width: i32,
    /// 是否显示字段序号
    pub show_index: bool,
}

impl Default for FormLayout {
    fn default() -> Self {
        Self {
            layout_type: "grid".to_string(),
            columns: 2,
            gutter: 16,
            label_position: "left".to_string(),
            label_width: 120,
            show_index: false,
        }
    }
}

/// 表单联动规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormLinkage {
    /// 规则 ID
    pub linkage_id: String,
    /// 规则名称
    pub name: String,
    /// 触发字段
    pub trigger_field: String,
    /// 触发条件表达式
    pub condition: String,
    /// 动作类型：show / hide / enable / disable / set_value / required / optional
    pub action: String,
    /// 目标字段列表
    pub target_fields: Vec<String>,
    /// 动作值（set_value 时使用）
    pub action_value: Option<serde_json::Value>,
    /// 是否启用
    pub enabled: bool,
}

/// 表单校验规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormValidation {
    /// 规则 ID
    pub validation_id: String,
    /// 规则名称
    pub name: String,
    /// 校验类型：required / format / range / pattern / custom
    pub validation_type: String,
    /// 目标字段
    pub target_field: String,
    /// 校验参数
    pub params: HashMap<String, serde_json::Value>,
    /// 错误提示
    pub error_message: String,
    /// 是否启用
    pub enabled: bool,
}

/// 字段级校验规则
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationRule {
    /// 规则类型：required / min / max / min_length / max_length / pattern / email / phone / id_card / url / custom
    pub rule_type: String,
    /// 规则值
    pub value: Option<serde_json::Value>,
    /// 错误提示
    pub message: Option<String>,
}

/// 表单实例（用户提交的表单数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInstance {
    /// 实例 ID
    pub instance_id: String,
    /// 表单 ID
    pub form_id: String,
    /// 表单版本
    pub form_version: i32,
    /// 租户 ID
    pub tenant_id: String,
    /// 提交人
    pub submitter_id: String,
    /// 提交人部门
    pub submitter_dept_id: Option<String>,
    /// 表单数据
    pub data: HashMap<String, serde_json::Value>,
    /// 状态：draft / submitted / approved / rejected / cancelled
    pub status: String,
    /// 关联的流程实例 ID
    pub process_instance_id: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 提交时间
    pub submitted_at: Option<String>,
}

/// 创建表单定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFormDefinitionRequest {
    pub name: String,
    pub code: String,
    pub category: String,
    pub description: Option<String>,
    pub fields: Vec<FormField>,
    pub layout: Option<FormLayout>,
    pub linkages: Option<Vec<FormLinkage>>,
    pub validations: Option<Vec<FormValidation>>,
    pub process_id: Option<String>,
}

/// 更新表单定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFormDefinitionRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub fields: Option<Vec<FormField>>,
    pub layout: Option<FormLayout>,
    pub linkages: Option<Vec<FormLinkage>>,
    pub validations: Option<Vec<FormValidation>>,
    pub process_id: Option<String>,
}

/// 支持的字段类型列表
pub fn supported_field_types() -> Vec<FormFieldType> {
    vec![
        FormFieldType::Input,
        FormFieldType::Textarea,
        FormFieldType::Number,
        FormFieldType::Money,
        FormFieldType::Percent,
        FormFieldType::Date,
        FormFieldType::DateTime,
        FormFieldType::Time,
        FormFieldType::Select,
        FormFieldType::MultiSelect,
        FormFieldType::Radio,
        FormFieldType::Checkbox,
        FormFieldType::Switch,
        FormFieldType::Rate,
        FormFieldType::Slider,
        FormFieldType::Color,
        FormFieldType::ImageUpload,
        FormFieldType::FileUpload,
        FormFieldType::RichText,
        FormFieldType::Signature,
        FormFieldType::Address,
        FormFieldType::Phone,
        FormFieldType::Email,
        FormFieldType::IdCard,
        FormFieldType::DeptSelect,
        FormFieldType::UserSelect,
        FormFieldType::RelationForm,
        FormFieldType::SubForm,
        FormFieldType::Divider,
        FormFieldType::Title,
        FormFieldType::Description,
        FormFieldType::Calculated,
    ]
}
