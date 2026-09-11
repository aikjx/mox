//! OA/ERP 集成适配器 API 端点

use crate::integration::model::*;
use crate::integration::connector::ConnectorRegistry;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// 集成适配器状态
pub struct IntegrationState {
    pub registry: Arc<ConnectorRegistry>,
    pub connectors: Arc<RwLock<HashMap<String, ConnectorConfig>>>,
    pub sync_tasks: Arc<RwLock<HashMap<String, SyncTask>>>,
    pub sync_logs: Arc<RwLock<Vec<SyncLogRecord>>>,
}

use tokio::sync::RwLock;

impl IntegrationState {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ConnectorRegistry::new()),
            connectors: Arc::new(RwLock::new(HashMap::new())),
            sync_tasks: Arc::new(RwLock::new(HashMap::new())),
            sync_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for IntegrationState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/integration/connector-types —— 获取支持的连接器类型
pub async fn list_connector_types_handler() -> Response {
    let types = supported_connector_types();
    let result: Vec<serde_json::Value> = types.iter().map(|t| {
        json!({
            "type": t.as_str(),
            "name": t.display_name(),
            "category": t.category(),
        })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/integration/connectors —— 获取连接器列表
pub async fn list_connectors_handler(
    State(state): State<Arc<IntegrationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let connectors = state.connectors.read().await;
    let mut list: Vec<&ConnectorConfig> = connectors.values().collect();

    // 按类型过滤
    if let Some(ct) = params.get("connector_type") {
        list.retain(|c| c.connector_type.as_str() == ct);
    }
    // 按状态过滤
    if let Some(st) = params.get("status") {
        list.retain(|c| format!("{:?}", c.status).to_lowercase() == st.to_lowercase());
    }

    let result: Vec<serde_json::Value> = list.iter().map(|c| {
        json!({
            "connector_id": c.connector_id,
            "name": c.name,
            "connector_type": c.connector_type.as_str(),
            "connector_type_name": c.connector_type.display_name(),
            "status": format!("{:?}", c.status).to_lowercase(),
            "description": c.description,
            "last_test_at": c.last_test_at,
            "last_sync_at": c.last_sync_at,
            "created_at": c.created_at,
            "updated_at": c.updated_at,
        })
    }).collect();

    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/integration/connectors/:id —— 获取连接器详情
pub async fn get_connector_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
) -> Response {
    let connectors = state.connectors.read().await;
    match connectors.get(&id) {
        Some(c) => Json(json!({ "code": 0, "data": c })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "连接器不存在" }))).into_response(),
    }
}

/// POST /api/integration/connectors —— 创建连接器
pub async fn create_connector_handler(
    State(state): State<Arc<IntegrationState>>,
    Json(req): Json<CreateConnectorRequest>,
) -> Response {
    let connector_id = format!("conn_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let config = ConnectorConfig {
        connector_id: connector_id.clone(),
        tenant_id: "default".to_string(),
        name: req.name,
        connector_type: req.connector_type,
        status: ConnectorStatus::Draft,
        connection: req.connection,
        auth: req.auth,
        field_mappings: req.field_mappings.unwrap_or_default(),
        sync_config: req.sync_config.unwrap_or_default(),
        description: req.description,
        created_at: now.clone(),
        updated_at: now,
        last_test_at: None,
        last_sync_at: None,
    };

    state.connectors.write().await.insert(connector_id.clone(), config);

    Json(json!({
        "code": 0,
        "message": "连接器创建成功",
        "data": { "connector_id": connector_id }
    })).into_response()
}

/// PUT /api/integration/connectors/:id —— 更新连接器
pub async fn update_connector_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConnectorRequest>,
) -> Response {
    let mut connectors = state.connectors.write().await;
    match connectors.get_mut(&id) {
        Some(c) => {
            if let Some(name) = req.name { c.name = name; }
            if let Some(status) = req.status { c.status = status; }
            if let Some(connection) = req.connection { c.connection = connection; }
            if let Some(auth) = req.auth { c.auth = auth; }
            if let Some(field_mappings) = req.field_mappings { c.field_mappings = field_mappings; }
            if let Some(sync_config) = req.sync_config { c.sync_config = sync_config; }
            if let Some(description) = req.description { c.description = Some(description); }
            c.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "连接器更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "连接器不存在" }))).into_response(),
    }
}

/// DELETE /api/integration/connectors/:id —— 删除连接器
pub async fn delete_connector_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
) -> Response {
    let mut connectors = state.connectors.write().await;
    if connectors.remove(&id).is_some() {
        Json(json!({ "code": 0, "message": "连接器删除成功" })).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "连接器不存在" }))).into_response()
    }
}

/// POST /api/integration/connectors/:id/test —— 测试连接
pub async fn test_connector_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
) -> Response {
    let connectors = state.connectors.read().await;
    let config = match connectors.get(&id) {
        Some(c) => c.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "连接器不存在" }))).into_response(),
    };
    drop(connectors);

    let connector = state.registry.get(&config.connector_type).await;
    let result = match connector {
        Some(c) => c.test_connection(&config).await,
        None => ConnectorTestResult {
            success: false,
            response_time_ms: 0,
            message: format!("不支持的连接器类型：{}", config.connector_type.display_name()),
            details: None,
        },
    };

    // 更新最后测试时间
    if let Some(c) = state.connectors.write().await.get_mut(&id) {
        c.last_test_at = Some(chrono::Utc::now().to_rfc3339());
        if result.success {
            c.status = ConnectorStatus::Active;
        } else {
            c.status = ConnectorStatus::Error;
        }
    }

    Json(json!({ "code": 0, "data": result })).into_response()
}

/// POST /api/integration/connectors/:id/sync —— 触发同步
pub async fn trigger_sync_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
    Json(req): Json<TriggerSyncRequest>,
) -> Response {
    let connectors = state.connectors.read().await;
    let config = match connectors.get(&id) {
        Some(c) => c.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "连接器不存在" }))).into_response(),
    };
    drop(connectors);

    let task_id = format!("sync_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let task = SyncTask {
        task_id: task_id.clone(),
        connector_id: id.clone(),
        tenant_id: "default".to_string(),
        name: format!("{}-{}", config.name, req.entity),
        entity: req.entity.clone(),
        status: SyncTaskStatus::Running,
        direction: req.direction.unwrap_or(config.sync_config.direction.clone()),
        mode: req.mode.unwrap_or(config.sync_config.mode.clone()),
        trigger_type: "manual".to_string(),
        started_at: Some(now.clone()),
        finished_at: None,
        total_count: 0,
        success_count: 0,
        failed_count: 0,
        skipped_count: 0,
        error_message: None,
        log_path: None,
        created_by: None,
        created_at: now,
    };

    state.sync_tasks.write().await.insert(task_id.clone(), task);

    // 异步执行同步（模拟）
    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Some(t) = state_clone.sync_tasks.write().await.get_mut(&task_id_clone) {
            t.status = SyncTaskStatus::Success;
            t.finished_at = Some(chrono::Utc::now().to_rfc3339());
            t.total_count = 100;
            t.success_count = 98;
            t.failed_count = 2;
            t.skipped_count = 0;
        }
    });

    Json(json!({
        "code": 0,
        "message": "同步任务已启动",
        "data": { "task_id": task_id }
    })).into_response()
}

/// GET /api/integration/sync-tasks —— 获取同步任务列表
pub async fn list_sync_tasks_handler(
    State(state): State<Arc<IntegrationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let tasks = state.sync_tasks.read().await;
    let mut list: Vec<&SyncTask> = tasks.values().collect();

    if let Some(connector_id) = params.get("connector_id") {
        list.retain(|t| t.connector_id == *connector_id);
    }
    if let Some(status) = params.get("status") {
        list.retain(|t| format!("{:?}", t.status).to_lowercase() == status.to_lowercase());
    }

    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/integration/sync-tasks/:id —— 获取同步任务详情
pub async fn get_sync_task_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
) -> Response {
    let tasks = state.sync_tasks.read().await;
    match tasks.get(&id) {
        Some(t) => Json(json!({ "code": 0, "data": t })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "同步任务不存在" }))).into_response(),
    }
}

/// GET /api/integration/sync-tasks/:id/logs —— 获取同步日志
pub async fn get_sync_task_logs_handler(
    State(state): State<Arc<IntegrationState>>,
    Path(id): Path<String>,
) -> Response {
    let logs = state.sync_logs.read().await;
    let task_logs: Vec<&SyncLogRecord> = logs.iter().filter(|l| l.task_id == id).collect();
    Json(json!({ "code": 0, "data": task_logs, "total": task_logs.len() })).into_response()
}

/// 构建集成适配器路由（泛型版本，支持任意状态类型）
pub fn build_integration_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<IntegrationState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        .route("/connector-types", get(list_connector_types_handler))
        .route("/connectors", get(list_connectors_handler).post(create_connector_handler))
        .route("/connectors/:id", get(get_connector_handler).put(update_connector_handler).delete(delete_connector_handler))
        .route("/connectors/:id/test", post(test_connector_handler))
        .route("/connectors/:id/sync", post(trigger_sync_handler))
        .route("/sync-tasks", get(list_sync_tasks_handler))
        .route("/sync-tasks/:id", get(get_sync_task_handler))
        .route("/sync-tasks/:id/logs", get(get_sync_task_logs_handler))
}
