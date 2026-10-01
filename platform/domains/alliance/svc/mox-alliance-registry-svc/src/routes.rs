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
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
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
        // N7 指标文本化：Prometheus exposition 0.0.4（原本未注册 /metrics，本次新增）
        .route("/metrics", get(metrics_handler))
        .route("/api/v1/experts", get(list_experts).post(create_expert))
        .route(
            "/api/v1/experts/:id",
            get(get_expert).put(update_expert).delete(delete_expert),
        )
        // ── 应用级专家实例注册中心 ──
        .route("/api/registry/health", get(registry_health))
        .route("/api/registry/overview", get(platform_overview))
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
        // 分级心跳聚合入口（node→rack→cell 10:1:1，rack/cell 代理批量续约）
        .route(
            "/api/registry/aggregated-heartbeat",
            post(heartbeat_aggregated),
        )
        // 一键传输加密（MOX_API_CRYPTO=sm4）：统一信封 data gzip+SM4-GCM，详见 mox-api-crypto
        .layer(axum::middleware::from_fn(
            mox_api_crypto::middleware::crypto_middleware,
        ))
        // 内部服务间鉴权：防绕过网关直连（MOX_INTERNAL_TOKEN；未配置则放行）
        .layer(middleware::from_fn(internal_auth_layer))
        .with_state(state)
}


/// N7：写一行 Prometheus 指标（HELP + TYPE + sample）。
fn prom_metric(o: &mut String, help: &str, ty: &str, name: &str, val: u64) {
    o.push_str(&format!("# HELP {name} {help}\n# TYPE {name} {ty}\n{name} {val}\n"));
}

/// N7 指标文本化：Prometheus exposition 0.0.4。
///
/// registry 原本未注册 `/metrics`（仅 internal_auth 白名单提到），本次新增。
/// 直接出文本（无既有 JSON 消费者），命名 `mox_alliance_registry_*`：
/// 注册中心实例总数 + 静态专家目录条目数。
async fn metrics_handler(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let instances = state.registry.count() as u64;
    let directory_experts = state
        .dir_store
        .list()
        .map(|v| v.len() as u64)
        .unwrap_or(0);
    let mut o = String::new();
    prom_metric(&mut o, "当前注册中心实例总数（含所有状态：Active/Unhealthy/Draining/Expired）", "gauge", "mox_alliance_registry_instances", instances);
    prom_metric(&mut o, "静态专家目录条目数（SQLite /api/v1/experts）", "gauge", "mox_alliance_registry_directory_experts", directory_experts);
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        o,
    )
}

/// 内部服务间鉴权：校验网关注入的共享令牌（MOX_INTERNAL_TOKEN，主值）。
///
/// 下游 svc 不直接对前端暴露，唯一入口是网关(:3080)。本层防止绕过网关直连：
/// - 配置 MOX_INTERNAL_TOKEN 后，所有非公开路径必须携带 `Authorization: Bearer <token>`；
/// - 未配置主/备任一（开发/本地默认）则放行，保持向后兼容；
/// - MOX_DEV_MODE=1 强制跳过；
/// - 探活/指标端点（/health、/metrics、/leadership、/api/registry/health）始终放行。
///
/// 双值滚动（零停机换令牌，P0-C 2026-09-29）：
/// - 新增可选 `MOX_INTERNAL_TOKEN_ALT`（备用值）。Bearer 等于主值 **或** 备用值即通过。
/// - 生产滚动流程：① 旧值写入 `MOX_INTERNAL_TOKEN_ALT` → 全集群滚动重启本 svc；
///   ② 网关出站把 `MOX_INTERNAL_TOKEN` 切到新值，全集群滚动重启网关；
///   ③ 观察一个周期后移除 `MOX_INTERNAL_TOKEN_ALT`，完成双窗口滚动。
///   两个窗口内网关旧请求带旧值（命中 ALT）、新请求带新值（命中主值），均不 401。
async fn internal_auth_layer(req: Request, next: Next) -> Result<Response, StatusCode> {
    let dev_mode = std::env::var("MOX_DEV_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(cfg!(debug_assertions));
    let expected = std::env::var("MOX_INTERNAL_TOKEN").unwrap_or_default();
    let expected_alt = std::env::var("MOX_INTERNAL_TOKEN_ALT").unwrap_or_default();
    if dev_mode || (expected.is_empty() && expected_alt.is_empty()) {
        return Ok(next.run(req).await);
    }
    let path = req.uri().path();
    const PUBLIC: &[&str] = &["/health", "/metrics", "/leadership", "/api/registry/health"];
    if PUBLIC.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(req).await);
    }
    let ok = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t == expected || (!expected_alt.is_empty() && t == expected_alt))
        .unwrap_or(false);
    if ok {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
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

/// GET /api/registry/overview — 平台概览（`get_platform_overview` 契约实现）
///
/// 直接复用 `mox-alliance-registry-core::inventory_platform_overview` 纯函数，
/// 从现存实例视图聚合 total/active/domains；total_consultations 暂为 0
/// （逐次调用遥测待后续接入调用成功率健康引擎填充）。
async fn platform_overview(
    State(state): State<AppState>,
) -> Json<mox_alliance_registry_proto::types::PlatformOverview> {
    let all = state.registry.all_instances();
    Json(mox_alliance_registry_core::inventory_platform_overview(&all))
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

/// POST /api/registry/aggregated-heartbeat — 分级聚合续约（10:1:1）
///
/// rack/cell 聚合代理（`mox-alliance-registry-core::RackAggregator/CellAggregator`）
/// 把叶级心跳合并为单条 [`AggregatedRenewal`] 提交：一次写锁、一次快照落盘。
/// 仅刷新已注册成员（聚合不隐式注册）；`unknown_members` 为聚合器中已消失
/// 的成员，注册中心侧同步摘除（显式注销沿聚合链传播）。
async fn heartbeat_aggregated(
    State(state): State<AppState>,
    Json(req): Json<mox_alliance_registry_core::AggregatedRenewal>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if req.members.is_empty() && req.unknown_members.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (renewed, unknown) = state
        .registry
        .heartbeat_batch(&req.members, req.reported_at);
    let pruned = state.registry.prune_members(&req.unknown_members);
    Ok(Json(serde_json::json!({
        "group_id": req.group_id,
        "seq": req.seq,
        "renewed": renewed,
        "unknown": unknown,
        "pruned": pruned,
        "member_count": req.member_count,
        "unhealthy_count": req.unhealthy_count,
        "load_total": req.load_total,
        "health": format!("{:?}", req.health).to_lowercase(),
    })))
}
