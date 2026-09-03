// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 知识库扩展域（KB Ext）HTTP 路由
//!
//! 提供实体搜索、文档-实体关联/解绑等知识库扩展能力。
//! 文档-实体关联关系使用内存存储（parking_lot::Mutex），可后续接入 JSON 持久化。
//!
//! 路径：`/kb/entities/search` · `/kb/documents/:id/entities`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocEntityRelation {
    document_id: String,
    entity_id: String,
    relation: String,
    created_at: i64,
}

#[derive(Clone)]
struct KbExtState {
    /// 文档-实体关联关系（内存存储，可后续持久化）
    relations: Arc<Mutex<Vec<DocEntityRelation>>>,
}

impl KbExtState {
    fn new() -> Self {
        Self { relations: Arc::new(Mutex::new(Vec::new())) }
    }
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 1. GET /kb/entities/search — 实体搜索
// =====================================================================

#[derive(Debug, Deserialize)]
struct EntitySearchQuery {
    q: Option<String>,
    #[serde(rename = "type")]
    entity_type: Option<String>,
    limit: Option<usize>,
}

async fn search_entities(
    Query(q): Query<EntitySearchQuery>,
) -> ApiResponse<Value> {
    // 当前无实体数据源，返回空数组
    let _limit = q.limit.unwrap_or(20).clamp(1, 100);
    let _ = q.q;
    let _ = q.entity_type;
    ok(json!([] as [Value; 0]))
}

// =====================================================================
// 2. POST /kb/documents/{id}/entities — 文档实体关联
// =====================================================================

#[derive(Debug, Deserialize)]
struct LinkEntityBody {
    entity_id: String,
    relation: Option<String>,
}

async fn link_document_entity(
    Path(id): Path<String>,
    State(s): State<Arc<KbExtState>>,
    Json(body): Json<LinkEntityBody>,
) -> ApiResponse<Value> {
    let relation = DocEntityRelation {
        document_id: id.clone(),
        entity_id: body.entity_id.clone(),
        relation: body.relation.unwrap_or_else(|| "related".into()),
        created_at: now_ts(),
    };
    s.relations.lock().push(relation.clone());
    ok(json!({
        "document_id": relation.document_id,
        "entity_id": relation.entity_id,
        "relation": relation.relation,
        "created_at": relation.created_at,
    }))
}

// =====================================================================
// 3. DELETE /kb/documents/{id}/entities — 文档实体解绑
// =====================================================================

#[derive(Debug, Deserialize)]
struct UnlinkEntityBody {
    entity_id: String,
}

async fn unlink_document_entity(
    Path(id): Path<String>,
    State(s): State<Arc<KbExtState>>,
    Json(body): Json<UnlinkEntityBody>,
) -> ApiResponse<Value> {
    let mut relations = s.relations.lock();
    let before = relations.len();
    relations.retain(|r| !(r.document_id == id && r.entity_id == body.entity_id));
    if relations.len() < before {
        ok(json!({ "deleted": true }))
    } else {
        api_error(404, format!("relation not found: document={}, entity={}", id, body.entity_id))
    }
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_kb_ext_router() -> Router {
    let state = Arc::new(KbExtState::new());
    Router::new()
        .route("/kb/entities/search", get(search_entities))
        .route("/kb/documents/:id/entities", post(link_document_entity).delete(unlink_document_entity))
        .with_state(state)
}
