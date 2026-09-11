//! 企业级管理 API 端点
//!
//! 提供：租户管理 / 部门管理 / 角色权限管理 / 审计日志查询 / 系统配置
//! 所有企业级管理功能统一在此注册

use crate::enterprise::api_response::*;
use crate::enterprise::audit::*;
use crate::enterprise::tenant::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// 企业级管理状态
#[derive(Clone)]
pub struct AdminState {
    /// 权限控制状态
    pub permission: Arc<PermissionState>,
    /// 审计日志状态
    pub audit: Arc<AuditState>,
}

impl AdminState {
    pub fn new() -> Self {
        Self {
            permission: Arc::new(PermissionState::new()),
            audit: Arc::new(AuditState::new()),
        }
    }
}

impl Default for AdminState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 租户管理 ====================

/// GET /api/enterprise/admin/tenants —— 获取租户列表
pub async fn list_tenants_handler(
    State(state): State<Arc<AdminState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let tenants = state.permission.tenants.read().await;
    let mut list: Vec<&Tenant> = tenants.values().collect();
    if let Some(status) = params.get("status") {
        list.retain(|t| format!("{:?}", t.status).to_lowercase() == *status);
    }
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// POST /api/enterprise/admin/tenants —— 创建租户
pub async fn create_tenant_handler(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateTenantRequest>,
) -> Response {
    if let Err(e) = validate_required("tenant_name", &req.tenant_name) {
        return e;
    }
    let tenant_id = format!("tenant_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let tenant = Tenant {
        tenant_id: tenant_id.clone(),
        tenant_name: req.tenant_name,
        tenant_code: req.tenant_code.unwrap_or_else(|| tenant_id.clone()),
        status: TenantStatus::Active,
        max_users: req.max_users,
        max_storage: req.max_storage,
        expire_at: req.expire_at,
        config: req.config.unwrap_or_default(),
        created_at: now.clone(),
        updated_at: now,
    };
    state.permission.tenants.write().await.insert(tenant_id.clone(), tenant);

    // 记录审计日志
    state.audit.record_success(
        AuditActionType::Create, "admin", "tenant",
        Some(&tenant_id), "创建租户",
        "system", "系统",
    ).await;

    success_with_message("租户创建成功", json!({ "tenant_id": tenant_id }))
}

/// GET /api/enterprise/admin/tenants/:id —— 获取租户详情
pub async fn get_tenant_handler(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Response {
    let tenants = state.permission.tenants.read().await;
    match tenants.get(&id) {
        Some(t) => success(t),
        None => not_found("租户不存在"),
    }
}

// ==================== 部门管理 ====================

/// GET /api/enterprise/admin/departments —— 获取部门列表
pub async fn list_departments_handler(
    State(state): State<Arc<AdminState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let departments = state.permission.departments.read().await;
    let mut list: Vec<&Department> = departments.values().collect();
    if let Some(parent_id) = params.get("parent_id") {
        list.retain(|d| d.parent_id.as_deref() == Some(parent_id));
    }
    if let Some(tenant_id) = params.get("tenant_id") {
        list.retain(|d| d.tenant_id == *tenant_id);
    }
    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success_with_message("success", json!(list))
}

/// POST /api/enterprise/admin/departments —— 创建部门
pub async fn create_department_handler(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateDepartmentRequest>,
) -> Response {
    if let Err(e) = validate_required("department_name", &req.department_name) {
        return e;
    }
    let department_id = format!("dept_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let department = Department {
        department_id: department_id.clone(),
        tenant_id: req.tenant_id.unwrap_or_else(|| "default".to_string()),
        department_name: req.department_name,
        department_code: req.department_code.unwrap_or_else(|| department_id.clone()),
        parent_id: req.parent_id,
        department_type: req.department_type.unwrap_or(DepartmentType::Department),
        manager_id: req.manager_id,
        sort_order: req.sort_order.unwrap_or(0),
        status: DepartmentStatus::Active,
        created_at: now.clone(),
        updated_at: now,
    };
    state.permission.departments.write().await.insert(department_id.clone(), department);

    state.audit.record_success(
        AuditActionType::Create, "admin", "department",
        Some(&department_id), "创建部门",
        "system", "系统",
    ).await;

    success_with_message("部门创建成功", json!({ "department_id": department_id }))
}

// ==================== 角色权限管理 ====================

/// GET /api/enterprise/admin/roles —— 获取角色列表
pub async fn list_roles_handler(
    State(state): State<Arc<AdminState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let roles = state.permission.roles.read().await;
    let mut list: Vec<&Role> = roles.values().collect();
    if let Some(tenant_id) = params.get("tenant_id") {
        list.retain(|r| r.tenant_id == *tenant_id || r.tenant_id == "system");
    }
    if let Some(role_type) = params.get("role_type") {
        list.retain(|r| format!("{:?}", r.role_type).to_lowercase() == *role_type);
    }
    success_with_message("success", json!(list))
}

/// POST /api/enterprise/admin/roles —— 创建角色
pub async fn create_role_handler(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateRoleRequest>,
) -> Response {
    if let Err(e) = validate_required("role_name", &req.role_name) {
        return e;
    }
    let role_id = format!("role_{}", uuid::Uuid::new_v4().simple());
    let role_code = req.role_code.unwrap_or_else(|| format!("custom_{}", uuid::Uuid::new_v4().simple()));
    let now = chrono::Utc::now().to_rfc3339();
    let role = Role {
        role_id: role_id.clone(),
        tenant_id: req.tenant_id.unwrap_or_else(|| "default".to_string()),
        role_name: req.role_name,
        role_code: role_code.clone(),
        role_type: RoleType::Custom,
        description: req.description,
        permissions: req.permissions.unwrap_or_default(),
        data_scope: req.data_scope.unwrap_or(DataScope::SelfOnly),
        is_system: false,
        status: RoleStatus::Active,
        created_at: now.clone(),
        updated_at: now,
    };
    state.permission.roles.write().await.insert(role_code.clone(), role);

    state.audit.record_success(
        AuditActionType::Create, "admin", "role",
        Some(&role_code), "创建角色",
        "system", "系统",
    ).await;

    success_with_message("角色创建成功", json!({ "role_id": role_id, "role_code": role_code }))
}

/// GET /api/enterprise/admin/permissions —— 获取权限列表
pub async fn list_permissions_handler(
    State(state): State<Arc<AdminState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let permissions = state.permission.permissions.read().await;
    let mut list: Vec<&Permission> = permissions.values().collect();
    if let Some(permission_type) = params.get("permission_type") {
        list.retain(|p| format!("{:?}", p.permission_type).to_lowercase() == *permission_type);
    }
    if let Some(resource_type) = params.get("resource_type") {
        list.retain(|p| p.resource_type == *resource_type);
    }
    success_with_message("success", json!(list))
}

/// POST /api/enterprise/admin/users/:id/roles —— 分配用户角色（快速分配）
pub async fn assign_user_roles_handler(
    State(state): State<Arc<AdminState>>,
    Path(user_id): Path<String>,
    Json(req): Json<AssignRolesRequest>,
) -> Response {
    let count = match assign_roles_to_users(&state.permission, &[user_id.clone()], &req.role_codes).await {
        Ok(c) => c,
        Err(e) => return internal_error(&e),
    };

    state.audit.record_success(
        AuditActionType::PermissionChange, "admin", "user_role",
        Some(&user_id), &format!("分配用户角色：{}", req.role_codes.join(", ")),
        "system", "系统",
    ).await;

    success_with_message(&format!("成功分配 {} 个角色", count), json!({ "user_id": user_id, "assigned_count": count }))
}

// ==================== 审计日志 ====================

/// GET /api/enterprise/admin/audit-logs —— 查询审计日志
pub async fn list_audit_logs_handler(
    State(state): State<Arc<AdminState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let logs = state.audit.query(
        params.get("tenant_id").map(|s| s.as_str()),
        params.get("user_id").map(|s| s.as_str()),
        params.get("action_type").map(|s| s.as_str()),
        params.get("module").map(|s| s.as_str()),
        params.get("resource_type").map(|s| s.as_str()),
        params.get("result").map(|s| s.as_str()),
        params.get("start_time").map(|s| s.as_str()),
        params.get("end_time").map(|s| s.as_str()),
    ).await;

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, logs.len());
    let page_items = pagination.paginate(&logs);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/admin/audit-stats —— 获取审计统计
pub async fn audit_stats_handler(
    State(state): State<Arc<AdminState>>,
) -> Response {
    let stats = state.audit.stats().await;
    success(stats)
}

// ==================== 请求结构体 ====================

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_name: String,
    pub tenant_code: Option<String>,
    pub max_users: Option<i64>,
    pub max_storage: Option<i64>,
    pub expire_at: Option<String>,
    pub config: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub department_name: String,
    pub department_code: Option<String>,
    pub tenant_id: Option<String>,
    pub parent_id: Option<String>,
    pub department_type: Option<DepartmentType>,
    pub manager_id: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub role_name: String,
    pub role_code: Option<String>,
    pub tenant_id: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<std::collections::HashSet<String>>,
    pub data_scope: Option<DataScope>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRolesRequest {
    pub role_codes: Vec<String>,
}

/// 构建企业级管理路由（泛型版本）
pub fn build_admin_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<AdminState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        // 租户管理
        .route("/tenants", get(list_tenants_handler).post(create_tenant_handler))
        .route("/tenants/:id", get(get_tenant_handler))
        // 部门管理
        .route("/departments", get(list_departments_handler).post(create_department_handler))
        // 角色权限管理
        .route("/roles", get(list_roles_handler).post(create_role_handler))
        .route("/permissions", get(list_permissions_handler))
        .route("/users/:id/roles", post(assign_user_roles_handler))
        // 审计日志
        .route("/audit-logs", get(list_audit_logs_handler))
        .route("/audit-stats", get(audit_stats_handler))
}
