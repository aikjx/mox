// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 多租户管理
//!
//! 支持：
//! - 租户生命周期管理
//! - 层级租户（父子关系）
//! - 租户配额管理
//! - 租户隔离验证

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{PermError, PermResult};
use crate::types::{Tenant, TenantStatus, User, UserStatus, now_ms};

/// 租户管理器
pub struct TenantManager {
    /// 租户表
    tenants: RwLock<HashMap<String, Tenant>>,
    /// 编码 -> ID 映射
    code_index: RwLock<HashMap<String, String>>,
    /// 用户表
    users: RwLock<HashMap<String, User>>,
    /// 用户名索引（租户内唯一）
    username_index: RwLock<HashMap<String, String>>, // "tenant:username" -> user_id
    /// 子租户索引
    child_tenants: RwLock<HashMap<String, Vec<String>>>,
}

impl TenantManager {
    /// 创建租户管理器
    pub fn new() -> Self {
        Self {
            tenants: RwLock::new(HashMap::new()),
            code_index: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
            username_index: RwLock::new(HashMap::new()),
            child_tenants: RwLock::new(HashMap::new()),
        }
    }

    // ---------- 租户管理 ----------

    /// 创建租户
    pub fn create_tenant(&self, tenant: Tenant) -> PermResult<Tenant> {
        // 检查编码唯一性
        if self.code_index.read().contains_key(&tenant.code) {
            return Err(PermError::AlreadyExists(format!(
                "tenant code '{}' already exists",
                tenant.code
            )));
        }

        // 验证父租户存在
        if let Some(parent_id) = &tenant.parent_id {
            if !self.tenants.read().contains_key(parent_id) {
                return Err(PermError::NotFound(format!(
                    "parent tenant '{}' not found",
                    parent_id
                )));
            }
        }

        self.code_index
            .write()
            .insert(tenant.code.clone(), tenant.id.clone());

        // 加入父租户的子列表
        if let Some(parent_id) = &tenant.parent_id {
            self.child_tenants
                .write()
                .entry(parent_id.clone())
                .or_default()
                .push(tenant.id.clone());
        }

        self.tenants
            .write()
            .insert(tenant.id.clone(), tenant.clone());
        Ok(tenant)
    }

    /// 获取租户
    pub fn get_tenant(&self, tenant_id: &str) -> PermResult<Tenant> {
        self.tenants
            .read()
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", tenant_id)))
    }

    /// 按编码获取租户
    pub fn get_tenant_by_code(&self, code: &str) -> PermResult<Tenant> {
        let tenant_id = self
            .code_index
            .read()
            .get(code)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", code)))?;
        self.get_tenant(&tenant_id)
    }

    /// 检查租户是否存在且活跃
    pub fn is_tenant_active(&self, tenant_id: &str) -> bool {
        self.tenants
            .read()
            .get(tenant_id)
            .map(|t| t.is_active())
            .unwrap_or(false)
    }

    /// 更新租户
    pub fn update_tenant(&self, tenant_id: &str, mut update: Tenant) -> PermResult<Tenant> {
        let mut tenants = self.tenants.write();
        let existing = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", tenant_id)))?;

        update.id = tenant_id.to_string();
        update.code = existing.code.clone(); // 编码不可改
        update.created_at = existing.created_at;
        update.updated_at = now_ms();

        *existing = update.clone();
        Ok(update)
    }

    /// 禁用租户
    pub fn suspend_tenant(&self, tenant_id: &str) -> PermResult<()> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", tenant_id)))?;
        tenant.status = TenantStatus::Suspended;
        tenant.updated_at = now_ms();
        Ok(())
    }

    /// 激活租户
    pub fn activate_tenant(&self, tenant_id: &str) -> PermResult<()> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", tenant_id)))?;
        tenant.status = TenantStatus::Active;
        tenant.updated_at = now_ms();
        Ok(())
    }

    /// 删除租户（软删除）
    pub fn delete_tenant(&self, tenant_id: &str) -> PermResult<()> {
        // 检查是否有子租户
        if let Some(children) = self.child_tenants.read().get(tenant_id) {
            if !children.is_empty() {
                return Err(PermError::InvalidArgument(
                    "cannot delete tenant with child tenants".to_string(),
                ));
            }
        }

        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| PermError::NotFound(format!("tenant '{}' not found", tenant_id)))?;

        tenant.status = TenantStatus::Deleted;
        tenant.updated_at = now_ms();
        Ok(())
    }

    /// 列出所有活跃租户
    pub fn list_active_tenants(&self) -> Vec<Tenant> {
        self.tenants
            .read()
            .values()
            .filter(|t| matches!(t.status, TenantStatus::Active))
            .cloned()
            .collect()
    }

    /// 获取子租户列表
    pub fn get_child_tenants(&self, parent_id: &str) -> Vec<Tenant> {
        let child_ids = self
            .child_tenants
            .read()
            .get(parent_id)
            .cloned()
            .unwrap_or_default();
        let tenants = self.tenants.read();
        child_ids
            .into_iter()
            .filter_map(|id| tenants.get(&id).cloned())
            .collect()
    }

    /// 获取祖先租户链（从父到根）
    pub fn get_ancestor_chain(&self, tenant_id: &str) -> Vec<Tenant> {
        let mut result = Vec::new();
        let tenants = self.tenants.read();
        let mut current = tenant_id.to_string();

        while let Some(tenant) = tenants.get(&current) {
            if let Some(parent_id) = &tenant.parent_id {
                if let Some(parent) = tenants.get(parent_id) {
                    result.push(parent.clone());
                    current = parent_id.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// 检查是否是目标租户的后代
    pub fn is_descendant_of(&self, tenant_id: &str, ancestor_id: &str) -> bool {
        let tenants = self.tenants.read();
        let mut current = tenant_id.to_string();

        while let Some(tenant) = tenants.get(&current) {
            if current == ancestor_id {
                return true;
            }
            match &tenant.parent_id {
                Some(pid) => current = pid.clone(),
                None => return false,
            }
        }

        false
    }

    // ---------- 用户管理 ----------

    /// 创建用户
    pub fn create_user(&self, user: User) -> PermResult<User> {
        // 验证租户存在
        self.get_tenant(&user.tenant_id)?;

        // 验证租户内用户名唯一
        let key = user_key(&user.tenant_id, &user.username);
        if self.username_index.read().contains_key(&key) {
            return Err(PermError::AlreadyExists(format!(
                "username '{}' already exists in tenant '{}'",
                user.username, user.tenant_id
            )));
        }

        self.username_index
            .write()
            .insert(key, user.id.clone());
        self.users.write().insert(user.id.clone(), user.clone());
        Ok(user)
    }

    /// 获取用户
    pub fn get_user(&self, user_id: &str) -> PermResult<User> {
        self.users
            .read()
            .get(user_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("user '{}' not found", user_id)))
    }

    /// 按用户名获取用户
    pub fn get_user_by_username(&self, tenant_id: &str, username: &str) -> PermResult<User> {
        let key = user_key(tenant_id, username);
        let user_id = self
            .username_index
            .read()
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                PermError::NotFound(format!(
                    "user '{}' not found in tenant '{}'",
                    username, tenant_id
                ))
            })?;
        self.get_user(&user_id)
    }

    /// 更新用户
    pub fn update_user(&self, user_id: &str, mut update: User) -> PermResult<User> {
        let mut users = self.users.write();
        let existing = users
            .get_mut(user_id)
            .ok_or_else(|| PermError::NotFound(format!("user '{}' not found", user_id)))?;

        update.id = user_id.to_string();
        update.tenant_id = existing.tenant_id.clone(); // 租户不可改
        update.username = existing.username.clone(); // 用户名不可改
        update.created_at = existing.created_at;
        update.updated_at = now_ms();

        *existing = update.clone();
        Ok(update)
    }

    /// 禁用用户
    pub fn disable_user(&self, user_id: &str) -> PermResult<()> {
        let mut users = self.users.write();
        let user = users
            .get_mut(user_id)
            .ok_or_else(|| PermError::NotFound(format!("user '{}' not found", user_id)))?;
        user.status = UserStatus::Disabled;
        user.updated_at = now_ms();
        Ok(())
    }

    /// 启用用户
    pub fn enable_user(&self, user_id: &str) -> PermResult<()> {
        let mut users = self.users.write();
        let user = users
            .get_mut(user_id)
            .ok_or_else(|| PermError::NotFound(format!("user '{}' not found", user_id)))?;
        user.status = UserStatus::Active;
        user.updated_at = now_ms();
        Ok(())
    }

    /// 记录登录
    pub fn record_login(&self, user_id: &str) -> PermResult<()> {
        let mut users = self.users.write();
        let user = users
            .get_mut(user_id)
            .ok_or_else(|| PermError::NotFound(format!("user '{}' not found", user_id)))?;
        user.last_login_at = Some(now_ms());
        Ok(())
    }

    /// 列出租户用户
    pub fn list_users(&self, tenant_id: &str) -> Vec<User> {
        self.users
            .read()
            .values()
            .filter(|u| u.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// 统计租户用户数
    pub fn count_users(&self, tenant_id: &str) -> u64 {
        self.users
            .read()
            .values()
            .filter(|u| u.tenant_id == tenant_id)
            .count() as u64
    }

    /// 租户总数
    pub fn tenant_count(&self) -> usize {
        self.tenants.read().len()
    }

    /// 用户总数
    pub fn user_count(&self) -> usize {
        self.users.read().len()
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成用户键
fn user_key(tenant_id: &str, username: &str) -> String {
    format!("{}:{}", tenant_id, username)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> TenantManager {
        let mgr = TenantManager::new();

        // 创建根租户
        let root = Tenant::new("Root Tenant", "root");
        mgr.create_tenant(root).unwrap();

        // 创建子租户
        let root_id = mgr.get_tenant_by_code("root").unwrap().id;
        let mut child = Tenant::new("Child Tenant", "child");
        child.parent_id = Some(root_id.clone());
        mgr.create_tenant(child).unwrap();

        mgr
    }

    #[test]
    fn test_create_tenant() {
        let mgr = setup();
        assert_eq!(mgr.tenant_count(), 2);
    }

    #[test]
    fn test_duplicate_code() {
        let mgr = setup();
        let tenant = Tenant::new("Another", "root");
        assert!(mgr.create_tenant(tenant).is_err());
    }

    #[test]
    fn test_child_tenants() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();
        let children = mgr.get_child_tenants(&root.id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].code, "child");
    }

    #[test]
    fn test_ancestor_chain() {
        let mgr = setup();
        let child = mgr.get_tenant_by_code("child").unwrap();
        let ancestors = mgr.get_ancestor_chain(&child.id);
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0].code, "root");
    }

    #[test]
    fn test_is_descendant() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();
        let child = mgr.get_tenant_by_code("child").unwrap();

        assert!(mgr.is_descendant_of(&child.id, &root.id));
        assert!(!mgr.is_descendant_of(&root.id, &child.id));
    }

    #[test]
    fn test_suspend_activate() {
        let mgr = setup();
        let child = mgr.get_tenant_by_code("child").unwrap();

        mgr.suspend_tenant(&child.id).unwrap();
        assert!(!mgr.is_tenant_active(&child.id));

        mgr.activate_tenant(&child.id).unwrap();
        assert!(mgr.is_tenant_active(&child.id));
    }

    #[test]
    fn test_create_user() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();

        let user = User::new("alice", "Alice", &root.id);
        let created = mgr.create_user(user).unwrap();
        assert_eq!(created.username, "alice");
        assert_eq!(mgr.user_count(), 1);
    }

    #[test]
    fn test_duplicate_username() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();

        let user1 = User::new("alice", "Alice", &root.id);
        mgr.create_user(user1).unwrap();

        let user2 = User::new("alice", "Alice 2", &root.id);
        assert!(mgr.create_user(user2).is_err());
    }

    #[test]
    fn test_user_status() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();
        let user = User::new("bob", "Bob", &root.id);
        let user = mgr.create_user(user).unwrap();

        assert!(user.is_active());

        mgr.disable_user(&user.id).unwrap();
        let user = mgr.get_user(&user.id).unwrap();
        assert!(!user.is_active());

        mgr.enable_user(&user.id).unwrap();
        let user = mgr.get_user(&user.id).unwrap();
        assert!(user.is_active());
    }

    #[test]
    fn test_login_record() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();
        let user = User::new("charlie", "Charlie", &root.id);
        let user = mgr.create_user(user).unwrap();

        assert!(user.last_login_at.is_none());
        mgr.record_login(&user.id).unwrap();
        let user = mgr.get_user(&user.id).unwrap();
        assert!(user.last_login_at.is_some());
    }

    #[test]
    fn test_delete_tenant_with_children_fails() {
        let mgr = setup();
        let root = mgr.get_tenant_by_code("root").unwrap();

        // 根租户有子租户，不能删除
        let result = mgr.delete_tenant(&root.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_active_tenants() {
        let mgr = setup();
        let active = mgr.list_active_tenants();
        assert_eq!(active.len(), 2);

        let child = mgr.get_tenant_by_code("child").unwrap();
        mgr.suspend_tenant(&child.id).unwrap();

        let active = mgr.list_active_tenants();
        assert_eq!(active.len(), 1);
    }
}
