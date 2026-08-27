// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Prometheus metrics registry and standard metric definitions.
//!
//! Provides:
//! - `MetricsRegistry`: per-service registry with standard HTTP + service metrics
//! - `HttpMetrics`: request count, latency histogram, error count by status code
//! - `ServiceMetrics`: business-level counters/gauges that any service can extend
//! - `record_http_request`: convenience function to record HTTP request metrics

use prometheus::{
    Encoder, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Standard HTTP latency buckets (ms): 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
const HTTP_LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

#[derive(Clone)]
pub struct HttpMetrics {
    pub requests_total: IntCounterVec,
    pub request_duration_seconds: prometheus::Histogram,
    pub errors_total: IntCounterVec,
    pub in_flight: IntGauge,
}

impl HttpMetrics {
    fn new(registry: &Registry, service: &str) -> Self {
        let requests_total = IntCounterVec::new(
            Opts::new(
                format!("{}_http_requests_total", service),
                "Total HTTP requests by method, path, and status",
            ),
            &["method", "path", "status"],
        )
        .expect("failed to create http_requests_total");

        let request_duration_seconds = prometheus::Histogram::with_opts(
            HistogramOpts::new(
                format!("{}_http_request_duration_seconds", service),
                "HTTP request latency in seconds",
            )
            .buckets(HTTP_LATENCY_BUCKETS.to_vec()),
        )
        .expect("failed to create request_duration_seconds");

        let errors_total = IntCounterVec::new(
            Opts::new(
                format!("{}_http_errors_total", service),
                "Total HTTP errors by method, path, and status",
            ),
            &["method", "path", "status"],
        )
        .expect("failed to create http_errors_total");

        let in_flight = IntGauge::new(
            format!("{}_http_in_flight_requests", service),
            "Number of in-flight HTTP requests",
        )
        .expect("failed to create in_flight_requests");

        registry
            .register(Box::new(requests_total.clone()))
            .ok();
        registry
            .register(Box::new(request_duration_seconds.clone()))
            .ok();
        registry
            .register(Box::new(errors_total.clone()))
            .ok();
        registry.register(Box::new(in_flight.clone())).ok();

        Self {
            requests_total,
            request_duration_seconds,
            errors_total,
            in_flight,
        }
    }
}

#[derive(Clone)]
pub struct ServiceMetrics {
    pub operations_total: IntCounterVec,
    pub operation_errors: IntCounterVec,
    pub operation_duration_seconds: prometheus::Histogram,
    custom_counters: Arc<parking_lot::Mutex<HashMap<String, IntCounter>>>,
}

impl ServiceMetrics {
    /// Register a custom counter metric and return it for direct use.
    pub fn register_custom_counter(
        &self,
        name: &str,
        help: &str,
        registry: &Registry,
    ) -> IntCounter {
        let counter = IntCounter::new(name, help).expect("failed to create custom counter");
        registry.register(Box::new(counter.clone())).ok();
        self.custom_counters.lock().insert(name.to_string(), counter.clone());
        counter
    }

    fn new(registry: &Registry, service: &str) -> Self {
        let operations_total = IntCounterVec::new(
            Opts::new(
                format!("{}_operations_total", service),
                "Total business operations by operation name and result",
            ),
            &["operation", "result"],
        )
        .expect("failed to create operations_total");

        let operation_errors = IntCounterVec::new(
            Opts::new(
                format!("{}_operation_errors_total", service),
                "Total operation errors by operation and error kind",
            ),
            &["operation", "error_kind"],
        )
        .expect("failed to create operation_errors");

        let operation_duration_seconds = prometheus::Histogram::with_opts(
            HistogramOpts::new(
                format!("{}_operation_duration_seconds", service),
                "Business operation latency in seconds",
            )
            .buckets(HTTP_LATENCY_BUCKETS.to_vec()),
        )
        .expect("failed to create operation_duration");

        registry
            .register(Box::new(operations_total.clone()))
            .ok();
        registry
            .register(Box::new(operation_errors.clone()))
            .ok();
        registry
            .register(Box::new(operation_duration_seconds.clone()))
            .ok();

        Self {
            operations_total,
            operation_errors,
            operation_duration_seconds,
            custom_counters: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    /// Record a business operation with its duration and result.
    pub fn record_operation(&self, operation: &str, result: &str, duration: std::time::Duration) {
        self.operations_total
            .with_label_values(&[operation, result])
            .inc();
        self.operation_duration_seconds
            .observe(duration.as_secs_f64());
    }

    /// Record an operation error.
    pub fn record_error(&self, operation: &str, error_kind: &str) {
        self.operation_errors
            .with_label_values(&[operation, error_kind])
            .inc();
    }

    /// Start timing an operation. Returns an `OperationTimer` that records on drop.
    pub fn time_operation(&self, operation: &'static str) -> OperationTimer<'_> {
        OperationTimer {
            metrics: self,
            operation,
            start: Instant::now(),
            result: "success",
        }
    }
}

pub struct OperationTimer<'a> {
    metrics: &'a ServiceMetrics,
    operation: &'static str,
    start: Instant,
    result: &'static str,
}

impl<'a> OperationTimer<'a> {
    pub fn set_result(&mut self, result: &'static str) {
        self.result = result;
    }

    pub fn set_error(&mut self, error_kind: &'static str) {
        self.result = "error";
        self.metrics.record_error(self.operation, error_kind);
    }
}

impl<'a> Drop for OperationTimer<'a> {
    fn drop(&mut self) {
        self.metrics
            .record_operation(self.operation, self.result, self.start.elapsed());
    }
}

#[derive(Clone)]
pub struct MetricsRegistry {
    pub service_name: String,
    pub http: HttpMetrics,
    pub service: ServiceMetrics,
    registry: Arc<Registry>,
}

impl MetricsRegistry {
    pub fn new(service_name: &str) -> Self {
        let registry = Registry::new();
        let http = HttpMetrics::new(&registry, service_name);
        let service = ServiceMetrics::new(&registry, service_name);
        Self {
            service_name: service_name.to_string(),
            http,
            service,
            registry: Arc::new(registry),
        }
    }

    /// Get the underlying Prometheus registry for registering custom metrics.
    pub fn prometheus_registry(&self) -> &Registry {
        &self.registry
    }

    /// Gather all metrics and render as Prometheus text format.
    pub fn gather_text(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap_or_default();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

/// Convenience: record an HTTP request with method, path, status, and duration.
pub fn record_http_request(
    metrics: &HttpMetrics,
    method: &str,
    path: &str,
    status: u16,
    duration: std::time::Duration,
) {
    let status_str = status.to_string();
    metrics
        .requests_total
        .with_label_values(&[method, path, &status_str])
        .inc();
    metrics
        .request_duration_seconds
        .observe(duration.as_secs_f64());
    if status >= 400 {
        metrics
            .errors_total
            .with_label_values(&[method, path, &status_str])
            .inc();
    }
}
