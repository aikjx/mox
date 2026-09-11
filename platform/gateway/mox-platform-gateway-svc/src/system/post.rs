// ====================================================================
// system/post.rs — 系统管理子模块
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, err, q_str, resolve_tenant,
    opt_str, opt_i64, opt_status, post_json};
use axum::extract::{Path, Query, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn list_posts_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_posts(&tenant) {
        Ok(list) => ok(json!(list.iter().map(post_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("post list: {e}")),
    }
}


pub(crate) async fn create_post_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "postCode").unwrap_or("");
    let name = opt_str(&body, "postName").unwrap_or("");
    let dept_id = opt_str(&body, "deptId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_post(&tenant, code, name, dept_id, sort, status, remark) {
        Ok(p) => ok(post_json(&p)),
        Err(e) => err(&format!("post create: {e}")),
    }
}


pub(crate) async fn list_posts_by_dept_handler(
    State(s): State<GatewayState>,
    Path(dept_id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_posts_by_dept(&tenant, &dept_id) {
        Ok(list) => ok(json!(list.iter().map(post_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("post list by dept: {e}")),
    }
}


pub(crate) async fn get_post_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_post(&id) {
        Ok(Some(p)) => ok(post_json(&p)),
        Ok(None) => err("post not found"),
        Err(e) => err(&format!("post detail: {e}")),
    }
}


pub(crate) async fn update_post_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let code = opt_str(&body, "postCode");
    let name = opt_str(&body, "postName");
    let dept_id = opt_str(&body, "deptId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_post(&id, code, name, dept_id, sort, status, remark) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("post update: {e}")),
    }
}


pub(crate) async fn delete_post_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_post(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("post delete: {e}")),
    }
}

// ----- 用户 User -----

