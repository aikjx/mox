// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库 HTTP 适配层（axum，对齐 legacy `/kb/*` API 面，前端零改动）
//!
//! 响应信封与 legacy 一致：成功 `{ "success": true, "data": ... }`，
//! 失败 `{ "success": false, "code": ..., "error": ... }`。

use crate::KbState;
use crate::analyze::{AnalysisResult, KbAnalyzer};
use crate::link::{GraphLinker, LinkResult};
use crate::model::{KbDocument, SearchRequest};
use crate::search::KbSearcher;
use crate::version::KbVersionService;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

/// 成功响应（统一 ApiResponse 信封）
fn ok<T: serde::Serialize>(data: T) -> ApiResponse<Value> {
    api_ok(serde_json::to_value(data).unwrap_or(Value::Null))
}

/// 错误响应（统一 ApiResponse 信封，code 取 HTTP 状态码）
fn err(status: StatusCode, _code: &str, message: &str) -> ApiResponse<Value> {
    api_error(status.as_u16() as i32, message)
}

/// 文档不存在统一错误
fn not_found(id: &str) -> ApiResponse<Value> {
    err(StatusCode::NOT_FOUND, "not_found", &format!("文档不存在: {id}"))
}

// ====================================================================
// 请求体
// ====================================================================
#[derive(Debug, Deserialize)]
struct CreateDocReq {
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct BatchAnalyzeReq {
    #[serde(default)]
    ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct VersionNoteReq {
    #[serde(default)]
    note: String,
}

#[derive(Debug, Deserialize)]
struct CompareReq {
    #[serde(default)]
    v1: String,
    #[serde(default)]
    v2: String,
}

#[derive(Debug, Deserialize)]
struct RevertReq {
    #[serde(default)]
    version: String,
}

// ====================================================================
// 文档 CRUD
// ====================================================================

async fn kb_documents_list(State(state): State<Arc<KbState>>) -> ApiResponse<Value> {
    match state.docs.list().await {
        Ok(items) => ok(json!({ "items": items, "total": items.len() })),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, "kb_list_failed", &e.to_string()),
    }
}

async fn kb_document_create(State(state): State<Arc<KbState>>, Json(payload): Json<CreateDocReq>) -> ApiResponse<Value> {
    let doc = match state
        .docs
        .create(&payload.title, &payload.content, payload.category.as_deref())
        .await
    {
        Ok(mut doc) => {
            if let Some(tags) = &payload.tags {
                let patched = KbDocument {
                    tags: tags.clone(),
                    ..doc
                };
                let _ = state.docs.save(&patched).await;
                doc = patched;
            }
            doc
        }
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_create_failed", &e.to_string()),
    };
    ok(json!({ "id": doc.id, "status": "created", "document": doc }))
}

async fn kb_document_get(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    match state.docs.get(&id).await {
        Ok(doc) => ok(doc),
        Err(_) => not_found(&id),
    }
}

async fn kb_document_update(State(state): State<Arc<KbState>>, Path(id): Path<String>, Json(payload): Json<Value>) -> ApiResponse<Value> {
    match state.docs.update(&id, &payload).await {
        Ok(doc) => ok(json!({ "id": id, "status": "updated", "document": doc })),
        Err(_) => not_found(&id),
    }
}

async fn kb_document_delete(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let existed = match state.docs.delete(&id).await {
        Ok(v) => v,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_delete_failed", &e.to_string()),
    };
    if !existed {
        return not_found(&id);
    }
    // 反挂图：移除文档子图
    GraphLinker.unlink(&state.graph, &id);
    ok(json!({ "id": id, "status": "deleted" }))
}

// ====================================================================
// 分析（专家联盟）
// ====================================================================

async fn kb_document_analyze(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let mut doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    let result: AnalysisResult = match KbAnalyzer.analyze(&mut doc).await {
        Ok(r) => r,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_analyze_failed", &e.to_string()),
    };
    if let Err(e) = state.docs.save(&doc).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_analyze_save_failed", &e.to_string());
    }
    ok(result)
}

async fn kb_batch_analyze(State(state): State<Arc<KbState>>, Json(payload): Json<BatchAnalyzeReq>) -> ApiResponse<Value> {
    let mut analyzed = 0usize;
    let mut failed = Vec::new();
    // 目标：显式 ids 或全量
    let target_ids: Vec<String> = match &payload.ids {
        Some(ids) => ids.clone(),
        None => {
            let mut ids = Vec::new();
            for item in state.docs.list().await.unwrap_or_default() {
                if let Some(id) = item["id"].as_str() {
                    ids.push(id.to_string());
                }
            }
            ids
        }
    };
    for id in target_ids {
        match state.docs.get(&id).await {
            Ok(mut doc) => {
                if let Ok(_result) = KbAnalyzer.analyze(&mut doc).await {
                    if state.docs.save(&doc).await.is_ok() {
                        analyzed += 1;
                    }
                }
            }
            Err(_) => failed.push(id),
        }
    }
    ok(json!({ "status": "completed", "analyzed": analyzed, "failed": failed }))
}

// ====================================================================
// 分类 / 标签
// ====================================================================

async fn kb_categories(State(state): State<Arc<KbState>>) -> ApiResponse<Value> {
    match state.docs.categories().await {
        Ok(items) => ok(items),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, "kb_categories_failed", &e.to_string()),
    }
}

async fn kb_tags(State(state): State<Arc<KbState>>) -> ApiResponse<Value> {
    match state.docs.tags().await {
        Ok(items) => ok(items),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, "kb_tags_failed", &e.to_string()),
    }
}

// ====================================================================
// 检索
// ====================================================================

async fn kb_search(State(state): State<Arc<KbState>>, Json(payload): Json<SearchRequest>) -> ApiResponse<Value> {
    let docs = match KbSearcher.search_docs(&state, &payload).await {
        Ok(h) => h,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_search_failed", &e.to_string()),
    };
    let graph_hits = KbSearcher::search_graph(&state.graph, &payload.query, payload.limit);
    ok(json!({
        "results": docs,
        "graph_hits": graph_hits,
        "total": docs.len(),
    }))
}

// ====================================================================
// 版本
// ====================================================================

async fn kb_doc_versions(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    ok(json!({ "doc_id": id, "versions": KbVersionService::list(&doc) }))
}

async fn kb_doc_version(State(state): State<Arc<KbState>>, Path((id, ver)): Path<(String, String)>) -> ApiResponse<Value> {
    let doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    match KbVersionService::get(&doc, &ver) {
        Some(v) => ok(json!({ "doc_id": id, "version": v.version, "title": v.title, "content": v.content, "note": v.note, "created_at": v.created_at })),
        None => err(StatusCode::NOT_FOUND, "version_not_found", &format!("版本不存在: {ver}")),
    }
}

async fn kb_doc_create_version(State(state): State<Arc<KbState>>, Path(id): Path<String>, Json(payload): Json<VersionNoteReq>) -> ApiResponse<Value> {
    let mut doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    let created = KbVersionService::create(&mut doc, &payload.note);
    if let Err(e) = state.docs.save(&doc).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_version_save_failed", &e.to_string());
    }
    ok(json!({ "doc_id": id, "version": created.version, "status": "created", "note": created.note }))
}

async fn kb_doc_compare_versions(State(state): State<Arc<KbState>>, Path(id): Path<String>, Json(payload): Json<CompareReq>) -> ApiResponse<Value> {
    let doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    let (v1, v2) = if payload.v1.is_empty() && payload.v2.is_empty() {
        // 缺省：最近两个版本
        let all = KbVersionService::list(&doc);
        if all.len() < 2 {
            return err(StatusCode::BAD_REQUEST, "version_compare", "版本数不足 2");
        }
        (all[1].version.clone(), all[0].version.clone())
    } else {
        (payload.v1, payload.v2)
    };
    match KbVersionService::compare(&doc, &v1, &v2) {
        Some(diff) => ok(json!({ "doc_id": id, "diff": diff })),
        None => err(StatusCode::NOT_FOUND, "version_not_found", "对比版本不存在"),
    }
}

async fn kb_doc_revert_version(State(state): State<Arc<KbState>>, Path(id): Path<String>, Json(payload): Json<RevertReq>) -> ApiResponse<Value> {
    let mut doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    let version = if payload.version.is_empty() {
        // 缺省：回滚到上一版本
        let all = KbVersionService::list(&doc);
        match all.get(1) {
            Some(v) => v.version.clone(),
            None => return err(StatusCode::BAD_REQUEST, "version_revert", "无历史版本可回滚"),
        }
    } else {
        payload.version
    };
    match KbVersionService::revert(&mut doc, &version) {
        Some(v) => {
            if let Err(e) = state.docs.save(&doc).await {
                return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_revert_save_failed", &e.to_string());
            }
            ok(json!({ "doc_id": id, "status": "reverted", "version": v.version }))
        }
        None => err(StatusCode::NOT_FOUND, "version_not_found", &format!("版本不存在: {version}")),
    }
}

// ====================================================================
// 实体 / 挂图 / 历史 / 统计
// ====================================================================

async fn kb_doc_entities(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    ok(json!({ "doc_id": id, "entities": doc.entities, "relations": doc.relations }))
}

async fn kb_doc_graph_link(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let mut doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    // 若未分析过则先分析，保证挂图有实体/分块
    if doc.entities.is_empty() {
        if let Err(e) = KbAnalyzer.analyze(&mut doc).await {
            return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_link_analyze_failed", &e.to_string());
        }
    }
    let chunks = crate::analyze::chunk_doc(&doc);
    let result: LinkResult = GraphLinker.link(&state.graph, &doc, &chunks);
    doc.status = crate::model::STATUS_LINKED.into();
    if let Err(e) = state.docs.save(&doc).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "kb_link_save_failed", &e.to_string());
    }
    // 挂图后节点清单
    let graph_nodes: Vec<Value> = state
        .graph
        .list_nodes()
        .into_iter()
        .filter(|n| n.id == crate::link::doc_node_id(&doc.id) || n.id.starts_with(&format!("kb-{}-", doc.id)))
        .map(|n| {
            json!({
                "id": n.id, "node_type": n.node_type, "label": n.label, "properties": n.properties,
            })
        })
        .collect();
    ok(json!({
        "doc_id": id,
        "status": result.status,
        "graph_nodes": graph_nodes,
        "nodes_added": result.nodes_added,
        "edges_added": result.edges_added,
        "graph_total_nodes": result.graph_nodes,
        "graph_total_edges": result.graph_edges,
    }))
}

async fn kb_doc_graph_unlink(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let removed = GraphLinker.unlink(&state.graph, &id);
    ok(json!({ "doc_id": id, "status": "unlinked", "nodes_removed": removed }))
}

async fn kb_doc_history(State(state): State<Arc<KbState>>, Path(id): Path<String>) -> ApiResponse<Value> {
    let doc = match state.docs.get(&id).await {
        Ok(d) => d,
        Err(_) => return not_found(&id),
    };
    let history: Vec<Value> = doc
        .versions
        .iter()
        .map(|v| {
            json!({
                "version": v.version, "note": v.note, "title": v.title, "created_at": v.created_at,
            })
        })
        .collect();
    ok(json!({ "doc_id": id, "history": history }))
}

async fn kb_stats(State(state): State<Arc<KbState>>) -> ApiResponse<Value> {
    match state.docs.stats().await {
        Ok(stats) => {
            let mut s = stats.as_object().cloned().unwrap_or_default();
            s.insert("graph_nodes".into(), json!(state.graph.node_count()));
            s.insert("graph_edges".into(), json!(state.graph.edge_count()));
            ok(Value::Object(s))
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, "kb_stats_failed", &e.to_string()),
    }
}

async fn kb_history(State(state): State<Arc<KbState>>, Query(_params): Query<HashMap<String, String>>) -> ApiResponse<Value> {
    // 全局操作历史：汇总各文档版本记录
    let mut history = Vec::new();
    if let Ok(items) = state.docs.list().await {
        for item in items {
            if let Some(id) = item["id"].as_str() {
                if let Ok(doc) = state.docs.get(id).await {
                    for v in &doc.versions {
                        history.push(json!({
                            "doc_id": doc.id, "doc_title": doc.title,
                            "version": v.version, "note": v.note, "created_at": v.created_at,
                        }));
                    }
                }
            }
        }
    }
    history.sort_by(|a, b| {
        b["created_at"]
            .as_str()
            .unwrap_or("")
            .cmp(a["created_at"].as_str().unwrap_or(""))
    });
    ok(history)
}

// ====================================================================
// 路由装配入口（网关 merge 挂接）
// ====================================================================

/// 构建知识库路由（对齐 legacy `/kb/*` API 面）
pub fn build_kb_router() -> Router {
    build_kb_router_with_state(Arc::new(KbState::from_env()))
}

/// 使用显式数据目录构建路由（测试注入专用，避免进程级环境变量竞态）
pub fn build_kb_router_with_dir(dir: std::path::PathBuf) -> Router {
    build_kb_router_with_state(Arc::new(KbState::with_data_dir(dir)))
}

/// 使用已装配状态构建路由
fn build_kb_router_with_state(state: Arc<KbState>) -> Router {
    Router::new()
        .route("/kb/documents", get(kb_documents_list).post(kb_document_create))
        .route("/kb/documents/:id", get(kb_document_get).put(kb_document_update).delete(kb_document_delete))
        .route("/kb/documents/:id/analyze", post(kb_document_analyze))
        .route("/kb/batch-analyze", post(kb_batch_analyze))
        .route("/kb/categories", get(kb_categories))
        .route("/kb/tags", get(kb_tags))
        .route("/kb/search", post(kb_search))
        .route("/kb/documents/:id/versions", get(kb_doc_versions).post(kb_doc_create_version))
        .route("/kb/documents/:id/versions/:ver", get(kb_doc_version))
        .route("/kb/documents/:id/versions/compare", post(kb_doc_compare_versions))
        .route("/kb/documents/:id/versions/revert", post(kb_doc_revert_version))
        .route("/kb/documents/:id/entities", get(kb_doc_entities))
        .route("/kb/documents/:id/graph-link", post(kb_doc_graph_link).delete(kb_doc_graph_unlink))
        .route("/kb/documents/:id/history", get(kb_doc_history))
        .route("/kb/stats", get(kb_stats))
        .route("/kb/history", get(kb_history))
        .with_state(state)
}



