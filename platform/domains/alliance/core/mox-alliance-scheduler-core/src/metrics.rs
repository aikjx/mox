// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 联盟可观测性指标（Metrics）
//!
//! 无锁原子计数器，覆盖匹配、LLM 调用、融合、DAG 执行四大维度。
//! 基于 `std::sync::atomic`，无需外部依赖，可跨线程安全共享。

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

/// 联盟系统全局指标收集器
///
/// 所有字段均为原子计数器，支持并发无锁更新。
/// 典型用法：将 `Arc<AllianceMetrics>` 注入调度器 / 匹配器 / 融合引擎。
#[derive(Debug, Default)]
pub struct AllianceMetrics {
    // ── 专家匹配 ──────────────────────────────────────────────────────────
    /// 匹配请求总数
    pub match_requests: AtomicU64,
    /// 匹配错误数
    pub match_errors: AtomicU64,
    /// 匹配延迟总和（微秒）
    pub match_latency_us_sum: AtomicU64,
    /// 匹配延迟计数
    pub match_latency_us_count: AtomicU64,

    // ── LLM 调用 ──────────────────────────────────────────────────────────
    /// LLM 调用总数
    pub llm_calls: AtomicU64,
    /// LLM 错误数
    pub llm_errors: AtomicU64,
    /// LLM 延迟总和（毫秒）
    pub llm_latency_ms_sum: AtomicU64,
    /// LLM 延迟计数
    pub llm_latency_ms_count: AtomicU64,

    // ── 结果融合 ──────────────────────────────────────────────────────────
    /// 融合调用总数
    pub fusion_calls: AtomicU64,
    /// 融合延迟总和（毫秒）
    pub fusion_latency_ms_sum: AtomicU64,
    /// 融合延迟计数
    pub fusion_latency_ms_count: AtomicU64,

    // ── DAG 执行 ──────────────────────────────────────────────────────────
    /// DAG 执行次数
    pub dag_executions: AtomicU64,
    /// DAG 节点执行总次数
    pub dag_node_executions: AtomicU64,
}

impl AllianceMetrics {
    /// 创建一个全部归零的指标实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次专家匹配
    ///
    /// - `latency_us`: 匹配耗时（微秒）
    /// - `success`: `true` 表示成功，`false` 计入错误
    pub fn record_match(&self, latency_us: u64, success: bool) {
        self.match_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.match_errors.fetch_add(1, Ordering::Relaxed);
        }
        self.match_latency_us_sum
            .fetch_add(latency_us, Ordering::Relaxed);
        self.match_latency_us_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次 LLM 调用
    ///
    /// - `latency_ms`: 调用耗时（毫秒）
    /// - `success`: `true` 表示成功，`false` 计入错误
    pub fn record_llm_call(&self, latency_ms: u64, success: bool) {
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.llm_errors.fetch_add(1, Ordering::Relaxed);
        }
        self.llm_latency_ms_sum
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.llm_latency_ms_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次结果融合
    ///
    /// - `latency_ms`: 融合耗时（毫秒）
    pub fn record_fusion(&self, latency_ms: u64) {
        self.fusion_calls.fetch_add(1, Ordering::Relaxed);
        self.fusion_latency_ms_sum
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.fusion_latency_ms_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次 DAG 执行
    ///
    /// - `nodes`: 本次执行涉及的节点数
    pub fn record_dag_execution(&self, nodes: u64) {
        self.dag_executions.fetch_add(1, Ordering::Relaxed);
        self.dag_node_executions.fetch_add(nodes, Ordering::Relaxed);
    }

    /// 生成当前指标的一致性快照
    ///
    /// 单次 `snapshot()` 调用中各字段的读取不保证全局事务一致性，
    /// 但每个字段自身是原子读取。适用于监控导出与 HTTP 端点输出。
    pub fn snapshot(&self) -> MetricsSnapshot {
        let match_requests = self.match_requests.load(Ordering::Relaxed);
        let match_errors = self.match_errors.load(Ordering::Relaxed);
        let match_latency_us_sum = self.match_latency_us_sum.load(Ordering::Relaxed);
        let match_latency_us_count = self.match_latency_us_count.load(Ordering::Relaxed);

        let llm_calls = self.llm_calls.load(Ordering::Relaxed);
        let llm_errors = self.llm_errors.load(Ordering::Relaxed);
        let llm_latency_ms_sum = self.llm_latency_ms_sum.load(Ordering::Relaxed);
        let llm_latency_ms_count = self.llm_latency_ms_count.load(Ordering::Relaxed);

        let fusion_calls = self.fusion_calls.load(Ordering::Relaxed);
        let fusion_latency_ms_sum = self.fusion_latency_ms_sum.load(Ordering::Relaxed);
        let fusion_latency_ms_count = self.fusion_latency_ms_count.load(Ordering::Relaxed);

        let dag_executions = self.dag_executions.load(Ordering::Relaxed);
        let dag_node_executions = self.dag_node_executions.load(Ordering::Relaxed);

        MetricsSnapshot {
            match_requests,
            match_errors,
            match_latency_us_sum,
            match_latency_us_count,
            match_avg_latency_us: avg(match_latency_us_sum, match_latency_us_count),

            llm_calls,
            llm_errors,
            llm_latency_ms_sum,
            llm_latency_ms_count,
            llm_avg_latency_ms: avg(llm_latency_ms_sum, llm_latency_ms_count),

            fusion_calls,
            fusion_latency_ms_sum,
            fusion_latency_ms_count,
            fusion_avg_latency_ms: avg(fusion_latency_ms_sum, fusion_latency_ms_count),

            dag_executions,
            dag_node_executions,
            dag_avg_nodes_per_execution: avg(dag_node_executions, dag_executions),
        }
    }
}

/// 指标快照（可序列化，便于 HTTP JSON 输出）
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    // ── 专家匹配 ──
    pub match_requests: u64,
    pub match_errors: u64,
    pub match_latency_us_sum: u64,
    pub match_latency_us_count: u64,
    /// 平均匹配延迟（微秒），无数据时为 0
    pub match_avg_latency_us: u64,

    // ── LLM 调用 ──
    pub llm_calls: u64,
    pub llm_errors: u64,
    pub llm_latency_ms_sum: u64,
    pub llm_latency_ms_count: u64,
    /// 平均 LLM 延迟（毫秒），无数据时为 0
    pub llm_avg_latency_ms: u64,

    // ── 结果融合 ──
    pub fusion_calls: u64,
    pub fusion_latency_ms_sum: u64,
    pub fusion_latency_ms_count: u64,
    /// 平均融合延迟（毫秒），无数据时为 0
    pub fusion_avg_latency_ms: u64,

    // ── DAG 执行 ──
    pub dag_executions: u64,
    pub dag_node_executions: u64,
    /// 平均每次 DAG 执行的节点数，无数据时为 0
    pub dag_avg_nodes_per_execution: u64,
}

/// 安全求平均：除数为 0 时返回 0
fn avg(sum: u64, count: u64) -> u64 {
    if count == 0 {
        0
    } else {
        sum / count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics_all_zero() {
        let m = AllianceMetrics::new();
        let s = m.snapshot();
        assert_eq!(s.match_requests, 0);
        assert_eq!(s.match_errors, 0);
        assert_eq!(s.match_avg_latency_us, 0);
        assert_eq!(s.llm_calls, 0);
        assert_eq!(s.llm_errors, 0);
        assert_eq!(s.llm_avg_latency_ms, 0);
        assert_eq!(s.fusion_calls, 0);
        assert_eq!(s.fusion_avg_latency_ms, 0);
        assert_eq!(s.dag_executions, 0);
        assert_eq!(s.dag_node_executions, 0);
        assert_eq!(s.dag_avg_nodes_per_execution, 0);
    }

    #[test]
    fn test_record_match_success() {
        let m = AllianceMetrics::new();
        m.record_match(1500, true);
        m.record_match(2500, true);

        let s = m.snapshot();
        assert_eq!(s.match_requests, 2);
        assert_eq!(s.match_errors, 0);
        assert_eq!(s.match_latency_us_sum, 4000);
        assert_eq!(s.match_latency_us_count, 2);
        assert_eq!(s.match_avg_latency_us, 2000);
    }

    #[test]
    fn test_record_match_with_errors() {
        let m = AllianceMetrics::new();
        m.record_match(1000, true);
        m.record_match(500, false);
        m.record_match(2000, false);

        let s = m.snapshot();
        assert_eq!(s.match_requests, 3);
        assert_eq!(s.match_errors, 2);
        assert_eq!(s.match_latency_us_sum, 3500);
        assert_eq!(s.match_latency_us_count, 3);
        assert_eq!(s.match_avg_latency_us, 1166); // 3500 / 3
    }

    #[test]
    fn test_record_llm_call() {
        let m = AllianceMetrics::new();
        m.record_llm_call(120, true);
        m.record_llm_call(80, true);
        m.record_llm_call(200, false);

        let s = m.snapshot();
        assert_eq!(s.llm_calls, 3);
        assert_eq!(s.llm_errors, 1);
        assert_eq!(s.llm_latency_ms_sum, 400);
        assert_eq!(s.llm_latency_ms_count, 3);
        assert_eq!(s.llm_avg_latency_ms, 133); // 400 / 3
    }

    #[test]
    fn test_record_fusion() {
        let m = AllianceMetrics::new();
        m.record_fusion(50);
        m.record_fusion(70);
        m.record_fusion(90);

        let s = m.snapshot();
        assert_eq!(s.fusion_calls, 3);
        assert_eq!(s.fusion_latency_ms_sum, 210);
        assert_eq!(s.fusion_latency_ms_count, 3);
        assert_eq!(s.fusion_avg_latency_ms, 70);
    }

    #[test]
    fn test_record_dag_execution() {
        let m = AllianceMetrics::new();
        m.record_dag_execution(5);
        m.record_dag_execution(10);
        m.record_dag_execution(3);

        let s = m.snapshot();
        assert_eq!(s.dag_executions, 3);
        assert_eq!(s.dag_node_executions, 18);
        assert_eq!(s.dag_avg_nodes_per_execution, 6); // 18 / 3
    }

    #[test]
    fn test_snapshot_is_independent() {
        let m = AllianceMetrics::new();
        m.record_match(100, true);

        let s1 = m.snapshot();
        assert_eq!(s1.match_requests, 1);

        // 继续记录不影响已生成的快照
        m.record_match(200, false);
        let s2 = m.snapshot();
        assert_eq!(s1.match_requests, 1);
        assert_eq!(s2.match_requests, 2);
        assert_eq!(s2.match_errors, 1);
    }

    #[test]
    fn test_snapshot_serializes_to_json() {
        let m = AllianceMetrics::new();
        m.record_match(1000, true);
        m.record_llm_call(50, true);
        m.record_fusion(30);
        m.record_dag_execution(4);

        let s = m.snapshot();
        let json = serde_json::to_string(&s).expect("serialize failed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse json failed");

        assert_eq!(parsed["match_requests"], 1);
        assert_eq!(parsed["match_avg_latency_us"], 1000);
        assert_eq!(parsed["llm_calls"], 1);
        assert_eq!(parsed["llm_avg_latency_ms"], 50);
        assert_eq!(parsed["fusion_calls"], 1);
        assert_eq!(parsed["fusion_avg_latency_ms"], 30);
        assert_eq!(parsed["dag_executions"], 1);
        assert_eq!(parsed["dag_node_executions"], 4);
        assert_eq!(parsed["dag_avg_nodes_per_execution"], 4);
    }

    #[test]
    fn test_concurrent_recording() {
        use std::sync::Arc;
        use std::thread;

        let m = Arc::new(AllianceMetrics::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let m_clone = m.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    m_clone.record_match(i, true);
                    m_clone.record_llm_call(i, i % 2 == 0);
                    m_clone.record_fusion(i);
                    m_clone.record_dag_execution(i);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let s = m.snapshot();
        assert_eq!(s.match_requests, 1000);
        assert_eq!(s.llm_calls, 1000);
        assert_eq!(s.fusion_calls, 1000);
        assert_eq!(s.dag_executions, 1000);
        // 每个线程 0..100 求和 = 4950，10 个线程 = 49500
        assert_eq!(s.dag_node_executions, 49500);
    }
}
