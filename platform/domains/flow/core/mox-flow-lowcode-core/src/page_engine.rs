// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 页面引擎
//!
//! 页面编排、组件管理、布局系统。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{LowcodeError, LowcodeResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 组件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    /// 容器
    Container,
    /// 栅格布局
    Row,
    Col,
    /// 卡片
    Card,
    /// 标签页
    Tabs,
    /// 标签页项
    TabPane,
    /// 表单
    Form,
    /// 表格
    Table,
    /// 列表
    List,
    /// 图表
    Chart,
    /// 按钮
    Button,
    /// 输入框
    Input,
    /// 选择器
    Select,
    /// 日期选择
    DatePicker,
    /// 文字
    Text,
    /// 标题
    Title,
    /// 图片
    Image,
    /// 链接
    Link,
    /// 图标
    Icon,
    /// 分割线
    Divider,
    /// 空间
    Space,
    /// 对话框
    Modal,
    /// 抽屉
    Drawer,
    /// 面包屑
    Breadcrumb,
    /// 分页
    Pagination,
    /// 搜索框
    Search,
    /// 步骤条
    Steps,
    /// 时间线
    Timeline,
    /// 统计卡片
    StatCard,
    /// 进度条
    Progress,
    /// 自定义组件
    Custom(String),
}

/// 布局类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutType {
    /// 绝对布局
    Absolute,
    /// 流式布局
    Flow,
    /// 栅格布局
    Grid,
    /// 弹性布局
    Flex,
    /// 自适应布局
    Responsive,
}

/// 页面组件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageWidget {
    /// 组件 ID
    pub id: String,
    /// 组件类型
    pub widget_type: WidgetType,
    /// 组件名称（用于引用）
    pub name: Option<String>,
    /// 标题/标签
    pub label: Option<String>,
    /// 子组件
    pub children: Vec<PageWidget>,
    /// 组件属性
    pub props: HashMap<String, serde_json::Value>,
    /// 样式
    pub style: HashMap<String, String>,
    /// 事件绑定
    pub events: Vec<WidgetEvent>,
    /// 显示条件表达式
    pub visible_on: Option<String>,
    /// 栅格宽度
    pub span: Option<u8>,
    /// 布局位置（绝对布局用）
    pub position: Option<Position>,
    /// 数据源绑定
    pub data_source: Option<DataSourceBind>,
    /// 权限要求
    pub permission: Option<String>,
}

/// 组件位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub top: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub bottom: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
}

/// 组件事件绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetEvent {
    /// 事件名（如 click, change）
    pub event: String,
    /// 动作类型
    pub action: EventAction,
    /// 动作参数
    pub params: HashMap<String, String>,
    /// 脚本代码
    pub script: Option<String>,
}

/// 事件动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAction {
    /// 导航
    Navigate,
    /// 打开对话框
    OpenModal,
    /// 关闭对话框
    CloseModal,
    /// 提交表单
    SubmitForm,
    /// 重置表单
    ResetForm,
    /// 刷新数据
    RefreshData,
    /// 调用接口
    CallApi,
    /// 执行脚本
    ExecuteScript,
    /// 显示消息
    ShowMessage,
    /// 导出
    Export,
    /// 打印
    Print,
}

/// 数据源绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceBind {
    /// 数据源类型
    pub source_type: DataSourceType,
    /// 数据源 ID（实体、API 等）
    pub source_id: String,
    /// 查询参数
    pub query_params: HashMap<String, String>,
    /// 字段映射
    pub field_mapping: HashMap<String, String>,
    /// 是否自动加载
    pub auto_load: bool,
}

/// 数据源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    /// 实体（数据表）
    Entity,
    /// API 接口
    Api,
    /// 静态数据
    Static,
    /// 脚本生成
    Script,
}

/// 页面 Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSchema {
    /// 页面 ID
    pub id: String,
    /// 页面名称
    pub name: String,
    /// 页面标题
    pub title: String,
    /// 路径
    pub path: String,
    /// 所属模块
    pub module: String,
    /// 布局类型
    pub layout: LayoutType,
    /// 根组件
    pub root: PageWidget,
    /// 页面变量
    pub variables: HashMap<String, PageVariable>,
    /// 页面级脚本
    pub scripts: Vec<String>,
    /// 页面样式
    pub style: HashMap<String, String>,
    /// 权限要求
    pub permission: Option<String>,
    /// 是否缓存
    pub keep_alive: bool,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 页面变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageVariable {
    pub name: String,
    pub var_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

impl PageSchema {
    /// 创建页面
    pub fn new(name: &str, title: &str, path: &str, module: &str) -> Self {
        let now = crate::types::now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            title: title.to_string(),
            path: path.to_string(),
            module: module.to_string(),
            layout: LayoutType::Flow,
            root: PageWidget {
                id: Uuid::new_v4().to_string(),
                widget_type: WidgetType::Container,
                name: Some("root".to_string()),
                label: None,
                children: Vec::new(),
                props: HashMap::new(),
                style: HashMap::new(),
                events: Vec::new(),
                visible_on: None,
                span: None,
                position: None,
                data_source: None,
                permission: None,
            },
            variables: HashMap::new(),
            scripts: Vec::new(),
            style: HashMap::new(),
            permission: None,
            keep_alive: false,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 查找组件
    pub fn find_widget(&self, widget_id: &str) -> Option<&PageWidget> {
        Self::find_widget_recursive(&self.root, widget_id)
    }

    fn find_widget_recursive<'a>(
        widget: &'a PageWidget,
        widget_id: &str,
    ) -> Option<&'a PageWidget> {
        if widget.id == widget_id {
            return Some(widget);
        }
        for child in &widget.children {
            if let Some(found) = Self::find_widget_recursive(child, widget_id) {
                return Some(found);
            }
        }
        None
    }

    /// 按名称查找组件
    pub fn find_widget_by_name(&self, name: &str) -> Option<&PageWidget> {
        Self::find_widget_by_name_recursive(&self.root, name)
    }

    fn find_widget_by_name_recursive<'a>(
        widget: &'a PageWidget,
        name: &str,
    ) -> Option<&'a PageWidget> {
        if widget.name.as_deref() == Some(name) {
            return Some(widget);
        }
        for child in &widget.children {
            if let Some(found) = Self::find_widget_by_name_recursive(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// 统计组件数量
    pub fn widget_count(&self) -> usize {
        Self::count_widgets(&self.root)
    }

    fn count_widgets(widget: &PageWidget) -> usize {
        let mut count = 1;
        for child in &widget.children {
            count += Self::count_widgets(child);
        }
        count
    }

    /// 获取所有表格组件
    pub fn find_tables(&self) -> Vec<&PageWidget> {
        let mut result = Vec::new();
        Self::find_widgets_by_type(&self.root, WidgetType::Table, &mut result);
        result
    }

    fn find_widgets_by_type<'a>(
        widget: &'a PageWidget,
        widget_type: WidgetType,
        result: &mut Vec<&'a PageWidget>,
    ) {
        if widget.widget_type == widget_type {
            result.push(widget);
        }
        for child in &widget.children {
            Self::find_widgets_by_type(child, widget_type.clone(), result);
        }
    }
}

/// 页面引擎
pub struct PageEngine {
    /// 页面 Schema 缓存
    pages: RwLock<HashMap<String, PageSchema>>,
    /// 路径 -> 页面 ID 映射
    path_index: RwLock<HashMap<String, String>>,
    /// 模块页面索引
    module_pages: RwLock<HashMap<String, Vec<String>>>,
}

impl PageEngine {
    /// 创建页面引擎
    pub fn new() -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
            path_index: RwLock::new(HashMap::new()),
            module_pages: RwLock::new(HashMap::new()),
        }
    }

    /// 注册页面
    pub fn register_page(&self, page: PageSchema) -> LowcodeResult<PageSchema> {
        // 检查路径唯一性
        if self.path_index.read().contains_key(&page.path) {
            return Err(LowcodeError::AlreadyExists(format!(
                "page path '{}' already exists",
                page.path
            )));
        }

        self.path_index
            .write()
            .insert(page.path.clone(), page.id.clone());
        self.module_pages
            .write()
            .entry(page.module.clone())
            .or_default()
            .push(page.id.clone());
        self.pages.write().insert(page.id.clone(), page.clone());
        Ok(page)
    }

    /// 获取页面
    pub fn get_page(&self, page_id: &str) -> LowcodeResult<PageSchema> {
        self.pages
            .read()
            .get(page_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("page '{}' not found", page_id)))
    }

    /// 按路径获取页面
    pub fn get_page_by_path(&self, path: &str) -> LowcodeResult<PageSchema> {
        let page_id = self
            .path_index
            .read()
            .get(path)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("page '{}' not found", path)))?;
        self.get_page(&page_id)
    }

    /// 检查页面是否存在
    pub fn page_exists(&self, page_id: &str) -> bool {
        self.pages.read().contains_key(page_id)
    }

    /// 更新页面
    pub fn update_page(
        &self,
        page_id: &str,
        mut update: PageSchema,
    ) -> LowcodeResult<PageSchema> {
        let mut pages = self.pages.write();
        let existing = pages
            .get_mut(page_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("page '{}' not found", page_id)))?;

        let old_path = existing.path.clone();
        let old_module = existing.module.clone();

        update.id = page_id.to_string();
        update.created_at = existing.created_at;
        update.updated_at = crate::types::now_ms();

        // 如果路径变了，更新索引
        if update.path != old_path {
            self.path_index.write().remove(&old_path);
            self.path_index
                .write()
                .insert(update.path.clone(), page_id.to_string());
        }

        // 如果模块变了，更新索引
        if update.module != old_module {
            if let Some(vec) = self.module_pages.write().get_mut(&old_module) {
                vec.retain(|id| id != page_id);
            }
            self.module_pages
                .write()
                .entry(update.module.clone())
                .or_default()
                .push(page_id.to_string());
        }

        *existing = update.clone();
        Ok(update)
    }

    /// 删除页面
    pub fn delete_page(&self, page_id: &str) -> LowcodeResult<bool> {
        let page = self.get_page(page_id)?;

        self.path_index.write().remove(&page.path);
        if let Some(vec) = self.module_pages.write().get_mut(&page.module) {
            vec.retain(|id| id != page_id);
        }

        Ok(self.pages.write().remove(page_id).is_some())
    }

    /// 列出模块下的所有页面
    pub fn list_pages_by_module(&self, module: &str) -> Vec<PageSchema> {
        let page_ids = self
            .module_pages
            .read()
            .get(module)
            .cloned()
            .unwrap_or_default();
        let pages = self.pages.read();
        page_ids
            .into_iter()
            .filter_map(|id| pages.get(&id).cloned())
            .collect()
    }

    /// 列出所有页面
    pub fn list_pages(&self) -> Vec<PageSchema> {
        self.pages.read().values().cloned().collect()
    }

    /// 添加组件到页面
    pub fn add_widget(
        &self,
        page_id: &str,
        parent_id: &str,
        widget: PageWidget,
    ) -> LowcodeResult<()> {
        let mut pages = self.pages.write();
        let page = pages
            .get_mut(page_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("page '{}' not found", page_id)))?;

        // 递归查找父组件并添加
        if Self::add_widget_to_parent(&mut page.root, parent_id, widget) {
            page.updated_at = crate::types::now_ms();
            Ok(())
        } else {
            Err(LowcodeError::NotFound(format!(
                "parent widget '{}' not found",
                parent_id
            )))
        }
    }

    fn add_widget_to_parent(
        widget: &mut PageWidget,
        parent_id: &str,
        child: PageWidget,
    ) -> bool {
        if widget.id == parent_id {
            widget.children.push(child);
            return true;
        }
        for c in &mut widget.children {
            if Self::add_widget_to_parent(c, parent_id, child.clone()) {
                return true;
            }
        }
        false
    }

    /// 页面总数
    pub fn page_count(&self) -> usize {
        self.pages.read().len()
    }
}

impl Default for PageEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_page() -> PageSchema {
        let mut page = PageSchema::new("user_list", "用户列表", "/users", "system");

        // 添加一个卡片
        let card = PageWidget {
            id: "card-1".to_string(),
            widget_type: WidgetType::Card,
            name: Some("mainCard".to_string()),
            label: Some("用户管理".to_string()),
            children: vec![
                PageWidget {
                    id: "table-1".to_string(),
                    widget_type: WidgetType::Table,
                    name: Some("userTable".to_string()),
                    label: None,
                    children: Vec::new(),
                    props: HashMap::new(),
                    style: HashMap::new(),
                    events: Vec::new(),
                    visible_on: None,
                    span: None,
                    position: None,
                    data_source: Some(DataSourceBind {
                        source_type: DataSourceType::Entity,
                        source_id: "user".to_string(),
                        query_params: HashMap::new(),
                        field_mapping: HashMap::new(),
                        auto_load: true,
                    }),
                    permission: None,
                },
            ],
            props: HashMap::new(),
            style: HashMap::new(),
            events: Vec::new(),
            visible_on: None,
            span: None,
            position: None,
            data_source: None,
            permission: None,
        };

        page.root.children.push(card);
        page
    }

    #[test]
    fn test_page_creation() {
        let page = create_test_page();
        assert_eq!(page.name, "user_list");
        assert_eq!(page.path, "/users");
        assert_eq!(page.widget_count(), 3); // root + card + table
    }

    #[test]
    fn test_find_widget() {
        let page = create_test_page();

        let found = page.find_widget("table-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.as_deref(), Some("userTable"));

        let not_found = page.find_widget("nonexist");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_widget_by_name() {
        let page = create_test_page();

        let found = page.find_widget_by_name("userTable");
        assert!(found.is_some());
        assert_eq!(found.unwrap().widget_type, WidgetType::Table);
    }

    #[test]
    fn test_find_tables() {
        let page = create_test_page();
        let tables = page.find_tables();
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn test_register_page() {
        let engine = PageEngine::new();
        let page = create_test_page();

        let registered = engine.register_page(page).unwrap();
        assert_eq!(engine.page_count(), 1);

        let by_id = engine.get_page(&registered.id).unwrap();
        assert_eq!(by_id.name, "user_list");

        let by_path = engine.get_page_by_path("/users").unwrap();
        assert_eq!(by_path.name, "user_list");
    }

    #[test]
    fn test_duplicate_path() {
        let engine = PageEngine::new();
        let page1 = create_test_page();
        engine.register_page(page1).unwrap();

        let page2 = create_test_page();
        let result = engine.register_page(page2);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_by_module() {
        let engine = PageEngine::new();
        engine.register_page(create_test_page()).unwrap();

        let page2 = PageSchema::new("role_list", "角色列表", "/roles", "system");
        engine.register_page(page2).unwrap();

        let page3 = PageSchema::new("dashboard", "仪表盘", "/dashboard", "home");
        engine.register_page(page3).unwrap();

        let system_pages = engine.list_pages_by_module("system");
        assert_eq!(system_pages.len(), 2);
    }

    #[test]
    fn test_delete_page() {
        let engine = PageEngine::new();
        let page = create_test_page();
        let id = page.id.clone();

        engine.register_page(page).unwrap();
        assert_eq!(engine.page_count(), 1);

        assert!(engine.delete_page(&id).unwrap());
        assert_eq!(engine.page_count(), 0);
    }

    #[test]
    fn test_add_widget() {
        let engine = PageEngine::new();
        let page = create_test_page();
        let id = page.id.clone();
        engine.register_page(page).unwrap();

        let new_btn = PageWidget {
            id: "btn-1".to_string(),
            widget_type: WidgetType::Button,
            name: Some("addBtn".to_string()),
            label: Some("新增".to_string()),
            children: Vec::new(),
            props: HashMap::new(),
            style: HashMap::new(),
            events: Vec::new(),
            visible_on: None,
            span: None,
            position: None,
            data_source: None,
            permission: None,
        };

        engine
            .add_widget(&id, "card-1", new_btn)
            .unwrap();

        let page = engine.get_page(&id).unwrap();
        assert!(page.find_widget_by_name("addBtn").is_some());
    }

    #[test]
    fn test_page_variables() {
        let mut page = create_test_page();
        page.variables.insert(
            "pageSize".to_string(),
            PageVariable {
                name: "pageSize".to_string(),
                var_type: "number".to_string(),
                default_value: Some("10".to_string()),
                description: Some("每页条数".to_string()),
            },
        );

        assert_eq!(page.variables.len(), 1);
        assert_eq!(page.variables.get("pageSize").unwrap().var_type, "number");
    }

    #[test]
    fn test_widget_events() {
        let widget = PageWidget {
            id: "btn-1".to_string(),
            widget_type: WidgetType::Button,
            name: Some("submitBtn".to_string()),
            label: Some("提交".to_string()),
            children: vec![],
            props: HashMap::new(),
            style: HashMap::new(),
            events: vec![WidgetEvent {
                event: "click".to_string(),
                action: EventAction::SubmitForm,
                params: HashMap::new(),
                script: None,
            }],
            visible_on: None,
            span: None,
            position: None,
            data_source: None,
            permission: None,
        };

        assert_eq!(widget.events.len(), 1);
        assert_eq!(widget.events[0].event, "click");
        assert_eq!(widget.events[0].action, EventAction::SubmitForm);
    }
}
