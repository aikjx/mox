// =============================================================================
// 路由模块
// =============================================================================

pub mod health;
pub mod auth;
pub mod alliance;
pub mod knowledge;
pub mod metrics;

use crate::app_state::AppState;
use axum::{routing::get, Router};

/// 创建应用路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 健康检查
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        // 认证
        .route("/auth/login", get(auth::login))
        .route("/auth/refresh", get(auth::refresh))
        .route("/auth/me", get(auth::me))
        // 联盟引擎
        .route("/alliance/tasks", get(alliance::list_tasks))
        .route("/alliance/tasks", axum::routing::post(alliance::create_task))
        .route("/alliance/tasks/{id}", get(alliance::get_task))
        // 知识库
        .route("/knowledge/documents", get(knowledge::list_documents))
        .route("/knowledge/search", get(knowledge::search))
        // 指标
        .route("/metrics", get(metrics::prometheus_metrics))
        .with_state(state)
}
