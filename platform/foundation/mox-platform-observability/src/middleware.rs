//! HTTP middleware for observability.
//!
//! Provides an axum-compatible middleware layer that:
//! - Injects trace_id into request extensions and response headers
//! - Records HTTP request metrics (count, latency, errors)
//! - Logs request/response with structured fields

use crate::metrics::{record_http_request, HttpMetrics};
use crate::tracing_ctx::TraceContext;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    response::Response,
    middleware::Next,
};
use std::time::Instant;

#[derive(Clone)]
pub struct ObservabilityState {
    pub service_name: String,
    pub http_metrics: HttpMetrics,
}

impl ObservabilityState {
    pub fn new(service_name: &str, http_metrics: HttpMetrics) -> Self {
        Self {
            service_name: service_name.to_string(),
            http_metrics,
        }
    }
}

/// Axum middleware layer for observability.
///
/// Usage:
/// ```ignore
/// use axum::Router;
/// use mox_platform_observability::{observability_layer, ObservabilityState, init};
///
/// let metrics = init("my-service", None);
/// let state = ObservabilityState::new("my-service", metrics.http.clone());
/// let app = Router::new()
///     .layer(axum::middleware::from_fn_with_state(state, observability_layer));
/// ```
pub async fn observability_layer(
    state: ObservabilityState,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract or create trace context
    let trace_ctx = TraceContext::from_headers(req.headers(), &state.service_name);
    let trace_id = trace_ctx.trace_id.clone();

    // In-flight gauge
    state.http_metrics.in_flight.inc();

    // Add trace_id to request extensions for downstream handlers
    let mut req = req;
    req.extensions_mut().insert(trace_ctx);

    // Execute request
    let response = next.run(req).await;

    // Record metrics
    let status = response.status().as_u16();
    let duration = start.elapsed();
    record_http_request(&state.http_metrics, &method, &path, status, duration);

    state.http_metrics.in_flight.dec();

    // Add trace headers to response
    let mut response = response;
    response.headers_mut().insert(
        "x-trace-id",
        trace_id.parse().unwrap_or(HeaderValue::from_static("unknown")),
    );

    // Structured log
    if status >= 500 {
        tracing::error!(
            trace.id = %trace_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration.as_millis() as u64,
            "request completed with server error"
        );
    } else if status >= 400 {
        tracing::warn!(
            trace.id = %trace_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration.as_millis() as u64,
            "request completed with client error"
        );
    } else {
        tracing::debug!(
            trace.id = %trace_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration.as_millis() as u64,
            "request completed"
        );
    }

    response
}

/// Health check response helper.
pub fn health_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap_or_else(|_| Response::new(Body::from("error")))
}

/// Metrics endpoint response helper (Prometheus text format).
pub fn metrics_response(metrics_text: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; version=0.0.4")
        .body(Body::from(metrics_text))
        .unwrap_or_else(|_| Response::new(Body::from("error")))
}
