// ====================================================================
// system/role.rs — 系统管理子模块
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, err, q_str, resolve_tenant, status_flag,
    opt_str, opt_i64, opt_status, user_json, role_json};
use axum::extract::{Path, Query, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn list_roles(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_roles(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|r| json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "type": r.role_type,
                "dataScope": r.data_scope,
                "sort": r.sort_order,
                "status": status_flag(&r.status),
                "remark": r.description,
                "createdAt": r.created_at,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("role list: {e}")),
    }
}

/// GET /api/system/role/:id —— 角色详情

pub(crate) async fn get_role(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_roles(&tenant) {
        Ok(list) => match list.into_iter().find(|r| r.role_id == id) {
            Some(r) => ok(json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "type": r.role_type,
                "dataScope": r.data_scope,
                "sort": r.sort_order,
                "status": status_flag(&r.status),
                "remark": r.description,
                "createdAt": r.created_at,
            })),
            None => err(&format!("role not found: {id}")),
        },
        Err(e) => err(&format!("role detail: {e}")),
    }
}

/// GET /api/system/user/:id/roles —— 用户已分配角色

pub(crate) async fn create_role_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "code").unwrap_or("");
    let name = opt_str(&body, "name").unwrap_or("");
    let role_type = opt_str(&body, "type");
    let data_scope = opt_str(&body, "dataScope");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let description = opt_str(&body, "remark");
    match s.iam.create_role(
        &tenant,
        code,
        name,
        role_type,
        data_scope,
        sort,
        status,
        description,
    ) {
        Ok(r) => ok(role_json(&r)),
        Err(e) => err(&format!("role create: {e}")),
    }
}


pub(crate) async fn update_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let code = opt_str(&body, "code");
    let data_scope = opt_str(&body, "dataScope");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let description = opt_str(&body, "remark");
    match s.iam.update_role(&id, name, code, data_scope, sort, status, description) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("role update: {e}")),
    }
}


pub(crate) async fn delete_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_role(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("role delete: {e}")),
    }
}


pub(crate) async fn get_role_menu_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_role_menu_ids(&tenant, &id) {
        Ok(menu_ids) => ok(json!(menu_ids)),
        Err(e) => err(&format!("role menu perms: {e}")),
    }
}


pub(crate) async fn set_role_menu_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let menu_ids: Vec<String> = body
        .get("menuIds")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_role_menus(&tenant, &id, &menu_ids) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("set role menus: {e}")),
    }
}


pub(crate) async fn get_role_data_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_role_data_perms(&tenant, &id) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|dp| json!({
                "id": dp.dp_id,
                "code": dp.dp_code,
                "name": dp.dp_name,
                "scopeType": dp.scope_type,
                "resourceCode": dp.resource_code,
                "status": status_flag(&dp.status),
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("role data perms: {e}")),
    }
}


pub(crate) async fn set_role_data_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dp_codes: Vec<String> = body
        .get("dpCodes")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_role_data_perms(&tenant, &id, &dp_codes) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("set role data perms: {e}")),
    }
}


pub(crate) async fn list_role_users_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users_by_role(&tenant, &id) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("role users: {e}")),
    }
}


pub(crate) async fn copy_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let new_code = opt_str(&body, "code").unwrap_or("");
    let new_name = opt_str(&body, "name").unwrap_or("");
    match s.iam.copy_role(&tenant, &id, new_code, new_name) {
        Ok(r) => ok(role_json(&r)),
        Err(e) => err(&format!("copy role: {e}")),
    }
}

// ----- 菜单 Menu -----

