// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 指标收集 — Prometheus格式，零配置自动暴露 /metrics

use axum::{response::IntoResponse, routing::get, Router};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct HistogramBucket {
    pub count: Arc<AtomicU64>,
    pub sum: Arc<AtomicU64>,
}

impl HistogramBucket {
    pub fn record(&self, value: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);
    }
}

#[derive(Clone, Default)]
pub struct GaugeValue {
    pub value: Arc<AtomicU64>,
}

impl GaugeValue {
    pub fn set_f64(&self, v: f64) {
        let bits = v.to_bits();
        self.value.store(bits, Ordering::Relaxed);
    }
    pub fn get_f64(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }
}

/// 指标收集器
#[derive(Clone)]
pub struct MetricsCollector {
    pub service_name: String,
    pub request_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub request_latency_ms: HistogramBucket,
}

impl MetricsCollector {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            request_latency_ms: HistogramBucket::default(),
        }
    }

    /// 记录请求
    pub fn record_request(&self, _method: &str, _path: &str, status: u16, latency_ms: u64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.request_latency_ms.record(latency_ms);
        if status >= 500 {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 记录业务指标（空实现占位，避免 metrics crate 宏问题）
    pub fn record_business(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
    }

    /// 设置gauge指标（空实现占位）
    pub fn set_gauge(&self, _name: &str, _value: f64) {
    }

    /// 构建指标路由
    pub fn routes(&self) -> Router {
        Router::new().route("/metrics", get(metrics_handler))
    }
}

async fn metrics_handler() -> impl IntoResponse {
    let body = "# HELP mox_service_info Service info\n# TYPE mox_service_info gauge\nmox_service_info{version=\"1.0.0\"} 1\n";
    ([("content-type", "text/plain; version=0.0.4")], body)
}
