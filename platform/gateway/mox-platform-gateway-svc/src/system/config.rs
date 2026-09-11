// ====================================================================
// system/config.rs — 参数配置管理
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, err, q_str, resolve_tenant,
    opt_str, opt_status, config_json};
use axum::extract::{Path, Query, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) async fn list_configs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_configs(&tenant) {
        Ok(list) => ok(json!(list.iter().map(config_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("config list: {e}")),
    }
}


pub(crate) async fn create_config_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let config_name = opt_str(&body, "configName").unwrap_or("");
    let config_key = opt_str(&body, "configKey").unwrap_or("");
    let config_value = opt_str(&body, "configValue");
    let config_type = opt_str(&body, "configType");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_config(
        &tenant,
        config_name,
        config_key,
        config_value,
        config_type,
        status,
        remark,
    ) {
        Ok(c) => ok(config_json(&c)),
        Err(e) => err(&format!("config create: {e}")),
    }
}


pub(crate) async fn get_config_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_config(&id) {
        Ok(Some(c)) => ok(config_json(&c)),
        Ok(None) => err("config not found"),
        Err(e) => err(&format!("config detail: {e}")),
    }
}


pub(crate) async fn update_config_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let config_name = opt_str(&body, "configName");
    let config_key = opt_str(&body, "configKey");
    let config_value = opt_str(&body, "configValue");
    let config_type = opt_str(&body, "configType");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_config(
        &id,
        config_name,
        config_key,
        config_value,
        config_type,
        status,
        remark,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("config update: {e}")),
    }
}


pub(crate) async fn delete_config_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_config(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("config delete: {e}")),
    }
}


pub(crate) async fn refresh_config_cache_handler() -> ApiResponse<Value> {
    ok(json!({
        "refreshed": true,
        "note": "SQLite direct-read, no cache layer",
    }))
}

// ----- 操作日志 OperLog -----


pub(crate) async fn get_config_by_key_handler(
    State(s): State<GatewayState>,
    Path(key): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_config_by_key(&tenant, &key) {
        Ok(Some(c)) => ok(config_json(&c)),
        Ok(None) => err("config not found"),
        Err(e) => err(&format!("config by key: {e}")),
    }
}
