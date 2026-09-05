//! DSQL 执行 Prometheus 指标模块
//!
//! 提供动态SQL执行的全维度可观测性指标：
//! - 执行次数/成功率（按 sql_code、operation_type 维度）
//! - 执行耗时分布（直方图）
//! - 缓存命中率
//! - 慢查询计数
//! - 审计写入统计
//!
//! 所有指标使用 lazy_static 全局注册，可通过 `gather_metrics()` 输出 Prometheus 文本格式。

use prometheus::{
    register_counter_vec_with_registry, register_histogram_vec_with_registry,
    register_int_counter_vec_with_registry, CounterVec, HistogramVec, IntCounterVec, Registry,
};
use std::time::Duration;

/// DSQL 指标集合
pub struct DsqlMetrics {
    /// 执行总次数（标签：sql_code, operation_type, success）
    pub execute_total: CounterVec,
    /// 执行耗时直方图（秒，标签：sql_code, operation_type）
    pub execute_duration_seconds: HistogramVec,
    /// 缓存命中次数（标签：sql_code）
    pub cache_hits_total: IntCounterVec,
    /// 缓存未命中次数（标签：sql_code）
    pub cache_misses_total: IntCounterVec,
    /// 慢查询次数（标签：sql_code）
    pub slow_queries_total: IntCounterVec,
    /// 审计写入次数（标签：result: success/failed）
    pub audit_write_total: IntCounterVec,
    /// 指标注册表
    registry: Registry,
}

impl DsqlMetrics {
    /// 创建并注册所有指标
    pub fn new() -> Self {
        let registry = Registry::new();

        let execute_total = register_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_execute_total",
                "Total number of DSQL executions"
            ),
            &["sql_code", "operation_type", "success"],
            registry
        )
        .expect("register dsql_execute_total");

        // 耗时直方图桶：1ms ~ 30s，覆盖快速查询到慢查询
        let duration_buckets = prometheus::exponential_buckets(0.001, 2.0, 15)
            .unwrap_or_else(|_| vec![0.001, 0.01, 0.1, 1.0, 10.0]);

        let execute_duration_seconds = register_histogram_vec_with_registry!(
            prometheus::HistogramOpts::new(
                "dsql_execute_duration_seconds",
                "DSQL execution duration in seconds"
            )
            .buckets(duration_buckets),
            &["sql_code", "operation_type"],
            registry
        )
        .expect("register dsql_execute_duration_seconds");

        let cache_hits_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_cache_hits_total",
                "Total number of DSQL cache hits"
            ),
            &["sql_code"],
            registry
        )
        .expect("register dsql_cache_hits_total");

        let cache_misses_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_cache_misses_total",
                "Total number of DSQL cache misses"
            ),
            &["sql_code"],
            registry
        )
        .expect("register dsql_cache_misses_total");

        let slow_queries_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_slow_queries_total",
                "Total number of DSQL slow queries"
            ),
            &["sql_code"],
            registry
        )
        .expect("register dsql_slow_queries_total");

        let audit_write_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_audit_write_total",
                "Total number of DSQL audit log writes"
            ),
            &["result"],
            registry
        )
        .expect("register dsql_audit_write_total");

        Self {
            execute_total,
            execute_duration_seconds,
            cache_hits_total,
            cache_misses_total,
            slow_queries_total,
            audit_write_total,
            registry,
        }
    }

    /// 记录一次执行
    pub fn record_execution(
        &self,
        sql_code: &str,
        operation_type: &str,
        success: bool,
        duration: Duration,
    ) {
        let success_str = if success { "true" } else { "false" };
        self.execute_total
            .with_label_values(&[sql_code, operation_type, success_str])
            .inc();
        self.execute_duration_seconds
            .with_label_values(&[sql_code, operation_type])
            .observe(duration.as_secs_f64());
    }

    /// 记录缓存命中
    pub fn record_cache_hit(&self, sql_code: &str) {
        self.cache_hits_total
            .with_label_values(&[sql_code])
            .inc();
    }

    /// 记录缓存未命中
    pub fn record_cache_miss(&self, sql_code: &str) {
        self.cache_misses_total
            .with_label_values(&[sql_code])
            .inc();
    }

    /// 记录慢查询
    pub fn record_slow_query(&self, sql_code: &str) {
        self.slow_queries_total
            .with_label_values(&[sql_code])
            .inc();
    }

    /// 记录审计写入结果
    pub fn record_audit_write(&self, success: bool) {
        let result = if success { "success" } else { "failed" };
        self.audit_write_total
            .with_label_values(&[result])
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

impl Default for DsqlMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = DsqlMetrics::new();
        // 先记录一些指标，确保 Prometheus 输出包含这些指标
        metrics.record_execution("test_sql", "read", true, Duration::from_millis(10));
        metrics.record_cache_hit("test_sql");
        metrics.record_cache_miss("test_sql");
        metrics.record_slow_query("test_sql");
        metrics.record_audit_write(true);
        let output = metrics.gather();
        assert!(output.contains("dsql_execute_total"));
        assert!(output.contains("dsql_execute_duration_seconds"));
        assert!(output.contains("dsql_cache_hits_total"));
        assert!(output.contains("dsql_cache_misses_total"));
        assert!(output.contains("dsql_slow_queries_total"));
        assert!(output.contains("dsql_audit_write_total"));
    }

    #[test]
    fn test_record_execution() {
        let metrics = DsqlMetrics::new();
        metrics.record_execution("test_sql", "read", true, Duration::from_millis(50));
        metrics.record_execution("test_sql", "write", false, Duration::from_millis(200));

        let output = metrics.gather();
        assert!(output.contains("dsql_execute_total{operation_type=\"read\",sql_code=\"test_sql\",success=\"true\"} 1"));
        assert!(output.contains("dsql_execute_total{operation_type=\"write\",sql_code=\"test_sql\",success=\"false\"} 1"));
    }

    #[test]
    fn test_cache_metrics() {
        let metrics = DsqlMetrics::new();
        metrics.record_cache_hit("sql_a");
        metrics.record_cache_hit("sql_a");
        metrics.record_cache_miss("sql_a");
        metrics.record_cache_miss("sql_b");

        let output = metrics.gather();
        assert!(output.contains("dsql_cache_hits_total{sql_code=\"sql_a\"} 2"));
        assert!(output.contains("dsql_cache_misses_total{sql_code=\"sql_a\"} 1"));
        assert!(output.contains("dsql_cache_misses_total{sql_code=\"sql_b\"} 1"));
    }

    #[test]
    fn test_slow_query_and_audit() {
        let metrics = DsqlMetrics::new();
        metrics.record_slow_query("slow_sql");
        metrics.record_audit_write(true);
        metrics.record_audit_write(true);
        metrics.record_audit_write(false);

        let output = metrics.gather();
        assert!(output.contains("dsql_slow_queries_total{sql_code=\"slow_sql\"} 1"));
        assert!(output.contains("dsql_audit_write_total{result=\"success\"} 2"));
        assert!(output.contains("dsql_audit_write_total{result=\"failed\"} 1"));
    }

    #[test]
    fn test_duration_histogram_buckets() {
        let metrics = DsqlMetrics::new();
        // 记录不同耗时的查询
        metrics.record_execution("sql1", "read", true, Duration::from_millis(5));
        metrics.record_execution("sql1", "read", true, Duration::from_millis(50));
        metrics.record_execution("sql1", "read", true, Duration::from_millis(500));
        metrics.record_execution("sql1", "read", true, Duration::from_secs(5));

        let output = metrics.gather();
        // 验证直方图计数
        assert!(output.contains("dsql_execute_duration_seconds_count{operation_type=\"read\",sql_code=\"sql1\"} 4"));
        // 验证总和
        assert!(output.contains("dsql_execute_duration_seconds_sum{operation_type=\"read\",sql_code=\"sql1\"}"));
    }
}
