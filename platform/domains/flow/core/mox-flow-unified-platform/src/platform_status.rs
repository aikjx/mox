// Copyright (c) 2026 璇玑 RelGraph · mox 模块化系统架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台状态监控
//!
//! 实时监控平台各模块状态、性能指标和健康度

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

/// 平台指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformMetrics {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_ms: f64,
    /// P99 响应时间（毫秒）
    pub p99_response_ms: f64,
    /// 当前并发数
    pub current_concurrency: u32,
    /// 峰值并发数
    pub peak_concurrency: u32,
}

/// 模块指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetrics {
    /// 模块 ID
    pub module_id: String,
    /// 调用次数
    pub call_count: u64,
    /// 错误次数
    pub error_count: u64,
    /// 平均耗时（毫秒）
    pub avg_latency_ms: f64,
}

/// 平台状态监控器
pub struct PlatformStatusMonitor {
    /// 平台指标
    metrics: RwLock<PlatformMetrics>,
    /// 各模块指标
    module_metrics: RwLock<HashMap<String, ModuleMetrics>>,
    /// 启动时间点
    start_time: Instant,
    /// 活跃会话数
    active_sessions: AtomicU64,
}

impl PlatformStatusMonitor {
    /// 创建监控器
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(PlatformMetrics::default()),
            module_metrics: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
            active_sessions: AtomicU64::new(0),
        }
    }

    /// 注册模块监控
    pub fn register_module(&self, module_id: &str) {
        self.module_metrics.write().insert(
            module_id.to_string(),
            ModuleMetrics {
                module_id: module_id.to_string(),
                call_count: 0,
                error_count: 0,
                avg_latency_ms: 0.0,
            },
        );
    }

    /// 记录一次请求
    pub fn record_request(&self, success: bool, duration_ms: f64) {
        let mut metrics = self.metrics.write();
        metrics.total_requests += 1;

        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // 滑动平均响应时间
        if metrics.total_requests == 1 {
            metrics.avg_response_ms = duration_ms;
        } else {
            metrics.avg_response_ms = metrics.avg_response_ms * 0.99 + duration_ms * 0.01;
        }
    }

    /// 记录模块调用
    pub fn record_module_call(&self, module_id: &str, duration_ms: f64, success: bool) {
        let mut module_metrics = self.module_metrics.write();
        if let Some(m) = module_metrics.get_mut(module_id) {
            m.call_count += 1;
            if !success {
                m.error_count += 1;
            }
            // 滑动平均
            if m.call_count == 1 {
                m.avg_latency_ms = duration_ms;
            } else {
                m.avg_latency_ms = m.avg_latency_ms * 0.9 + duration_ms * 0.1;
            }
        }
    }

    /// 增加活跃会话
    pub fn inc_session(&self) -> u64 {
        self.active_sessions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }

    /// 减少活跃会话
    pub fn dec_session(&self) -> u64 {
        let current = self
            .active_sessions
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        if current == 0 {
            self.active_sessions
                .store(0, std::sync::atomic::Ordering::Relaxed);
            0
        } else {
            current - 1
        }
    }

    /// 获取平台指标
    pub fn get_metrics(&self) -> PlatformMetrics {
        self.metrics.read().clone()
    }

    /// 获取模块指标
    pub fn get_module_metrics(&self, module_id: &str) -> Option<ModuleMetrics> {
        self.module_metrics.read().get(module_id).cloned()
    }

    /// 获取所有模块指标
    pub fn all_module_metrics(&self) -> Vec<ModuleMetrics> {
        self.module_metrics
            .read()
            .values()
            .cloned()
            .collect()
    }

    /// 获取运行时长（秒）
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 获取活跃会话数
    pub fn active_sessions(&self) -> u64 {
        self.active_sessions
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 成功率
    pub fn success_rate(&self) -> f64 {
        let metrics = self.metrics.read();
        if metrics.total_requests == 0 {
            return 1.0;
        }
        metrics.successful_requests as f64 / metrics.total_requests as f64
    }
}

impl Default for PlatformStatusMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_request() {
        let monitor = PlatformStatusMonitor::new();
        monitor.record_request(true, 50.0);
        monitor.record_request(true, 30.0);
        monitor.record_request(false, 100.0);

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[test]
    fn test_module_metrics() {
        let monitor = PlatformStatusMonitor::new();
        monitor.register_module("mod-a");

        monitor.record_module_call("mod-a", 20.0, true);
        monitor.record_module_call("mod-a", 30.0, true);
        monitor.record_module_call("mod-a", 100.0, false);

        let m = monitor.get_module_metrics("mod-a").unwrap();
        assert_eq!(m.call_count, 3);
        assert_eq!(m.error_count, 1);
    }

    #[test]
    fn test_sessions() {
        let monitor = PlatformStatusMonitor::new();
        assert_eq!(monitor.active_sessions(), 0);

        monitor.inc_session();
        monitor.inc_session();
        assert_eq!(monitor.active_sessions(), 2);

        monitor.dec_session();
        assert_eq!(monitor.active_sessions(), 1);
    }

    #[test]
    fn test_success_rate() {
        let monitor = PlatformStatusMonitor::new();
        assert_eq!(monitor.success_rate(), 1.0);

        monitor.record_request(true, 10.0);
        monitor.record_request(false, 20.0);
        assert_eq!(monitor.success_rate(), 0.5);
    }

    #[test]
    fn test_all_module_metrics() {
        let monitor = PlatformStatusMonitor::new();
        monitor.register_module("a");
        monitor.register_module("b");

        let all = monitor.all_module_metrics();
        assert_eq!(all.len(), 2);
    }
}
