// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 权限核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// 主体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    /// 用户
    User,
    /// 角色
    Role,
    /// 组
    Group,
    /// 服务账号
    ServiceAccount,
    /// API Key
    ApiKey,
}

/// 主体（权限授予的对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    /// 主体 ID
    pub id: String,
    /// 主体类型
    pub subject_type: SubjectType,
    /// 所属租户 ID
    pub tenant_id: String,
    /// 主体属性（用于 ABAC）
    pub attributes: HashMap<String, String>,
}

impl Subject {
    /// 创建用户主体
    pub fn user(user_id: &str, tenant_id: &str) -> Self {
        Self {
            id: user_id.to_string(),
            subject_type: SubjectType::User,
            tenant_id: tenant_id.to_string(),
            attributes: HashMap::new(),
        }
    }

    /// 创建服务账号主体
    pub fn service_account(account_id: &str, tenant_id: &str) -> Self {
        Self {
            id: account_id.to_string(),
            subject_type: SubjectType::ServiceAccount,
            tenant_id: tenant_id.to_string(),
            attributes: HashMap::new(),
        }
    }

    /// 设置属性
    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// 获取属性
    pub fn get_attr(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }
}

/// 操作类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Action(pub String);

impl Action {
    /// 创建操作
    pub fn new(action: &str) -> Self {
        Self(action.to_string())
    }

    /// 通配操作
    pub fn all() -> Self {
        Self("*".to_string())
    }

    /// 是否通配
    pub fn is_all(&self) -> bool {
        self.0 == "*"
    }

    /// 匹配操作（支持通配）
    pub fn matches(&self, required: &Action) -> bool {
        if self.is_all() {
            return true;
        }
        if required.is_all() {
            return true;
        }
        // 支持前缀通配：read:* 匹配 read:user
        let self_parts: Vec<&str> = self.0.split(':').collect();
        let req_parts: Vec<&str> = required.0.split(':').collect();

        for (i, &req_part) in req_parts.iter().enumerate() {
            if i >= self_parts.len() {
                return false;
            }
            if self_parts[i] == "*" {
                return true;
            }
            if self_parts[i] != req_part {
                return false;
            }
        }

        self_parts.len() >= req_parts.len()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        Action(s.to_string())
    }
}

/// 资源范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceScope {
    /// 资源类型
    pub resource_type: String,
    /// 资源 ID（None 表示所有该类型资源）
    pub resource_id: Option<String>,
    /// 资源路径（层级资源用）
    pub resource_path: Option<String>,
    /// 资源属性（用于 ABAC）
    pub attributes: HashMap<String, String>,
}

impl ResourceScope {
    /// 所有资源
    pub fn all(resource_type: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            resource_id: None,
            resource_path: None,
            attributes: HashMap::new(),
        }
    }

    /// 特定资源
    pub fn of(resource_type: &str, resource_id: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            resource_id: Some(resource_id.to_string()),
            resource_path: None,
            attributes: HashMap::new(),
        }
    }

    /// 匹配资源
    pub fn matches(&self, required: &ResourceScope) -> bool {
        if self.resource_type != required.resource_type {
            return false;
        }
        // 通配：resource_id 为 None 表示所有
        if self.resource_id.is_none() {
            return true;
        }
        match (&self.resource_id, &required.resource_id) {
            (Some(id), Some(rid)) => id == rid,
            (Some(_), None) => false, // 特定 ID 不能匹配"所有"请求
            (None, _) => true,
        }
    }
}

/// 权限效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionEffect {
    /// 允许
    Allow,
    /// 拒绝（优先于 Allow）
    Deny,
}

/// 权限定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// 权限 ID
    pub id: String,
    /// 权限名称
    pub name: String,
    /// 操作
    pub action: Action,
    /// 资源范围
    pub resource: ResourceScope,
    /// 效果
    pub effect: PermissionEffect,
    /// 所属租户
    pub tenant_id: String,
    /// 描述
    pub description: Option<String>,
    /// 条件表达式（ABAC 用）
    pub condition: Option<String>,
    /// 数据范围限制
    pub data_scope: Option<String>,
}

impl Permission {
    /// 创建允许权限
    pub fn allow(
        name: &str,
        action: Action,
        resource: ResourceScope,
        tenant_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            action,
            resource,
            effect: PermissionEffect::Allow,
            tenant_id: tenant_id.to_string(),
            description: None,
            condition: None,
            data_scope: None,
        }
    }

    /// 创建拒绝权限
    pub fn deny(
        name: &str,
        action: Action,
        resource: ResourceScope,
        tenant_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            action,
            resource,
            effect: PermissionEffect::Deny,
            tenant_id: tenant_id.to_string(),
            description: None,
            condition: None,
            data_scope: None,
        }
    }

    /// 检查是否匹配
    pub fn matches(&self, action: &Action, resource: &ResourceScope) -> bool {
        self.action.matches(action) && self.resource.matches(resource)
    }
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// 角色 ID
    pub id: String,
    /// 角色名称
    pub name: String,
    /// 角色编码（唯一）
    pub code: String,
    /// 所属租户
    pub tenant_id: String,
    /// 描述
    pub description: Option<String>,
    /// 权限 ID 列表
    pub permission_ids: HashSet<String>,
    /// 继承的角色 ID 列表
    pub inherited_role_ids: HashSet<String>,
    /// 是否系统内置角色
    pub is_system: bool,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

impl Role {
    /// 创建新角色
    pub fn new(name: &str, code: &str, tenant_id: &str) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            code: code.to_string(),
            tenant_id: tenant_id.to_string(),
            description: None,
            permission_ids: HashSet::new(),
            inherited_role_ids: HashSet::new(),
            is_system: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建系统角色
    pub fn system(name: &str, code: &str, tenant_id: &str) -> Self {
        let mut role = Self::new(name, code, tenant_id);
        role.is_system = true;
        role
    }

    /// 添加权限
    pub fn add_permission(&mut self, perm_id: &str) {
        self.permission_ids.insert(perm_id.to_string());
        self.updated_at = now_ms();
    }

    /// 移除权限
    pub fn remove_permission(&mut self, perm_id: &str) -> bool {
        let removed = self.permission_ids.remove(perm_id);
        if removed {
            self.updated_at = now_ms();
        }
        removed
    }

    /// 添加继承角色
    pub fn add_inherited_role(&mut self, role_id: &str) {
        self.inherited_role_ids.insert(role_id.to_string());
        self.updated_at = now_ms();
    }
}

/// 角色绑定（用户-角色关联）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    /// 绑定 ID
    pub id: String,
    /// 主体 ID
    pub subject_id: String,
    /// 主体类型
    pub subject_type: SubjectType,
    /// 角色 ID
    pub role_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 绑定范围（如特定项目、部门）
    pub scope: Option<String>,
    /// 过期时间（临时授权用）
    pub expires_at: Option<u64>,
    /// 创建时间
    pub created_at: u64,
}

impl RoleBinding {
    /// 创建角色绑定
    pub fn new(subject_id: &str, subject_type: SubjectType, role_id: &str, tenant_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            subject_id: subject_id.to_string(),
            subject_type,
            role_id: role_id.to_string(),
            tenant_id: tenant_id.to_string(),
            scope: None,
            expires_at: None,
            created_at: now_ms(),
        }
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            now_ms() > exp
        } else {
            false
        }
    }
}

/// 用户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// 活跃
    Active,
    /// 禁用
    Disabled,
    /// 待激活
    Pending,
    /// 锁定
    Locked,
}

/// 用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户 ID
    pub id: String,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: Option<String>,
    /// 手机号
    pub phone: Option<String>,
    /// 显示名
    pub display_name: String,
    /// 所属租户 ID
    pub tenant_id: String,
    /// 状态
    pub status: UserStatus,
    /// 密码哈希（内部使用，不对外暴露）
    pub password_hash: Option<String>,
    /// 用户属性
    pub attributes: HashMap<String, String>,
    /// 标签
    pub tags: HashSet<String>,
    /// 最后登录时间
    pub last_login_at: Option<u64>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

impl User {
    /// 创建新用户
    pub fn new(username: &str, display_name: &str, tenant_id: &str) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email: None,
            phone: None,
            display_name: display_name.to_string(),
            tenant_id: tenant_id.to_string(),
            status: UserStatus::Active,
            password_hash: None,
            attributes: HashMap::new(),
            tags: HashSet::new(),
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 转换为 Subject
    pub fn to_subject(&self) -> Subject {
        Subject {
            id: self.id.clone(),
            subject_type: SubjectType::User,
            tenant_id: self.tenant_id.clone(),
            attributes: self.attributes.clone(),
        }
    }

    /// 检查是否活跃
    pub fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }
}

/// 租户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    /// 活跃
    Active,
    /// 已暂停
    Suspended,
    /// 已过期
    Expired,
    /// 已删除
    Deleted,
}

/// 租户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// 租户 ID
    pub id: String,
    /// 租户名称
    pub name: String,
    /// 租户编码（唯一标识）
    pub code: String,
    /// 状态
    pub status: TenantStatus,
    /// 父租户 ID（用于层级租户）
    pub parent_id: Option<String>,
    /// 租户管理员用户 ID
    pub admin_user_id: Option<String>,
    /// 租户属性
    pub attributes: HashMap<String, String>,
    /// 配额限制
    pub quota: TenantQuota,
    /// 到期时间
    pub expires_at: Option<u64>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 租户配额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuota {
    /// 最大用户数
    pub max_users: u64,
    /// 最大角色数
    pub max_roles: u64,
    /// 最大存储空间（字节）
    pub max_storage_bytes: u64,
    /// 最大 API 调用/天
    pub max_api_calls_per_day: u64,
}

impl Default for TenantQuota {
    fn default() -> Self {
        Self {
            max_users: 100,
            max_roles: 50,
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_api_calls_per_day: 100_000,
        }
    }
}

impl Tenant {
    /// 创建新租户
    pub fn new(name: &str, code: &str) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            code: code.to_string(),
            status: TenantStatus::Active,
            parent_id: None,
            admin_user_id: None,
            attributes: HashMap::new(),
            quota: TenantQuota::default(),
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 检查是否活跃
    pub fn is_active(&self) -> bool {
        if !matches!(self.status, TenantStatus::Active) {
            return false;
        }
        if let Some(exp) = self.expires_at {
            if now_ms() > exp {
                return false;
            }
        }
        true
    }

    /// 检查是否有层级关系
    pub fn is_child_of(&self, parent_id: &str) -> bool {
        self.parent_id.as_deref() == Some(parent_id)
    }
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_matches() {
        let read_all = Action::new("read:*");
        let read_user = Action::new("read:user");
        let write_user = Action::new("write:user");
        let all = Action::all();

        assert!(read_all.matches(&read_user));
        assert!(!read_all.matches(&write_user));
        assert!(all.matches(&read_user));
        assert!(all.matches(&write_user));
        assert!(read_user.matches(&read_user));
    }

    #[test]
    fn test_resource_scope_matches() {
        let all_nodes = ResourceScope::all("graph.node");
        let specific_node = ResourceScope::of("graph.node", "node-123");

        assert!(all_nodes.matches(&specific_node));
        assert!(!specific_node.matches(&all_nodes));
        assert!(specific_node.matches(&specific_node));

        let edges = ResourceScope::all("graph.edge");
        assert!(!all_nodes.matches(&edges));
    }

    #[test]
    fn test_permission_matches() {
        let perm = Permission::allow(
            "read_all_nodes",
            Action::new("read:*"),
            ResourceScope::all("graph.node"),
            "tenant-1",
        );

        assert!(perm.matches(&Action::new("read:user"), &ResourceScope::of("graph.node", "123")));
        assert!(!perm.matches(&Action::new("write:user"), &ResourceScope::of("graph.node", "123")));
    }

    #[test]
    fn test_role_operations() {
        let mut role = Role::new("Test Role", "test_role", "tenant-1");
        assert!(!role.is_system);
        assert_eq!(role.permission_ids.len(), 0);

        role.add_permission("perm-1");
        role.add_permission("perm-2");
        assert_eq!(role.permission_ids.len(), 2);

        assert!(role.remove_permission("perm-1"));
        assert!(!role.remove_permission("perm-1"));
        assert_eq!(role.permission_ids.len(), 1);
    }

    #[test]
    fn test_tenant_active() {
        let mut tenant = Tenant::new("Test Tenant", "test");
        assert!(tenant.is_active());

        tenant.status = TenantStatus::Suspended;
        assert!(!tenant.is_active());
    }

    #[test]
    fn test_user_subject() {
        let user = User::new("alice", "Alice", "tenant-1");
        let subject = user.to_subject();
        assert_eq!(subject.id, user.id);
        assert_eq!(subject.tenant_id, user.tenant_id);
        assert_eq!(subject.subject_type, SubjectType::User);
    }

    #[test]
    fn test_role_binding_expiry() {
        let mut binding = RoleBinding::new("user-1", SubjectType::User, "role-1", "tenant-1");
        assert!(!binding.is_expired());

        binding.expires_at = Some(now_ms() - 1000);
        assert!(binding.is_expired());
    }
}
