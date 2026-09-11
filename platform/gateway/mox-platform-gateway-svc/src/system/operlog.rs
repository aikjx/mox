// ====================================================================
// system/operlog.rs — 系统管理子模块
// ====================================================================

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, err, q_str, resolve_tenant, oper_log_json};
use axum::extract::{Path, Query, State};
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};
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


pub(crate) async fn export_oper_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let list = match s.iam.list_oper_logs(&tenant) {
        Ok(l) => l,
        Err(e) => return err(&format!("oper log list: {e}")),
    };

    // 生成 CSV 内容
    let mut csv = String::new();
    csv.push_str("操作ID,系统模块,业务类型,方法名称,请求方式,操作人员,部门名称,请求URL,操作IP,操作地点,操作状态,错误消息,操作时间,消耗时间(ms)\n");
    for log in &list {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(Some(&log.oper_id)),
            csv_escape(log.title.as_deref()),
            log.business_type.unwrap_or(0),
            csv_escape(log.method.as_deref()),
            csv_escape(log.request_method.as_deref()),
            csv_escape(log.oper_name.as_deref()),
            csv_escape(log.dept_name.as_deref()),
            csv_escape(log.oper_url.as_deref()),
            csv_escape(log.oper_ip.as_deref()),
            csv_escape(log.oper_location.as_deref()),
            log.status.unwrap_or(0),
            csv_escape(log.error_msg.as_deref()),
            csv_escape(Some(&log.oper_time)),
            log.cost_time.unwrap_or(0),
        ));
    }

    let filename = format!("oper_logs_{}.csv", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    ok(json!({
        "exported": true,
        "filename": filename,
        "total": list.len(),
        "content": csv,
        "mimeType": "text/csv;charset=utf-8",
    }))
}

/// CSV 字段转义：包含逗号、引号、换行的字段用双引号包裹，内部双引号转义为两个双引号
fn csv_escape(s: Option<&str>) -> String {
    let s = s.unwrap_or("");
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ----- 登录日志 LoginLog -----

