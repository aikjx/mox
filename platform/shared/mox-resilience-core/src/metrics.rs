//! 弹性容错 Prometheus 指标模块
//!
//! 提供熔断器、重试、降级等弹性容错组件的可观测性指标：
//! - 熔断器当前状态（Closed/HalfOpen/Open）
//! - 熔断器请求/成功/失败计数
//! - 熔断器打开/恢复次数
//! - 重试次数统计
//!
//! 所有指标使用独立 Registry，可通过 `gather()` 输出 Prometheus 文本格式。

use prometheus::{
    register_gauge_vec_with_registry, register_int_counter_vec_with_registry, GaugeVec,
    IntCounterVec, Registry,
};
use std::sync::Arc;

/// 熔断器状态枚举值（用于 Prometheus Gauge）
pub const CB_STATE_CLOSED: f64 = 0.0;
pub const CB_STATE_HALF_OPEN: f64 = 1.0;
pub const CB_STATE_OPEN: f64 = 2.0;

/// 弹性容错指标集合
pub struct ResilienceMetrics {
    /// 熔断器当前状态（标签：name）
    /// 0=Closed, 1=HalfOpen, 2=Open
    pub circuit_breaker_state: GaugeVec,
    /// 熔断器累计请求数（标签：name, result: success/failure/rejected）
    pub circuit_breaker_requests_total: IntCounterVec,
    /// 熔断器累计打开次数（标签：name）
    pub circuit_breaker_opened_total: IntCounterVec,
    /// 熔断器累计恢复关闭次数（标签：name）
    pub circuit_breaker_closed_total: IntCounterVec,
    /// 重试累计次数（标签：name, attempt）
    pub retry_attempts_total: IntCounterVec,
    /// 降级累计触发次数（标签：name, type: static/function）
    pub fallback_triggered_total: IntCounterVec,
    /// 指标注册表
    registry: Registry,
}

impl ResilienceMetrics {
    /// 创建并注册所有指标
    pub fn new() -> Self {
        let registry = Registry::new();

        let circuit_breaker_state = register_gauge_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_circuit_breaker_state",
                "Current state of circuit breaker (0=Closed, 1=HalfOpen, 2=Open)"
            ),
            &["name"],
            registry
        )
        .expect("register resilience_circuit_breaker_state");

        let circuit_breaker_requests_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_circuit_breaker_requests_total",
                "Total number of requests through circuit breaker"
            ),
            &["name", "result"],
            registry
        )
        .expect("register resilience_circuit_breaker_requests_total");

        let circuit_breaker_opened_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_circuit_breaker_opened_total",
                "Total number of times circuit breaker opened"
            ),
            &["name"],
            registry
        )
        .expect("register resilience_circuit_breaker_opened_total");

        let circuit_breaker_closed_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_circuit_breaker_closed_total",
                "Total number of times circuit breaker recovered to Closed"
            ),
            &["name"],
            registry
        )
        .expect("register resilience_circuit_breaker_closed_total");

        let retry_attempts_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_retry_attempts_total",
                "Total number of retry attempts"
            ),
            &["name", "attempt"],
            registry
        )
        .expect("register resilience_retry_attempts_total");

        let fallback_triggered_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "resilience_fallback_triggered_total",
                "Total number of fallback triggers"
            ),
            &["name", "type"],
            registry
        )
        .expect("register resilience_fallback_triggered_total");

        Self {
            circuit_breaker_state,
            circuit_breaker_requests_total,
            circuit_breaker_opened_total,
            circuit_breaker_closed_total,
            retry_attempts_total,
            fallback_triggered_total,
            registry,
        }
    }

    /// 记录熔断器状态变更
    pub fn record_circuit_breaker_state(&self, name: &str, state: f64) {
        self.circuit_breaker_state
            .with_label_values(&[name])
            .set(state);
    }

    /// 记录熔断器请求结果
    pub fn record_circuit_breaker_request(&self, name: &str, result: &str) {
        self.circuit_breaker_requests_total
            .with_label_values(&[name, result])
            .inc();
    }

    /// 记录熔断器打开
    pub fn record_circuit_breaker_opened(&self, name: &str) {
        self.circuit_breaker_opened_total
            .with_label_values(&[name])
            .inc();
        self.circuit_breaker_state
            .with_label_values(&[name])
            .set(CB_STATE_OPEN);
    }

    /// 记录熔断器恢复关闭
    pub fn record_circuit_breaker_closed(&self, name: &str) {
        self.circuit_breaker_closed_total
            .with_label_values(&[name])
            .inc();
        self.circuit_breaker_state
            .with_label_values(&[name])
            .set(CB_STATE_CLOSED);
    }

    /// 记录熔断器进入HalfOpen
    pub fn record_circuit_breaker_half_open(&self, name: &str) {
        self.circuit_breaker_state
            .with_label_values(&[name])
            .set(CB_STATE_HALF_OPEN);
    }

    /// 记录重试尝试
    pub fn record_retry_attempt(&self, name: &str, attempt: u32) {
        self.retry_attempts_total
            .with_label_values(&[name, &attempt.to_string()])
            .inc();
    }

    /// 记录降级触发
    pub fn record_fallback_triggered(&self, name: &str, fallback_type: &str) {
        self.fallback_triggered_total
            .with_label_values(&[name, fallback_type])
            .inc();
    }

    /// 收集所有指标，输出 Prometheus 文本格式
    pub fn gather(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap_or_default();
        String::from_utf8_lossy(&buffer).to_string()
    }
}

impl Default for ResilienceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局共享的弹性容错指标实例
pub type SharedMetrics = Arc<ResilienceMetrics>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ResilienceMetrics::new();
        // 记录一些指标
        metrics.record_circuit_breaker_state("test_cb", CB_STATE_CLOSED);
        metrics.record_circuit_breaker_request("test_cb", "success");
        metrics.record_circuit_breaker_opened("test_cb");
        metrics.record_circuit_breaker_closed("test_cb");
        metrics.record_retry_attempt("test_retry", 1);
        metrics.record_fallback_triggered("test_fallback", "static");

        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_state"));
        assert!(output.contains("resilience_circuit_breaker_requests_total"));
        assert!(output.contains("resilience_circuit_breaker_opened_total"));
        assert!(output.contains("resilience_circuit_breaker_closed_total"));
        assert!(output.contains("resilience_retry_attempts_total"));
        assert!(output.contains("resilience_fallback_triggered_total"));
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let metrics = ResilienceMetrics::new();

        // Closed -> Open
        metrics.record_circuit_breaker_opened("cb1");
        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_state{name=\"cb1\"} 2"));
        assert!(output.contains("resilience_circuit_breaker_opened_total{name=\"cb1\"} 1"));

        // Open -> HalfOpen
        metrics.record_circuit_breaker_half_open("cb1");
        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_state{name=\"cb1\"} 1"));

        // HalfOpen -> Closed
        metrics.record_circuit_breaker_closed("cb1");
        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_state{name=\"cb1\"} 0"));
        assert!(output.contains("resilience_circuit_breaker_closed_total{name=\"cb1\"} 1"));
    }

    #[test]
    fn test_circuit_breaker_requests() {
        let metrics = ResilienceMetrics::new();

        metrics.record_circuit_breaker_request("cb1", "success");
        metrics.record_circuit_breaker_request("cb1", "success");
        metrics.record_circuit_breaker_request("cb1", "failure");
        metrics.record_circuit_breaker_request("cb1", "rejected");

        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_requests_total{name=\"cb1\",result=\"success\"} 2"));
        assert!(output.contains("resilience_circuit_breaker_requests_total{name=\"cb1\",result=\"failure\"} 1"));
        assert!(output.contains("resilience_circuit_breaker_requests_total{name=\"cb1\",result=\"rejected\"} 1"));
    }

    #[test]
    fn test_retry_and_fallback_metrics() {
        let metrics = ResilienceMetrics::new();

        metrics.record_retry_attempt("op1", 1);
        metrics.record_retry_attempt("op1", 2);
        metrics.record_retry_attempt("op1", 3);
        metrics.record_fallback_triggered("op1", "static");
        metrics.record_fallback_triggered("op1", "function");

        let output = metrics.gather();
        assert!(output.contains("resilience_retry_attempts_total{attempt=\"1\",name=\"op1\"} 1"));
        assert!(output.contains("resilience_retry_attempts_total{attempt=\"2\",name=\"op1\"} 1"));
        assert!(output.contains("resilience_retry_attempts_total{attempt=\"3\",name=\"op1\"} 1"));
        assert!(output.contains("resilience_fallback_triggered_total{name=\"op1\",type=\"static\"} 1"));
        assert!(output.contains("resilience_fallback_triggered_total{name=\"op1\",type=\"function\"} 1"));
    }

    #[test]
    fn test_multiple_circuit_breakers() {
        let metrics = ResilienceMetrics::new();

        metrics.record_circuit_breaker_opened("cb_a");
        metrics.record_circuit_breaker_opened("cb_b");
        metrics.record_circuit_breaker_closed("cb_a");

        let output = metrics.gather();
        assert!(output.contains("resilience_circuit_breaker_state{name=\"cb_a\"} 0"));
        assert!(output.contains("resilience_circuit_breaker_state{name=\"cb_b\"} 2"));
        assert!(output.contains("resilience_circuit_breaker_opened_total{name=\"cb_a\"} 1"));
        assert!(output.contains("resilience_circuit_breaker_opened_total{name=\"cb_b\"} 1"));
    }
}
