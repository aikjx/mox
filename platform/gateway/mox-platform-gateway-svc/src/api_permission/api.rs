//! API权限管理中心 API 端点
//!
//! 提供：API端点管理 / 权限点管理 / 角色管理 / 用户权限 / 一键授权 / 权限验证 / 统计

use crate::api_permission::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

// ==================== API端点管理 ====================

/// GET /api/enterprise/api-permission/endpoints —— 获取API端点列表
pub async fn list_endpoints_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let endpoints = state.endpoints.read().await;
    let mut list: Vec<&ApiEndpoint> = endpoints.values().collect();

    if let Some(module) = params.get("module") {
        list.retain(|e| e.module == *module);
    }
    if let Some(method) = params.get("method") {
        list.retain(|e| e.method.eq_ignore_ascii_case(method));
    }
    if let Some(category) = params.get("category") {
        list.retain(|e| e.category == *category);
    }
    if let Some(has_permission) = params.get("has_permission") {
        let has = has_permission == "true";
        list.retain(|e| e.permission_code.is_some() == has);
    }
    if let Some(keyword) = params.get("keyword") {
        list.retain(|e| {
            e.path.contains(keyword) || e.description.as_deref().unwrap_or("").contains(keyword)
        });
    }

    list.sort_by(|a, b| a.endpoint_id.cmp(&b.endpoint_id));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/api-permission/endpoints/:id —— 获取API端点详情
pub async fn get_endpoint_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(id): Path<String>,
) -> Response {
    let endpoints = state.endpoints.read().await;
    match endpoints.get(&id) {
        Some(e) => success(e),
        None => not_found("API端点不存在"),
    }
}

/// PUT /api/enterprise/api-permission/endpoints/:id —— 更新API端点（关联权限）
pub async fn update_endpoint_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateEndpointRequest>,
) -> Response {
    let mut endpoints = state.endpoints.write().await;
    match endpoints.get_mut(&id) {
        Some(e) => {
            if let Some(permission_code) = req.permission_code {
                e.permission_code = Some(permission_code);
            }
            if let Some(description) = req.description {
                e.description = Some(description);
            }
            if let Some(require_permission) = req.require_permission {
                e.require_permission = require_permission;
            }
            if let Some(is_public) = req.is_public {
                e.is_public = is_public;
            }
            if let Some(category) = req.category {
                e.category = category;
            }
            success_message("API端点更新成功")
        }
        None => not_found("API端点不存在"),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateEndpointRequest {
    pub permission_code: Option<String>,
    pub description: Option<String>,
    pub require_permission: Option<bool>,
    pub is_public: Option<bool>,
    pub category: Option<String>,
}

// ==================== 权限点管理 ====================

/// GET /api/enterprise/api-permission/permissions —— 获取权限点列表
pub async fn list_permissions_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let permissions = state.permissions.read().await;
    let mut list: Vec<&Permission> = permissions.values().collect();

    if let Some(module) = params.get("module") {
        list.retain(|p| p.module == *module);
    }
    if let Some(permission_type) = params.get("permission_type") {
        list.retain(|p| p.permission_type == *permission_type);
    }

    list.sort_by(|a, b| a.module.cmp(&b.module).then_with(|| a.sort_order.cmp(&b.sort_order)));
    success(list)
}

/// POST /api/enterprise/api-permission/permissions —— 创建权限点
pub async fn create_permission_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Json(permission): Json<Permission>,
) -> Response {
    let mut permissions = state.permissions.write().await;
    if permissions.contains_key(&permission.permission_code) {
        return conflict(&format!("权限编码 '{}' 已存在", permission.permission_code));
    }
    permissions.insert(permission.permission_code.clone(), permission.clone());
    success_with_message("权限点创建成功", json!({ "permission_code": permission.permission_code }))
}

/// DELETE /api/enterprise/api-permission/permissions/:code —— 删除权限点
pub async fn delete_permission_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(code): Path<String>,
) -> Response {
    let mut permissions = state.permissions.write().await;
    match permissions.get(&code) {
        Some(p) if p.is_system => forbidden("系统内置权限不可删除"),
        Some(_) => {
            permissions.remove(&code);
            success_message("权限点删除成功")
        }
        None => not_found("权限点不存在"),
    }
}

// ==================== 角色管理 ====================

/// GET /api/enterprise/api-permission/roles —— 获取角色列表
pub async fn list_roles_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let roles = state.roles.read().await;
    let mut list: Vec<&Role> = roles.values().collect();

    if let Some(role_type) = params.get("role_type") {
        list.retain(|r| r.role_type == *role_type);
    }
    if let Some(tenant_id) = params.get("tenant_id") {
        list.retain(|r| r.tenant_id.as_deref() == Some(tenant_id.as_str()));
    }

    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success(list)
}

/// POST /api/enterprise/api-permission/roles —— 创建角色
pub async fn create_role_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Json(role): Json<Role>,
) -> Response {
    let mut roles = state.roles.write().await;
    if roles.values().any(|r| r.role_code == role.role_code) {
        return conflict(&format!("角色编码 '{}' 已存在", role.role_code));
    }
    roles.insert(role.role_id.clone(), role.clone());
    success_with_message("角色创建成功", json!({ "role_id": role.role_id, "role_code": role.role_code }))
}

/// PUT /api/enterprise/api-permission/roles/:id —— 更新角色
pub async fn update_role_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRoleRequest>,
) -> Response {
    let mut roles = state.roles.write().await;
    match roles.get_mut(&id) {
        Some(role) => {
            if role.role_type == "system" && req.permissions.is_some() {
                return forbidden("系统角色权限不可修改");
            }
            if let Some(role_name) = req.role_name { role.role_name = role_name; }
            if let Some(description) = req.description { role.description = Some(description); }
            if let Some(permissions) = req.permissions { role.permissions = permissions; }
            if let Some(data_scope) = req.data_scope { role.data_scope = data_scope; }
            if let Some(custom_data_scopes) = req.custom_data_scopes { role.custom_data_scopes = Some(custom_data_scopes); }
            if let Some(enabled) = req.enabled { role.enabled = enabled; }
            role.updated_at = chrono::Utc::now().to_rfc3339();
            success_message("角色更新成功")
        }
        None => not_found("角色不存在"),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role_name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub data_scope: Option<String>,
    pub custom_data_scopes: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// DELETE /api/enterprise/api-permission/roles/:id —— 删除角色
pub async fn delete_role_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(id): Path<String>,
) -> Response {
    let mut roles = state.roles.write().await;
    match roles.get(&id) {
        Some(r) if r.role_type == "system" => forbidden("系统角色不可删除"),
        Some(_) => {
            roles.remove(&id);
            success_message("角色删除成功")
        }
        None => not_found("角色不存在"),
    }
}

// ==================== 用户权限管理 ====================

/// GET /api/enterprise/api-permission/users/:user_id —— 获取用户权限
pub async fn get_user_permissions_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(user_id): Path<String>,
) -> Response {
    let user_roles = state.user_roles.read().await;
    let user_role = user_roles.get(&user_id);

    let roles_map = state.roles.read().await;
    let mut effective_permissions: Vec<String> = Vec::new();
    let mut user_role_list: Vec<Role> = Vec::new();

    if let Some(ur) = user_role {
        effective_permissions.extend(ur.extra_permissions.clone());
        for role_id in &ur.role_ids {
            if let Some(role) = roles_map.get(role_id) {
                if role.enabled {
                    effective_permissions.extend(role.permissions.clone());
                    user_role_list.push(role.clone());
                }
            }
        }
    }

    effective_permissions.sort();
    effective_permissions.dedup();

    success(json!({
        "user_id": user_id,
        "roles": user_role_list,
        "extra_permissions": user_role.map(|r| r.extra_permissions.clone()).unwrap_or_default(),
        "disabled_permissions": user_role.map(|r| r.disabled_permissions.clone()).unwrap_or_default(),
        "effective_permissions": effective_permissions,
        "permission_count": effective_permissions.len(),
    }))
}

/// PUT /api/enterprise/api-permission/users/:user_id —— 更新用户权限
pub async fn update_user_permissions_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserPermissionsRequest>,
) -> Response {
    let mut user_roles = state.user_roles.write().await;
    let user_role = user_roles.entry(user_id.clone()).or_insert_with(|| UserRole {
        user_id: user_id.clone(),
        role_ids: Vec::new(),
        extra_permissions: Vec::new(),
        disabled_permissions: Vec::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    });

    if let Some(role_ids) = req.role_ids {
        user_role.role_ids = role_ids;
    }
    if let Some(extra_permissions) = req.extra_permissions {
        user_role.extra_permissions = extra_permissions;
    }
    if let Some(disabled_permissions) = req.disabled_permissions {
        user_role.disabled_permissions = disabled_permissions;
    }
    user_role.updated_at = chrono::Utc::now().to_rfc3339();

    success_message("用户权限更新成功")
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserPermissionsRequest {
    pub role_ids: Option<Vec<String>>,
    pub extra_permissions: Option<Vec<String>>,
    pub disabled_permissions: Option<Vec<String>>,
}

// ==================== 一键授权 ====================

/// POST /api/enterprise/api-permission/quick-grant —— 一键授权
pub async fn quick_grant_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Json(req): Json<QuickGrantRequest>,
) -> Response {
    let now = chrono::Utc::now().to_rfc3339();

    match req.target_type.as_str() {
        "user" => {
            // 给用户授权
            let mut user_roles = state.user_roles.write().await;
            let user_role = user_roles.entry(req.target_id.clone()).or_insert_with(|| UserRole {
                user_id: req.target_id.clone(),
                role_ids: Vec::new(),
                extra_permissions: Vec::new(),
                disabled_permissions: Vec::new(),
                updated_at: now.clone(),
            });

            let append = req.append.unwrap_or(true);

            if let Some(role_ids) = req.role_ids {
                if append {
                    for rid in role_ids {
                        if !user_role.role_ids.contains(&rid) {
                            user_role.role_ids.push(rid);
                        }
                    }
                } else {
                    user_role.role_ids = role_ids;
                }
            }

            if let Some(permission_codes) = req.permission_codes {
                if append {
                    for pc in permission_codes {
                        if !user_role.extra_permissions.contains(&pc) {
                            user_role.extra_permissions.push(pc);
                        }
                    }
                } else {
                    user_role.extra_permissions = permission_codes;
                }
            }

            // 模块权限：一键授权某模块的所有权限
            if let Some(module_permissions) = req.module_permissions {
                let permissions = state.permissions.read().await;
                let module_codes: Vec<String> = permissions.values()
                    .filter(|p| module_permissions.contains(&p.module))
                    .map(|p| p.permission_code.clone())
                    .collect();
                drop(permissions);

                if append {
                    for pc in module_codes {
                        if !user_role.extra_permissions.contains(&pc) {
                            user_role.extra_permissions.push(pc);
                        }
                    }
                } else {
                    user_role.extra_permissions = module_codes;
                }
            }

            user_role.updated_at = chrono::Utc::now().to_rfc3339();

            success_with_message("一键授权成功", json!({
                "target_type": "user",
                "target_id": req.target_id,
                "role_count": user_role.role_ids.len(),
                "permission_count": user_role.extra_permissions.len(),
            }))
        }
        "role" => {
            // 给角色授权
            let mut roles = state.roles.write().await;
            match roles.get_mut(&req.target_id) {
                Some(role) => {
                    if role.role_type == "system" {
                        return forbidden("系统角色权限不可修改");
                    }

                    let append = req.append.unwrap_or(true);

                    if let Some(permission_codes) = req.permission_codes {
                        if append {
                            for pc in permission_codes {
                                if !role.permissions.contains(&pc) {
                                    role.permissions.push(pc);
                                }
                            }
                        } else {
                            role.permissions = permission_codes;
                        }
                    }

                    // 模块权限
                    if let Some(module_permissions) = req.module_permissions {
                        let permissions = state.permissions.read().await;
                        let module_codes: Vec<String> = permissions.values()
                            .filter(|p| module_permissions.contains(&p.module))
                            .map(|p| p.permission_code.clone())
                            .collect();
                        drop(permissions);

                        if append {
                            for pc in module_codes {
                                if !role.permissions.contains(&pc) {
                                    role.permissions.push(pc);
                                }
                            }
                        } else {
                            role.permissions = module_codes;
                        }
                    }

                    role.updated_at = chrono::Utc::now().to_rfc3339();
                    success_with_message("角色一键授权成功", json!({
                        "role_id": req.target_id,
                        "permission_count": role.permissions.len(),
                    }))
                }
                None => not_found("角色不存在"),
            }
        }
        _ => bad_request("不支持的授权目标类型，支持：user/role"),
    }
}

// ==================== 权限验证 ====================

/// POST /api/enterprise/api-permission/check —— 权限验证
pub async fn check_permission_handler(
    State(state): State<Arc<ApiPermissionState>>,
    Json(req): Json<CheckPermissionRequest>,
) -> Response {
    let result = state.check_permission(&req.user_id, &req.method, &req.path).await;
    success(result)
}

#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub user_id: String,
    pub method: String,
    pub path: String,
}

// ==================== 统计 ====================

/// GET /api/enterprise/api-permission/stats —— 权限统计
pub async fn permission_stats_handler(
    State(state): State<Arc<ApiPermissionState>>,
) -> Response {
    let endpoints = state.endpoints.read().await;
    let permissions = state.permissions.read().await;
    let roles = state.roles.read().await;
    let user_roles = state.user_roles.read().await;

    let mut stats = PermissionStats::default();
    stats.total_endpoints = endpoints.len() as i64;
    stats.endpoints_with_permission = endpoints.values().filter(|e| e.permission_code.is_some()).count() as i64;
    stats.public_endpoints = endpoints.values().filter(|e| e.is_public).count() as i64;
    stats.total_permissions = permissions.len() as i64;
    stats.total_roles = roles.len() as i64;
    stats.users_with_roles = user_roles.values().filter(|u| !u.role_ids.is_empty()).count() as i64;

    for e in endpoints.values() {
        *stats.endpoints_by_module.entry(e.module.clone()).or_insert(0) += 1;
    }
    for p in permissions.values() {
        *stats.permissions_by_module.entry(p.module.clone()).or_insert(0) += 1;
    }

    success(stats)
}

/// 构建API权限管理路由（泛型版本）
pub fn build_api_permission_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<ApiPermissionState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        // API端点管理
        .route("/endpoints", get(list_endpoints_handler))
        .route("/endpoints/:id", get(get_endpoint_handler).put(update_endpoint_handler))
        // 权限点管理
        .route("/permissions", get(list_permissions_handler).post(create_permission_handler))
        .route("/permissions/:code", delete(delete_permission_handler))
        // 角色管理
        .route("/roles", get(list_roles_handler).post(create_role_handler))
        .route("/roles/:id", put(update_role_handler).delete(delete_role_handler))
        // 用户权限管理
        .route("/users/:user_id", get(get_user_permissions_handler).put(update_user_permissions_handler))
        // 一键授权
        .route("/quick-grant", post(quick_grant_handler))
        // 权限验证
        .route("/check", post(check_permission_handler))
        // 统计
        .route("/stats", get(permission_stats_handler))
}
