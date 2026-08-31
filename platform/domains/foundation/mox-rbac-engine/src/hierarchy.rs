// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 角色继承树
//!
//! 管理角色间的继承关系，支持：
//! - 多继承（一个角色可以继承多个父角色）
//! - 传递性（A > B > C => A > C）
//! - 循环检测
//! - 权限展开（收集角色及其所有祖先的权限）
//! - 继承链查询（获取角色的所有祖先/后代）

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::RbacError;
use crate::types::{Permission, Role};

/// 角色继承树
///
/// 以有向无环图（DAG）的形式管理角色继承关系。
/// 支持高效的权限展开和继承关系查询。
#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    /// 角色名 → 角色定义
    roles: HashMap<String, Role>,
    /// 角色名 → 直接父角色列表（children -> parents）
    parents: HashMap<String, Vec<String>>,
    /// 角色名 → 直接子角色列表（parents -> children）
    children: HashMap<String, Vec<String>>,
}

impl RoleHierarchy {
    /// 创建空的继承树
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// 从角色列表构建继承树
    ///
    /// 会自动检测循环继承，如果检测到循环则返回错误。
    pub fn from_roles(roles: Vec<Role>) -> Result<Self, RbacError> {
        let mut hierarchy = Self::new();
        for role in roles {
            hierarchy.add_role(role)?;
        }
        Ok(hierarchy)
    }

    /// 添加角色到继承树
    ///
    /// 如果角色已存在，则更新其定义和继承关系。
    pub fn add_role(&mut self, role: Role) -> Result<(), RbacError> {
        let name = role.name.clone();
        let extends = role.extends.clone();

        // 先检查添加后是否会产生循环
        // 临时构建新的父关系来检测
        let mut test_parents = self.parents.clone();
        test_parents.insert(name.clone(), extends.clone());

        if Self::detect_cycle(&name, &test_parents) {
            return Err(RbacError::CyclicInheritance(name));
        }

        // 如果角色已存在，先移除旧的继承关系
        if self.roles.contains_key(&name) {
            self.remove_role_internal(&name);
        }

        // 添加角色定义
        self.roles.insert(name.clone(), role);

        // 更新父角色映射
        self.parents.insert(name.clone(), extends.clone());

        // 更新子角色映射
        for parent in &extends {
            self.children
                .entry(parent.clone())
                .or_default()
                .push(name.clone());
        }

        // 确保父角色在 children map 中有条目（即使没有子角色）
        for parent in &extends {
            self.children.entry(parent.clone()).or_default();
        }

        // 确保角色在 parents map 中有条目
        self.parents.entry(name.clone()).or_default();

        Ok(())
    }

    /// 移除角色
    pub fn remove_role(&mut self, name: &str) -> Option<Role> {
        if !self.roles.contains_key(name) {
            return None;
        }
        self.remove_role_internal(name);
        self.roles.remove(name)
    }

    /// 内部移除（不删除 roles 条目，由调用者处理）
    fn remove_role_internal(&mut self, name: &str) {
        // 从父角色的子列表中移除
        if let Some(parents) = self.parents.get(name).cloned() {
            for parent in &parents {
                if let Some(children) = self.children.get_mut(parent) {
                    children.retain(|c| c != name);
                }
            }
        }

        // 从子角色的父列表中移除
        if let Some(children) = self.children.get(name).cloned() {
            for child in &children {
                if let Some(parents) = self.parents.get_mut(child) {
                    parents.retain(|p| p != name);
                }
            }
        }

        // 清理映射
        self.parents.remove(name);
        self.children.remove(name);
    }

    /// 获取角色定义
    pub fn get_role(&self, name: &str) -> Option<&Role> {
        self.roles.get(name)
    }

    /// 检查角色是否存在
    pub fn has_role(&self, name: &str) -> bool {
        self.roles.contains_key(name)
    }

    /// 获取所有角色名
    pub fn role_names(&self) -> Vec<&str> {
        self.roles.keys().map(|s| s.as_str()).collect()
    }

    /// 获取角色的直接父角色
    pub fn direct_parents(&self, role: &str) -> Vec<&str> {
        self.parents
            .get(role)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// 获取角色的直接子角色
    pub fn direct_children(&self, role: &str) -> Vec<&str> {
        self.children
            .get(role)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// 获取角色的所有祖先角色（包括间接继承，按 BFS 顺序）
    pub fn all_ancestors(&self, role: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // 从直接父角色开始
        if let Some(parents) = self.parents.get(role) {
            for p in parents {
                if visited.insert(p.clone()) {
                    queue.push_back(p.clone());
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());
            if let Some(parents) = self.parents.get(&current) {
                for p in parents {
                    if visited.insert(p.clone()) {
                        queue.push_back(p.clone());
                    }
                }
            }
        }

        result
    }

    /// 获取角色的所有后代角色（包括间接，按 BFS 顺序）
    pub fn all_descendants(&self, role: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        if let Some(children) = self.children.get(role) {
            for c in children {
                if visited.insert(c.clone()) {
                    queue.push_back(c.clone());
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());
            if let Some(children) = self.children.get(&current) {
                for c in children {
                    if visited.insert(c.clone()) {
                        queue.push_back(c.clone());
                    }
                }
            }
        }

        result
    }

    /// 展开角色的所有权限（包括继承的权限）
    ///
    /// 返回去重后的权限列表。
    pub fn resolve_permissions(&self, role: &str) -> Vec<Permission> {
        let mut perms = Vec::new();
        let mut seen = HashSet::new();

        // 收集角色本身及其所有祖先的权限
        let mut all_roles = vec![role.to_string()];
        all_roles.extend(self.all_ancestors(role));

        for role_name in &all_roles {
            if let Some(role_def) = self.roles.get(role_name) {
                for perm in &role_def.permissions {
                    let key = format!("{}:{}", perm.action.as_str(), perm.resource_pattern);
                    if seen.insert(key) {
                        perms.push(perm.clone());
                    }
                }
            }
        }

        perms
    }

    /// 展开多个角色的所有权限（合并去重）
    pub fn resolve_permissions_multi(&self, roles: &[String]) -> Vec<Permission> {
        let mut perms = Vec::new();
        let mut seen = HashSet::new();

        for role in roles {
            let role_perms = self.resolve_permissions(role);
            for perm in role_perms {
                let key = format!("{}:{}", perm.action.as_str(), perm.resource_pattern);
                if seen.insert(key) {
                    perms.push(perm);
                }
            }
        }

        perms
    }

    /// 检查角色是否继承自另一个角色（传递性）
    pub fn inherits_from(&self, role: &str, ancestor: &str) -> bool {
        self.all_ancestors(role).iter().any(|a| a == ancestor)
    }

    /// 检测图中是否存在循环（从指定角色开始 DFS）
    fn detect_cycle(start: &str, parents: &HashMap<String, Vec<String>>) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start.to_string()];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                // 如果回到了起点且不是第一次访问，则有循环
                // 更精确的循环检测
                continue;
            }

            if let Some(parent_list) = parents.get(&current) {
                for parent in parent_list {
                    if parent == start {
                        return true; // 回到起点，发现循环
                    }
                    if !visited.contains(parent) {
                        stack.push(parent.clone());
                    }
                }
            }
        }

        false
    }

    /// 验证整个继承图无环
    pub fn validate_no_cycles(&self) -> Result<(), RbacError> {
        for role in self.roles.keys() {
            if Self::detect_cycle(role, &self.parents) {
                return Err(RbacError::CyclicInheritance(role.clone()));
            }
        }
        Ok(())
    }

    /// 获取角色总数
    pub fn len(&self) -> usize {
        self.roles.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty()
    }

    /// 获取所有角色的引用
    pub fn roles(&self) -> &HashMap<String, Role> {
        &self.roles
    }
}

impl Default for RoleHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Role};

    fn make_test_hierarchy() -> RoleHierarchy {
        let roles = vec![
            Role::new("admin")
                .extends("editor")
                .grant(Action::Admin, "*"),
            Role::new("editor")
                .extends("viewer")
                .grant(Action::Write, "db:test/*")
                .grant(Action::Write, "flow:*"),
            Role::new("viewer")
                .grant(Action::Read, "db:*")
                .grant(Action::Read, "flow:*"),
            Role::new("auditor").grant(Action::Read, "audit:*"),
        ];
        RoleHierarchy::from_roles(roles).unwrap()
    }

    #[test]
    fn create_hierarchy_from_roles() {
        let h = make_test_hierarchy();
        assert_eq!(h.len(), 4);
        assert!(h.has_role("admin"));
        assert!(h.has_role("editor"));
        assert!(h.has_role("viewer"));
        assert!(h.has_role("auditor"));
    }

    #[test]
    fn direct_parents() {
        let h = make_test_hierarchy();
        assert_eq!(h.direct_parents("admin"), vec!["editor"]);
        assert_eq!(h.direct_parents("editor"), vec!["viewer"]);
        assert!(h.direct_parents("viewer").is_empty());
        assert!(h.direct_parents("auditor").is_empty());
    }

    #[test]
    fn direct_children() {
        let h = make_test_hierarchy();
        assert_eq!(h.direct_children("viewer"), vec!["editor"]);
        assert_eq!(h.direct_children("editor"), vec!["admin"]);
        assert!(h.direct_children("admin").is_empty());
    }

    #[test]
    fn all_ancestors_single_chain() {
        let h = make_test_hierarchy();
        let ancestors = h.all_ancestors("admin");
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&"editor".to_string()));
        assert!(ancestors.contains(&"viewer".to_string()));
    }

    #[test]
    fn all_ancestors_no_parents() {
        let h = make_test_hierarchy();
        assert!(h.all_ancestors("viewer").is_empty());
        assert!(h.all_ancestors("auditor").is_empty());
    }

    #[test]
    fn all_descendants() {
        let h = make_test_hierarchy();
        let descendants = h.all_descendants("viewer");
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&"editor".to_string()));
        assert!(descendants.contains(&"admin".to_string()));
    }

    #[test]
    fn inherits_from_transitive() {
        let h = make_test_hierarchy();
        assert!(h.inherits_from("admin", "editor"));
        assert!(h.inherits_from("admin", "viewer"));
        assert!(h.inherits_from("editor", "viewer"));
        assert!(!h.inherits_from("viewer", "admin"));
        assert!(!h.inherits_from("auditor", "viewer"));
    }

    #[test]
    fn resolve_permissions_with_inheritance() {
        let h = make_test_hierarchy();

        // viewer 只有读权限
        let viewer_perms = h.resolve_permissions("viewer");
        assert_eq!(viewer_perms.len(), 2);

        // editor 有自己的写权限 + viewer 的读权限
        let editor_perms = h.resolve_permissions("editor");
        assert_eq!(editor_perms.len(), 4); // 2 (viewer) + 2 (editor)

        // admin 有 admin 权限 + editor 的所有权限 + viewer 的所有权限
        let admin_perms = h.resolve_permissions("admin");
        assert_eq!(admin_perms.len(), 5); // 4 (editor+viewer) + 1 (admin)
    }

    #[test]
    fn resolve_permissions_multi() {
        let h = make_test_hierarchy();

        // 同时有 editor 和 auditor 角色
        let perms =
            h.resolve_permissions_multi(&["editor".to_string(), "auditor".to_string()]);

        // editor 4 个权限 + auditor 1 个权限 = 5 个
        assert_eq!(perms.len(), 5);
    }

    #[test]
    fn cyclic_inheritance_detected_on_add() {
        let mut h = RoleHierarchy::new();
        h.add_role(Role::new("a").extends("b").grant_str("read", "x"))
            .unwrap();

        // 添加 b 继承 a 应该失败（形成循环）
        let result = h.add_role(Role::new("b").extends("a").grant_str("write", "y"));
        assert!(result.is_err());
        match result.unwrap_err() {
            RbacError::CyclicInheritance(role) => assert_eq!(role, "b"),
            _ => panic!("expected CyclicInheritance"),
        }
    }

    #[test]
    fn cyclic_inheritance_detected_on_construction() {
        let roles = vec![
            Role::new("a").extends("b"),
            Role::new("b").extends("c"),
            Role::new("c").extends("a"),
        ];
        let result = RoleHierarchy::from_roles(roles);
        assert!(result.is_err());
    }

    #[test]
    fn validate_no_cycles_ok() {
        let h = make_test_hierarchy();
        assert!(h.validate_no_cycles().is_ok());
    }

    #[test]
    fn remove_role() {
        let mut h = make_test_hierarchy();
        assert!(h.has_role("auditor"));

        let removed = h.remove_role("auditor");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "auditor");
        assert!(!h.has_role("auditor"));
        assert_eq!(h.len(), 3);

        // 移除不存在的角色
        let removed = h.remove_role("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn remove_role_updates_children() {
        let mut h = make_test_hierarchy();
        assert!(h.inherits_from("admin", "viewer"));

        // 移除 editor 应该中断 admin 到 viewer 的继承链
        h.remove_role("editor");
        assert!(!h.has_role("editor"));
        assert!(!h.inherits_from("admin", "viewer"));

        // admin 的直接父角色应该为空
        assert!(h.direct_parents("admin").is_empty());
    }

    #[test]
    fn add_role_updates_existing() {
        let mut h = make_test_hierarchy();
        assert_eq!(h.resolve_permissions("viewer").len(), 2);

        // 更新 viewer，添加新权限
        let updated_viewer = Role::new("viewer")
            .grant_str("read", "db:*")
            .grant_str("read", "flow:*")
            .grant_str("read", "audit:*");

        h.add_role(updated_viewer).unwrap();
        assert_eq!(h.resolve_permissions("viewer").len(), 3);
    }

    #[test]
    fn multiple_inheritance() {
        let mut h = RoleHierarchy::new();
        h.add_role(Role::new("reader").grant_str("read", "*"))
            .unwrap();
        h.add_role(Role::new("writer").grant_str("write", "*"))
            .unwrap();
        h.add_role(
            Role::new("editor")
                .extends("reader")
                .extends("writer"),
        )
        .unwrap();

        // editor 应该继承 reader 和 writer 的权限
        let perms = h.resolve_permissions("editor");
        assert_eq!(perms.len(), 2); // read + write
    }

    #[test]
    fn diamond_inheritance_dedup() {
        // A > B, A > C, B > D, C > D（菱形继承）
        // D 的权限不应重复计算
        let mut h = RoleHierarchy::new();
        h.add_role(Role::new("base").grant_str("read", "shared:*"))
            .unwrap();
        h.add_role(Role::new("left").extends("base").grant_str("read", "left:*"))
            .unwrap();
        h.add_role(
            Role::new("right")
                .extends("base")
                .grant_str("read", "right:*"),
        )
        .unwrap();
        h.add_role(
            Role::new("top")
                .extends("left")
                .extends("right")
                .grant_str("admin", "*"),
        )
        .unwrap();

        let perms = h.resolve_permissions("top");
        // base: 1, left: 1, right: 1, top: 1 = 4（base 不重复）
        assert_eq!(perms.len(), 4);
    }

    #[test]
    fn role_names_list() {
        let h = make_test_hierarchy();
        let mut names = h.role_names();
        names.sort();
        assert_eq!(names, vec!["admin", "auditor", "editor", "viewer"]);
    }

    #[test]
    fn empty_hierarchy() {
        let h = RoleHierarchy::new();
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert!(h.validate_no_cycles().is_ok());
    }
}
