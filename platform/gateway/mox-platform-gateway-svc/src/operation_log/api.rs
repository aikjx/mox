//! 操作日志查询 API 端点

use crate::operation_log::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 操作日志状态
pub struct OperationLogState {
    pub operation_logs: Arc<RwLock<Vec<OperationLog>>>,
    pub login_logs: Arc<RwLock<Vec<LoginLog>>>,
}

impl OperationLogState {
    pub fn new() -> Self {
        Self {
            operation_logs: Arc::new(RwLock::new(sample_operation_logs())),
            login_logs: Arc::new(RwLock::new(sample_login_logs())),
        }
    }
}

impl Default for OperationLogState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/operation-logs —— 查询操作日志
pub async fn list_operation_logs_handler(
    State(state): State<Arc<OperationLogState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let logs = state.operation_logs.read().await;
    let mut list: Vec<&OperationLog> = logs.iter().collect();

    // 按用户过滤
    if let Some(user_id) = params.get("user_id") {
        list.retain(|l| l.user_id == *user_id);
    }
    // 按模块过滤
    if let Some(module) = params.get("module") {
        list.retain(|l| l.module == *module);
    }
    // 按操作类型过滤
    if let Some(op_type) = params.get("operation_type") {
        list.retain(|l| l.operation_type == *op_type);
    }
    // 按结果过滤
    if let Some(result) = params.get("result") {
        list.retain(|l| l.result == *result);
    }
    // 按租户过滤
    if let Some(tenant_id) = params.get("tenant_id") {
        list.retain(|l| l.tenant_id.as_deref() == Some(tenant_id.as_str()));
    }
    // 关键词搜索
    if let Some(keyword) = params.get("keyword") {
        list.retain(|l| {
            l.description.contains(keyword) || l.username.contains(keyword)
        });
    }

    // 按时间倒序
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/operation-logs/:log_id —— 获取操作日志详情
pub async fn get_operation_log_handler(
    State(state): State<Arc<OperationLogState>>,
    Path(log_id): Path<String>,
) -> Response {
    let logs = state.operation_logs.read().await;
    match logs.iter().find(|l| l.log_id == log_id) {
        Some(log) => success(log),
        None => not_found("操作日志不存在"),
    }
}

/// GET /api/enterprise/operation-logs/stats —— 操作日志统计
pub async fn operation_log_stats_handler(
    State(state): State<Arc<OperationLogState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let logs = state.operation_logs.read().await;
    let mut filtered: Vec<&OperationLog> = logs.iter().collect();

    if let Some(tenant_id) = params.get("tenant_id") {
        filtered.retain(|l| l.tenant_id.as_deref() == Some(tenant_id.as_str()));
    }

    let mut stats = OperationLogStats::default();
    stats.total_operations = filtered.len() as i64;
    stats.success_count = filtered.iter().filter(|l| l.result == "success").count() as i64;
    stats.failure_count = filtered.iter().filter(|l| l.result == "failure").count() as i64;
    stats.success_rate = if stats.total_operations > 0 {
        stats.success_count as f64 / stats.total_operations as f64
    } else {
        0.0
    };

    let durations: Vec<i64> = filtered.iter().filter_map(|l| l.duration_ms).collect();
    stats.avg_duration_ms = if !durations.is_empty() {
        durations.iter().sum::<i64>() as f64 / durations.len() as f64
    } else {
        0.0
    };

    for log in &filtered {
        *stats.by_module.entry(log.module.clone()).or_insert(0) += 1;
        *stats.by_operation_type.entry(log.operation_type.clone()).or_insert(0) += 1;
    }

    success(stats)
}

/// GET /api/enterprise/operation-logs/login —— 查询登录日志
pub async fn list_login_logs_handler(
    State(state): State<Arc<OperationLogState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let logs = state.login_logs.read().await;
    let mut list: Vec<&LoginLog> = logs.iter().collect();

    if let Some(username) = params.get("username") {
        list.retain(|l| l.username == *username);
    }
    if let Some(result) = params.get("result") {
        list.retain(|l| l.result == *result);
    }
    if let Some(login_type) = params.get("login_type") {
        list.retain(|l| l.login_type == *login_type);
    }

    list.sort_by(|a, b| b.login_at.cmp(&a.login_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/operation-logs/modules —— 获取所有操作模块列表
pub async fn list_operation_modules_handler(
    State(state): State<Arc<OperationLogState>>,
) -> Response {
    let logs = state.operation_logs.read().await;
    let mut modules: Vec<String> = logs.iter().map(|l| l.module.clone()).collect();
    modules.sort();
    modules.dedup();
    success(modules)
}

/// GET /api/enterprise/operation-logs/operation-types —— 获取所有操作类型列表
pub async fn list_operation_types_handler() -> Response {
    let types = vec![
        "CREATE", "UPDATE", "DELETE", "QUERY", "EXPORT", "IMPORT",
        "LOGIN", "LOGOUT", "UPLOAD", "DOWNLOAD", "APPROVE", "REJECT",
        "CONFIG_CHANGE", "PERMISSION_CHANGE", "SYSTEM",
    ];
    success(types)
}

/// 构建操作日志路由（泛型版本）
pub fn build_operation_log_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<OperationLogState>: axum::extract::FromRef<S>,
{
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(list_operation_logs_handler))
        .route("/stats", get(operation_log_stats_handler))
        .route("/login", get(list_login_logs_handler))
        .route("/modules", get(list_operation_modules_handler))
        .route("/operation-types", get(list_operation_types_handler))
        .route("/:log_id", get(get_operation_log_handler))
}
