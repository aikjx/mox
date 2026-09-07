// ====================================================================
// system/user.rs — 系统管理子模块
// ====================================================================

use crate::auth::ApiAuth;
use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, DEFAULT_USER, ok, err, q_str, resolve_tenant, now_iso, status_flag, build_tree,
    opt_str, opt_i64, opt_status, user_json, dept_json, role_json, post_json,
    menu_json, dict_type_json, dict_data_json, config_json, oper_log_json,
    login_log_json, api_key_json};
use axum::extract::{Path, Query, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub(crate) async fn list_users_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users(&tenant) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("user list: {e}")),
    }
}


pub(crate) async fn create_user_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let username = opt_str(&body, "username").unwrap_or("");
    let user_code = opt_str(&body, "userCode")
        .map(String::from)
        .unwrap_or_else(|| format!("U{}", chrono::Utc::now().timestamp()));
    let real_name = opt_str(&body, "realName");
    let password_hash = opt_str(&body, "password");
    let dept_id = opt_str(&body, "deptId");
    match s.iam.create_user(
        &tenant,
        &user_code,
        username,
        real_name,
        password_hash,
        dept_id,
        false,
    ) {
        Ok(u) => ok(user_json(&u)),
        Err(e) => err(&format!("user create: {e}")),
    }
}


pub(crate) async fn get_user_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_user(&id) {
        Ok(Some(u)) => ok(user_json(&u)),
        Ok(None) => err("user not found"),
        Err(e) => err(&format!("user detail: {e}")),
    }
}


pub(crate) async fn update_user_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let username = opt_str(&body, "username");
    let real_name = opt_str(&body, "realName");
    let email = opt_str(&body, "email");
    let phone = opt_str(&body, "phone");
    let dept_id = opt_str(&body, "deptId");
    let position = opt_str(&body, "position");
    let user_status = opt_status(&body);
    match s.iam.update_user(
        &id,
        username,
        real_name,
        email,
        phone,
        dept_id,
        position,
        user_status,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("user update: {e}")),
    }
}


pub(crate) async fn delete_user_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_user(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("user delete: {e}")),
    }
}


pub(crate) async fn reset_user_pwd_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let password = opt_str(&body, "password").unwrap_or("");
    match s.iam.reset_password(&id, password) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("reset password: {e}")),
    }
}


pub(crate) async fn change_user_status_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let status = opt_status(&body).unwrap_or("active");
    match s.iam.change_user_status(&id, status) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("change status: {e}")),
    }
}


pub(crate) async fn assign_user_roles_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let role_ids: Vec<String> = body
        .get("roleIds")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_user_roles(&tenant, &id, &role_ids) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("assign roles: {e}")),
    }
}

// ----- 角色 Role -----

