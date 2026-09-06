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
    register_counter_vec_with_registry, register_gauge_vec_with_registry,
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    CounterVec, GaugeVec, HistogramVec, IntCounterVec, Registry,
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
    /// 连接池空闲连接数（标签：pool）
    pub pool_idle_connections: GaugeVec,
    /// 连接池活跃连接数（标签：pool）
    pub pool_active_connections: GaugeVec,
    /// 连接池最大连接数（标签：pool）
    pub pool_max_connections: GaugeVec,
    /// 连接池累计等待次数（标签：pool）
    pub pool_wait_total: IntCounterVec,
    /// 连接池累计超时次数（标签：pool）
    pub pool_timeout_total: IntCounterVec,
    /// 连接池累计创建连接次数（标签：pool）
    pub pool_create_total: IntCounterVec,
    /// 连接池累计丢弃连接次数（标签：pool）
    pub pool_discard_total: IntCounterVec,
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

        // 连接池指标
        let pool_idle_connections = register_gauge_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_idle_connections",
                "Number of idle connections in the pool"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_idle_connections");

        let pool_active_connections = register_gauge_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_active_connections",
                "Number of active (borrowed) connections in the pool"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_active_connections");

        let pool_max_connections = register_gauge_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_max_connections",
                "Maximum number of connections in the pool"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_max_connections");

        let pool_wait_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_wait_total",
                "Total number of connection acquisition waits"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_wait_total");

        let pool_timeout_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_timeout_total",
                "Total number of connection acquisition timeouts"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_timeout_total");

        let pool_create_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_create_total",
                "Total number of connections created"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_create_total");

        let pool_discard_total = register_int_counter_vec_with_registry!(
            prometheus::Opts::new(
                "dsql_pool_discard_total",
                "Total number of connections discarded (unhealthy/expired)"
            ),
            &["pool"],
            registry
        )
        .expect("register dsql_pool_discard_total");

        Self {
            execute_total,
            execute_duration_seconds,
            cache_hits_total,
            cache_misses_total,
            slow_queries_total,
            audit_write_total,
            pool_idle_connections,
            pool_active_connections,
            pool_max_connections,
            pool_wait_total,
            pool_timeout_total,
            pool_create_total,
            pool_discard_total,
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

    /// 记录连接池统计指标（Gauge实时更新，Counter增量更新）
    ///
    /// 注意：Counter类型指标（wait/timeout/create/discard）使用绝对值设置，
    /// 因为PoolStats中存储的是累计值，需要用set而非inc。
    pub fn record_pool_stats(&self, pool_name: &str, stats: &crate::pool::PoolStats) {
        // Gauge指标：实时状态
        self.pool_idle_connections
            .with_label_values(&[pool_name])
            .set(stats.idle_count as f64);
        self.pool_active_connections
            .with_label_values(&[pool_name])
            .set(stats.active_count as f64);
        self.pool_max_connections
            .with_label_values(&[pool_name])
            .set(stats.max_size as f64);

        // Counter指标：累计值（使用set设置绝对值）
        self.pool_wait_total
            .with_label_values(&[pool_name])
            .reset();
        self.pool_wait_total
            .with_label_values(&[pool_name])
            .inc_by(stats.wait_count as u64);

        self.pool_timeout_total
            .with_label_values(&[pool_name])
            .reset();
        self.pool_timeout_total
            .with_label_values(&[pool_name])
            .inc_by(stats.timeout_count as u64);

        self.pool_create_total
            .with_label_values(&[pool_name])
            .reset();
        self.pool_create_total
            .with_label_values(&[pool_name])
            .inc_by(stats.create_count as u64);

        self.pool_discard_total
            .with_label_values(&[pool_name])
            .reset();
        self.pool_discard_total
            .with_label_values(&[pool_name])
            .inc_by(stats.discard_count as u64);
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

    #[test]
    fn test_pool_metrics() {
        use crate::pool::PoolStats;
        let metrics = DsqlMetrics::new();

        // 构造连接池统计
        let stats = PoolStats {
            max_size: 10,
            idle_count: 3,
            active_count: 5,
            total_count: 8,
            wait_count: 12,
            timeout_count: 2,
            total_acquire_time_ns: 1_000_000_000,
            create_count: 15,
            discard_count: 3,
        };

        metrics.record_pool_stats("test_pool", &stats);

        let output = metrics.gather();
        // 验证Gauge指标
        assert!(output.contains("dsql_pool_idle_connections{pool=\"test_pool\"} 3"));
        assert!(output.contains("dsql_pool_active_connections{pool=\"test_pool\"} 5"));
        assert!(output.contains("dsql_pool_max_connections{pool=\"test_pool\"} 10"));
        // 验证Counter指标
        assert!(output.contains("dsql_pool_wait_total{pool=\"test_pool\"} 12"));
        assert!(output.contains("dsql_pool_timeout_total{pool=\"test_pool\"} 2"));
        assert!(output.contains("dsql_pool_create_total{pool=\"test_pool\"} 15"));
        assert!(output.contains("dsql_pool_discard_total{pool=\"test_pool\"} 3"));
    }

    #[test]
    fn test_pool_metrics_update() {
        use crate::pool::PoolStats;
        let metrics = DsqlMetrics::new();

        // 第一次记录
        let stats1 = PoolStats {
            max_size: 10,
            idle_count: 5,
            active_count: 3,
            total_count: 8,
            wait_count: 10,
            timeout_count: 1,
            total_acquire_time_ns: 500_000_000,
            create_count: 8,
            discard_count: 1,
        };
        metrics.record_pool_stats("pool1", &stats1);

        // 第二次记录（更新）
        let stats2 = PoolStats {
            max_size: 10,
            idle_count: 2,
            active_count: 6,
            total_count: 8,
            wait_count: 20,
            timeout_count: 3,
            total_acquire_time_ns: 2_000_000_000,
            create_count: 12,
            discard_count: 4,
        };
        metrics.record_pool_stats("pool1", &stats2);

        let output = metrics.gather();
        // 验证Gauge指标已更新
        assert!(output.contains("dsql_pool_idle_connections{pool=\"pool1\"} 2"));
        assert!(output.contains("dsql_pool_active_connections{pool=\"pool1\"} 6"));
        // 验证Counter指标已更新（绝对值）
        assert!(output.contains("dsql_pool_wait_total{pool=\"pool1\"} 20"));
        assert!(output.contains("dsql_pool_timeout_total{pool=\"pool1\"} 3"));
    }
}
