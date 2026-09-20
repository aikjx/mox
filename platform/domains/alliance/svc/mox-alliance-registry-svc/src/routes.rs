// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 路由定义

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
    http::StatusCode,
};
use std::collections::HashMap;

use crate::{
    AppState,
    models::{Expert, CreateExpertRequest, UpdateExpertRequest},
};

/// 创建路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/experts", get(list_experts).post(create_expert))
        .route("/api/v1/experts/:id", get(get_expert).put(update_expert).delete(delete_expert))
        .with_state(state)
}

/// 健康检查
async fn health_check() -> &'static str {
    "ok"
}

/// GET /api/v1/experts — 专家列表
async fn list_experts(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let experts = state.store.list().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let domain_filter = params.get("domain").cloned();
    let search = params.get("search").cloned();

    let mut filtered: Vec<Expert> = experts.into_iter()
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
) -> Result<Json<Expert>, StatusCode> {
    state.store.get(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// POST /api/v1/experts — 创建专家
async fn create_expert(
    State(state): State<AppState>,
    Json(req): Json<CreateExpertRequest>,
) -> Result<Json<Expert>, StatusCode> {
    let mut expert = Expert::new(req.name);
    if let Some(title) = req.title { expert.title = title; }
    if let Some(org) = req.organization { expert.organization = org; }
    if let Some(domains) = req.domains { expert.domains = domains; }
    if let Some(skills) = req.skills { expert.skills = skills; }
    if let Some(bio) = req.bio { expert.bio = bio; }

    state.store.create(&expert)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(expert))
}

/// PUT /api/v1/experts/:id — 更新专家
async fn update_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateExpertRequest>,
) -> Result<Json<Expert>, StatusCode> {
    let mut expert = state.store.get(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = req.name { expert.name = name; }
    if let Some(title) = req.title { expert.title = title; }
    if let Some(org) = req.organization { expert.organization = org; }
    if let Some(domains) = req.domains { expert.domains = domains; }
    if let Some(skills) = req.skills { expert.skills = skills; }
    if let Some(bio) = req.bio { expert.bio = bio; }
    if let Some(enabled) = req.enabled { expert.enabled = enabled; }

    state.store.update(&expert)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(expert))
}

/// DELETE /api/v1/experts/:id — 删除专家
async fn delete_expert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state.store.delete(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
