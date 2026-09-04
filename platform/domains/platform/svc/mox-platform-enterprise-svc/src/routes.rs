// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use mox_platform_meta_core::{codegen, EntityWithFields, EnumOption, FieldDef, FieldType, MetaRepository};

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
    let iam = state.iam.clone();
    let tenant_code = req.tenant_code.clone();
    let username = req.username.clone();

    let (tenant, user, roles) = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
        let tenant = iam
            .find_tenant_by_code(&tenant_code)
            .ok_or_else(|| anyhow::anyhow!("tenant not found"))?;
        let user = iam
            .find_user_by_tenant_username(&tenant.tenant_id, &username)
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;
        let roles = iam.user_roles(&user.user_id);
        Ok((tenant, user, roles))
    })
    .await
    .map_err(|e| internal_err(format!("join error: {}", e)))?
    .map_err(|e| unauthorized(&e.to_string()))?;

    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
    let perms: Vec<String> = roles.iter().flat_map(|r| r.permissions.clone()).collect();

    let secret = std::env::var("AUTH_SECRET")
        .unwrap_or_else(|_| "enterprise-dev-secret-change-me".to_string());
    let token = crate::auth::generate_token(
        &secret,
        &user.user_id,
        &tenant.tenant_id,
        role_codes.clone(),
        perms,
        3600 * 24,
    )
    .map_err(|e| internal_err(e.to_string()))?;

    let display_name = user
        .nickname
        .clone()
        .or(user.real_name.clone())
        .unwrap_or_else(|| user.username.clone());

    Ok(Json(LoginResponse {
        token,
        user_id: user.user_id,
        tenant_id: tenant.tenant_id,
        display_name,
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
    pub slot_map: HashMap<String, String>,
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

    let meta = state.meta.clone();
    let tenant_id = req.tenant_id.clone();
    let entity_code = req.entity_code.clone();
    let entity_name = req.entity_name.clone();

    let (entity_id, slot_map) = tokio::task::spawn_blocking(move || {
        meta.define_entity(tenant_id, entity_code, entity_name, fields)
    })
    .await
    .map_err(|e| internal_err(format!("join error: {}", e)))?
    .map_err(|e| internal_err(e.to_string()))?;

    Ok(Json(DefineEntityResponse {
        entity_id,
        slot_map,
    }))
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
    let data = if req.data.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            req.data.into_iter().collect()
        ))
    };
    let rec = tokio::task::spawn_blocking(move || {
        orch.create_sync(&entity_code_cloned, tenant_id, data, &actor)
    })
    .await
    .map_err(|e| internal_err(format!("join error: {}", e)))?
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
    let patch = if req.data.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            req.data.into_iter().collect()
        ))
    };
    let rec = tokio::task::spawn_blocking(move || orch.update_sync(&biz_id_cloned, patch, &actor))
        .await
        .map_err(|e| internal_err(format!("join error: {}", e)))?
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
    tokio::task::spawn_blocking(move || orch.delete_sync(&biz_id_cloned, &actor))
        .await
        .map_err(|e| internal_err(format!("join error: {}", e)))?
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
    let rec = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
        let rec = orch.get_sync(&biz_id_cloned)?;
        Ok(rec)
    })
    .await
    .map_err(|e| internal_err(format!("join error: {}", e)))?
    .map_err(|e| internal_err(e.to_string()))?
    .ok_or_else(|| not_found("biz not found"))?;

    let orch2 = state.orch.clone();
    let biz_id_for_count = biz_id.clone();
    let (version_count, audit_chain) = tokio::task::spawn_blocking(move || {
        let vc = orch2.version_count_sync(&biz_id_for_count);
        let ac = orch2.audit_chain_sync(&biz_id_for_count);
        (vc, ac)
    })
    .await
    .map_err(|e| internal_err(format!("join error: {}", e)))?;

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

async fn list_data_handler(
    State(state): State<Arc<AppState>>,
    Path(entity_code): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let orch = state.orch.clone();
    let entity_code_cloned = entity_code.clone();
    let tid = q.get("tenant_id").cloned();
    let items =
        tokio::task::spawn_blocking(move || orch.list_sync(&entity_code_cloned, tid.as_deref()))
            .await
            .map_err(|e| internal_err(format!("join error: {}", e)))?
            .map_err(|e| internal_err(e.to_string()))?;

    let metrics_total = state.orch.metrics.total();
    let metrics_failed = state.orch.metrics.failed();
    let fail_rate = state.orch.metrics.fail_rate();

    Ok(Json(serde_json::json!({
        "success": true,
        "total": items.len(),
        "items": items,
        "entity_code": entity_code,
        "metrics": {
            "total_calls": metrics_total,
            "failed_calls": metrics_failed,
        },
        "failRate": fail_rate
    })))
}

// ─── Codegen（meta.codegen 能力接线：元数据 → 确定性出码）───

/// codegen 请求：实体元数据 + 模板编号（TPL-01~06，缺省 TPL-01；TPL-03 需 detail_entity_code）。
#[derive(Debug, Deserialize)]
pub struct CodegenRequest {
    pub tenant_id: Option<String>,
    pub entity_code: String,
    #[serde(default)]
    pub detail_entity_code: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
}

/// codegen 响应：产物集合（相对路径 → 文件内容）。
#[derive(Debug, Serialize)]
pub struct CodegenResponse {
    pub entity_code: String,
    pub template: String,
    pub artifacts: BTreeMap<String, String>,
}

/// 解析模板编号（委托 meta-core `codegen::parse_template`，单一来源）。
fn parse_template(raw: &str) -> Result<codegen::CodegenTemplate, String> {
    codegen::parse_template(raw)
}

/// svc 层 codegen 编排：取实体元数据 → 调 meta-core codegen → 产物（纯同步，handler 侧包 spawn_blocking）。
///
/// # Errors
/// 模板不支持 / 实体不存在 / 元数据非法 / TPL-03 缺明细实体编码，均以 Err(说明) 返回。
pub fn run_codegen(meta: &MetaRepository, req: &CodegenRequest) -> Result<CodegenResponse, String> {
    let tenant = req
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let load = |code: &str| -> Result<EntityWithFields, String> {
        meta.get_entity(&tenant, code)
            .map_err(|e| format!("meta get_entity `{code}` failed: {e}"))?
            .ok_or_else(|| format!("entity `{code}` not found under tenant `{tenant}`"))
    };
    let raw = req.template.as_deref().unwrap_or("TPL-01");
    let out = if raw.trim().eq_ignore_ascii_case("TPL-03") {
        let master = load(&req.entity_code)?;
        let detail_code = req
            .detail_entity_code
            .as_deref()
            .ok_or_else(|| "TPL-03 requires `detail_entity_code`".to_string())?;
        let detail = load(detail_code)?;
        codegen::generate_master_detail(&master, &detail).map_err(|e| e.to_string())?
    } else {
        let tpl = parse_template(raw)?;
        let entity = load(&req.entity_code)?;
        codegen::generate(&entity, tpl).map_err(|e| e.to_string())?
    };
    Ok(CodegenResponse {
        entity_code: out.entity_code,
        template: out.template,
        artifacts: out.artifacts,
    })
}

async fn codegen_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CodegenRequest>,
) -> Result<Json<CodegenResponse>, (StatusCode, Json<serde_json::Value>)> {
    let meta = state.meta.clone();
    let result =
        tokio::task::spawn_blocking(move || run_codegen(&meta, &req))
            .await
            .map_err(|e| internal_err(format!("join error: {}", e)))?;
    result.map(Json).map_err(|msg| bad_request(&msg))
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login_handler))
        .route("/entities/define", post(define_entity_handler))
        .route("/entities/codegen", post(codegen_handler))
        .route("/data/:entity_code/create", post(create_data_handler))
        .route(
            "/data/:entity_code/update/:biz_id",
            post(update_data_handler),
        )
        .route(
            "/data/:entity_code/delete/:biz_id",
            post(delete_data_handler),
        )
        .route("/data/:entity_code/get/:biz_id", get(get_data_handler))
        .route("/data/:entity_code/list", get(list_data_handler))
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

fn bad_request(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error":"bad_request","message":msg})),
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
