// ====================================================================
// system/dept.rs — 系统管理子模块
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

pub(crate) async fn list_dept(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_departments(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|d| json!({
                "id": d.dept_id,
                "name": d.dept_name,
                "code": d.dept_code,
                "parentId": d.parent_id,
                "sort": d.sort_order,
                "status": status_flag(&d.status),
                "leaderId": d.manager_user_id,
                "createdAt": d.created_at,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dept list: {e}")),
    }
}

/// GET /api/system/dept/tree —— 部门树

pub(crate) async fn dept_tree(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_departments(&tenant) {
        Ok(list) => {
            let flat = list
                .iter()
                .map(|d| {
                    let mut m = Map::new();
                    m.insert("id".into(), json!(d.dept_id));
                    m.insert("name".into(), json!(d.dept_name));
                    m.insert("code".into(), json!(d.dept_code));
                    m.insert("parentId".into(), json!(d.parent_id));
                    m.insert("sort".into(), json!(d.sort_order));
                    m.insert("status".into(), json!(status_flag(&d.status)));
                    m
                })
                .collect();
            ok(json!(build_tree(flat)))
        }
        Err(e) => err(&format!("dept tree: {e}")),
    }
}

/// GET /api/system/role —— 角色列表

pub(crate) async fn create_dept_handler(
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
    let parent_id = opt_str(&body, "parentId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let manager = opt_str(&body, "leaderId");
    match s.iam.create_dept(&tenant, code, name, parent_id, sort, status, manager) {
        Ok(d) => ok(dept_json(&d)),
        Err(e) => err(&format!("dept create: {e}")),
    }
}


pub(crate) async fn get_dept_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dept(&id) {
        Ok(Some(d)) => ok(dept_json(&d)),
        Ok(None) => err("dept not found"),
        Err(e) => err(&format!("dept detail: {e}")),
    }
}


pub(crate) async fn update_dept_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let parent_id = opt_str(&body, "parentId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let manager = opt_str(&body, "leaderId");
    match s.iam.update_dept(&id, name, parent_id, sort, status, manager) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dept update: {e}")),
    }
}


pub(crate) async fn delete_dept_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dept(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dept delete: {e}")),
    }
}


pub(crate) async fn list_dept_users_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users_by_dept(&tenant, &id) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("dept users: {e}")),
    }
}

// ----- 岗位 Post -----

