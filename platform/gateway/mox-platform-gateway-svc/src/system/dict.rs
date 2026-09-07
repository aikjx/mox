// ====================================================================
// system/dict.rs — 系统管理子模块
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

pub(crate) async fn list_dict_types_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_types(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_type_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict type list: {e}")),
    }
}


pub(crate) async fn create_dict_type_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dict_name = opt_str(&body, "dictName").unwrap_or("");
    let dict_type = opt_str(&body, "dictType").unwrap_or("");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_dict_type(&tenant, dict_name, dict_type, status, remark) {
        Ok(d) => ok(dict_type_json(&d)),
        Err(e) => err(&format!("dict type create: {e}")),
    }
}


pub(crate) async fn list_all_dict_types_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_types(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_type_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict type all: {e}")),
    }
}


pub(crate) async fn get_dict_type_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dict_type(&id) {
        Ok(Some(d)) => ok(dict_type_json(&d)),
        Ok(None) => err("dict type not found"),
        Err(e) => err(&format!("dict type detail: {e}")),
    }
}


pub(crate) async fn update_dict_type_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let dict_name = opt_str(&body, "dictName");
    let dict_type = opt_str(&body, "dictType");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_dict_type(&id, dict_name, dict_type, status, remark) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict type update: {e}")),
    }
}


pub(crate) async fn delete_dict_type_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dict_type(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict type delete: {e}")),
    }
}

// ----- 字典数据 DictData -----



// ===== dict_data 子模块 =====

pub(crate) async fn list_dict_data_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_data(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_data_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict data list: {e}")),
    }
}


pub(crate) async fn create_dict_data_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dict_label = opt_str(&body, "dictLabel").unwrap_or("");
    let dict_value = opt_str(&body, "dictValue").unwrap_or("");
    let dict_type = opt_str(&body, "dictType").unwrap_or("");
    let dict_sort = opt_i64(&body, "dictSort");
    let css_class = opt_str(&body, "cssClass");
    let list_class = opt_str(&body, "listClass");
    let is_default = opt_str(&body, "isDefault");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_dict_data(
        &tenant,
        dict_sort,
        dict_label,
        dict_value,
        dict_type,
        css_class,
        list_class,
        is_default,
        status,
        remark,
    ) {
        Ok(d) => ok(dict_data_json(&d)),
        Err(e) => err(&format!("dict data create: {e}")),
    }
}


pub(crate) async fn list_dict_data_by_type_handler(
    State(s): State<GatewayState>,
    Path(dict_type): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_data_by_type(&tenant, &dict_type) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_data_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict data by type: {e}")),
    }
}


pub(crate) async fn get_dict_data_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dict_data(&id) {
        Ok(Some(d)) => ok(dict_data_json(&d)),
        Ok(None) => err("dict data not found"),
        Err(e) => err(&format!("dict data detail: {e}")),
    }
}


pub(crate) async fn update_dict_data_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let dict_sort = opt_i64(&body, "dictSort");
    let dict_label = opt_str(&body, "dictLabel");
    let dict_value = opt_str(&body, "dictValue");
    let dict_type = opt_str(&body, "dictType");
    let css_class = opt_str(&body, "cssClass");
    let list_class = opt_str(&body, "listClass");
    let is_default = opt_str(&body, "isDefault");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_dict_data(
        &id,
        dict_sort,
        dict_label,
        dict_value,
        dict_type,
        css_class,
        list_class,
        is_default,
        status,
        remark,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict data update: {e}")),
    }
}


pub(crate) async fn delete_dict_data_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dict_data(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict data delete: {e}")),
    }
}

// ----- 参数配置 Config -----
