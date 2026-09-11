// ====================================================================
// system/logininfor.rs — 系统管理子模块
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, err, q_str, resolve_tenant,
    login_log_json};
use axum::extract::{Path, Query, State};
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn list_login_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_login_logs(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(login_log_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("login log list: {e}")),
    }
}



pub(crate) async fn delete_login_log_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_login_log(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("login log delete: {e}")),
    }
}


pub(crate) async fn clean_login_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.clean_login_logs(&tenant) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("login log clean: {e}")),
    }
}


pub(crate) async fn export_login_logs_handler() -> ApiResponse<Value> {
    ok(json!({
        "exported": false,
        "note": "CSV export not yet implemented; use list endpoint",
    }))
}

