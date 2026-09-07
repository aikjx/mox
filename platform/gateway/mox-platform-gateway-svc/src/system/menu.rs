// ====================================================================
// system/menu.rs — 系统管理子模块
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

pub(crate) async fn menu_tree(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let user = q_str(&q, "user_id", DEFAULT_USER);
    match s.iam.list_user_menus(&tenant, &user) {
        Ok(list) => {
            let flat = list
                .iter()
                .map(|m| {
                    let mut node = Map::new();
                    node.insert("id".into(), json!(m.menu_id));
                    node.insert("name".into(), json!(m.menu_name));
                    node.insert("parentId".into(), json!(m.parent_id));
                    node.insert("type".into(), json!(m.menu_type));
                    node.insert("path".into(), json!(m.route_path));
                    node.insert("component".into(), json!(m.component_path));
                    node.insert("icon".into(), json!(m.icon));
                    node.insert("permission".into(), json!(m.permission_code));
                    node.insert("visible".into(), json!(m.is_visible));
                    node.insert("isCache".into(), json!(m.is_cached));
                    node.insert("sort".into(), json!(m.sort_order));
                    node.insert("status".into(), json!(status_flag(&m.status)));
                    node
                })
                .collect();
            ok(json!(build_tree(flat)))
        }
        Err(e) => err(&format!("menu tree: {e}")),
    }
}


pub(crate) async fn list_menus_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_all_menus(&tenant) {
        Ok(list) => ok(json!(list.iter().map(menu_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("menu list: {e}")),
    }
}


pub(crate) async fn create_menu_handler(
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
    let menu_type = opt_str(&body, "type");
    let parent_id = opt_str(&body, "parentId");
    let route_path = opt_str(&body, "path");
    let component_path = opt_str(&body, "component");
    let icon = opt_str(&body, "icon");
    let permission_code = opt_str(&body, "permission");
    let sort = opt_i64(&body, "sort");
    let is_visible = opt_i64(&body, "visible");
    let status = opt_status(&body).unwrap_or("active");
    match s.iam.create_menu(
        &tenant,
        code,
        name,
        menu_type,
        parent_id,
        route_path,
        component_path,
        icon,
        permission_code,
        sort,
        is_visible,
        status,
    ) {
        Ok(m) => ok(menu_json(&m)),
        Err(e) => err(&format!("menu create: {e}")),
    }
}


pub(crate) async fn get_menu_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_menu(&id) {
        Ok(Some(m)) => ok(menu_json(&m)),
        Ok(None) => err("menu not found"),
        Err(e) => err(&format!("menu detail: {e}")),
    }
}


pub(crate) async fn update_menu_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let parent_id = opt_str(&body, "parentId");
    let route_path = opt_str(&body, "path");
    let component_path = opt_str(&body, "component");
    let icon = opt_str(&body, "icon");
    let permission_code = opt_str(&body, "permission");
    let sort = opt_i64(&body, "sort");
    let is_visible = opt_i64(&body, "visible");
    let status = opt_status(&body);
    match s.iam.update_menu(
        &id,
        name,
        parent_id,
        route_path,
        component_path,
        icon,
        permission_code,
        sort,
        is_visible,
        status,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("menu update: {e}")),
    }
}


pub(crate) async fn delete_menu_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_menu(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("menu delete: {e}")),
    }
}

// ----- 字典类型 DictType -----

