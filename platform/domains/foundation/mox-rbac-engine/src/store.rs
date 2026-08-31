// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 策略存储层
//!
//! 定义可插拔的策略存储 trait，支持多种后端实现：
//! - [`MemoryPolicyStore`] — 内存存储（默认，适用于单进程/测试场景）
//! - 可扩展：数据库存储、配置文件存储、远程策略服务等
//!
//! 存储层负责角色和策略的持久化与查询，与评估引擎解耦。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::RbacError;
use crate::hierarchy::RoleHierarchy;
use crate::types::{Policy, Role};

/// 策略存储 trait
///
/// 定义角色和策略的 CRUD 操作。
/// 所有操作都返回 `Result`，支持存储层错误传播。
pub trait PolicyStore: Send + Sync {
    // ── 角色操作 ──────────────────────────────────────────────────────────

    /// 获取角色
    fn get_role(&self, name: &str) -> Result<Option<Role>, RbacError>;

    /// 列出所有角色
    fn list_roles(&self) -> Result<Vec<Role>, RbacError>;

    /// 创建角色
    fn create_role(&self, role: Role) -> Result<(), RbacError>;

    /// 更新角色
    fn update_role(&self, role: Role) -> Result<(), RbacError>;

    /// 删除角色
    fn delete_role(&self, name: &str) -> Result<bool, RbacError>;

    /// 检查角色是否存在
    fn has_role(&self, name: &str) -> Result<bool, RbacError> {
        Ok(self.get_role(name)?.is_some())
    }

    // ── 策略操作 ──────────────────────────────────────────────────────────

    /// 获取策略
    fn get_policy(&self, id: &str) -> Result<Option<Policy>, RbacError>;

    /// 列出所有策略
    fn list_policies(&self) -> Result<Vec<Policy>, RbacError>;

    /// 创建策略
    fn create_policy(&self, policy: Policy) -> Result<(), RbacError>;

    /// 更新策略
    fn update_policy(&self, policy: Policy) -> Result<(), RbacError>;

    /// 删除策略
    fn delete_policy(&self, id: &str) -> Result<bool, RbacError>;

    /// 按角色模式查找适用的策略
    fn find_policies_by_role(&self, role: &str) -> Result<Vec<Policy>, RbacError>;

    // ── 批量操作 ──────────────────────────────────────────────────────────

    /// 批量加载角色和策略（用于初始化）
    fn load_all(&self) -> Result<(Vec<Role>, Vec<Policy>), RbacError> {
        let roles = self.list_roles()?;
        let policies = self.list_policies()?;
        Ok((roles, policies))
    }

    /// 批量保存（用于导入）
    fn save_all(&self, roles: Vec<Role>, policies: Vec<Policy>) -> Result<(), RbacError> {
        for role in roles {
            self.create_role(role)?;
        }
        for policy in policies {
            self.create_policy(policy)?;
        }
        Ok(())
    }

    // ── 构建继承树 ────────────────────────────────────────────────────────

    /// 构建角色继承树
    fn build_hierarchy(&self) -> Result<RoleHierarchy, RbacError> {
        let roles = self.list_roles()?;
        RoleHierarchy::from_roles(roles)
    }
}

// ── MemoryPolicyStore ───────────────────────────────────────────────────────

/// 内存策略存储
///
/// 基于 `HashMap` 的内存实现，适用于：
/// - 单进程应用
/// - 测试和开发环境
/// - 策略从外部加载后缓存到内存
///
/// 线程安全：内部使用 `RwLock` 保护，支持并发读写。
#[derive(Debug, Clone)]
pub struct MemoryPolicyStore {
    inner: Arc<MemoryStoreInner>,
}

#[derive(Debug)]
struct MemoryStoreInner {
    roles: RwLock<HashMap<String, Role>>,
    policies: RwLock<HashMap<String, Policy>>,
}

impl MemoryPolicyStore {
    /// 创建空的内存存储
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MemoryStoreInner {
                roles: RwLock::new(HashMap::new()),
                policies: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// 使用内置角色初始化
    pub fn with_builtin_roles() -> Self {
        let store = Self::new();
        for role in crate::types::BuiltinRoles::all() {
            store.create_role(role).expect("create builtin role");
        }
        store
    }

    /// 角色数量
    pub fn role_count(&self) -> usize {
        self.inner.roles.read().unwrap().len()
    }

    /// 策略数量
    pub fn policy_count(&self) -> usize {
        self.inner.policies.read().unwrap().len()
    }
}

impl Default for MemoryPolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyStore for MemoryPolicyStore {
    fn get_role(&self, name: &str) -> Result<Option<Role>, RbacError> {
        Ok(self
            .inner
            .roles
            .read()
            .map_err(|e| RbacError::StoreError(format!("roles lock poisoned: {e}")))?
            .get(name)
            .cloned())
    }

    fn list_roles(&self) -> Result<Vec<Role>, RbacError> {
        Ok(self
            .inner
            .roles
            .read()
            .map_err(|e| RbacError::StoreError(format!("roles lock poisoned: {e}")))?
            .values()
            .cloned()
            .collect())
    }

    fn create_role(&self, role: Role) -> Result<(), RbacError> {
        let name = role.name.clone();
        let mut roles = self
            .inner
            .roles
            .write()
            .map_err(|e| RbacError::StoreError(format!("roles lock poisoned: {e}")))?;

        if roles.contains_key(&name) {
            return Err(RbacError::StoreError(format!(
                "role '{}' already exists",
                name
            )));
        }

        roles.insert(name, role);
        Ok(())
    }

    fn update_role(&self, role: Role) -> Result<(), RbacError> {
        let name = role.name.clone();
        let mut roles = self
            .inner
            .roles
            .write()
            .map_err(|e| RbacError::StoreError(format!("roles lock poisoned: {e}")))?;

        if !roles.contains_key(&name) {
            return Err(RbacError::RoleNotFound(name));
        }

        roles.insert(name, role);
        Ok(())
    }

    fn delete_role(&self, name: &str) -> Result<bool, RbacError> {
        Ok(self
            .inner
            .roles
            .write()
            .map_err(|e| RbacError::StoreError(format!("roles lock poisoned: {e}")))?
            .remove(name)
            .is_some())
    }

    fn get_policy(&self, id: &str) -> Result<Option<Policy>, RbacError> {
        Ok(self
            .inner
            .policies
            .read()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?
            .get(id)
            .cloned())
    }

    fn list_policies(&self) -> Result<Vec<Policy>, RbacError> {
        Ok(self
            .inner
            .policies
            .read()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?
            .values()
            .cloned()
            .collect())
    }

    fn create_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let id = policy.id.clone();
        let mut policies = self
            .inner
            .policies
            .write()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?;

        if policies.contains_key(&id) {
            return Err(RbacError::StoreError(format!(
                "policy '{}' already exists",
                id
            )));
        }

        policies.insert(id, policy);
        Ok(())
    }

    fn update_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let id = policy.id.clone();
        let mut policies = self
            .inner
            .policies
            .write()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?;

        if !policies.contains_key(&id) {
            return Err(RbacError::PolicyNotFound(id));
        }

        policies.insert(id, policy);
        Ok(())
    }

    fn delete_policy(&self, id: &str) -> Result<bool, RbacError> {
        Ok(self
            .inner
            .policies
            .write()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?
            .remove(id)
            .is_some())
    }

    fn find_policies_by_role(&self, role: &str) -> Result<Vec<Policy>, RbacError> {
        let policies = self
            .inner
            .policies
            .read()
            .map_err(|e| RbacError::StoreError(format!("policies lock poisoned: {e}")))?;

        let mut result = Vec::new();
        for policy in policies.values() {
            for pattern in &policy.role_patterns {
                if role_pattern_matches(pattern, role) {
                    result.push(policy.clone());
                    break;
                }
            }
        }

        // 按优先级排序（数字越小优先级越高）
        result.sort_by_key(|p| p.priority);
        Ok(result)
    }
}

/// 角色模式匹配
///
/// 支持通配符：
/// - `*` 匹配所有角色
/// - `admin_*` 前缀匹配
/// - `*_admin` 后缀匹配
/// - 精确匹配
fn role_pattern_matches(pattern: &str, role: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return role.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return role.ends_with(suffix);
    }
    pattern == role
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Effect, Policy, Role};

    fn make_store() -> MemoryPolicyStore {
        let store = MemoryPolicyStore::new();
        store
            .create_role(Role::new("admin").grant_str("admin", "*"))
            .unwrap();
        store
            .create_role(Role::new("viewer").grant_str("read", "*"))
            .unwrap();
        store
            .create_policy(
                Policy::new("p1", "admin-all", Effect::Allow)
                    .for_role("admin")
                    .on_resource("*")
                    .with_action(Action::All),
            )
            .unwrap();
        store
            .create_policy(
                Policy::new("p2", "viewer-read", Effect::Allow)
                    .for_role("viewer")
                    .on_resource("*")
                    .with_action(Action::Read),
            )
            .unwrap();
        store
    }

    #[test]
    fn create_and_get_role() {
        let store = MemoryPolicyStore::new();
        assert_eq!(store.role_count(), 0);

        let role = Role::new("test_role").grant_str("read", "test:*");
        store.create_role(role).unwrap();
        assert_eq!(store.role_count(), 1);

        let got = store.get_role("test_role").unwrap().unwrap();
        assert_eq!(got.name, "test_role");
        assert_eq!(got.permissions.len(), 1);
    }

    #[test]
    fn create_duplicate_role_fails() {
        let store = MemoryPolicyStore::new();
        store.create_role(Role::new("r1")).unwrap();
        let result = store.create_role(Role::new("r1"));
        assert!(result.is_err());
    }

    #[test]
    fn update_role() {
        let store = MemoryPolicyStore::new();
        store
            .create_role(Role::new("r1").grant_str("read", "a:*"))
            .unwrap();

        store
            .update_role(Role::new("r1").grant_str("read", "a:*").grant_str("write", "b:*"))
            .unwrap();

        let got = store.get_role("r1").unwrap().unwrap();
        assert_eq!(got.permissions.len(), 2);
    }

    #[test]
    fn update_nonexistent_role_fails() {
        let store = MemoryPolicyStore::new();
        let result = store.update_role(Role::new("nonexistent"));
        assert!(result.is_err());
        match result.unwrap_err() {
            RbacError::RoleNotFound(_) => {}
            _ => panic!("expected RoleNotFound"),
        }
    }

    #[test]
    fn delete_role() {
        let store = MemoryPolicyStore::new();
        store.create_role(Role::new("r1")).unwrap();
        assert!(store.has_role("r1").unwrap());

        let deleted = store.delete_role("r1").unwrap();
        assert!(deleted);
        assert!(!store.has_role("r1").unwrap());

        let deleted = store.delete_role("r1").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn list_roles() {
        let store = make_store();
        let roles = store.list_roles().unwrap();
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn policy_crud() {
        let store = MemoryPolicyStore::new();

        let policy = Policy::new("p1", "test-policy", Effect::Allow)
            .for_role("admin")
            .on_resource("db:*")
            .with_action(Action::Read);

        store.create_policy(policy).unwrap();
        assert_eq!(store.policy_count(), 1);

        let got = store.get_policy("p1").unwrap().unwrap();
        assert_eq!(got.name, "test-policy");
        assert_eq!(got.role_patterns, vec!["admin"]);

        // 更新
        store
            .update_policy(
                Policy::new("p1", "updated-policy", Effect::Allow)
                    .for_role("admin")
                    .on_resource("db:*")
                    .with_action(Action::Read)
                    .with_priority(50),
            )
            .unwrap();

        let got = store.get_policy("p1").unwrap().unwrap();
        assert_eq!(got.name, "updated-policy");
        assert_eq!(got.priority, 50);

        // 删除
        assert!(store.delete_policy("p1").unwrap());
        assert_eq!(store.policy_count(), 0);
    }

    #[test]
    fn find_policies_by_role() {
        let store = make_store();

        // admin 角色应匹配 p1
        let admin_policies = store.find_policies_by_role("admin").unwrap();
        assert_eq!(admin_policies.len(), 1);
        assert_eq!(admin_policies[0].id, "p1");

        // viewer 角色应匹配 p2
        let viewer_policies = store.find_policies_by_role("viewer").unwrap();
        assert_eq!(viewer_policies.len(), 1);
        assert_eq!(viewer_policies[0].id, "p2");
    }

    #[test]
    fn role_pattern_matching() {
        assert!(role_pattern_matches("*", "anything"));
        assert!(role_pattern_matches("admin", "admin"));
        assert!(!role_pattern_matches("admin", "viewer"));
        assert!(role_pattern_matches("admin_*", "admin_user"));
        assert!(role_pattern_matches("admin_*", "admin_"));
        assert!(!role_pattern_matches("admin_*", "xadmin_"));
        assert!(role_pattern_matches("*_admin", "super_admin"));
        assert!(!role_pattern_matches("*_admin", "admin_super"));
    }

    #[test]
    fn find_policies_with_wildcard_pattern() {
        let store = MemoryPolicyStore::new();
        store
            .create_policy(
                Policy::new("p1", "all-roles", Effect::Allow)
                    .for_role("*")
                    .on_resource("public:*")
                    .with_action(Action::Read),
            )
            .unwrap();

        // 任何角色都应该匹配通配符策略
        let policies = store.find_policies_by_role("any_random_role").unwrap();
        assert_eq!(policies.len(), 1);
    }

    #[test]
    fn policies_sorted_by_priority() {
        let store = MemoryPolicyStore::new();
        store
            .create_policy(
                Policy::new("p-low", "low", Effect::Allow)
                    .for_role("admin")
                    .with_priority(200),
            )
            .unwrap();
        store
            .create_policy(
                Policy::new("p-high", "high", Effect::Deny)
                    .for_role("admin")
                    .with_priority(50),
            )
            .unwrap();
        store
            .create_policy(
                Policy::new("p-mid", "mid", Effect::Allow)
                    .for_role("admin")
                    .with_priority(100),
            )
            .unwrap();

        let policies = store.find_policies_by_role("admin").unwrap();
        assert_eq!(policies.len(), 3);
        // 按优先级从小到大排序
        assert_eq!(policies[0].id, "p-high"); // 50
        assert_eq!(policies[1].id, "p-mid"); // 100
        assert_eq!(policies[2].id, "p-low"); // 200
    }

    #[test]
    fn load_all_and_save_all() {
        let store = MemoryPolicyStore::new();
        let roles = vec![
            Role::new("r1").grant_str("read", "a:*"),
            Role::new("r2").grant_str("write", "b:*"),
        ];
        let policies = vec![
            Policy::new("p1", "pol1", Effect::Allow).for_role("r1"),
            Policy::new("p2", "pol2", Effect::Deny).for_role("r2"),
        ];

        store.save_all(roles, policies).unwrap();
        assert_eq!(store.role_count(), 2);
        assert_eq!(store.policy_count(), 2);

        let (loaded_roles, loaded_policies) = store.load_all().unwrap();
        assert_eq!(loaded_roles.len(), 2);
        assert_eq!(loaded_policies.len(), 2);
    }

    #[test]
    fn build_hierarchy() {
        let store = MemoryPolicyStore::new();
        store
            .create_role(Role::new("admin").extends("viewer").grant_str("admin", "*"))
            .unwrap();
        store
            .create_role(Role::new("viewer").grant_str("read", "*"))
            .unwrap();

        let hierarchy = store.build_hierarchy().unwrap();
        assert_eq!(hierarchy.len(), 2);
        assert!(hierarchy.inherits_from("admin", "viewer"));
    }

    #[test]
    fn with_builtin_roles() {
        let store = MemoryPolicyStore::with_builtin_roles();
        assert_eq!(store.role_count(), 6);
        assert!(store.has_role("admin").unwrap());
        assert!(store.has_role("viewer").unwrap());
        assert!(store.has_role("editor").unwrap());
    }

    #[test]
    fn has_role() {
        let store = make_store();
        assert!(store.has_role("admin").unwrap());
        assert!(store.has_role("viewer").unwrap());
        assert!(!store.has_role("nonexistent").unwrap());
    }
}
