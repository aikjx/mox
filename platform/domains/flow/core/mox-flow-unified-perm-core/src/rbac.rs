// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! RBAC（基于角色的访问控制）管理器
//!
//! 支持：
//! - 用户-角色-权限三层模型
//! - 角色继承
//! - 多租户隔离
//! - 临时授权
//! - 作用域绑定

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

use crate::error::{PermError, PermResult};
use crate::types::{
    Action, Permission, PermissionEffect, ResourceScope, Role, RoleBinding, Subject,
};

/// RBAC 管理器
pub struct RbacManager {
    /// 角色表
    roles: RwLock<HashMap<String, Role>>,
    /// 角色编码 -> ID 映射
    role_codes: RwLock<HashMap<String, String>>,
    /// 权限表
    permissions: RwLock<HashMap<String, Permission>>,
    /// 角色绑定表
    bindings: RwLock<HashMap<String, RoleBinding>>,
    /// 主体绑定索引：subject_id -> Vec<binding_id>
    subject_bindings: RwLock<HashMap<String, Vec<String>>>,
}

impl RbacManager {
    /// 创建 RBAC 管理器
    pub fn new() -> Self {
        Self {
            roles: RwLock::new(HashMap::new()),
            role_codes: RwLock::new(HashMap::new()),
            permissions: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            subject_bindings: RwLock::new(HashMap::new()),
        }
    }

    // ---------- 权限管理 ----------

    /// 创建权限
    pub fn create_permission(&self, perm: Permission) -> PermResult<Permission> {
        let tenant_key = format!("{}:{}", perm.tenant_id, perm.name);
        // 简单的租户内名称唯一性检查
        for p in self.permissions.read().values() {
            if p.tenant_id == perm.tenant_id && p.name == perm.name {
                return Err(PermError::AlreadyExists(format!(
                    "permission '{}' already exists in tenant '{}'",
                    perm.name, perm.tenant_id
                )));
            }
        }
        let _ = tenant_key;
        self.permissions
            .write()
            .insert(perm.id.clone(), perm.clone());
        Ok(perm)
    }

    /// 获取权限
    pub fn get_permission(&self, perm_id: &str) -> PermResult<Permission> {
        self.permissions
            .read()
            .get(perm_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("permission '{}' not found", perm_id)))
    }

    /// 列出租户所有权限
    pub fn list_permissions(&self, tenant_id: &str) -> Vec<Permission> {
        self.permissions
            .read()
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// 删除权限
    pub fn delete_permission(&self, perm_id: &str) -> PermResult<bool> {
        // 检查是否被角色引用
        for role in self.roles.read().values() {
            if role.permission_ids.contains(perm_id) {
                return Err(PermError::InvalidArgument(format!(
                    "permission '{}' is used by role '{}'",
                    perm_id, role.name
                )));
            }
        }
        Ok(self.permissions.write().remove(perm_id).is_some())
    }

    // ---------- 角色管理 ----------

    /// 创建角色
    pub fn create_role(&self, role: Role) -> PermResult<Role> {
        let code_key = format!("{}:{}", role.tenant_id, role.code);
        if self.role_codes.read().contains_key(&code_key) {
            return Err(PermError::AlreadyExists(format!(
                "role code '{}' already exists in tenant '{}'",
                role.code, role.tenant_id
            )));
        }

        self.role_codes
            .write()
            .insert(code_key, role.id.clone());
        self.roles.write().insert(role.id.clone(), role.clone());
        Ok(role)
    }

    /// 获取角色
    pub fn get_role(&self, role_id: &str) -> PermResult<Role> {
        self.roles
            .read()
            .get(role_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("role '{}' not found", role_id)))
    }

    /// 按编码获取角色
    pub fn get_role_by_code(&self, tenant_id: &str, code: &str) -> PermResult<Role> {
        let code_key = format!("{}:{}", tenant_id, code);
        let role_id = self
            .role_codes
            .read()
            .get(&code_key)
            .cloned()
            .ok_or_else(|| {
                PermError::NotFound(format!("role '{}' not found in tenant '{}'", code, tenant_id))
            })?;
        self.get_role(&role_id)
    }

    /// 列出租户角色
    pub fn list_roles(&self, tenant_id: &str) -> Vec<Role> {
        self.roles
            .read()
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// 给角色添加权限
    pub fn add_permission_to_role(&self, role_id: &str, perm_id: &str) -> PermResult<()> {
        // 检查权限存在且同租户
        let perm = self.get_permission(perm_id)?;
        let mut roles = self.roles.write();
        let role = roles
            .get_mut(role_id)
            .ok_or_else(|| PermError::NotFound(format!("role '{}' not found", role_id)))?;

        if perm.tenant_id != role.tenant_id {
            return Err(PermError::TenantMismatch(
                "permission and role must be in same tenant".to_string(),
            ));
        }

        role.add_permission(perm_id);
        Ok(())
    }

    /// 从角色移除权限
    pub fn remove_permission_from_role(&self, role_id: &str, perm_id: &str) -> PermResult<bool> {
        let mut roles = self.roles.write();
        let role = roles
            .get_mut(role_id)
            .ok_or_else(|| PermError::NotFound(format!("role '{}' not found", role_id)))?;
        Ok(role.remove_permission(perm_id))
    }

    /// 添加继承角色
    pub fn add_inherited_role(&self, role_id: &str, inherited_role_id: &str) -> PermResult<()> {
        // 避免循环继承
        if self.would_cause_cycle(role_id, inherited_role_id) {
            return Err(PermError::InvalidArgument(
                "would cause circular role inheritance".to_string(),
            ));
        }

        let mut roles = self.roles.write();
        let role = roles
            .get_mut(role_id)
            .ok_or_else(|| PermError::NotFound(format!("role '{}' not found", role_id)))?;
        role.add_inherited_role(inherited_role_id);
        Ok(())
    }

    /// 检查是否会造成循环继承
    fn would_cause_cycle(&self, role_id: &str, inherited_role_id: &str) -> bool {
        if role_id == inherited_role_id {
            return true;
        }

        // DFS 从 inherited_role 出发，看能否回到 role_id
        let mut visited = HashSet::new();
        let mut stack = vec![inherited_role_id.to_string()];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if current == role_id {
                return true;
            }
            if let Some(role) = self.roles.read().get(&current) {
                for inherited in &role.inherited_role_ids {
                    stack.push(inherited.clone());
                }
            }
        }

        false
    }

    /// 删除角色
    pub fn delete_role(&self, role_id: &str) -> PermResult<bool> {
        let role = self.get_role(role_id)?;

        // 检查是否被其他角色继承
        for r in self.roles.read().values() {
            if r.inherited_role_ids.contains(role_id) {
                return Err(PermError::InvalidArgument(format!(
                    "role '{}' is inherited by role '{}'",
                    role_id, r.name
                )));
            }
        }

        // 移除所有绑定
        let mut bindings = self.bindings.write();
        let mut subject_bindings = self.subject_bindings.write();
        bindings.retain(|_, b| {
            if b.role_id == role_id {
                if let Some(vec) = subject_bindings.get_mut(&b.subject_id) {
                    vec.retain(|bid| bid != &b.id);
                }
                false
            } else {
                true
            }
        });

        // 移除编码索引
        let code_key = format!("{}:{}", role.tenant_id, role.code);
        self.role_codes.write().remove(&code_key);

        Ok(self.roles.write().remove(role_id).is_some())
    }

    // ---------- 角色绑定 ----------

    /// 创建角色绑定
    pub fn create_binding(&self, binding: RoleBinding) -> PermResult<RoleBinding> {
        // 验证角色存在
        let role = self.get_role(&binding.role_id)?;

        if role.tenant_id != binding.tenant_id {
            return Err(PermError::TenantMismatch(
                "binding and role must be in same tenant".to_string(),
            ));
        }

        let key = binding_key(&binding.subject_id, &binding.role_id, binding.scope.as_deref());

        // 检查重复绑定
        for b in self.bindings.read().values() {
            if binding_key(&b.subject_id, &b.role_id, b.scope.as_deref()) == key
                && b.tenant_id == binding.tenant_id
            {
                return Err(PermError::AlreadyExists(
                    "role binding already exists".to_string(),
                ));
            }
        }

        self.subject_bindings
            .write()
            .entry(binding.subject_id.clone())
            .or_default()
            .push(binding.id.clone());
        self.bindings
            .write()
            .insert(binding.id.clone(), binding.clone());
        Ok(binding)
    }

    /// 删除绑定
    pub fn delete_binding(&self, binding_id: &str) -> PermResult<bool> {
        let binding = self
            .bindings
            .read()
            .get(binding_id)
            .cloned()
            .ok_or_else(|| {
                PermError::NotFound(format!("binding '{}' not found", binding_id))
            })?;

        // 移除主体索引
        if let Some(vec) = self.subject_bindings.write().get_mut(&binding.subject_id) {
            vec.retain(|id| id != binding_id);
        }

        Ok(self.bindings.write().remove(binding_id).is_some())
    }

    /// 获取主体的有效角色（含继承）
    pub fn get_subject_roles(&self, subject: &Subject) -> PermResult<Vec<Role>> {
        let mut role_ids = HashSet::new();
        let bindings = self.bindings.read();

        // 收集直接绑定的角色
        for binding in bindings.values() {
            if binding.subject_id == subject.id
                && binding.tenant_id == subject.tenant_id
                && !binding.is_expired()
            {
                role_ids.insert(binding.role_id.clone());
            }
        }

        // 解析继承（BFS）
        let roles = self.roles.read();
        let mut result = Vec::new();
        let mut queue: Vec<String> = role_ids.into_iter().collect();
        let mut visited = HashSet::new();

        while let Some(role_id) = queue.pop() {
            if !visited.insert(role_id.clone()) {
                continue;
            }
            if let Some(role) = roles.get(&role_id) {
                if role.tenant_id == subject.tenant_id {
                    result.push(role.clone());
                    for inherited in &role.inherited_role_ids {
                        queue.push(inherited.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// 获取主体的所有权限（通过角色+继承）
    pub fn get_subject_permissions(&self, subject: &Subject) -> PermResult<Vec<Permission>> {
        let roles = self.get_subject_roles(subject)?;
        let perms = self.permissions.read();
        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for role in &roles {
            for perm_id in &role.permission_ids {
                if seen.insert(perm_id.clone()) {
                    if let Some(perm) = perms.get(perm_id) {
                        result.push(perm.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// 检查主体是否有指定权限（RBAC 层，不考虑 ABAC）
    pub fn check_permission(
        &self,
        subject: &Subject,
        action: &Action,
        resource: &ResourceScope,
    ) -> PermResult<bool> {
        let permissions = self.get_subject_permissions(subject)?;

        // 先检查是否有明确拒绝
        for perm in &permissions {
            if perm.effect == PermissionEffect::Deny && perm.matches(action, resource) {
                return Ok(false);
            }
        }

        // 再检查是否有允许
        for perm in &permissions {
            if perm.effect == PermissionEffect::Allow && perm.matches(action, resource) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 获取主体的角色绑定列表
    pub fn get_subject_bindings(&self, subject_id: &str, tenant_id: &str) -> Vec<RoleBinding> {
        self.bindings
            .read()
            .values()
            .filter(|b| b.subject_id == subject_id && b.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// 角色总数
    pub fn role_count(&self) -> usize {
        self.roles.read().len()
    }

    /// 权限总数
    pub fn permission_count(&self) -> usize {
        self.permissions.read().len()
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成绑定键
fn binding_key(subject_id: &str, role_id: &str, scope: Option<&str>) -> String {
    format!(
        "{}:{}:{}",
        subject_id,
        role_id,
        scope.unwrap_or("*")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SubjectType, now_ms};

    fn setup() -> RbacManager {
        let mgr = RbacManager::new();
        let tenant = "t-1";

        // 创建权限
        let read_nodes = Permission::allow(
            "read_nodes",
            Action::new("read:*"),
            ResourceScope::all("graph.node"),
            tenant,
        );
        let write_nodes = Permission::allow(
            "write_nodes",
            Action::new("write:*"),
            ResourceScope::all("graph.node"),
            tenant,
        );
        let delete_nodes = Permission::deny(
            "delete_nodes_deny",
            Action::new("delete:*"),
            ResourceScope::all("graph.node"),
            tenant,
        );

        mgr.create_permission(read_nodes.clone()).unwrap();
        mgr.create_permission(write_nodes.clone()).unwrap();
        mgr.create_permission(delete_nodes.clone()).unwrap();

        // 创建角色
        let viewer = Role::new("Viewer", "viewer", tenant);
        let editor = Role::new("Editor", "editor", tenant);
        mgr.create_role(viewer.clone()).unwrap();
        mgr.create_role(editor.clone()).unwrap();

        // 分配权限
        mgr.add_permission_to_role(&viewer.id, &read_nodes.id).unwrap();
        mgr.add_permission_to_role(&editor.id, &read_nodes.id).unwrap();
        mgr.add_permission_to_role(&editor.id, &write_nodes.id).unwrap();
        mgr.add_permission_to_role(&editor.id, &delete_nodes.id).unwrap();

        mgr
    }

    #[test]
    fn test_create_role_and_permissions() {
        let mgr = setup();
        assert_eq!(mgr.role_count(), 2);
        assert_eq!(mgr.permission_count(), 3);
    }

    #[test]
    fn test_role_binding_and_check() {
        let mgr = setup();
        let tenant = "t-1";
        let viewer = mgr.get_role_by_code(tenant, "viewer").unwrap();

        let subject = Subject::user("alice", tenant);
        let binding =
            RoleBinding::new(&subject.id, SubjectType::User, &viewer.id, tenant);
        mgr.create_binding(binding).unwrap();

        // Viewer 有读权限
        assert!(mgr
            .check_permission(
                &subject,
                &Action::new("read:node"),
                &ResourceScope::of("graph.node", "n1")
            )
            .unwrap());

        // Viewer 没有写权限
        assert!(!mgr
            .check_permission(
                &subject,
                &Action::new("write:node"),
                &ResourceScope::of("graph.node", "n1")
            )
            .unwrap());
    }

    #[test]
    fn test_role_inheritance() {
        let mgr = setup();
        let tenant = "t-1";
        let viewer = mgr.get_role_by_code(tenant, "viewer").unwrap();
        let editor = mgr.get_role_by_code(tenant, "editor").unwrap();

        // editor 继承 viewer（虽然这里已经包含了权限，但测试继承机制）
        mgr.add_inherited_role(&editor.id, &viewer.id).unwrap();

        let subject = Subject::user("bob", tenant);
        let binding =
            RoleBinding::new(&subject.id, SubjectType::User, &editor.id, tenant);
        mgr.create_binding(binding).unwrap();

        let roles = mgr.get_subject_roles(&subject).unwrap();
        assert!(roles.len() >= 2); // editor + viewer
    }

    #[test]
    fn test_circular_inheritance_prevention() {
        let mgr = setup();
        let tenant = "t-1";
        let viewer = mgr.get_role_by_code(tenant, "viewer").unwrap();
        let editor = mgr.get_role_by_code(tenant, "editor").unwrap();

        mgr.add_inherited_role(&editor.id, &viewer.id).unwrap();

        // 尝试让 viewer 继承 editor，应该失败
        let result = mgr.add_inherited_role(&viewer.id, &editor.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_deny_overrides_allow() {
        let mgr = setup();
        let tenant = "t-1";
        let editor = mgr.get_role_by_code(tenant, "editor").unwrap();

        let subject = Subject::user("charlie", tenant);
        let binding =
            RoleBinding::new(&subject.id, SubjectType::User, &editor.id, tenant);
        mgr.create_binding(binding).unwrap();

        // Editor 有 delete 拒绝权限，所以 delete 应该被拒绝
        assert!(!mgr
            .check_permission(
                &subject,
                &Action::new("delete:node"),
                &ResourceScope::of("graph.node", "n1")
            )
            .unwrap());
    }

    #[test]
    fn test_delete_role() {
        let mgr = setup();
        let tenant = "t-1";
        let viewer = mgr.get_role_by_code(tenant, "viewer").unwrap();

        // 添加一个绑定
        let subject = Subject::user("dave", tenant);
        let binding =
            RoleBinding::new(&subject.id, SubjectType::User, &viewer.id, tenant);
        mgr.create_binding(binding).unwrap();

        assert!(mgr.delete_role(&viewer.id).unwrap());
        assert_eq!(mgr.role_count(), 1);
    }

    #[test]
    fn test_duplicate_role_code() {
        let mgr = setup();
        let role = Role::new("Another Viewer", "viewer", "t-1");
        let result = mgr.create_role(role);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_binding() {
        let mgr = setup();
        let tenant = "t-1";
        let viewer = mgr.get_role_by_code(tenant, "viewer").unwrap();

        let subject = Subject::user("eve", tenant);
        let mut binding =
            RoleBinding::new(&subject.id, SubjectType::User, &viewer.id, tenant);
        binding.expires_at = Some(now_ms() - 1000); // 已过期
        mgr.create_binding(binding).unwrap();

        // 过期绑定不应生效
        let perms = mgr.get_subject_permissions(&subject).unwrap();
        assert_eq!(perms.len(), 0);
    }

    #[test]
    fn test_cross_tenant_isolation() {
        let mgr = setup(); // t-1

        // 创建 t-2 的用户
        let subject = Subject::user("alice", "t-2");

        // 不应该看到 t-1 的任何权限
        let perms = mgr.get_subject_permissions(&subject).unwrap();
        assert_eq!(perms.len(), 0);
    }
}
