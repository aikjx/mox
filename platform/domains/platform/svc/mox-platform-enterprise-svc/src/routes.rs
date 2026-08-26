//! Axum 路由定义：/health、/auth、/entities/define、/data/*
//!
//! 所有业务路由统一通过 Orchestrator，保证：
//! - 每个写操作都有审计链
//! - 版本计数自增
//! - 指标（成功率/失败率）可观测

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use mox_platform_meta_core::{EnumOption, FieldDef, FieldType};
use mox_platform_orchestrator_core::Orchestrator;

pub fn health_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_ok))
        .route("/health/live", get(health_ok))
        .route("/health/ready", get(health_ok))
}

async fn health_ok() -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok","uptime_ms":0}))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub tenant_code: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
    pub tenant_id: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let tenant = state
        .iam
        .find_tenant_by_code(&req.tenant_code)
        .ok_or_else(|| unauthorized("tenant not found"))?;
    let user = state
        .iam
        .find_user_by_tenant_username(&tenant.id, &req.username)
        .ok_or_else(|| unauthorized("user not found"))?;

    let roles = state.iam.user_roles(&user.id);
    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
    let perms: Vec<String> = roles.iter().flat_map(|r| r.permissions.clone()).collect();

    let secret = std::env::var("AUTH_SECRET").unwrap_or_else(|_| "enterprise-dev-secret-change-me".to_string());
    let token = mox_framework::auth::generate_token(
        &secret,
        &user.id,
        &tenant.id,
        role_codes.clone(),
        perms,
        3600 * 24,
    )
    .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user_id: user.id,
        tenant_id: tenant.id,
        display_name: user.display_name,
        roles: role_codes,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DefineEntityField {
    pub code: String,
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub indexed: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub filterable: bool,
    #[serde(default)]
    pub ui_component: Option<String>,
    #[serde(default)]
    pub options_inline: Option<Vec<DefineEnumOption>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefineEnumOption {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DefineEntityRequest {
    pub tenant_id: Option<String>,
    pub entity_code: String,
    pub entity_name: String,
    pub fields: Vec<DefineEntityField>,
}

#[derive(Debug, Serialize)]
pub struct DefineEntityResponse {
    pub entity_id: String,
    pub slot_map: BTreeMap<String, String>,
}

fn parse_field_type(t: &str) -> FieldType {
    match t.to_lowercase().as_str() {
        "int" | "integer" => FieldType::Int,
        "decimal" | "number" | "float" | "double" => FieldType::Decimal,
        "bool" | "boolean" => FieldType::Boolean,
        "datetime" | "date" | "time" => FieldType::DateTime,
        "enum" | "select" => FieldType::Enum,
        "text" | "longtext" | "richtext" => FieldType::Text,
        "json" | "object" => FieldType::Json,
        _ => FieldType::String,
    }
}

async fn define_entity_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DefineEntityRequest>,
) -> Result<Json<DefineEntityResponse>, (StatusCode, Json<serde_json::Value>)> {
    let fields: Vec<FieldDef> = req
        .fields
        .into_iter()
        .map(|f| FieldDef {
            code: f.code,
            name: f.name,
            r#type: parse_field_type(&f.r#type),
            required: f.required,
            indexed: f.indexed,
            searchable: f.searchable,
            sortable: f.sortable,
            filterable: f.filterable,
            ui_component: f.ui_component,
            options_inline: f.options_inline.map(|opts| {
                opts.into_iter()
                    .map(|o| EnumOption {
                        value: o.value,
                        label: o.label,
                        color: o.color,
                    })
                    .collect()
            }),
        })
        .collect();

    let (entity_id, slot_map) = Orchestrator::execute_with_tokio(move || {
        state
            .meta
            .define_entity(req.tenant_id, req.entity_code, req.entity_name, fields)
    })
    .await
    .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(DefineEntityResponse { entity_id, slot_map }))
}

#[derive(Debug, Deserialize)]
pub struct BizDataRequest {
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(flatten)]
    pub data: BTreeMap<String, serde_json::Value>,
}

async fn create_data_handler(
    State(state): State<Arc<AppState>>,
    Path(entity_code): Path<String>,
    Json(req): Json<BizDataRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let actor = req.actor.clone().unwrap_or_else(|| "sys_actor".to_string());
    let entity_code_cloned = entity_code.clone();
    let tenant_id = req.tenant_id.clone();
    let data = req.data.clone();
    let rec = Orchestrator::execute_with_tokio(move || {
        orch.create_sync(&entity_code_cloned, tenant_id, data, &actor)
    })
    .await
    .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "biz_id": rec.biz_id,
        "version": rec.version,
        "data": rec.data,
        "entity_code": entity_code
    })))
}

async fn update_data_handler(
    State(state): State<Arc<AppState>>,
    Path((entity_code, biz_id)): Path<(String, String)>,
    Json(req): Json<BizDataRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let actor = req.actor.clone().unwrap_or_else(|| "sys_actor".to_string());
    let biz_id_cloned = biz_id.clone();
    let patch = req.data.clone();
    let rec = Orchestrator::execute_with_tokio(move || orch.update_sync(&biz_id_cloned, patch, &actor))
        .await
        .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "biz_id": biz_id,
        "version": rec.version,
        "data": rec.data,
        "entity_code": entity_code
    })))
}

async fn delete_data_handler(
    State(state): State<Arc<AppState>>,
    Path((entity_code, biz_id)): Path<(String, String)>,
    Json(req): Json<Option<BizDataRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let actor = req
        .as_ref()
        .and_then(|r| r.actor.clone())
        .unwrap_or_else(|| "sys_actor".to_string());
    let biz_id_cloned = biz_id.clone();
    Orchestrator::execute_with_tokio(move || orch.delete_sync(&biz_id_cloned, &actor))
        .await
        .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "biz_id": biz_id,
        "entity_code": entity_code
    })))
}

async fn get_data_handler(
    State(state): State<Arc<AppState>>,
    Path((entity_code, biz_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let biz_id_cloned = biz_id.clone();
    let rec = Orchestrator::execute_with_tokio(move || orch.get_sync(&biz_id_cloned))
        .await
        .map_err(|e| internal_err(e.to_string()))?
        .ok_or_else(|| not_found("biz not found"))?;

    let version_count = state.orch.version_count_sync(&biz_id);
    let audit_chain = state.orch.audit_chain_sync(&biz_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "biz_id": biz_id,
        "version": rec.version,
        "version_count": version_count,
        "data": rec.data,
        "entity_code": entity_code,
        "audit_chain": audit_chain
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub tenant_id: Option<String>,
}

async fn list_data_handler(
    State(state): State<Arc<AppState>>,
    Path(entity_code): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let entity_code_cloned = entity_code.clone();
    let tid = q.tenant_id.clone();
    let items = Orchestrator::execute_with_tokio(move || {
        orch.list_sync(&entity_code_cloned, tid.as_deref())
    })
    .await
    .map_err(|e| internal_err(e.to_string()))?;

    let metrics = state.orch.metrics.snapshot();
    let fail_rate = state.orch.metrics.fail_rate();

    Ok(Json(serde_json::json!({
        "success": true,
        "total": items.len(),
        "items": items,
        "entity_code": entity_code,
        "metrics": metrics,
        "failRate": fail_rate
    })))
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login_handler))
        .route("/entities/define", post(define_entity_handler))
        .route("/data/:entity_code/create", post(create_data_handler))
        .route("/data/:entity_code/update/:biz_id", post(update_data_handler))
        .route("/data/:entity_code/delete/:biz_id", post(delete_data_handler))
        .route("/data/:entity_code/get/:biz_id", get(get_data_handler))
        .route("/data/list/:entity_code", get(list_data_handler))
}

fn unauthorized(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error":"unauthorized","message":msg})),
    )
}

fn not_found(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error":"not_found","message":msg})),
    )
}

fn internal_err(msg: String) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error":"internal","message":msg})),
    )
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new().nest("/api/enterprise/v1", api_routes());
    Router::new()
        .merge(health_routes())
        .merge(api)
        .with_state(state)
}
