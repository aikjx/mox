// ====================================================================
// system/permission.rs — 系统管理子模块
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, DEFAULT_USER, ok, err, q_str, resolve_tenant, status_flag};
use axum::extract::{Path, Query, State};
use mox_api_protocol::{ApiResponse, api_ok};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn current_user_handler(crate::auth::ApiAuth(user): crate::auth::ApiAuth) -> ApiResponse<Value> {
    api_ok(json!(user))
}


pub(crate) async fn get_permissions(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let user = q_str(&q, "user_id", DEFAULT_USER);
    let roles = s.iam.get_user_roles(&tenant, &user).unwrap_or_default();
    let perms = s.iam.get_user_permissions(&tenant, &user).unwrap_or_default();
    ok(json!({
        "user_id": user,
        "tenant_id": tenant,
        "roles": roles.iter().map(|r| json!({
            "id": r.role_id, "code": r.role_code, "name": r.role_name
        })).collect::<Vec<_>>(),
        "permissions": perms,
        "menus": [],
    }))
}

/// GET /api/system/dept —— 部门列表

pub(crate) async fn get_user_roles(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_user_roles(&tenant, &id) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|r| json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "status": status_flag(&r.status),
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("user roles: {e}")),
    }
}

