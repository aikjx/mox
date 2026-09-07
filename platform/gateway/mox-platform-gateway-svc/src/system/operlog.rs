// ====================================================================
// system/operlog.rs — 系统管理子模块
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

pub(crate) async fn list_oper_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_oper_logs(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(oper_log_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("oper log list: {e}")),
    }
}


pub(crate) async fn get_oper_log_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_oper_log(&id) {
        Ok(Some(l)) => ok(oper_log_json(&l)),
        Ok(None) => err("oper log not found"),
        Err(e) => err(&format!("oper log detail: {e}")),
    }
}


pub(crate) async fn delete_oper_log_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_oper_log(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("oper log delete: {e}")),
    }
}


pub(crate) async fn clean_oper_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.clean_oper_logs(&tenant) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("oper log clean: {e}")),
    }
}


pub(crate) async fn export_oper_logs_handler() -> ApiResponse<Value> {
    ok(json!({
        "exported": false,
        "note": "CSV export not yet implemented; use list endpoint",
    }))
}

// ----- 登录日志 LoginLog -----

