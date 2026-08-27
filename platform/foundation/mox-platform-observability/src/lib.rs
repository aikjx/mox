// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Platform Observability Foundation
//!
//! Unified observability layer providing:
//! - Structured JSON logging with trace_id/span_id correlation
//! - Prometheus metrics (standard + domain-specific)
//! - Distributed tracing context propagation
//! - HTTP middleware for request/response observability
//!
//! All svc-layer crates MUST depend on this crate for observability.
//! Core-layer crates should use `tracing` directly (no I/O).

pub mod logging;
pub mod metrics;
pub mod tracing_ctx;
pub mod middleware;

pub use logging::{init_logging, LogConfig, LogFormat};
pub use metrics::{MetricsRegistry, HttpMetrics, ServiceMetrics, record_http_request};
pub use tracing_ctx::{TraceContext, current_trace_id, with_trace_context};
pub use middleware::{observability_layer, ObservabilityState};

use std::sync::OnceLock;

static GLOBAL_METRICS: OnceLock<MetricsRegistry> = OnceLock::new();

/// Initialize global observability (logging + metrics).
/// Call once at service startup before any async work.
pub fn init(service_name: &str, config: Option<LogConfig>) -> &'static MetricsRegistry {
    let cfg = config.unwrap_or_default();
    init_logging(service_name, &cfg);
    let registry = MetricsRegistry::new(service_name);
    GLOBAL_METRICS.get_or_init(|| registry)
}

/// Get the global metrics registry (must call `init` first).
pub fn global_metrics() -> Option<&'static MetricsRegistry> {
    GLOBAL_METRICS.get()
}
