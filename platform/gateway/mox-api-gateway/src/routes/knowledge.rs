// =============================================================================
// 知识库路由
// =============================================================================

use crate::app_state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

/// 搜索请求
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub top_k: Option<usize>,
    pub filters: Option<serde_json::Value>,
}

/// 搜索结果
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f64,
    pub source: String,
}

/// 搜索响应
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub latency_ms: u64,
}

/// 文档列表响应
#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentInfo>,
    pub total: usize,
}

/// 文档信息
#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub char_count: usize,
    pub chunk_count: usize,
    pub created_at: String,
}

/// 搜索知识库（简化实现）
pub async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let top_k = req.top_k.unwrap_or(5);

    state.record_request("POST", "/knowledge/search", 200, 0);

    // 简化实现：返回空结果，实际应调用知识库核心
    let response = SearchResponse {
        query: req.query,
        results: vec![],
        total: 0,
        latency_ms: start.elapsed().as_millis() as u64,
    };

    let _ = top_k; // 抑制未使用警告

    (StatusCode::OK, Json(response))
}

/// 列出文档
pub async fn list_documents(State(state): State<AppState>) -> impl IntoResponse {
    state.record_request("GET", "/knowledge/documents", 200, 0);

    let response = DocumentListResponse {
        documents: vec![],
        total: 0,
    };

    (StatusCode::OK, Json(response))
}
