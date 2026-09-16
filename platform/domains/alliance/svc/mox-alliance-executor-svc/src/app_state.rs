// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器应用状态

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use mox_alliance_executor_core::DagEngineImpl;
use mox_alliance_executor_proto::types::ExecutorConfig;
use serde::Serialize;

/// 执行器运行指标（纯 `std::sync::atomic` 无锁计数，无外部依赖）。
///
/// 与 `mox-alliance-scheduler-core` 的 `AllianceMetrics` 同一模式：字段均为原子计数，
/// 经 `GET /metrics` 导出一致性快照 JSON。任务/节点完成计数由引擎异步调度循环产生，
/// svc 层在 HTTP 边界记录可观测事件（提交 / 取消 / 错误）。
#[derive(Debug, Default)]
pub struct ExecutorMetrics {
    /// 累计提交执行任务数
    pub tasks_submitted: AtomicU64,
    /// 累计完成任务数（由引擎调度循环回填；svc 边界当前记录提交侧）
    pub tasks_completed: AtomicU64,
    /// 累计完成节点数
    pub nodes_completed: AtomicU64,
    /// 累计错误数（handler 返回错误响应时计数）
    pub errors: AtomicU64,
    /// 累计取消任务数
    pub tasks_cancelled: AtomicU64,
}

impl ExecutorMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次执行提交成功
    pub fn record_submit(&self) {
        self.tasks_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次任务完成
    pub fn record_task_completed(&self) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次节点完成
    pub fn record_node_completed(&self) {
        self.nodes_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次错误响应
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次任务取消
    pub fn record_cancel(&self) {
        self.tasks_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    /// 读取当前一致性快照（各字段原子读取，适用于 HTTP 导出）
    pub fn snapshot(&self) -> ExecutorMetricsSnapshot {
        ExecutorMetricsSnapshot {
            tasks_submitted: self.tasks_submitted.load(Ordering::Relaxed),
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            nodes_completed: self.nodes_completed.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            tasks_cancelled: self.tasks_cancelled.load(Ordering::Relaxed),
        }
    }
}

/// 指标快照（可序列化，供 `GET /metrics` 输出 JSON）
#[derive(Debug, Clone, Serialize)]
pub struct ExecutorMetricsSnapshot {
    pub tasks_submitted: u64,
    pub tasks_completed: u64,
    pub nodes_completed: u64,
    pub errors: u64,
    pub tasks_cancelled: u64,
}

/// 执行器应用状态
#[derive(Clone)]
pub struct ExecutorAppState {
    pub config: ExecutorConfig,
    pub engine: Arc<DagEngineImpl>,
    pub execution_ready: bool,
    pub execution_mode: &'static str,
    /// 运行指标（经 `GET /metrics` 暴露）
    pub metrics: Arc<ExecutorMetrics>,
}

impl ExecutorAppState {
    pub fn new(config: ExecutorConfig, engine: Arc<DagEngineImpl>) -> Self {
        Self {
            config,
            engine,
            execution_ready: true,
            execution_mode: "custom",
            metrics: Arc::new(ExecutorMetrics::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_all_zero() {
        let m = ExecutorMetrics::new();
        let s = m.snapshot();
        assert_eq!(s.tasks_submitted, 0);
        assert_eq!(s.tasks_completed, 0);
        assert_eq!(s.nodes_completed, 0);
        assert_eq!(s.errors, 0);
        assert_eq!(s.tasks_cancelled, 0);
    }

    #[test]
    fn record_events_accumulate() {
        let m = ExecutorMetrics::new();
        m.record_submit();
        m.record_submit();
        m.record_cancel();
        m.record_error();
        m.record_task_completed();
        m.record_node_completed();
        m.record_node_completed();
        let s = m.snapshot();
        assert_eq!(s.tasks_submitted, 2);
        assert_eq!(s.tasks_cancelled, 1);
        assert_eq!(s.errors, 1);
        assert_eq!(s.tasks_completed, 1);
        assert_eq!(s.nodes_completed, 2);
    }

    #[test]
    fn snapshot_serializes_to_json() {
        let m = ExecutorMetrics::new();
        m.record_submit();
        m.record_error();
        let s = m.snapshot();
        let json = serde_json::to_string(&s).expect("serialize failed");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse failed");
        assert_eq!(v["tasks_submitted"], 1);
        assert_eq!(v["errors"], 1);
        assert!(v.get("tasks_completed").is_some());
        assert!(v.get("nodes_completed").is_some());
    }
}
