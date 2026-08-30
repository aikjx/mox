// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 组件注册表
//!
//! 统一管理所有前端组件，支持：
//! - 按分类/类型检索
//! - 组件属性定义
//! - 组件依赖声明
//! - 版本管理

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{FrontendError, FrontendResult};
use crate::types::{ComponentCategory, ComponentType};

/// 组件属性定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentProp {
    /// 属性名
    pub name: String,
    /// 属性类型
    pub prop_type: String,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
    /// 描述
    pub description: Option<String>,
    /// 可选值（枚举）
    pub options: Vec<serde_json::Value>,
}

/// 组件事件定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEvent {
    /// 事件名
    pub name: String,
    /// 事件描述
    pub description: Option<String>,
    /// 参数列表
    pub params: Vec<ComponentProp>,
}

/// 插槽定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSlot {
    /// 插槽名
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 是否默认插槽
    pub is_default: bool,
}

/// 组件定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    /// 组件 ID
    pub id: String,
    /// 组件名（标签名，如 mox-button）
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 组件分类
    pub category: ComponentCategory,
    /// 组件类型
    pub component_type: ComponentType,
    /// 描述
    pub description: Option<String>,
    /// 版本
    pub version: String,
    /// 属性列表
    pub props: Vec<ComponentProp>,
    /// 事件列表
    pub events: Vec<ComponentEvent>,
    /// 插槽列表
    pub slots: Vec<ComponentSlot>,
    /// 依赖的其他组件
    pub dependencies: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 图标
    pub icon: Option<String>,
    /// 标签/关键词
    pub tags: Vec<String>,
    /// 文档链接
    pub doc_url: Option<String>,
    /// 示例代码
    pub examples: Vec<ComponentExample>,
}

/// 组件示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentExample {
    pub title: String,
    pub code: String,
    pub description: Option<String>,
}

impl ComponentDef {
    /// 创建组件定义
    pub fn new(
        name: &str,
        display_name: &str,
        category: ComponentCategory,
        component_type: ComponentType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            display_name: display_name.to_string(),
            category,
            component_type,
            description: None,
            version: "1.0.0".to_string(),
            props: Vec::new(),
            events: Vec::new(),
            slots: Vec::new(),
            dependencies: Vec::new(),
            enabled: true,
            icon: None,
            tags: Vec::new(),
            doc_url: None,
            examples: Vec::new(),
        }
    }

    /// 添加属性
    pub fn add_prop(
        &mut self,
        name: &str,
        prop_type: &str,
        required: bool,
        default: Option<serde_json::Value>,
    ) {
        self.props.push(ComponentProp {
            name: name.to_string(),
            prop_type: prop_type.to_string(),
            required,
            default_value: default,
            description: None,
            options: Vec::new(),
        });
    }

    /// 添加事件
    pub fn add_event(&mut self, name: &str) {
        self.events.push(ComponentEvent {
            name: name.to_string(),
            description: None,
            params: Vec::new(),
        });
    }

    /// 添加标签
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }
}

/// 组件注册表
pub struct ComponentRegistry {
    /// 组件表
    components: RwLock<HashMap<String, ComponentDef>>,
    /// 名称索引
    name_index: RwLock<HashMap<String, String>>, // name -> id
    /// 分类索引
    category_index: RwLock<HashMap<ComponentCategory, Vec<String>>>,
    /// 类型索引
    type_index: RwLock<HashMap<ComponentType, Vec<String>>>,
}

impl ComponentRegistry {
    /// 创建组件注册表
    pub fn new() -> Self {
        Self {
            components: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
            category_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }

    /// 注册组件
    pub fn register(&self, component: ComponentDef) -> FrontendResult<ComponentDef> {
        if self.name_index.read().contains_key(&component.name) {
            return Err(FrontendError::AlreadyExists(format!(
                "component '{}' already exists",
                component.name
            )));
        }

        self.name_index
            .write()
            .insert(component.name.clone(), component.id.clone());
        self.category_index
            .write()
            .entry(component.category)
            .or_default()
            .push(component.id.clone());
        self.type_index
            .write()
            .entry(component.component_type)
            .or_default()
            .push(component.id.clone());
        self.components
            .write()
            .insert(component.id.clone(), component.clone());

        Ok(component)
    }

    /// 按名称获取组件
    pub fn get_by_name(&self, name: &str) -> Option<ComponentDef> {
        let id = self.name_index.read().get(name)?.clone();
        self.components.read().get(&id).cloned()
    }

    /// 按 ID 获取组件
    pub fn get_by_id(&self, id: &str) -> Option<ComponentDef> {
        self.components.read().get(id).cloned()
    }

    /// 按分类获取组件
    pub fn get_by_category(&self, category: ComponentCategory) -> Vec<ComponentDef> {
        let ids = self
            .category_index
            .read()
            .get(&category)
            .cloned()
            .unwrap_or_default();
        let components = self.components.read();
        ids.iter()
            .filter_map(|id| components.get(id).cloned())
            .collect()
    }

    /// 按类型获取组件
    pub fn get_by_type(&self, component_type: ComponentType) -> Vec<ComponentDef> {
        let ids = self
            .type_index
            .read()
            .get(&component_type)
            .cloned()
            .unwrap_or_default();
        let components = self.components.read();
        ids.iter()
            .filter_map(|id| components.get(id).cloned())
            .collect()
    }

    /// 搜索组件
    pub fn search(&self, keyword: &str) -> Vec<ComponentDef> {
        let keyword = keyword.to_lowercase();
        let components = self.components.read();
        components
            .values()
            .filter(|c| {
                c.name.to_lowercase().contains(&keyword)
                    || c.display_name.to_lowercase().contains(&keyword)
                    || c.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&keyword))
            })
            .cloned()
            .collect()
    }

    /// 列出所有组件
    pub fn list_all(&self) -> Vec<ComponentDef> {
        self.components.read().values().cloned().collect()
    }

    /// 组件总数
    pub fn count(&self) -> usize {
        self.components.read().len()
    }

    /// 分类数量
    pub fn category_count(&self) -> usize {
        self.category_index.read().len()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_registry() -> ComponentRegistry {
        let registry = ComponentRegistry::new();

        // 基础组件
        let mut button = ComponentDef::new(
            "mox-button",
            "按钮",
            ComponentCategory::Basic,
            ComponentType::Atom,
        );
        button.add_prop("type", "string", false, Some(json!("primary")));
        button.add_prop("size", "string", false, Some(json!("md")));
        button.add_prop("disabled", "boolean", false, Some(json!(false)));
        button.add_event("click");
        button.add_tag("基础");
        registry.register(button).unwrap();

        let mut input = ComponentDef::new(
            "mox-input",
            "输入框",
            ComponentCategory::Form,
            ComponentType::Atom,
        );
        input.add_prop("value", "string", false, Some(json!("")));
        input.add_prop("placeholder", "string", false, None);
        input.add_event("change");
        input.add_tag("表单");
        registry.register(input).unwrap();

        // 数据展示
        let table = ComponentDef::new(
            "mox-table",
            "表格",
            ComponentCategory::DataDisplay,
            ComponentType::Organism,
        );
        registry.register(table).unwrap();

        // 图谱组件
        let graph = ComponentDef::new(
            "mox-graph-canvas",
            "图谱画布",
            ComponentCategory::Graph,
            ComponentType::Organism,
        );
        registry.register(graph).unwrap();

        registry
    }

    #[test]
    fn test_register_and_get() {
        let registry = create_test_registry();
        assert_eq!(registry.count(), 4);

        let button = registry.get_by_name("mox-button").unwrap();
        assert_eq!(button.display_name, "按钮");
        assert_eq!(button.props.len(), 3);
        assert_eq!(button.events.len(), 1);
    }

    #[test]
    fn test_get_by_category() {
        let registry = create_test_registry();
        let basic = registry.get_by_category(ComponentCategory::Basic);
        assert_eq!(basic.len(), 1);
        assert_eq!(basic[0].name, "mox-button");

        let form = registry.get_by_category(ComponentCategory::Form);
        assert_eq!(form.len(), 1);
    }

    #[test]
    fn test_get_by_type() {
        let registry = create_test_registry();
        let atoms = registry.get_by_type(ComponentType::Atom);
        assert_eq!(atoms.len(), 2); // button + input
    }

    #[test]
    fn test_search() {
        let registry = create_test_registry();
        let results = registry.search("按钮");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "mox-button");

        let results2 = registry.search("mox");
        assert_eq!(results2.len(), 4);
    }

    #[test]
    fn test_duplicate_register() {
        let registry = ComponentRegistry::new();
        let btn = ComponentDef::new(
            "mox-btn",
            "按钮",
            ComponentCategory::Basic,
            ComponentType::Atom,
        );
        registry.register(btn.clone()).unwrap();
        assert!(registry.register(btn).is_err());
    }

    #[test]
    fn test_list_all() {
        let registry = create_test_registry();
        let all = registry.list_all();
        assert_eq!(all.len(), 4);
    }
}
