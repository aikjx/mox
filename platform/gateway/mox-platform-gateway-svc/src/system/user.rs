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
    let created = match s.iam.create_user(
        &tenant,
        &user_code,
        username,
        real_name,
        password_hash,
        dept_id.as_deref(),
        false,
    ) {
        Ok(u) => u,
        Err(e) => return err(&format!("user create: {e}")),
    };
    // 多部门归属：deptIds 数组（首个为主部门；显式 deptId 若不在数组中则提升为主）
    if let Some(mut ids) = dept_ids_of(&body) {
        if let Some(primary) = &dept_id {
            if !ids.iter().any(|i| i == primary) {
                ids.insert(0, primary.to_string());
            }
        }
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        if let Err(e) = s.iam.set_user_depts(&tenant, &created.user_id, &refs) {
            return err(&format!("user multi-dept set: {e}"));
        }
    }
    match s.iam.get_user(&created.user_id) {
        Ok(Some(u)) => ok(user_json(&u)),
        _ => ok(user_json(&created)),
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
        dept_id.as_deref(),
        position,
        user_status,
    ) {
        Ok(_) => {}
        Err(e) => return err(&format!("user update: {e}")),
    }
    // deptIds 显式传入时全量重设归属（首个为主部门），优先级高于单值 deptId
    if let Some(ids) = dept_ids_of(&body) {
        let tenant = user_tenant_of(&s, &id);
        if tenant.is_empty() {
            return err("user not found");
        }
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        if let Err(e) = s.iam.set_user_depts(&tenant, &id, &refs) {
            return err(&format!("user multi-dept set: {e}"));
        }
    }
    ok(json!(null))
}

/// 解析 body.deptIds（字符串数组）；不存在或非数组时返回 None（单值 deptId 路径不受影响）
fn dept_ids_of(body: &Value) -> Option<Vec<String>> {
    body.get("deptIds").and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(String::from))
            .filter(|s| !s.is_empty())
            .collect()
    })
}

fn user_tenant_of(s: &GatewayState, user_id: &str) -> String {
    s.iam.get_user(user_id)
        .ok()
        .flatten()
        .map(|u| u.tenant_id)
        .unwrap_or_default()
}

/// GET /api/system/user/:id/depts — 用户归属的全部部门（含主部门标记）
pub(crate) async fn get_user_depts_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.list_user_depts(&id) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|m| json!({
                "deptId": m.dept_id,
                "isPrimary": m.is_primary == 1,
                "sort": m.sort_order,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("user depts: {e}")),
    }
}

/// PUT /api/system/user/:id/depts — 全量重设归属部门（deptIds 数组，首个为主部门）
pub(crate) async fn set_user_depts_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let Some(ids) = dept_ids_of(&body) else {
        return err("deptIds (string array) is required");
    };
    let tenant = user_tenant_of(&s, &id);
    if tenant.is_empty() {
        return err("user not found");
    }
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    match s.iam.set_user_depts(&tenant, &id, &refs) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|m| json!({"deptId": m.dept_id, "isPrimary": m.is_primary == 1}))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("user depts set: {e}")),
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

