//! 弹性容错指标集成模块
//!
//! 将 mox-resilience-core 的 ResilienceMetrics 集成到 mox-server-runtime 的
//! MetricsRegistry 中，使 /metrics 端点自动包含熔断器状态、重试次数、降级触发等指标。
//!
//! 使用方式：
//! ```ignore
//! use mox_server_runtime::metrics::MetricsRegistry;
//! use mox_server_runtime::resilience_metrics::ResilienceMetricsProvider;
//! use mox_resilience_core::ResilienceMetrics;
//! use std::sync::Arc;
//!
//! let registry = MetricsRegistry::new();
//! let resilience_metrics = Arc::new(ResilienceMetrics::new());
//! let provider = ResilienceMetricsProvider::new(resilience_metrics.clone());
//! registry.register(Arc::new(provider));
//!
//! // 记录熔断状态变更
//! resilience_metrics.record_circuit_breaker_opened("my_api");
//!
//! // /metrics 端点将自动包含 resilience_circuit_breaker_state 等指标
//! ```

use crate::metrics::MetricsProvider;
use mox_resilience_core::ResilienceMetrics;
use std::sync::Arc;

/// 弹性容错指标提供者
///
/// 实现 MetricsProvider trait，将 ResilienceMetrics 的 Prometheus 输出
/// 注册到 MetricsRegistry，实现统一指标收集。
pub struct ResilienceMetricsProvider {
    metrics: Arc<ResilienceMetrics>,
}

impl ResilienceMetricsProvider {
    /// 创建弹性容错指标提供者
    pub fn new(metrics: Arc<ResilienceMetrics>) -> Self {
        Self { metrics }
    }

    /// 获取内部 ResilienceMetrics 引用
    pub fn inner(&self) -> &Arc<ResilienceMetrics> {
        &self.metrics
    }
}

impl MetricsProvider for ResilienceMetricsProvider {
    fn name(&self) -> &str {
        "resilience"
    }

    fn gather(&self) -> String {
        self.metrics.gather()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricsRegistry;

    #[test]
    fn test_provider_name() {
        let metrics = Arc::new(ResilienceMetrics::new());
        let provider = ResilienceMetricsProvider::new(metrics);
        assert_eq!(provider.name(), "resilience");
    }

    #[test]
    fn test_provider_gather() {
        let metrics = Arc::new(ResilienceMetrics::new());
        metrics.record_circuit_breaker_opened("test_cb");
        let provider = ResilienceMetricsProvider::new(metrics);
        let output = provider.gather();
        assert!(output.contains("resilience_circuit_breaker_state"));
        assert!(output.contains("resilience_circuit_breaker_opened_total"));
    }

    #[test]
    fn test_integration_with_registry() {
        let registry = MetricsRegistry::new();
        let metrics = Arc::new(ResilienceMetrics::new());
        metrics.record_circuit_breaker_opened("api_v1");
        metrics.record_retry_attempt("api_v1", 1);

        let provider = ResilienceMetricsProvider::new(metrics);
        registry.register(Arc::new(provider));

        assert_eq!(registry.len(), 1);
        assert!(registry.provider_names().contains(&"resilience".to_string()));

        let all = registry.gather_all();
        assert!(all.contains("resilience_circuit_breaker_state"));
        assert!(all.contains("resilience_circuit_breaker_opened_total"));
        assert!(all.contains("resilience_retry_attempts_total"));
    }

    #[test]
    fn test_multiple_providers_in_registry() {
        let registry = MetricsRegistry::new();

        // 注册弹性容错指标
        let resilience_metrics = Arc::new(ResilienceMetrics::new());
        resilience_metrics.record_circuit_breaker_opened("cb1");
        let resilience_provider = ResilienceMetricsProvider::new(resilience_metrics);
        registry.register(Arc::new(resilience_provider));

        // 注册自定义指标
        struct CustomProvider;
        impl MetricsProvider for CustomProvider {
            fn name(&self) -> &str {
                "custom"
            }
            fn gather(&self) -> String {
                "custom_metric 42\n".to_string()
            }
        }
        registry.register(Arc::new(CustomProvider));

        assert_eq!(registry.len(), 2);

        let all = registry.gather_all();
        assert!(all.contains("resilience_circuit_breaker_state"));
        assert!(all.contains("custom_metric 42"));
    }
}
