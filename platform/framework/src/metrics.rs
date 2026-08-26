//! 指标收集 — Prometheus格式，零配置自动暴露 /metrics

use axum::{response::IntoResponse, routing::get, Router};
use metrics::{counter, gauge, histogram};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 指标收集器
#[derive(Clone)]
pub struct MetricsCollector {
    service_name: String,
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 记录请求
    pub fn record_request(&self, method: &str, path: &str, status: u16, latency_ms: u64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        counter!("http_requests_total", "service" => self.service_name.clone(), "method" => method.to_string(), "path" => path.to_string(), "status" => status.to_string()).increment(1);
        histogram!("http_request_duration_ms", "service" => self.service_name.clone()).record(latency_ms as f64);
        if status >= 500 {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            counter!("http_errors_total", "service" => self.service_name.clone(), "status" => status.to_string()).increment(1);
        }
    }

    /// 记录业务指标
    pub fn record_business(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let mut label_vec: Vec<(String, String)> = vec![("service".into(), self.service_name.clone())];
        for (k, v) in labels {
            label_vec.push((k.to_string(), v.to_string()));
        }
        histogram!(name, label_vec).record(value);
    }

    /// 设置gauge指标
    pub fn set_gauge(&self, name: &str, value: f64) {
        gauge!(name, "service" => self.service_name.clone()).set(value);
    }

    /// 构建指标路由
    pub fn routes(&self) -> Router {
        Router::new().route("/metrics", get(metrics_handler))
    }
}

async fn metrics_handler() -> impl IntoResponse {
    // 简化：返回Prometheus格式文本
    let body = "# HELP mox_service_info Service info\n# TYPE mox_service_info gauge\nmox_service_info{version=\"1.0.0\"} 1\n";
    ([("content-type", "text/plain; version=0.0.4")], body)
}
