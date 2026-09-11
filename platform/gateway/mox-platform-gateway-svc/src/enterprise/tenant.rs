//! 多租户 / 部门 / 权限 最小颗粒度控制框架
//!
//! 支持：
//! - 租户级隔离（Tenant）
//! - 部门级隔离（Department，支持树形结构）
//! - 角色权限控制（Role，支持自定义角色）
//! - 资源级权限（Resource，支持API/数据/功能三级）
//! - 数据行级权限（Row Level Security）
//! - 快速分配（批量授权、角色继承、权限模板）

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 租户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_code: String,
    pub status: TenantStatus,
    pub max_users: Option<i64>,
    pub max_storage: Option<i64>,
    pub expire_at: Option<String>,
    pub config: HashMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 租户状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    Active,
    Suspended,
    Expired,
    Deleted,
}

/// 部门信息（支持树形结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub department_id: String,
    pub tenant_id: String,
    pub department_name: String,
    pub department_code: String,
    pub parent_id: Option<String>,
    pub department_type: DepartmentType,
    pub manager_id: Option<String>,
    pub sort_order: i32,
    pub status: DepartmentStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// 部门类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DepartmentType {
    Company,
    Division,
    Department,
    Team,
    Group,
    Virtual,
}

/// 部门状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DepartmentStatus {
    Active,
    Inactive,
    Disbanded,
}

/// 角色信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub role_id: String,
    pub tenant_id: String,
    pub role_name: String,
    pub role_code: String,
    pub role_type: RoleType,
    pub description: Option<String>,
    pub permissions: HashSet<String>,
    pub data_scope: DataScope,
    pub is_system: bool,
    pub status: RoleStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// 角色类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleType {
    SuperAdmin,
    TenantAdmin,
    DepartmentAdmin,
    Business,
    Custom,
}

/// 角色状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleStatus {
    Active,
    Inactive,
}

/// 数据权限范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataScope {
    /// 全部数据（跨租户）
    All,
    /// 本租户全部数据
    TenantAll,
    /// 本部门及子部门数据
    DepartmentAndSub,
    /// 仅本部门数据
    DepartmentOnly,
    /// 仅本人数据
    SelfOnly,
    /// 自定义数据范围
    Custom(Vec<String>),
}

/// 权限定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub permission_id: String,
    pub permission_code: String,
    pub permission_name: String,
    pub permission_type: PermissionType,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub action: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: String,
}

/// 权限类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionType {
    /// API接口权限
    Api,
    /// 数据权限
    Data,
    /// 功能权限
    Feature,
    /// 菜单权限
    Menu,
    /// 按钮权限
    Button,
}

/// 用户权限上下文（请求级）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissionContext {
    pub user_id: String,
    pub tenant_id: String,
    pub department_id: Option<String>,
    pub department_path: Vec<String>,
    pub roles: Vec<String>,
    pub permissions: HashSet<String>,
    pub data_scope: DataScope,
    pub is_super_admin: bool,
    pub is_tenant_admin: bool,
}

impl UserPermissionContext {
    /// 检查是否有指定权限
    pub fn has_permission(&self, permission_code: &str) -> bool {
        if self.is_super_admin {
            return true;
        }
        self.permissions.contains(permission_code)
    }

    /// 检查是否有任意一个权限
    pub fn has_any_permission(&self, permission_codes: &[&str]) -> bool {
        if self.is_super_admin {
            return true;
        }
        permission_codes.iter().any(|p| self.permissions.contains(*p))
    }

    /// 检查是否有所有权限
    pub fn has_all_permissions(&self, permission_codes: &[&str]) -> bool {
        if self.is_super_admin {
            return true;
        }
        permission_codes.iter().all(|p| self.permissions.contains(*p))
    }

    /// 检查是否有指定角色
    pub fn has_role(&self, role_code: &str) -> bool {
        self.roles.contains(&role_code.to_string())
    }

    /// 检查是否可以访问指定租户的数据
    pub fn can_access_tenant(&self, target_tenant_id: &str) -> bool {
        if self.is_super_admin {
            return true;
        }
        self.tenant_id == target_tenant_id
    }

    /// 检查是否可以访问指定部门的数据
    pub fn can_access_department(&self, target_department_id: &str) -> bool {
        if self.is_super_admin || self.is_tenant_admin {
            return true;
        }
        match &self.data_scope {
            DataScope::All | DataScope::TenantAll => true,
            DataScope::DepartmentAndSub => {
                self.department_path.contains(&target_department_id.to_string())
            }
            DataScope::DepartmentOnly => {
                self.department_id.as_deref() == Some(target_department_id)
            }
            DataScope::SelfOnly => false,
            DataScope::Custom(depts) => depts.contains(&target_department_id.to_string()),
        }
    }
}

/// 权限服务状态
pub struct PermissionState {
    pub tenants: Arc<RwLock<HashMap<String, Tenant>>>,
    pub departments: Arc<RwLock<HashMap<String, Department>>>,
    pub roles: Arc<RwLock<HashMap<String, Role>>>,
    pub permissions: Arc<RwLock<HashMap<String, Permission>>>,
    pub user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PermissionState {
    pub fn new() -> Self {
        let mut permissions = HashMap::new();
        // 预置系统权限
        let sys_perms = [
            ("api:user:read", "读取用户", "api", "user", "read"),
            ("api:user:write", "写入用户", "api", "user", "write"),
            ("api:user:delete", "删除用户", "api", "user", "delete"),
            ("api:department:read", "读取部门", "api", "department", "read"),
            ("api:department:write", "写入部门", "api", "department", "write"),
            ("api:role:read", "读取角色", "api", "role", "read"),
            ("api:role:write", "写入角色", "api", "role", "write"),
            ("api:tenant:read", "读取租户", "api", "tenant", "read"),
            ("api:tenant:write", "写入租户", "api", "tenant", "write"),
            ("feature:approval:initiate", "发起审批", "feature", "approval", "initiate"),
            ("feature:approval:approve", "审批", "feature", "approval", "approve"),
            ("feature:document:read", "读取文档", "feature", "document", "read"),
            ("feature:document:write", "写入文档", "feature", "document", "write"),
            ("feature:document:sign", "签署文档", "feature", "document", "sign"),
        ];
        for (code, name, ptype, resource, action) in sys_perms.iter() {
            let perm = Permission {
                permission_id: format!("perm_{}", code.replace(':', "_")),
                permission_code: code.to_string(),
                permission_name: name.to_string(),
                permission_type: match *ptype {
                    "api" => PermissionType::Api,
                    "data" => PermissionType::Data,
                    "feature" => PermissionType::Feature,
                    "menu" => PermissionType::Menu,
                    _ => PermissionType::Button,
                },
                resource_type: resource.to_string(),
                resource_id: None,
                action: action.to_string(),
                description: None,
                is_system: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            permissions.insert(perm.permission_code.clone(), perm);
        }

        let mut roles = HashMap::new();
        // 预置超级管理员角色
        let super_admin = Role {
            role_id: "role_super_admin".to_string(),
            tenant_id: "system".to_string(),
            role_name: "超级管理员".to_string(),
            role_code: "super_admin".to_string(),
            role_type: RoleType::SuperAdmin,
            description: Some("系统超级管理员，拥有全部权限".to_string()),
            permissions: HashSet::new(),
            data_scope: DataScope::All,
            is_system: true,
            status: RoleStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        roles.insert(super_admin.role_code.clone(), super_admin);

        // 预置租户管理员角色
        let tenant_admin = Role {
            role_id: "role_tenant_admin".to_string(),
            tenant_id: "default".to_string(),
            role_name: "租户管理员".to_string(),
            role_code: "tenant_admin".to_string(),
            role_type: RoleType::TenantAdmin,
            description: Some("租户管理员，拥有本租户全部权限".to_string()),
            permissions: HashSet::new(),
            data_scope: DataScope::TenantAll,
            is_system: true,
            status: RoleStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        roles.insert(tenant_admin.role_code.clone(), tenant_admin);

        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            departments: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(roles)),
            permissions: Arc::new(RwLock::new(permissions)),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::new()
    }
}

/// 快速分配：批量授权用户角色
pub async fn assign_roles_to_users(
    state: &PermissionState,
    user_ids: &[String],
    role_codes: &[String],
) -> Result<usize, String> {
    let mut user_roles = state.user_roles.write().await;
    let mut count = 0;
    for user_id in user_ids {
        let roles = user_roles.entry(user_id.clone()).or_insert_with(Vec::new);
        for role_code in role_codes {
            if !roles.contains(role_code) {
                roles.push(role_code.clone());
                count += 1;
            }
        }
    }
    Ok(count)
}

/// 快速分配：从角色模板创建角色
pub async fn create_role_from_template(
    state: &PermissionState,
    template_name: &str,
    new_role_name: &str,
    tenant_id: &str,
) -> Result<String, String> {
    let roles = state.roles.read().await;
    let template = roles.get(template_name)
        .ok_or_else(|| format!("角色模板 '{}' 不存在", template_name))?;

    let new_role = Role {
        role_id: format!("role_{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        role_name: new_role_name.to_string(),
        role_code: format!("custom_{}", uuid::Uuid::new_v4().simple()),
        role_type: RoleType::Custom,
        description: Some(format!("从模板 '{}' 创建", template_name)),
        permissions: template.permissions.clone(),
        data_scope: template.data_scope.clone(),
        is_system: false,
        status: RoleStatus::Active,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let role_code = new_role.role_code.clone();
    drop(roles);
    state.roles.write().await.insert(role_code.clone(), new_role);
    Ok(role_code)
}
