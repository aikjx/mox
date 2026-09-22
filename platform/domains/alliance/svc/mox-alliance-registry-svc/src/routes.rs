// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 路由定义
//!
//! 两类路由并存：
//! - `/api/v1/experts*`：静态专家目录 CRUD（旧版，向后兼容）
//! - `/api/registry/experts*`：**应用级专家实例注册中心**（核心：注册/发现/心跳/注销）
//! - `/api/registry/health`：注册中心自身健康
//! - `/health`：进程存活探针

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        CreateExpertRequest, HeartbeatRequest, InstanceQuery, InstanceStatus,
        RegisteredInstance, UpdateExpertRequest,
    },
};

/// 创建路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── 旧版静态专家目录 ──
        .route("/health", get(legacy_health))
        .route("/api/v1/experts", get(list_experts).post(create_expert))
        .route(
            "/api/v1/experts/:id",
            get(get_expert).put(update_expert).delete(delete_expert),
        )
        // ── 应用级专家实例注册中心 ──
        .route("/api/registry/health", get(registry_health))
        .route(
            "/api/registry/experts",
            get(list_registrations).post(register_expert),
        )
        .route(
            "/api/registry/experts/:id",
            get(get_registration).delete(deregister_expert),
        )
        .route(
            "/api/registry/experts/:id/heartbeat",
            post(heartbeat_expert),
        )
        // 一键传输加密（MOX_API_CRYPTO=sm4）：统一信封 data gzip+SM4-GCM，详见 mox-api-crypto
        .layer(axum::middleware::from_fn(
            mox_api_crypto::middleware::crypto_middleware,
        ))
        .with_state(state)
}

// ─── 旧版静态专家目录 ─────────────────────────────────────────────────────

/// 进程存活探针
async fn legacy_health() -> &'static str {
    "ok"
}

/// GET /api/v1/experts — 专家列表
async fn list_experts(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let experts = state
        .dir_store
        .list()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let domain_filter = params.get("domain").cloned();
    let search = params.get("search").cloned();

    let mut filtered: Vec<crate::models::Expert> = experts
        .into_iter()
        .filter(|e| e.enabled)
        .filter(|e| {
            if let Some(df) = &domain_filter {
                return e.domains.iter().any(|d| d.contains(df));
            }
            true
        })
        .filter(|e| {
            if let Some(q) = &search {
                return e.name.contains(q) || e.bio.contains(q);
            }
            true
        })
        .collect();

    filtered.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));

    let total = filtered.len();
    Ok(Json(serde_json::json!({
        "experts": filtered,
        "total": total,
    })))
}

/// GET /api/v1/experts/:id — 专家详情
async fn get_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::models::Expert>, StatusCode> {
    state
        .dir_store
        .get(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// POST /api/v1/experts — 创建专家
async fn create_expert(
    State(state): State<AppState>,
    Json(req): Json<CreateExpertRequest>,
) -> Result<Json<crate::models::Expert>, StatusCode> {
    let mut expert = crate::models::Expert::new(req.name);
    if let Some(title) = req.title {
        expert.title = title;
    }
    if let Some(org) = req.organization {
        expert.organization = org;
    }
    if let Some(domains) = req.domains {
        expert.domains = domains;
    }
    if let Some(skills) = req.skills {
        expert.skills = skills;
    }
    if let Some(bio) = req.bio {
        expert.bio = bio;
    }

    state
        .dir_store
        .create(&expert)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(expert))
}

/// PUT /api/v1/experts/:id — 更新专家
async fn update_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateExpertRequest>,
) -> Result<Json<crate::models::Expert>, StatusCode> {
    let mut expert = state
        .dir_store
        .get(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = req.name {
        expert.name = name;
    }
    if let Some(title) = req.title {
        expert.title = title;
    }
    if let Some(org) = req.organization {
        expert.organization = org;
    }
    if let Some(domains) = req.domains {
        expert.domains = domains;
    }
    if let Some(skills) = req.skills {
        expert.skills = skills;
    }
    if let Some(bio) = req.bio {
        expert.bio = bio;
    }
    if let Some(enabled) = req.enabled {
        expert.enabled = enabled;
    }

    state
        .dir_store
        .update(&expert)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(expert))
}

/// DELETE /api/v1/experts/:id — 删除专家
async fn delete_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .dir_store
        .delete(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── 应用级专家实例注册中心 ──────────────────────────────────────────────

/// GET /api/registry/health — 注册中心自身健康
async fn registry_health(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let all = state.registry.count();
    let active = state
        .registry
        .list(&InstanceQuery {
            status: Some("active".to_string()),
            ..Default::default()
        })
        .len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "service": "mox-alliance-registry",
            "instances_total": all,
            "instances_active": active,
        })),
    )
}

/// POST /api/registry/experts — 注册专家实例
async fn register_expert(
    State(state): State<AppState>,
    Json(req): Json<crate::models::RegisterRequest>,
) -> Result<Json<RegisteredInstance>, StatusCode> {
    // 基本校验：端点必填（RegisterRequest 已要求 name/endpoint）
    if req.endpoint.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let now = chrono::Utc::now();
    let inst = RegisteredInstance {
        id: req.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: req.name,
        version: req.version,
        endpoint: req.endpoint,
        health_check_url: req.health_check_url,
        capabilities: req.capabilities,
        domain: req.domain,
        weight: req.weight,
        load_current: req.load_current,
        load_capacity: req.load_capacity,
        status: InstanceStatus::Active,
        registered_at: now,
        last_heartbeat_at: now,
        lease_seconds: req.lease_seconds,
        metadata: req.metadata,
    };
    state.registry.register(inst.clone());
    Ok(Json(inst))
}

/// GET /api/registry/experts — 实例列表/发现
///
/// 支持 `?capability= &domain= &status= &name=`；不带 status 时只返回可发现实例。
async fn list_registrations(
    State(state): State<AppState>,
    Query(query): Query<InstanceQuery>,
) -> Json<serde_json::Value> {
    let instances = state.registry.list(&query);
    let total = instances.len();
    Json(serde_json::json!({
        "instances": instances,
        "total": total,
    }))
}

/// GET /api/registry/experts/:id — 实例详情
async fn get_registration(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RegisteredInstance>, StatusCode> {
    state
        .registry
        .get(&id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// DELETE /api/registry/experts/:id — 优雅注销
async fn deregister_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .registry
        .deregister(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/registry/experts/:id/heartbeat — 心跳续约
async fn heartbeat_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<HeartbeatRequest>>,
) -> Result<Json<RegisteredInstance>, StatusCode> {
    let (load, status) = match body {
        Some(Json(req)) => (req.load_current, req.status),
        None => (None, None),
    };
    state
        .registry
        .heartbeat(&id, load, status)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
