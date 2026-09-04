// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 低代码平台核心
//!
//! mox 模块化系统架构维度低代码能力，支持：
//! - 元数据驱动（实体、字段、关系）
//! - 表单引擎（动态表单、校验、联动）
//! - 页面引擎（页面编排、布局、组件）
//! - 脚本扩展（表达式、自定义逻辑、钩子）

pub mod error;
pub mod types;
pub mod metadata;
pub mod form_engine;
pub mod page_engine;
pub mod script_engine;
pub mod expression;

pub use error::{LowcodeError, LowcodeResult};
pub use types::{
    DataType, FieldDef, FieldType, EntityDef, RelationDef, RelationType,
    ValidationRule, ValidationType,
};
pub use metadata::MetadataManager;
pub use form_engine::{FormEngine, FormSchema, FormField, FormLayout, FormValidationResult};
pub use page_engine::{PageEngine, PageSchema, PageWidget, WidgetType, LayoutType};
pub use script_engine::{ScriptEngine, ScriptContext, ScriptHook, HookType};
pub use expression::ExpressionEvaluator;
