// =============================================================================
// 健康检查路由
// =============================================================================

use crate::app_state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

/// 存活探针（轻量检查，不检查依赖）
pub async fn liveness(State(state): State<AppState>) -> impl IntoResponse {
    let report = state.health.liveness();
    let status = StatusCode::from_u16(report.http_status_code()).unwrap_or(StatusCode::OK);
    (status, Json(report))
}

/// 就绪探针（完整检查，包括所有依赖）
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let report = state.health.readiness().await;
    let status = StatusCode::from_u16(report.http_status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE);
    (status, Json(report))
}
