// =============================================================================
// 指标路由
// =============================================================================

use crate::app_state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

/// Prometheus 指标导出
pub async fn prometheus_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.metrics.export();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        metrics,
    )
}
