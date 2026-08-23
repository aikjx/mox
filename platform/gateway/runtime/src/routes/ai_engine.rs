//! /ai/engine/* 路由树挂载

use crate::handlers::ai_engine::{
    AiEngineState, analyze_handler, capabilities_handler, metrics_handler, process_handler,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn ai_engine_routes(state: Arc<AiEngineState>) -> Router {
    Router::new()
        .route("/process", post(process_handler))
        .route("/analyze", post(analyze_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
