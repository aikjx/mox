//! 路由注册
//!
//! 统一注册所有域的 HTTP 路由

use axum::{Router, routing::get, http::StatusCode, Json};
use mox_platform_foundation::ApiResponse;
use uuid::Uuid;

/// 构建总路由
pub fn build_router() -> Router {
    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        .route("/api/v1/health", get(health_check))
        // API v1 路由（占位，后续各域注册）
        .nest("/api/v1/kg", kg_routes())
        .nest("/api/v1/ai", ai_routes())
}

async fn health_check() -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let req_id = format!("req_{}", Uuid::new_v4().simple());
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({"status": "ok"}), req_id)),
    )
}

fn kg_routes() -> Router {
    Router::new()
        .route("/graphs", get(|| async { "KG API placeholder" }))
}

fn ai_routes() -> Router {
    Router::new()
        .route("/chat", get(|| async { "AI API placeholder" }))
}
