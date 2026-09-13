//! API权限管理中心
//!
//! 统一管理所有API端点与用户权限的映射，实现一键授权。
//! 核心能力：API端点自动发现 / 权限点管理 / 角色权限 / 用户权限 / 一键授权 / 权限验证

pub mod api;
pub mod middleware;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API端点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// 端点ID（自动生成：METHOD:PATH）
    pub endpoint_id: String,
    /// HTTP方法
    pub method: String,
    /// API路径
    pub path: String,
    /// 所属模块
    pub module: String,
    /// 端点描述
    pub description: Option<String>,
    /// 关联的权限点编码
    pub permission_code: Option<String>,
    /// 是否需要认证
    pub require_auth: bool,
    /// 是否需要权限验证
    pub require_permission: bool,
    /// 是否公开接口
    pub is_public: bool,
    /// 接口分类（read/write/admin/system）
    pub category: String,
    /// 注册时间
    pub registered_at: String,
}

/// 权限点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// 权限编码（唯一，如 user:create, user:read）
    pub permission_code: String,
    /// 权限名称
    pub permission_name: String,
    /// 所属模块
    pub module: String,
    /// 权限类型（read/write/admin/delete/export/import）
    pub permission_type: String,
    /// 权限描述
    pub description: Option<String>,
    /// 关联的API端点列表
    pub endpoints: Vec<String>,
    /// 是否系统内置（不可删除）
    pub is_system: bool,
    /// 父权限编码（用于树形结构）
    pub parent_code: Option<String>,
    /// 排序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: String,
}

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// 角色ID
    pub role_id: String,
    /// 角色编码（唯一）
    pub role_code: String,
    /// 角色名称
    pub role_name: String,
    /// 角色描述
    pub description: Option<String>,
    /// 所属租户ID
    pub tenant_id: Option<String>,
    /// 所属部门ID（部门角色）
    pub dept_id: Option<String>,
    /// 角色类型（system/custom/dept）
    pub role_type: String,
    /// 权限编码列表
    pub permissions: Vec<String>,
    /// 数据范围（all/dept/dept_and_sub/self/custom）
    pub data_scope: String,
    /// 自定义数据范围（部门ID列表，data_scope=custom时使用）
    pub custom_data_scopes: Option<Vec<String>>,
    /// 是否启用
    pub enabled: bool,
    /// 排序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 用户角色关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    /// 用户ID
    pub user_id: String,
    /// 角色ID列表
    pub role_ids: Vec<String>,
    /// 额外权限（用户直接分配的权限，不通过角色）
    pub extra_permissions: Vec<String>,
    /// 禁用权限（用户被禁用的权限，即使角色有权限也不可用）
    pub disabled_permissions: Vec<String>,
    /// 更新时间
    pub updated_at: String,
}

/// 一键授权请求
#[derive(Debug, Deserialize)]
pub struct QuickGrantRequest {
    /// 授权目标类型（user/role/dept）
    pub target_type: String,
    /// 授权目标ID
    pub target_id: String,
    /// 角色ID列表（target_type=user时使用）
    pub role_ids: Option<Vec<String>>,
    /// 权限编码列表
    pub permission_codes: Option<Vec<String>>,
    /// 模块权限（一键授权某模块的所有权限）
    pub module_permissions: Option<Vec<String>>,
    /// 是否追加（true=追加，false=覆盖）
    pub append: Option<bool>,
    /// 授权原因
    pub reason: Option<String>,
}

/// 权限验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    /// 是否有权限
    pub allowed: bool,
    /// 用户ID
    pub user_id: String,
    /// 请求的API端点
    pub endpoint_id: String,
    /// 需要的权限编码
    pub required_permission: Option<String>,
    /// 用户拥有的角色
    pub user_roles: Vec<String>,
    /// 用户拥有的权限（合并后）
    pub effective_permissions: Vec<String>,
    /// 拒绝原因
    pub deny_reason: Option<String>,
}

/// 权限统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionStats {
    /// API端点总数
    pub total_endpoints: i64,
    /// 已关联权限的端点数
    pub endpoints_with_permission: i64,
    /// 公开接口数
    pub public_endpoints: i64,
    /// 权限点总数
    pub total_permissions: i64,
    /// 角色总数
    pub total_roles: i64,
    /// 已分配角色的用户数
    pub users_with_roles: i64,
    /// 按模块统计端点
    pub endpoints_by_module: HashMap<String, i64>,
    /// 按模块统计权限
    pub permissions_by_module: HashMap<String, i64>,
}

/// API权限管理状态
pub struct ApiPermissionState {
    /// API端点注册表
    pub endpoints: Arc<RwLock<HashMap<String, ApiEndpoint>>>,
    /// 权限点表
    pub permissions: Arc<RwLock<HashMap<String, Permission>>>,
    /// 角色表
    pub roles: Arc<RwLock<HashMap<String, Role>>>,
    /// 用户角色关联表
    pub user_roles: Arc<RwLock<HashMap<String, UserRole>>>,
}

impl ApiPermissionState {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册API端点
    pub async fn register_endpoint(&self, endpoint: ApiEndpoint) {
        self.endpoints.write().await.insert(endpoint.endpoint_id.clone(), endpoint);
    }

    /// 批量注册API端点
    pub async fn register_endpoints(&self, endpoints: Vec<ApiEndpoint>) {
        let mut map = self.endpoints.write().await;
        for ep in endpoints {
            map.insert(ep.endpoint_id.clone(), ep);
        }
    }

    /// 检查用户是否有权限访问API端点
    pub async fn check_permission(&self, user_id: &str, method: &str, path: &str) -> PermissionCheckResult {
        let endpoint_id = format!("{}:{}", method.to_uppercase(), path);
        let endpoints = self.endpoints.read().await;
        let endpoint = endpoints.get(&endpoint_id);

        // 如果端点不存在或为公开接口，直接允许
        if endpoint.is_none() {
            return PermissionCheckResult {
                allowed: true,
                user_id: user_id.to_string(),
                endpoint_id,
                required_permission: None,
                user_roles: vec![],
                effective_permissions: vec![],
                deny_reason: None,
            };
        }

        let endpoint = endpoint.unwrap();
        if endpoint.is_public || !endpoint.require_permission {
            return PermissionCheckResult {
                allowed: true,
                user_id: user_id.to_string(),
                endpoint_id,
                required_permission: endpoint.permission_code.clone(),
                user_roles: vec![],
                effective_permissions: vec![],
                deny_reason: None,
            };
        }

        let required_permission = match &endpoint.permission_code {
            Some(p) => p.clone(),
            None => {
                return PermissionCheckResult {
                    allowed: true,
                    user_id: user_id.to_string(),
                    endpoint_id,
                    required_permission: None,
                    user_roles: vec![],
                    effective_permissions: vec![],
                    deny_reason: None,
                };
            }
        };

        // 获取用户角色和权限
        let user_roles_map = self.user_roles.read().await;
        let user_role = user_roles_map.get(user_id);

        let role_ids = user_role.map(|r| r.role_ids.clone()).unwrap_or_default();
        let extra_permissions = user_role.map(|r| r.extra_permissions.clone()).unwrap_or_default();
        let disabled_permissions = user_role.map(|r| r.disabled_permissions.clone()).unwrap_or_default();

        // 合并角色权限
        let roles_map = self.roles.read().await;
        let mut effective_permissions: Vec<String> = extra_permissions.clone();
        for role_id in &role_ids {
            if let Some(role) = roles_map.get(role_id) {
                if role.enabled {
                    effective_permissions.extend(role.permissions.clone());
                }
            }
        }

        // 去重
        effective_permissions.sort();
        effective_permissions.dedup();

        // 移除禁用的权限
        let effective_permissions: Vec<String> = effective_permissions
            .into_iter()
            .filter(|p| !disabled_permissions.contains(p))
            .collect();

        // 检查是否有权限（支持通配符，如 user:* 匹配 user:read, user:create）
        let allowed = effective_permissions.iter().any(|p| {
            if p == "*" || p == &required_permission {
                return true;
            }
            // 通配符匹配：user:* 匹配 user:read
            if let Some(prefix) = p.strip_suffix(":*") {
                return required_permission.starts_with(&format!("{}:", prefix));
            }
            false
        });

        PermissionCheckResult {
            allowed,
            user_id: user_id.to_string(),
            endpoint_id,
            required_permission: Some(required_permission),
            user_roles: role_ids,
            effective_permissions,
            deny_reason: if allowed { None } else { Some("权限不足".to_string()) },
        }
    }
}

impl Default for ApiPermissionState {
    fn default() -> Self {
        Self::new()
    }
}

/// 内置系统角色
pub fn builtin_roles() -> Vec<Role> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Role {
            role_id: "role_admin".to_string(),
            role_code: "admin".to_string(),
            role_name: "超级管理员".to_string(),
            description: Some("拥有所有权限".to_string()),
            tenant_id: None,
            dept_id: None,
            role_type: "system".to_string(),
            permissions: vec!["*".to_string()],
            data_scope: "all".to_string(),
            custom_data_scopes: None,
            enabled: true,
            sort_order: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Role {
            role_id: "role_user".to_string(),
            role_code: "user".to_string(),
            role_name: "普通用户".to_string(),
            description: Some("基础用户权限".to_string()),
            tenant_id: None,
            dept_id: None,
            role_type: "system".to_string(),
            permissions: vec![
                "user:read".to_string(),
                "user:update_self".to_string(),
                "file:read".to_string(),
                "file:upload".to_string(),
                "dictionary:read".to_string(),
                "operation_log:read_self".to_string(),
            ],
            data_scope: "self".to_string(),
            custom_data_scopes: None,
            enabled: true,
            sort_order: 2,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Role {
            role_id: "role_auditor".to_string(),
            role_code: "auditor".to_string(),
            role_name: "审计员".to_string(),
            description: Some("审计日志查看权限".to_string()),
            tenant_id: None,
            dept_id: None,
            role_type: "system".to_string(),
            permissions: vec![
                "operation_log:read".to_string(),
                "operation_log:export".to_string(),
                "audit:read".to_string(),
            ],
            data_scope: "all".to_string(),
            custom_data_scopes: None,
            enabled: true,
            sort_order: 3,
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}
