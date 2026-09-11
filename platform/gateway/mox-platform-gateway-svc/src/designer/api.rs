//! 低代码设计器 API 端点
//!
//! 包含：审批流程设计器 / 表单设计器 / 报表设计器 的统一 API

use crate::designer::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 设计器状态
pub struct DesignerState {
    pub process_definitions: Arc<RwLock<HashMap<String, ProcessDefinition>>>,
    pub form_definitions: Arc<RwLock<HashMap<String, FormDefinition>>>,
    pub form_instances: Arc<RwLock<HashMap<String, FormInstance>>>,
    pub report_definitions: Arc<RwLock<HashMap<String, ReportDefinition>>>,
}

impl DesignerState {
    pub fn new() -> Self {
        Self {
            process_definitions: Arc::new(RwLock::new(HashMap::new())),
            form_definitions: Arc::new(RwLock::new(HashMap::new())),
            form_instances: Arc::new(RwLock::new(HashMap::new())),
            report_definitions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for DesignerState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 审批流程设计器 API ====================

/// GET /api/designer/process/templates —— 获取流程模板列表
pub async fn list_process_templates_handler() -> Response {
    let templates = builtin_templates();
    let result: Vec<serde_json::Value> = templates.iter().map(|t| {
        json!({
            "template_id": t.template_id,
            "name": t.name,
            "category": t.category,
            "description": t.description,
            "node_count": t.nodes.len(),
            "edge_count": t.edges.len(),
        })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/designer/process/templates/:id —— 获取流程模板详情
pub async fn get_process_template_handler(Path(id): Path<String>) -> Response {
    let templates = builtin_templates();
    match templates.iter().find(|t| t.template_id == id) {
        Some(t) => Json(json!({ "code": 0, "data": t })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "模板不存在" }))).into_response(),
    }
}

/// GET /api/designer/process/definitions —— 获取流程定义列表
pub async fn list_process_definitions_handler(
    State(state): State<Arc<DesignerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let defs = state.process_definitions.read().await;
    let mut list: Vec<&ProcessDefinition> = defs.values().collect();
    if let Some(cat) = params.get("category") {
        list.retain(|d| d.category == *cat);
    }
    if let Some(status) = params.get("status") {
        list.retain(|d| d.status == *status);
    }
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/designer/process/definitions/:id —— 获取流程定义详情
pub async fn get_process_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let defs = state.process_definitions.read().await;
    match defs.get(&id) {
        Some(d) => Json(json!({ "code": 0, "data": d })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "流程定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/process/definitions —— 创建流程定义
pub async fn create_process_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Json(req): Json<CreateProcessDefinitionRequest>,
) -> Response {
    let process_id = format!("proc_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let def = ProcessDefinition {
        process_id: process_id.clone(),
        tenant_id: "default".to_string(),
        name: req.name,
        code: req.code,
        category: req.category,
        description: req.description,
        version: 1,
        status: "draft".to_string(),
        nodes: req.nodes,
        edges: req.edges,
        form_id: req.form_id,
        initiator_config: req.initiator_config.unwrap_or_default(),
        notification_config: req.notification_config.unwrap_or_default(),
        created_by: None,
        created_at: now.clone(),
        updated_at: now,
        published_at: None,
    };
    state.process_definitions.write().await.insert(process_id.clone(), def);
    Json(json!({ "code": 0, "message": "流程定义创建成功", "data": { "process_id": process_id } })).into_response()
}

/// PUT /api/designer/process/definitions/:id —— 更新流程定义
pub async fn update_process_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProcessDefinitionRequest>,
) -> Response {
    let mut defs = state.process_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            if let Some(name) = req.name { d.name = name; }
            if let Some(cat) = req.category { d.category = cat; }
            if let Some(desc) = req.description { d.description = Some(desc); }
            if let Some(nodes) = req.nodes { d.nodes = nodes; }
            if let Some(edges) = req.edges { d.edges = edges; }
            if let Some(form_id) = req.form_id { d.form_id = Some(form_id); }
            if let Some(ic) = req.initiator_config { d.initiator_config = ic; }
            if let Some(nc) = req.notification_config { d.notification_config = nc; }
            d.version += 1;
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "流程定义更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "流程定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/process/definitions/:id/publish —— 发布流程定义
pub async fn publish_process_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut defs = state.process_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            d.status = "published".to_string();
            d.published_at = Some(chrono::Utc::now().to_rfc3339());
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "流程定义发布成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "流程定义不存在" }))).into_response(),
    }
}

/// DELETE /api/designer/process/definitions/:id —— 删除流程定义
pub async fn delete_process_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut defs = state.process_definitions.write().await;
    if defs.remove(&id).is_some() {
        Json(json!({ "code": 0, "message": "流程定义删除成功" })).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "流程定义不存在" }))).into_response()
    }
}

// ==================== 表单设计器 API ====================

/// GET /api/designer/form/field-types —— 获取支持的字段类型
pub async fn list_form_field_types_handler() -> Response {
    let types = supported_field_types();
    let result: Vec<serde_json::Value> = types.iter().map(|t| {
        json!({ "type": t.as_str(), "category": t.category() })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/designer/form/definitions —— 获取表单定义列表
pub async fn list_form_definitions_handler(
    State(state): State<Arc<DesignerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let defs = state.form_definitions.read().await;
    let mut list: Vec<&FormDefinition> = defs.values().collect();
    if let Some(cat) = params.get("category") { list.retain(|d| d.category == *cat); }
    if let Some(status) = params.get("status") { list.retain(|d| d.status == *status); }
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/designer/form/definitions/:id —— 获取表单定义详情
pub async fn get_form_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let defs = state.form_definitions.read().await;
    match defs.get(&id) {
        Some(d) => Json(json!({ "code": 0, "data": d })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "表单定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/form/definitions —— 创建表单定义
pub async fn create_form_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Json(req): Json<CreateFormDefinitionRequest>,
) -> Response {
    let form_id = format!("form_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let def = FormDefinition {
        form_id: form_id.clone(),
        tenant_id: "default".to_string(),
        name: req.name,
        code: req.code,
        category: req.category,
        description: req.description,
        version: 1,
        status: "draft".to_string(),
        fields: req.fields,
        layout: req.layout.unwrap_or_default(),
        linkages: req.linkages.unwrap_or_default(),
        validations: req.validations.unwrap_or_default(),
        process_id: req.process_id,
        created_by: None,
        created_at: now.clone(),
        updated_at: now,
        published_at: None,
    };
    state.form_definitions.write().await.insert(form_id.clone(), def);
    Json(json!({ "code": 0, "message": "表单定义创建成功", "data": { "form_id": form_id } })).into_response()
}

/// PUT /api/designer/form/definitions/:id —— 更新表单定义
pub async fn update_form_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateFormDefinitionRequest>,
) -> Response {
    let mut defs = state.form_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            if let Some(name) = req.name { d.name = name; }
            if let Some(cat) = req.category { d.category = cat; }
            if let Some(desc) = req.description { d.description = Some(desc); }
            if let Some(fields) = req.fields { d.fields = fields; }
            if let Some(layout) = req.layout { d.layout = layout; }
            if let Some(linkages) = req.linkages { d.linkages = linkages; }
            if let Some(validations) = req.validations { d.validations = validations; }
            if let Some(pid) = req.process_id { d.process_id = Some(pid); }
            d.version += 1;
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "表单定义更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "表单定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/form/definitions/:id/publish —— 发布表单定义
pub async fn publish_form_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut defs = state.form_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            d.status = "published".to_string();
            d.published_at = Some(chrono::Utc::now().to_rfc3339());
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "表单定义发布成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "表单定义不存在" }))).into_response(),
    }
}

// ==================== 报表设计器 API ====================

/// GET /api/designer/report/component-types —— 获取支持的组件类型
pub async fn list_report_component_types_handler() -> Response {
    let types = supported_component_types();
    let result: Vec<serde_json::Value> = types.iter().map(|t| {
        json!({ "type": t.as_str(), "category": t.category() })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/designer/report/definitions —— 获取报表定义列表
pub async fn list_report_definitions_handler(
    State(state): State<Arc<DesignerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let defs = state.report_definitions.read().await;
    let mut list: Vec<&ReportDefinition> = defs.values().collect();
    if let Some(cat) = params.get("category") { list.retain(|d| d.category == *cat); }
    if let Some(rt) = params.get("report_type") { list.retain(|d| d.report_type == *rt); }
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/designer/report/definitions/:id —— 获取报表定义详情
pub async fn get_report_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let defs = state.report_definitions.read().await;
    match defs.get(&id) {
        Some(d) => Json(json!({ "code": 0, "data": d })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "报表定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/report/definitions —— 创建报表定义
pub async fn create_report_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Json(req): Json<CreateReportDefinitionRequest>,
) -> Response {
    let report_id = format!("report_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let def = ReportDefinition {
        report_id: report_id.clone(),
        tenant_id: "default".to_string(),
        name: req.name,
        code: req.code,
        category: req.category,
        description: req.description,
        report_type: req.report_type,
        status: "draft".to_string(),
        version: 1,
        data_sources: req.data_sources,
        datasets: req.datasets,
        components: req.components,
        layout: req.layout.unwrap_or_default(),
        filters: req.filters.unwrap_or_default(),
        permission_config: req.permission_config.unwrap_or_default(),
        export_config: req.export_config.unwrap_or_default(),
        refresh_config: req.refresh_config.unwrap_or_default(),
        created_by: None,
        created_at: now.clone(),
        updated_at: now,
        published_at: None,
    };
    state.report_definitions.write().await.insert(report_id.clone(), def);
    Json(json!({ "code": 0, "message": "报表定义创建成功", "data": { "report_id": report_id } })).into_response()
}

/// PUT /api/designer/report/definitions/:id —— 更新报表定义
pub async fn update_report_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateReportDefinitionRequest>,
) -> Response {
    let mut defs = state.report_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            if let Some(name) = req.name { d.name = name; }
            if let Some(cat) = req.category { d.category = cat; }
            if let Some(desc) = req.description { d.description = Some(desc); }
            if let Some(ds) = req.data_sources { d.data_sources = ds; }
            if let Some(dt) = req.datasets { d.datasets = dt; }
            if let Some(comp) = req.components { d.components = comp; }
            if let Some(layout) = req.layout { d.layout = layout; }
            if let Some(filters) = req.filters { d.filters = filters; }
            if let Some(pc) = req.permission_config { d.permission_config = pc; }
            if let Some(ec) = req.export_config { d.export_config = ec; }
            if let Some(rc) = req.refresh_config { d.refresh_config = rc; }
            d.version += 1;
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "报表定义更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "报表定义不存在" }))).into_response(),
    }
}

/// POST /api/designer/report/definitions/:id/publish —— 发布报表定义
pub async fn publish_report_definition_handler(
    State(state): State<Arc<DesignerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut defs = state.report_definitions.write().await;
    match defs.get_mut(&id) {
        Some(d) => {
            d.status = "published".to_string();
            d.published_at = Some(chrono::Utc::now().to_rfc3339());
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "报表定义发布成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "报表定义不存在" }))).into_response(),
    }
}

/// 构建设计器路由
pub fn build_designer_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<DesignerState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        // 审批流程设计器
        .route("/process/templates", get(list_process_templates_handler))
        .route("/process/templates/:id", get(get_process_template_handler))
        .route("/process/definitions", get(list_process_definitions_handler).post(create_process_definition_handler))
        .route("/process/definitions/:id", get(get_process_definition_handler).put(update_process_definition_handler).delete(delete_process_definition_handler))
        .route("/process/definitions/:id/publish", post(publish_process_definition_handler))
        // 表单设计器
        .route("/form/field-types", get(list_form_field_types_handler))
        .route("/form/definitions", get(list_form_definitions_handler).post(create_form_definition_handler))
        .route("/form/definitions/:id", get(get_form_definition_handler).put(update_form_definition_handler))
        .route("/form/definitions/:id/publish", post(publish_form_definition_handler))
        // 报表设计器
        .route("/report/component-types", get(list_report_component_types_handler))
        .route("/report/definitions", get(list_report_definitions_handler).post(create_report_definition_handler))
        .route("/report/definitions/:id", get(get_report_definition_handler).put(update_report_definition_handler))
        .route("/report/definitions/:id/publish", post(publish_report_definition_handler))
}
