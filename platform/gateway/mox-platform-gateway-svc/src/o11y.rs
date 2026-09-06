// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # o11y（可观测性）模块
//!
//! 网关运行时指标的真实实现（Prometheus 文本格式，供 `GET /metrics` 抓取与
//! `deploy/docs/trace-8stages-dashboard.json` Grafana 面板消费）：
//!
//! - `mox_gateway_requests_total{method,status_class}` —— 请求计数
//! - `mox_gateway_request_duration_seconds{method}` —— 请求延迟分布（直方图）
//! - `mox_gateway_errors_total{status_class}` —— 4xx/5xx 错误计数
//! - `mox_gateway_active_requests` —— 活跃请求数
//! - `mox_gateway_uptime_seconds` —— 进程存活时长
//!
//! 输出端点：`GET /metrics`（Prometheus 文本）；`/actuator/metrics`（JSON 概览）。

use std::time::Instant;

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder};

/// 可观测性配置（占位字段保留，待逐模块迁移后扩展）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub logging_enabled: bool,
}

/// 指标收集器：Prometheus 注册表 + 预定义指标族
///
/// 指标族在 `new()` 中一次性注册（名称/标签固定），运行时只做原子自增与采样，
/// 线程安全，无锁开销。
#[derive(Debug)]
pub struct MetricsCollector {
    registry: Registry,
    requests: IntCounterVec,
    errors: IntCounterVec,
    duration: HistogramVec,
    active: IntGauge,
    uptime_secs: IntGauge,
    started: Instant,
    #[allow(dead_code)]
    config: ObservabilityConfig,
}

impl MetricsCollector {
    /// 创建收集器并注册全部指标族（panic 仅发生在指标定义错误，属程序 bug）
    pub fn new(config: ObservabilityConfig) -> Self {
        let registry = Registry::new();

        let requests = IntCounterVec::new(
            Opts::new("mox_gateway_requests_total", "Total HTTP requests handled by gateway"),
            &["method", "status_class"],
        )
        .expect("requests counter");
        let errors = IntCounterVec::new(
            Opts::new("mox_gateway_errors_total", "Error responses (4xx/5xx)"),
            &["status_class"],
        )
        .expect("errors counter");
        let duration = HistogramVec::new(
            HistogramOpts::new(
                "mox_gateway_request_duration_seconds",
                "Request handling latency",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["method"],
        )
        .expect("duration histogram");
        let active = IntGauge::new(
            "mox_gateway_active_requests",
            "Currently in-flight requests",
        )
        .expect("active gauge");
        let uptime_secs = IntGauge::new(
            "mox_gateway_uptime_seconds",
            "Process uptime in seconds",
        )
        .expect("uptime gauge");

        for metric in [
            Box::new(requests.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(errors.clone()),
            Box::new(duration.clone()),
            Box::new(active.clone()),
            Box::new(uptime_secs.clone()),
        ] {
            registry
                .register(metric)
                .expect("register metric family");
        }

        Self {
            registry,
            requests,
            errors,
            duration,
            active,
            uptime_secs,
            started: Instant::now(),
            config,
        }
    }

    /// 状态码 → 分类标签（低基数）
    fn status_class(status: u16) -> &'static str {
        match status {
            200..=299 => "2xx",
            400..=499 => "4xx",
            500..=599 => "5xx",
            _ => "other",
        }
    }

    /// 记录一次请求（由 `observability_middleware` 调用）
    pub fn record_request(&self, method: &str, status: u16, dur: std::time::Duration) {
        let sc = Self::status_class(status);
        let _ = self
            .requests
            .get_metric_with_label_values(&[method, sc])
            .map(|c| c.inc());
        if status >= 400 {
            let _ = self
                .errors
                .get_metric_with_label_values(&[sc])
                .map(|c| c.inc());
        }
        self.duration.with_label_values(&[method]).observe(dur.as_secs_f64());
    }

    /// 活跃请求数 +1（请求进入时）
    pub fn active_inc(&self) {
        self.active.inc();
    }

    /// 活跃请求数 -1（请求完成时）
    pub fn active_dec(&self) {
        self.active.dec();
    }

    /// 导出 Prometheus 文本格式（`GET /metrics`）
    pub fn render(&self) -> String {
        self.uptime_secs.set(self.started.elapsed().as_secs() as i64);
        let families = self.registry.gather();
        let mut buf = String::new();
        match TextEncoder::new().encode_utf8(&families, &mut buf) {
            Ok(()) => buf,
            Err(e) => format!("# ERROR encoding metrics: {e}\n"),
        }
    }

    /// 兼容旧接口：按名称路由到预定义指标（未知指标安全忽略）
    pub fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) {
        if name == "requests_total" {
            let method = labels
                .iter()
                .find(|(k, _)| *k == "method")
                .map(|(_, v)| *v)
                .unwrap_or("");
            let sc = labels
                .iter()
                .find(|(k, _)| *k == "status_class")
                .map(|(_, v)| *v)
                .unwrap_or("other");
            let _ = self
                .requests
                .get_metric_with_label_values(&[method, sc])
                .map(|c| c.inc());
        }
    }

    /// 兼容旧接口：直方图采样（未定义指标安全忽略）
    pub fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        if name == "request_duration" {
            let method = labels
                .iter()
                .find(|(k, _)| *k == "method")
                .map(|(_, v)| *v)
                .unwrap_or("");
            self.duration.with_label_values(&[method]).observe(value);
        }
    }

    /// 指标是否启用（供未来开关使用）
    pub fn metrics_enabled(&self) -> bool {
        self.config.metrics_enabled
    }
}
