// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法联盟性能指标

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// 算法联盟性能指标
pub struct AlgoMetrics {
    /// 总执行次数
    pub total_executions: AtomicU64,
    /// 成功执行次数
    pub successful_executions: AtomicU64,
    /// 失败执行次数
    pub failed_executions: AtomicU64,
    /// 总执行时间（纳秒）
    pub total_execution_ns: AtomicU64,

    /// 按算法统计
    by_algorithm: Mutex<HashMap<String, AlgorithmMetrics>>,
}

/// 单个算法的指标
#[derive(Debug, Clone, Default)]
pub struct AlgorithmMetrics {
    /// 执行次数
    pub executions: u64,
    /// 成功次数
    pub successes: u64,
    /// 失败次数
    pub failures: u64,
    /// 总执行时间（纳秒）
    pub total_ns: u64,
    /// 最小执行时间（纳秒）
    pub min_ns: u64,
    /// 最大执行时间（纳秒）
    pub max_ns: u64,
}

impl AlgorithmMetrics {
    /// 平均执行时间（纳秒）
    pub fn avg_ns(&self) -> u64 {
        if self.executions == 0 {
            0
        } else {
            self.total_ns / self.executions
        }
    }

    /// 成功率
    pub fn success_rate(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            self.successes as f64 / self.executions as f64
        }
    }
}

impl AlgoMetrics {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            total_execution_ns: AtomicU64::new(0),
            by_algorithm: Mutex::new(HashMap::new()),
        }
    }

    /// 记录执行开始
    pub fn record_execution_start(&self, algo_id: &str) {
        self.total_executions.fetch_add(1, Ordering::SeqCst);
        let mut by_algo = self.by_algorithm.lock();
        by_algo
            .entry(algo_id.to_string())
            .or_insert_with(AlgorithmMetrics::default)
            .executions += 1;
    }

    /// 记录执行成功
    pub fn record_execution_success(&self, algo_id: &str) {
        self.successful_executions.fetch_add(1, Ordering::SeqCst);
        let mut by_algo = self.by_algorithm.lock();
        if let Some(m) = by_algo.get_mut(algo_id) {
            m.successes += 1;
        }
    }

    /// 记录执行失败
    pub fn record_execution_failure(&self, algo_id: &str, _error: &str) {
        self.failed_executions.fetch_add(1, Ordering::SeqCst);
        let mut by_algo = self.by_algorithm.lock();
        if let Some(m) = by_algo.get_mut(algo_id) {
            m.failures += 1;
        }
    }

    /// 记录执行时间
    pub fn record_execution_time(&self, algo_id: &str, duration_ns: u64) {
        self.total_execution_ns.fetch_add(duration_ns, Ordering::SeqCst);
        let mut by_algo = self.by_algorithm.lock();
        if let Some(m) = by_algo.get_mut(algo_id) {
            m.total_ns += duration_ns;
            if m.min_ns == 0 || duration_ns < m.min_ns {
                m.min_ns = duration_ns;
            }
            if duration_ns > m.max_ns {
                m.max_ns = duration_ns;
            }
        }
    }

    /// 获取指定算法的指标
    pub fn get_algorithm_metrics(&self, algo_id: &str) -> Option<AlgorithmMetrics> {
        self.by_algorithm.lock().get(algo_id).cloned()
    }

    /// 获取所有算法的指标
    pub fn all_algorithm_metrics(&self) -> HashMap<String, AlgorithmMetrics> {
        self.by_algorithm.lock().clone()
    }

    /// 总执行次数
    pub fn total_executions(&self) -> u64 {
        self.total_executions.load(Ordering::SeqCst)
    }

    /// 成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.total_executions.load(Ordering::SeqCst);
        if total == 0 {
            0.0
        } else {
            self.successful_executions.load(Ordering::SeqCst) as f64 / total as f64
        }
    }

    /// 平均执行时间（毫秒）
    pub fn avg_execution_ms(&self) -> f64 {
        let total = self.total_executions.load(Ordering::SeqCst);
        if total == 0 {
            0.0
        } else {
            self.total_execution_ns.load(Ordering::SeqCst) as f64 / total as f64 / 1_000_000.0
        }
    }

    /// 指标快照（用于监控导出）
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "algo_total_executions".to_string(),
            self.total_executions.load(Ordering::SeqCst),
        );
        m.insert(
            "algo_successful_executions".to_string(),
            self.successful_executions.load(Ordering::SeqCst),
        );
        m.insert(
            "algo_failed_executions".to_string(),
            self.failed_executions.load(Ordering::SeqCst),
        );
        m.insert(
            "algo_total_execution_ns".to_string(),
            self.total_execution_ns.load(Ordering::SeqCst),
        );
        m.insert(
            "algo_registered_count".to_string(),
            self.by_algorithm.lock().len() as u64,
        );
        m
    }

    /// 重置所有指标
    pub fn reset(&self) {
        self.total_executions.store(0, Ordering::SeqCst);
        self.successful_executions.store(0, Ordering::SeqCst);
        self.failed_executions.store(0, Ordering::SeqCst);
        self.total_execution_ns.store(0, Ordering::SeqCst);
        self.by_algorithm.lock().clear();
    }
}

impl Default for AlgoMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = AlgoMetrics::new();
        assert_eq!(metrics.total_executions(), 0);
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.avg_execution_ms(), 0.0);
    }

    #[test]
    fn test_record_execution() {
        let metrics = AlgoMetrics::new();

        metrics.record_execution_start("test.algo");
        metrics.record_execution_success("test.algo");
        metrics.record_execution_time("test.algo", 1_000_000); // 1ms

        assert_eq!(metrics.total_executions(), 1);
        assert_eq!(metrics.success_rate(), 1.0);
        assert!((metrics.avg_execution_ms() - 1.0).abs() < 0.001);

        let algo_metrics = metrics.get_algorithm_metrics("test.algo").unwrap();
        assert_eq!(algo_metrics.executions, 1);
        assert_eq!(algo_metrics.successes, 1);
        assert_eq!(algo_metrics.failures, 0);
        assert_eq!(algo_metrics.min_ns, 1_000_000);
        assert_eq!(algo_metrics.max_ns, 1_000_000);
        assert_eq!(algo_metrics.avg_ns(), 1_000_000);
    }

    #[test]
    fn test_multiple_executions() {
        let metrics = AlgoMetrics::new();

        metrics.record_execution_start("algo1");
        metrics.record_execution_success("algo1");
        metrics.record_execution_time("algo1", 1_000_000);

        metrics.record_execution_start("algo1");
        metrics.record_execution_success("algo1");
        metrics.record_execution_time("algo1", 3_000_000);

        metrics.record_execution_start("algo2");
        metrics.record_execution_failure("algo2", "error");

        assert_eq!(metrics.total_executions(), 3);
        assert_eq!(metrics.successful_executions.load(Ordering::SeqCst), 2);
        assert_eq!(metrics.failed_executions.load(Ordering::SeqCst), 1);

        let algo1 = metrics.get_algorithm_metrics("algo1").unwrap();
        assert_eq!(algo1.executions, 2);
        assert_eq!(algo1.min_ns, 1_000_000);
        assert_eq!(algo1.max_ns, 3_000_000);
        assert_eq!(algo1.avg_ns(), 2_000_000);
    }

    #[test]
    fn test_snapshot() {
        let metrics = AlgoMetrics::new();
        metrics.record_execution_start("test");
        metrics.record_execution_success("test");

        let snap = metrics.snapshot();
        assert_eq!(snap["algo_total_executions"], 1);
        assert_eq!(snap["algo_successful_executions"], 1);
        assert_eq!(snap["algo_registered_count"], 1);
    }

    #[test]
    fn test_reset() {
        let metrics = AlgoMetrics::new();
        metrics.record_execution_start("test");
        metrics.record_execution_success("test");

        assert_eq!(metrics.total_executions(), 1);
        metrics.reset();
        assert_eq!(metrics.total_executions(), 0);
        assert!(metrics.get_algorithm_metrics("test").is_none());
    }

    #[test]
    fn test_algorithm_metrics_success_rate() {
        let mut m = AlgorithmMetrics::default();
        assert_eq!(m.success_rate(), 0.0);

        m.executions = 10;
        m.successes = 8;
        m.failures = 2;
        assert_eq!(m.success_rate(), 0.8);
    }
}
