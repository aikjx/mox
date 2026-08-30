// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 布局系统
//!
//! 统一的页面布局、导航、侧边栏管理

use serde::{Deserialize, Serialize};

use crate::types::ModuleCategory;

/// 布局类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutType {
    /// 侧边栏布局（左侧导航 + 主内容）
    Sidebar,
    /// 顶部导航布局
    TopNav,
    /// 混合布局（顶部 + 侧边栏）
    Mixed,
    /// 全宽布局（无侧边栏）
    FullWidth,
}

/// 导航项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItem {
    /// 路径
    pub path: String,
    /// 名称
    pub name: String,
    /// 图标
    pub icon: Option<String>,
    /// 排序
    pub order: u32,
    /// 是否启用
    pub enabled: bool,
    /// 权限要求
    pub permissions: Vec<String>,
    /// 子导航
    pub children: Vec<NavItem>,
    /// 是否在面包屑中显示
    pub show_in_breadcrumb: bool,
    /// 徽章（如未读数）
    pub badge: Option<String>,
}

impl NavItem {
    pub fn new(path: &str, name: &str) -> Self {
        Self {
            path: path.to_string(),
            name: name.to_string(),
            icon: None,
            order: 100,
            enabled: true,
            permissions: Vec::new(),
            children: Vec::new(),
            show_in_breadcrumb: true,
            badge: None,
        }
    }

    /// 添加子导航
    pub fn add_child(&mut self, child: NavItem) {
        self.children.push(child);
        self.children.sort_by(|a, b| a.order.cmp(&b.order));
    }
}

/// 导航分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavGroup {
    /// 分组 ID
    pub id: String,
    /// 分组名称
    pub title: String,
    /// 分类
    pub category: ModuleCategory,
    /// 导航项列表
    pub items: Vec<NavItem>,
    /// 排序
    pub order: u32,
    /// 是否可折叠
    pub collapsible: bool,
    /// 是否默认展开
    pub default_expanded: bool,
}

impl NavGroup {
    pub fn new(id: &str, title: &str, category: ModuleCategory) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            category,
            items: Vec::new(),
            order: 100,
            collapsible: true,
            default_expanded: true,
        }
    }

    /// 添加导航项
    pub fn add_item(&mut self, item: NavItem) {
        self.items.push(item);
        self.items.sort_by(|a, b| a.order.cmp(&b.order));
    }
}

/// 布局系统
pub struct LayoutSystem {
    /// 当前布局类型
    pub layout_type: LayoutType,
    /// 侧边栏是否折叠
    pub sidebar_collapsed: bool,
    /// 侧边栏宽度
    pub sidebar_width: u32,
    /// 折叠后的侧边栏宽度
    pub collapsed_width: u32,
    /// 顶部栏高度
    pub header_height: u32,
    /// 是否显示标签页
    pub show_tabs: bool,
    /// 是否显示面包屑
    pub show_breadcrumb: bool,
    /// 是否显示页脚
    pub show_footer: bool,
    /// 导航分组
    nav_groups: Vec<NavGroup>,
}

impl LayoutSystem {
    /// 创建默认布局系统
    pub fn new() -> Self {
        Self {
            layout_type: LayoutType::Mixed,
            sidebar_collapsed: false,
            sidebar_width: 240,
            collapsed_width: 64,
            header_height: 56,
            show_tabs: true,
            show_breadcrumb: true,
            show_footer: true,
            nav_groups: Vec::new(),
        }
    }

    /// 添加导航分组
    pub fn add_nav_group(&mut self, group: NavGroup) {
        self.nav_groups.push(group);
        self.nav_groups.sort_by(|a, b| a.order.cmp(&b.order));
    }

    /// 获取所有导航分组
    pub fn nav_groups(&self) -> &[NavGroup] {
        &self.nav_groups
    }

    /// 切换侧边栏
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
    }

    /// 获取侧边栏当前宽度
    pub fn current_sidebar_width(&self) -> u32 {
        if self.sidebar_collapsed {
            self.collapsed_width
        } else {
            self.sidebar_width
        }
    }

    /// 设置布局类型
    pub fn set_layout_type(&mut self, layout_type: LayoutType) {
        self.layout_type = layout_type;
    }

    /// 根据权限过滤导航
    pub fn filter_nav_by_permissions(&self, permissions: &[String]) -> Vec<NavGroup> {
        let mut result = Vec::new();

        for group in &self.nav_groups {
            let filtered_items: Vec<NavItem> = group
                .items
                .iter()
                .filter(|item| self.nav_item_visible(item, permissions))
                .map(|item| self.filter_item_children(item, permissions))
                .collect();

            if !filtered_items.is_empty() {
                let mut filtered_group = group.clone();
                filtered_group.items = filtered_items;
                result.push(filtered_group);
            }
        }

        result
    }

    fn nav_item_visible(&self, item: &NavItem, permissions: &[String]) -> bool {
        if !item.enabled {
            return false;
        }
        if item.permissions.is_empty() {
            return true;
        }
        item.permissions
            .iter()
            .any(|p| permissions.contains(p))
    }

    fn filter_item_children(&self, item: &NavItem, permissions: &[String]) -> NavItem {
        let mut result = item.clone();
        result.children = item
            .children
            .iter()
            .filter(|c| self.nav_item_visible(c, permissions))
            .map(|c| self.filter_item_children(c, permissions))
            .collect();
        result
    }

    /// 生成面包屑
    pub fn build_breadcrumb(&self, current_path: &str) -> Vec<NavItem> {
        let mut breadcrumb = Vec::new();

        for group in &self.nav_groups {
            for item in &group.items {
                if let Some(path) = self.find_path(item, current_path) {
                    breadcrumb = path;
                    break;
                }
            }
        }

        breadcrumb
    }

    fn find_path(&self, item: &NavItem, target_path: &str) -> Option<Vec<NavItem>> {
        if item.path == target_path {
            return Some(vec![item.clone()]);
        }

        for child in &item.children {
            if let Some(mut path) = self.find_path(child, target_path) {
                let mut result = vec![item.clone()];
                result.append(&mut path);
                return Some(result);
            }
        }

        None
    }

    /// 导航分组数量
    pub fn group_count(&self) -> usize {
        self.nav_groups.len()
    }

    /// 总导航项数量
    pub fn total_nav_items(&self) -> usize {
        self.nav_groups
            .iter()
            .map(|g| g.items.len())
            .sum()
    }
}

impl Default for LayoutSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_layout() -> LayoutSystem {
        let mut layout = LayoutSystem::new();

        // 知识图谱分组
        let mut kg_group = NavGroup::new("kg", "知识图谱", ModuleCategory::KnowledgeGraph);
        kg_group.order = 10;

        let mut graph_item = NavItem::new("/kg/graph", "图谱浏览");
        graph_item.order = 1;
        graph_item.add_child(NavItem::new("/kg/graph/explore", "图谱探索"));
        graph_item.add_child(NavItem::new("/kg/graph/search", "高级搜索"));
        kg_group.add_item(graph_item);

        let entity_item = NavItem::new("/kg/entities", "实体管理");
        kg_group.add_item(entity_item);

        layout.add_nav_group(kg_group);

        // 知识库分组
        let mut kb_group = NavGroup::new("kb", "知识库", ModuleCategory::KnowledgeBase);
        kb_group.order = 20;
        kb_group.add_item(NavItem::new("/kb/docs", "文档列表"));
        kb_group.add_item(NavItem::new("/kb/search", "全文检索"));
        layout.add_nav_group(kb_group);

        layout
    }

    #[test]
    fn test_layout_default() {
        let layout = LayoutSystem::new();
        assert_eq!(layout.layout_type, LayoutType::Mixed);
        assert!(!layout.sidebar_collapsed);
        assert_eq!(layout.current_sidebar_width(), 240);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut layout = LayoutSystem::new();
        layout.toggle_sidebar();
        assert!(layout.sidebar_collapsed);
        assert_eq!(layout.current_sidebar_width(), 64);
    }

    #[test]
    fn test_nav_groups() {
        let layout = create_test_layout();
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.total_nav_items(), 4); // 2 + 2

        // 按 order 排序
        let groups = layout.nav_groups();
        assert_eq!(groups[0].id, "kg");
        assert_eq!(groups[1].id, "kb");
    }

    #[test]
    fn test_filter_by_permissions() {
        let layout = create_test_layout();

        // 空权限 -> 所有都显示（因为没有权限要求）
        let filtered = layout.filter_nav_by_permissions(&[]);
        assert_eq!(filtered.len(), 2);

        // 有特定权限要求的项（现在所有项都没有权限要求，所以都显示）
        let filtered2 = layout.filter_nav_by_permissions(&["kg:read".to_string()]);
        assert_eq!(filtered2.len(), 2);
    }

    #[test]
    fn test_nav_with_permissions() {
        let mut layout = LayoutSystem::new();
        let mut group = NavGroup::new("test", "测试", ModuleCategory::System);

        let mut item1 = NavItem::new("/a", "A");
        item1.permissions = vec!["a:read".to_string()];
        group.add_item(item1);

        let mut item2 = NavItem::new("/b", "B");
        item2.permissions = vec!["b:read".to_string()];
        group.add_item(item2);

        layout.add_nav_group(group);

        let filtered = layout.filter_nav_by_permissions(&["a:read".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].items.len(), 1);
        assert_eq!(filtered[0].items[0].name, "A");
    }

    #[test]
    fn test_breadcrumb() {
        let layout = create_test_layout();

        let bc = layout.build_breadcrumb("/kg/graph/explore");
        assert_eq!(bc.len(), 2);
        assert_eq!(bc[0].name, "图谱浏览");
        assert_eq!(bc[1].name, "图谱探索");
    }

    #[test]
    fn test_set_layout_type() {
        let mut layout = LayoutSystem::new();
        layout.set_layout_type(LayoutType::Sidebar);
        assert_eq!(layout.layout_type, LayoutType::Sidebar);
    }
}
