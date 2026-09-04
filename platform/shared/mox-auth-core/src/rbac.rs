// =============================================================================
// RBAC 权限模块
// =============================================================================

use crate::{AuthError, AuthResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// =============================================================================
// 资源与操作
// =============================================================================

/// 资源类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    /// 资源域（如 ai, cloud, kg, admin）
    pub domain: String,
    /// 资源类型（如 task, document, user）
    pub resource_type: String,
    /// 资源ID（可选，None表示所有该类型资源）
    pub resource_id: Option<String>,
}

impl Resource {
    pub fn new(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            resource_type: resource_type.into(),
            resource_id: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.resource_id = Some(id.into());
        self
    }

    /// 资源标识符（domain:type:id 或 domain:type:*）
    pub fn identifier(&self) -> String {
        match &self.resource_id {
            Some(id) => format!("{}:{}:{}", self.domain, self.resource_type, id),
            None => format!("{}:{}:*", self.domain, self.resource_type),
        }
    }

    /// 是否匹配另一个资源（通配符匹配）
    pub fn matches(&self, other: &Resource) -> bool {
        if self.domain != "*" && self.domain != other.domain {
            return false;
        }
        if self.resource_type != "*" && self.resource_type != other.resource_type {
            return false;
        }
        if let Some(id) = &self.resource_id {
            if id != "*" && Some(id.as_str()) != other.resource_id.as_deref() {
                return false;
            }
        }
        true
    }
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier())
    }
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// 创建
    Create,
    /// 读取
    Read,
    /// 更新
    Update,
    /// 删除
    Delete,
    /// 执行（如运行任务、触发操作）
    Execute,
    /// 管理（所有操作）
    Admin,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Read => "read",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::Execute => "execute",
            Action::Admin => "admin",
        }
    }

    pub fn all() -> [Action; 6] {
        [
            Action::Create,
            Action::Read,
            Action::Update,
            Action::Delete,
            Action::Execute,
            Action::Admin,
        ]
    }

    /// 是否包含另一个操作（Admin 包含所有）
    pub fn contains(&self, other: Action) -> bool {
        matches!(self, Action::Admin) || self == &other
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// 权限
// =============================================================================

/// 权限定义
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// 资源
    pub resource: Resource,
    /// 允许的操作
    pub actions: BTreeSet<Action>,
}

impl Permission {
    pub fn new(resource: Resource, actions: impl IntoIterator<Item = Action>) -> Self {
        Self {
            resource,
            actions: actions.into_iter().collect(),
        }
    }

    /// 权限标识符
    pub fn identifier(&self) -> String {
        let actions: Vec<&str> = self.actions.iter().map(|a| a.as_str()).collect();
        format!("{}:{}", self.resource.identifier(), actions.join(","))
    }

    /// 是否允许对资源执行操作
    pub fn allows(&self, resource: &Resource, action: Action) -> bool {
        if !self.resource.matches(resource) {
            return false;
        }
        self.actions.iter().any(|a| a.contains(action))
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier())
    }
}

// =============================================================================
// 策略效果
// =============================================================================

/// 策略效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    /// 允许
    Allow,
    /// 拒绝（优先于允许）
    Deny,
}

// =============================================================================
// 策略
// =============================================================================

/// 访问控制策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 效果
    pub effect: PolicyEffect,
    /// 权限列表
    pub permissions: Vec<Permission>,
    /// 策略描述
    pub description: Option<String>,
}

impl Policy {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        effect: PolicyEffect,
        permissions: Vec<Permission>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            effect,
            permissions,
            description: None,
        }
    }

    /// 是否允许访问
    pub fn allows(&self, resource: &Resource, action: Action) -> Option<bool> {
        let has_match = self
            .permissions
            .iter()
            .any(|p| p.allows(resource, action));

        if has_match {
            Some(matches!(self.effect, PolicyEffect::Allow))
        } else {
            None
        }
    }
}

// =============================================================================
// 角色
// =============================================================================

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// 角色ID
    pub id: String,
    /// 角色名称
    pub name: String,
    /// 角色描述
    pub description: Option<String>,
    /// 关联的策略ID列表
    pub policy_ids: BTreeSet<String>,
    /// 继承的角色ID列表
    pub inherits_from: BTreeSet<String>,
    /// 是否系统内置角色（不可删除）
    pub is_system: bool,
}

impl Role {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            policy_ids: BTreeSet::new(),
            inherits_from: BTreeSet::new(),
            is_system: false,
        }
    }

    pub fn with_policy(mut self, policy_id: impl Into<String>) -> Self {
        self.policy_ids.insert(policy_id.into());
        self
    }

    pub fn with_inherit(mut self, role_id: impl Into<String>) -> Self {
        self.inherits_from.insert(role_id.into());
        self
    }

    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }
}

// =============================================================================
// 访问控制器
// =============================================================================

/// 访问控制器（RBAC 核心）
#[derive(Debug, Clone)]
pub struct AccessControl {
    /// 策略注册表
    policies: Arc<parking_lot::RwLock<BTreeMap<String, Policy>>>,
    /// 角色注册表
    roles: Arc<parking_lot::RwLock<BTreeMap<String, Role>>>,
    /// 用户角色映射（user_id -> role_ids）
    user_roles: Arc<parking_lot::RwLock<BTreeMap<String, BTreeSet<String>>>>,
}

impl AccessControl {
    /// 创建新的访问控制器
    pub fn new() -> Self {
        let ac = Self {
            policies: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
            roles: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
            user_roles: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
        };
        ac.init_default_roles();
        ac
    }

    /// 初始化默认角色和策略
    fn init_default_roles(&self) {
        // 超级管理员策略：所有资源所有操作
        let admin_policy = Policy::new(
            "policy-admin-all",
            "超级管理员全部权限",
            PolicyEffect::Allow,
            vec![Permission::new(
                Resource::new("*", "*"),
                Action::all().to_vec(),
            )],
        );

        // 普通用户策略：AI域读取和执行
        let user_policy = Policy::new(
            "policy-user-basic",
            "普通用户基础权限",
            PolicyEffect::Allow,
            vec![
                Permission::new(
                    Resource::new("ai", "task"),
                    vec![Action::Create, Action::Read, Action::Execute],
                ),
                Permission::new(
                    Resource::new("cloud", "document"),
                    vec![Action::Create, Action::Read],
                ),
            ],
        );

        // 只读用户策略
        let viewer_policy = Policy::new(
            "policy-viewer-readonly",
            "只读用户权限",
            PolicyEffect::Allow,
            vec![Permission::new(
                Resource::new("*", "*"),
                vec![Action::Read],
            )],
        );

        self.add_policy(admin_policy);
        self.add_policy(user_policy);
        self.add_policy(viewer_policy);

        // 默认角色
        let admin_role = Role::new("admin", "超级管理员")
            .with_policy("policy-admin-all")
            .system();

        let user_role = Role::new("user", "普通用户")
            .with_policy("policy-user-basic")
            .system();

        let viewer_role = Role::new("viewer", "只读用户")
            .with_policy("policy-viewer-readonly")
            .system();

        self.add_role(admin_role);
        self.add_role(user_role);
        self.add_role(viewer_role);
    }

    /// 添加策略
    pub fn add_policy(&self, policy: Policy) {
        self.policies.write().insert(policy.id.clone(), policy);
    }

    /// 获取策略
    pub fn get_policy(&self, id: &str) -> Option<Policy> {
        self.policies.read().get(id).cloned()
    }

    /// 删除策略
    pub fn remove_policy(&self, id: &str) -> AuthResult<()> {
        let mut policies = self.policies.write();
        if let Some(p) = policies.get(id) {
            // 检查是否有角色引用
            let roles = self.roles.read();
            for role in roles.values() {
                if role.policy_ids.contains(id) {
                    return Err(AuthError::PermissionDenied {
                        required: "删除策略".to_string(),
                        actual: format!("策略被角色 '{}' 引用", role.name),
                    });
                }
            }
        }
        policies.remove(id);
        Ok(())
    }

    /// 添加角色
    pub fn add_role(&self, role: Role) {
        self.roles.write().insert(role.id.clone(), role);
    }

    /// 获取角色
    pub fn get_role(&self, id: &str) -> Option<Role> {
        self.roles.read().get(id).cloned()
    }

    /// 删除角色
    pub fn remove_role(&self, id: &str) -> AuthResult<()> {
        let roles = self.roles.read();
        if let Some(role) = roles.get(id) {
            if role.is_system {
                return Err(AuthError::PermissionDenied {
                    required: "删除角色".to_string(),
                    actual: "系统内置角色不可删除".to_string(),
                });
            }
        }
        drop(roles);
        self.roles.write().remove(id);
        Ok(())
    }

    /// 为用户分配角色
    pub fn assign_role(&self, user_id: &str, role_id: &str) -> AuthResult<()> {
        // 验证角色存在
        if !self.roles.read().contains_key(role_id) {
            return Err(AuthError::PermissionDenied {
                required: "分配角色".to_string(),
                actual: format!("角色 '{}' 不存在", role_id),
            });
        }

        self.user_roles
            .write()
            .entry(user_id.to_string())
            .or_default()
            .insert(role_id.to_string());
        Ok(())
    }

    /// 移除用户角色
    pub fn remove_role_from_user(&self, user_id: &str, role_id: &str) {
        if let Some(roles) = self.user_roles.write().get_mut(user_id) {
            roles.remove(role_id);
        }
    }

    /// 获取用户的所有角色（含继承）
    pub fn get_user_roles(&self, user_id: &str) -> BTreeSet<String> {
        let user_roles = self.user_roles.read();
        let direct_roles = user_roles.get(user_id).cloned().unwrap_or_default();

        // 解析继承关系
        let mut all_roles = BTreeSet::new();
        let mut queue: Vec<String> = direct_roles.into_iter().collect();

        while let Some(role_id) = queue.pop() {
            if all_roles.contains(&role_id) {
                continue;
            }
            all_roles.insert(role_id.clone());

            if let Some(role) = self.roles.read().get(&role_id) {
                for inherited in &role.inherits_from {
                    if !all_roles.contains(inherited) {
                        queue.push(inherited.clone());
                    }
                }
            }
        }

        all_roles
    }

    /// 获取用户的所有策略（含角色继承）
    fn get_user_policies(&self, user_id: &str) -> Vec<Policy> {
        let roles = self.get_user_roles(user_id);
        let policies_map = self.policies.read();
        let roles_map = self.roles.read();

        let mut policies = Vec::new();
        for role_id in &roles {
            if let Some(role) = roles_map.get(role_id) {
                for policy_id in &role.policy_ids {
                    if let Some(policy) = policies_map.get(policy_id) {
                        policies.push(policy.clone());
                    }
                }
            }
        }
        policies
    }

    /// 检查用户是否有权限访问资源
    pub fn check_access(
        &self,
        user_id: &str,
        resource: &Resource,
        action: Action,
    ) -> AuthResult<()> {
        let policies = self.get_user_policies(user_id);

        // 拒绝优先：先检查所有 Deny 策略
        for policy in &policies {
            if policy.effect == PolicyEffect::Deny {
                if let Some(false) = policy.allows(resource, action) {
                    return Err(AuthError::PermissionDenied {
                        required: format!("{}:{}", resource.identifier(), action),
                        actual: format!("被策略 '{}' 拒绝", policy.name),
                    });
                }
            }
        }

        // 再检查 Allow 策略
        for policy in &policies {
            if policy.effect == PolicyEffect::Allow {
                if let Some(true) = policy.allows(resource, action) {
                    return Ok(());
                }
            }
        }

        Err(AuthError::PermissionDenied {
            required: format!("{}:{}", resource.identifier(), action),
            actual: "没有匹配的允许策略".to_string(),
        })
    }

    /// 检查用户是否有指定角色
    pub fn has_role(&self, user_id: &str, role_id: &str) -> bool {
        self.get_user_roles(user_id).contains(role_id)
    }

    /// 列出所有策略
    pub fn list_policies(&self) -> Vec<Policy> {
        self.policies.read().values().cloned().collect()
    }

    /// 列出所有角色
    pub fn list_roles(&self) -> Vec<Role> {
        self.roles.read().values().cloned().collect()
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_identifier() {
        let r1 = Resource::new("ai", "task");
        assert_eq!(r1.identifier(), "ai:task:*");

        let r2 = Resource::new("ai", "task").with_id("123");
        assert_eq!(r2.identifier(), "ai:task:123");
    }

    #[test]
    fn test_resource_matches() {
        let wildcard = Resource::new("*", "*");
        let specific = Resource::new("ai", "task").with_id("123");

        assert!(wildcard.matches(&specific));
        assert!(!specific.matches(&wildcard));

        let ai_task = Resource::new("ai", "task");
        assert!(ai_task.matches(&specific));
        assert!(!specific.matches(&ai_task));
    }

    #[test]
    fn test_action_contains() {
        assert!(Action::Admin.contains(Action::Read));
        assert!(Action::Admin.contains(Action::Delete));
        assert!(!Action::Read.contains(Action::Create));
        assert!(Action::Read.contains(Action::Read));
    }

    #[test]
    fn test_permission_allows() {
        let perm = Permission::new(
            Resource::new("ai", "task"),
            vec![Action::Read, Action::Create],
        );

        assert!(perm.allows(&Resource::new("ai", "task"), Action::Read));
        assert!(perm.allows(&Resource::new("ai", "task").with_id("123"), Action::Create));
        assert!(!perm.allows(&Resource::new("ai", "task"), Action::Delete));
        assert!(!perm.allows(&Resource::new("cloud", "document"), Action::Read));
    }

    #[test]
    fn test_default_roles() {
        let ac = AccessControl::new();

        assert!(ac.get_role("admin").is_some());
        assert!(ac.get_role("user").is_some());
        assert!(ac.get_role("viewer").is_some());

        assert_eq!(ac.list_roles().len(), 3);
        assert_eq!(ac.list_policies().len(), 3);
    }

    #[test]
    fn test_admin_full_access() {
        let ac = AccessControl::new();
        ac.assign_role("user-admin", "admin").unwrap();

        // 管理员可以访问任何资源任何操作
        ac.check_access("user-admin", &Resource::new("ai", "task"), Action::Delete)
            .unwrap();
        ac.check_access("user-admin", &Resource::new("cloud", "document"), Action::Admin)
            .unwrap();
        ac.check_access("user-admin", &Resource::new("*", "*"), Action::Delete)
            .unwrap();
    }

    #[test]
    fn test_user_limited_access() {
        let ac = AccessControl::new();
        ac.assign_role("user-001", "user").unwrap();

        // 普通用户可以创建和读取AI任务
        ac.check_access("user-001", &Resource::new("ai", "task"), Action::Create)
            .unwrap();
        ac.check_access("user-001", &Resource::new("ai", "task"), Action::Read)
            .unwrap();

        // 普通用户不能删除AI任务
        assert!(ac
            .check_access("user-001", &Resource::new("ai", "task"), Action::Delete)
            .is_err());

        // 普通用户不能访问管理域
        assert!(ac
            .check_access("user-001", &Resource::new("admin", "user"), Action::Read)
            .is_err());
    }

    #[test]
    fn test_viewer_readonly() {
        let ac = AccessControl::new();
        ac.assign_role("viewer-001", "viewer").unwrap();

        // 只读用户可以读取任何资源
        ac.check_access("viewer-001", &Resource::new("ai", "task"), Action::Read)
            .unwrap();
        ac.check_access("viewer-001", &Resource::new("admin", "user"), Action::Read)
            .unwrap();

        // 只读用户不能创建/更新/删除
        assert!(ac
            .check_access("viewer-001", &Resource::new("ai", "task"), Action::Create)
            .is_err());
        assert!(ac
            .check_access("viewer-001", &Resource::new("ai", "task"), Action::Delete)
            .is_err());
    }

    #[test]
    fn test_no_role_denied() {
        let ac = AccessControl::new();

        // 没有角色的用户被拒绝
        assert!(ac
            .check_access("nobody", &Resource::new("ai", "task"), Action::Read)
            .is_err());
    }

    #[test]
    fn test_role_inheritance() {
        let ac = AccessControl::new();

        // 创建高级用户角色，继承普通用户
        let power_user_policy = Policy::new(
            "policy-power-user",
            "高级用户权限",
            PolicyEffect::Allow,
            vec![Permission::new(
                Resource::new("ai", "task"),
                vec![Action::Update, Action::Delete],
            )],
        );
        ac.add_policy(power_user_policy);

        let power_user_role = Role::new("power_user", "高级用户")
            .with_policy("policy-power-user")
            .with_inherit("user");
        ac.add_role(power_user_role);

        ac.assign_role("power-001", "power_user").unwrap();

        // 高级用户有继承的普通用户权限
        ac.check_access("power-001", &Resource::new("ai", "task"), Action::Create)
            .unwrap();
        ac.check_access("power-001", &Resource::new("ai", "task"), Action::Read)
            .unwrap();

        // 高级用户有自己的额外权限
        ac.check_access("power-001", &Resource::new("ai", "task"), Action::Update)
            .unwrap();
        ac.check_access("power-001", &Resource::new("ai", "task"), Action::Delete)
            .unwrap();

        // 验证角色继承解析
        let roles = ac.get_user_roles("power-001");
        assert!(roles.contains("power_user"));
        assert!(roles.contains("user"));
    }

    #[test]
    fn test_deny_policy_overrides_allow() {
        let ac = AccessControl::new();

        // 创建拒绝策略：禁止删除AI任务
        let deny_policy = Policy::new(
            "policy-deny-delete",
            "禁止删除AI任务",
            PolicyEffect::Deny,
            vec![Permission::new(
                Resource::new("ai", "task"),
                vec![Action::Delete],
            )],
        );
        ac.add_policy(deny_policy);

        // 创建角色：管理员 + 拒绝策略
        let restricted_admin = Role::new("restricted_admin", "受限管理员")
            .with_policy("policy-admin-all")
            .with_policy("policy-deny-delete");
        ac.add_role(restricted_admin);

        ac.assign_role("r-admin", "restricted_admin").unwrap();

        // 管理员可以读取
        ac.check_access("r-admin", &Resource::new("ai", "task"), Action::Read)
            .unwrap();

        // 但拒绝策略优先，不能删除
        assert!(ac
            .check_access("r-admin", &Resource::new("ai", "task"), Action::Delete)
            .is_err());
    }

    #[test]
    fn test_system_role_cannot_delete() {
        let ac = AccessControl::new();
        assert!(ac.remove_role("admin").is_err());
        assert!(ac.remove_role("user").is_err());
    }

    #[test]
    fn test_assign_nonexistent_role_fails() {
        let ac = AccessControl::new();
        assert!(ac.assign_role("user-001", "nonexistent").is_err());
    }

    #[test]
    fn test_has_role() {
        let ac = AccessControl::new();
        ac.assign_role("user-001", "user").unwrap();

        assert!(ac.has_role("user-001", "user"));
        assert!(!ac.has_role("user-001", "admin"));
        assert!(!ac.has_role("nobody", "user"));
    }
}
