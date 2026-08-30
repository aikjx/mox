// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 功能模块注册表
//!
//! 按领域组织前端功能模块，支持：
//! - 模块化注册与发现
//! - 路由管理
//! - 菜单配置
//! - 权限控制
//! - 模块依赖

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{FrontendError, FrontendResult};
use crate::types::ModuleCategory;

/// 模块路由定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRoute {
    /// 路径
    pub path: String,
    /// 名称
    pub name: String,
    /// 组件名
    pub component: String,
    /// 是否需要鉴权
    pub requires_auth: bool,
    /// 权限要求
    pub permissions: Vec<String>,
    /// 子路由
    pub children: Vec<ModuleRoute>,
    /// 是否在菜单中显示
    pub show_in_menu: bool,
    /// 图标
    pub icon: Option<String>,
    /// 排序
    pub order: u32,
}

impl ModuleRoute {
    pub fn new(path: &str, name: &str, component: &str) -> Self {
        Self {
            path: path.to_string(),
            name: name.to_string(),
            component: component.to_string(),
            requires_auth: true,
            permissions: Vec::new(),
            children: Vec::new(),
            show_in_menu: true,
            icon: None,
            order: 100,
        }
    }
}

/// 功能模块定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureModule {
    /// 模块 ID
    pub id: String,
    /// 模块名称
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 模块分类
    pub category: ModuleCategory,
    /// 描述
    pub description: Option<String>,
    /// 版本
    pub version: String,
    /// 模块根路径
    pub base_path: String,
    /// 路由列表
    pub routes: Vec<ModuleRoute>,
    /// 依赖的其他模块
    pub dependencies: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 是否为核心模块
    pub is_core: bool,
    /// 图标
    pub icon: Option<String>,
    /// 排序
    pub order: u32,
    /// 菜单项
    pub menu_items: Vec<ModuleRoute>,
}

impl FeatureModule {
    /// 创建功能模块
    pub fn new(
        name: &str,
        display_name: &str,
        category: ModuleCategory,
        base_path: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            display_name: display_name.to_string(),
            category,
            description: None,
            version: "1.0.0".to_string(),
            base_path: base_path.to_string(),
            routes: Vec::new(),
            dependencies: Vec::new(),
            enabled: true,
            is_core: false,
            icon: None,
            order: 100,
            menu_items: Vec::new(),
        }
    }

    /// 添加路由
    pub fn add_route(&mut self, route: ModuleRoute) {
        self.routes.push(route);
    }

    /// 添加菜单项
    pub fn add_menu_item(&mut self, item: ModuleRoute) {
        self.menu_items.push(item);
    }
}

/// 功能模块注册表
pub struct FeatureModuleRegistry {
    /// 模块表
    modules: RwLock<HashMap<String, FeatureModule>>,
    /// 名称索引
    name_index: RwLock<HashMap<String, String>>, // name -> id
    /// 分类索引
    category_index: RwLock<HashMap<ModuleCategory, Vec<String>>>,
}

impl FeatureModuleRegistry {
    /// 创建注册表
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
            category_index: RwLock::new(HashMap::new()),
        }
    }

    /// 注册模块
    pub fn register(&self, module: FeatureModule) -> FrontendResult<FeatureModule> {
        if self.name_index.read().contains_key(&module.name) {
            return Err(FrontendError::AlreadyExists(format!(
                "module '{}' already exists",
                module.name
            )));
        }

        self.name_index
            .write()
            .insert(module.name.clone(), module.id.clone());
        self.category_index
            .write()
            .entry(module.category)
            .or_default()
            .push(module.id.clone());
        self.modules
            .write()
            .insert(module.id.clone(), module.clone());

        Ok(module)
    }

    /// 按名称获取模块
    pub fn get_by_name(&self, name: &str) -> Option<FeatureModule> {
        let id = self.name_index.read().get(name)?.clone();
        self.modules.read().get(&id).cloned()
    }

    /// 按 ID 获取模块
    pub fn get_by_id(&self, id: &str) -> Option<FeatureModule> {
        self.modules.read().get(id).cloned()
    }

    /// 按分类获取模块
    pub fn get_by_category(&self, category: ModuleCategory) -> Vec<FeatureModule> {
        let ids = self
            .category_index
            .read()
            .get(&category)
            .cloned()
            .unwrap_or_default();
        let modules = self.modules.read();
        let mut result: Vec<FeatureModule> = ids
            .iter()
            .filter_map(|id| modules.get(id).cloned())
            .collect();
        result.sort_by(|a, b| a.order.cmp(&b.order));
        result
    }

    /// 获取所有启用的模块
    pub fn list_enabled(&self) -> Vec<FeatureModule> {
        let modules = self.modules.read();
        let mut result: Vec<FeatureModule> =
            modules.values().filter(|m| m.enabled).cloned().collect();
        result.sort_by(|a, b| a.order.cmp(&b.order));
        result
    }

    /// 获取所有模块
    pub fn list_all(&self) -> Vec<FeatureModule> {
        let modules = self.modules.read();
        let mut result: Vec<FeatureModule> = modules.values().cloned().collect();
        result.sort_by(|a, b| a.order.cmp(&b.order));
        result
    }

    /// 获取所有路由（聚合所有模块的路由）
    pub fn all_routes(&self) -> Vec<ModuleRoute> {
        let modules = self.modules.read();
        let mut all_routes = Vec::new();
        for module in modules.values() {
            if module.enabled {
                all_routes.extend(module.routes.clone());
            }
        }
        all_routes
    }

    /// 获取所有菜单项（按分类分组）
    pub fn all_menu_items(&self) -> HashMap<ModuleCategory, Vec<ModuleRoute>> {
        let modules = self.modules.read();
        let mut result: HashMap<ModuleCategory, Vec<ModuleRoute>> = HashMap::new();

        for module in modules.values() {
            if !module.enabled || module.menu_items.is_empty() {
                continue;
            }
            let items = result.entry(module.category).or_default();
            items.extend(module.menu_items.clone());
        }

        // 每个分类内按 order 排序
        for items in result.values_mut() {
            items.sort_by(|a, b| a.order.cmp(&b.order));
        }

        result
    }

    /// 模块总数
    pub fn count(&self) -> usize {
        self.modules.read().len()
    }

    /// 启用模块数量
    pub fn enabled_count(&self) -> usize {
        self.modules.read().values().filter(|m| m.enabled).count()
    }
}

impl Default for FeatureModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> FeatureModuleRegistry {
        let registry = FeatureModuleRegistry::new();

        // 知识图谱模块
        let mut kg = FeatureModule::new(
            "knowledge-graph",
            "知识图谱",
            ModuleCategory::KnowledgeGraph,
            "/kg",
        );
        kg.is_core = true;
        kg.order = 10;
        kg.add_route(ModuleRoute::new("/kg/graph", "图谱浏览", "GraphView"));
        kg.add_menu_item(ModuleRoute::new("/kg/graph", "图谱浏览", "GraphView"));
        registry.register(kg).unwrap();

        // 知识库模块
        let mut kb = FeatureModule::new(
            "knowledge-base",
            "知识库",
            ModuleCategory::KnowledgeBase,
            "/kb",
        );
        kb.order = 20;
        kb.add_route(ModuleRoute::new("/kb/docs", "文档列表", "DocList"));
        kb.add_menu_item(ModuleRoute::new("/kb/docs", "文档", "DocList"));
        registry.register(kb).unwrap();

        // 云盘模块
        let mut drive = FeatureModule::new(
            "cloud-drive",
            "云盘",
            ModuleCategory::CloudDrive,
            "/drive",
        );
        drive.order = 30;
        drive.add_route(ModuleRoute::new("/drive/files", "文件管理", "FileManager"));
        registry.register(drive).unwrap();

        // 系统管理模块
        let mut sys = FeatureModule::new(
            "system",
            "系统管理",
            ModuleCategory::System,
            "/system",
        );
        sys.is_core = true;
        sys.order = 100;
        registry.register(sys).unwrap();

        registry
    }

    #[test]
    fn test_register_and_get() {
        let registry = create_test_registry();
        assert_eq!(registry.count(), 4);

        let kg = registry.get_by_name("knowledge-graph").unwrap();
        assert_eq!(kg.display_name, "知识图谱");
        assert!(kg.is_core);
        assert_eq!(kg.routes.len(), 1);
    }

    #[test]
    fn test_get_by_category() {
        let registry = create_test_registry();
        let kg_modules = registry.get_by_category(ModuleCategory::KnowledgeGraph);
        assert_eq!(kg_modules.len(), 1);
        assert_eq!(kg_modules[0].name, "knowledge-graph");
    }

    #[test]
    fn test_list_enabled() {
        let registry = create_test_registry();
        let enabled = registry.list_enabled();
        assert_eq!(enabled.len(), 4);
        // 按 order 排序
        assert_eq!(enabled[0].name, "knowledge-graph");
        assert_eq!(enabled[1].name, "knowledge-base");
    }

    #[test]
    fn test_all_routes() {
        let registry = create_test_registry();
        let routes = registry.all_routes();
        assert_eq!(routes.len(), 3);
    }

    #[test]
    fn test_all_menu_items() {
        let registry = create_test_registry();
        let menus = registry.all_menu_items();
        assert!(menus.contains_key(&ModuleCategory::KnowledgeGraph));
        assert!(menus.contains_key(&ModuleCategory::KnowledgeBase));
        // 云盘模块没有菜单项
        assert!(!menus.contains_key(&ModuleCategory::CloudDrive));
    }

    #[test]
    fn test_duplicate_module() {
        let registry = FeatureModuleRegistry::new();
        let m = FeatureModule::new("test", "测试", ModuleCategory::System, "/test");
        registry.register(m.clone()).unwrap();
        assert!(registry.register(m).is_err());
    }

    #[test]
    fn test_enabled_count() {
        let registry = create_test_registry();
        assert_eq!(registry.enabled_count(), 4);
    }
}
