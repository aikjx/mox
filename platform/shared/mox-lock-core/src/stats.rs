// =============================================================================
// 锁统计（LockStats）
// =============================================================================
//
// 锁的统计指标，用于可观测性：
//
// - 获取次数：成功/失败/超时
// - 等待时间：平均/最大/P99
// - 持有时间：平均/最大
// - 续期次数
// - 释放次数
// - 当前持有锁数量
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 锁统计
#[derive(Debug)]
pub struct LockStats {
    /// 总获取次数
    acquire_total: AtomicU64,
    /// 成功获取次数
    acquire_success: AtomicU64,
    /// 获取超时次数
    acquire_timeout: AtomicU64,
    /// 总释放次数
    release_total: AtomicU64,
    /// 总续期次数
    renew_total: AtomicU64,
    /// 总等待时间（微秒）
    total_wait_time_us: AtomicU64,
    /// 最大等待时间（微秒）
    max_wait_time_us: AtomicU64,
}

impl LockStats {
    /// 创建新的统计实例
    pub fn new() -> Self {
        Self {
            acquire_total: AtomicU64::new(0),
            acquire_success: AtomicU64::new(0),
            acquire_timeout: AtomicU64::new(0),
            release_total: AtomicU64::new(0),
            renew_total: AtomicU64::new(0),
            total_wait_time_us: AtomicU64::new(0),
            max_wait_time_us: AtomicU64::new(0),
        }
    }

    /// 记录获取成功
    pub fn record_acquire(&self, wait_duration: Duration) {
        self.acquire_total.fetch_add(1, Ordering::SeqCst);
        self.acquire_success.fetch_add(1, Ordering::SeqCst);

        let wait_us = wait_duration.as_micros() as u64;
        self.total_wait_time_us.fetch_add(wait_us, Ordering::SeqCst);

        // 更新最大等待时间
        let mut current_max = self.max_wait_time_us.load(Ordering::SeqCst);
        while wait_us > current_max {
            match self.max_wait_time_us.compare_exchange(
                current_max,
                wait_us,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// 记录获取超时
    pub fn record_timeout(&self) {
        self.acquire_total.fetch_add(1, Ordering::SeqCst);
        self.acquire_timeout.fetch_add(1, Ordering::SeqCst);
    }

    /// 记录释放
    pub fn record_release(&self) {
        self.release_total.fetch_add(1, Ordering::SeqCst);
    }

    /// 记录续期
    pub fn record_renew(&self) {
        self.renew_total.fetch_add(1, Ordering::SeqCst);
    }

    /// 获取总获取次数
    pub fn acquire_total(&self) -> u64 {
        self.acquire_total.load(Ordering::SeqCst)
    }

    /// 获取成功次数
    pub fn acquire_success(&self) -> u64 {
        self.acquire_success.load(Ordering::SeqCst)
    }

    /// 获取超时次数
    pub fn acquire_timeout(&self) -> u64 {
        self.acquire_timeout.load(Ordering::SeqCst)
    }

    /// 获取释放次数
    pub fn release_total(&self) -> u64 {
        self.release_total.load(Ordering::SeqCst)
    }

    /// 获取续期次数
    pub fn renew_total(&self) -> u64 {
        self.renew_total.load(Ordering::SeqCst)
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.acquire_total.load(Ordering::SeqCst);
        if total == 0 {
            return 1.0;
        }
        self.acquire_success.load(Ordering::SeqCst) as f64 / total as f64
    }

    /// 获取平均等待时间
    pub fn avg_wait_time(&self) -> Duration {
        let success = self.acquire_success.load(Ordering::SeqCst);
        if success == 0 {
            return Duration::from_secs(0);
        }
        let total_us = self.total_wait_time_us.load(Ordering::SeqCst);
        Duration::from_micros(total_us / success)
    }

    /// 获取最大等待时间
    pub fn max_wait_time(&self) -> Duration {
        Duration::from_micros(self.max_wait_time_us.load(Ordering::SeqCst))
    }

    /// 获取统计快照
    pub fn snapshot(&self) -> LockStatsSnapshot {
        LockStatsSnapshot {
            acquire_total: self.acquire_total(),
            acquire_success: self.acquire_success(),
            acquire_timeout: self.acquire_timeout(),
            release_total: self.release_total(),
            renew_total: self.renew_total(),
            success_rate: self.success_rate(),
            avg_wait_time: self.avg_wait_time(),
            max_wait_time: self.max_wait_time(),
        }
    }
}

impl Default for LockStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 锁统计快照（用于导出和展示）
#[derive(Debug, Clone)]
pub struct LockStatsSnapshot {
    /// 总获取次数
    pub acquire_total: u64,
    /// 成功获取次数
    pub acquire_success: u64,
    /// 获取超时次数
    pub acquire_timeout: u64,
    /// 总释放次数
    pub release_total: u64,
    /// 总续期次数
    pub renew_total: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均等待时间
    pub avg_wait_time: Duration,
    /// 最大等待时间
    pub max_wait_time: Duration,
}

impl LockStatsSnapshot {
    /// 格式化为可读字符串
    pub fn to_readable_string(&self) -> String {
        format!(
            "锁统计: 获取总数={}, 成功={}, 超时={}, 释放={}, 续期={}, 成功率={:.2}%, 平均等待={:?}, 最大等待={:?}",
            self.acquire_total,
            self.acquire_success,
            self.acquire_timeout,
            self.release_total,
            self.renew_total,
            self.success_rate * 100.0,
            self.avg_wait_time,
            self.max_wait_time,
        )
    }
}

/// 锁指标（用于 Prometheus 等监控系统）
#[derive(Debug, Clone)]
pub struct LockMetrics {
    /// 当前活跃锁数量
    pub active_locks: u64,
    /// 等待中的请求数
    pub waiting_requests: u64,
}

impl LockMetrics {
    /// 创建新的指标实例
    pub fn new() -> Self {
        Self {
            active_locks: 0,
            waiting_requests: 0,
        }
    }
}

impl Default for LockMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_stats() {
        let stats = LockStats::new();

        // 记录获取
        stats.record_acquire(Duration::from_millis(100));
        stats.record_acquire(Duration::from_millis(200));
        stats.record_timeout();

        assert_eq!(stats.acquire_total(), 3);
        assert_eq!(stats.acquire_success(), 2);
        assert_eq!(stats.acquire_timeout(), 1);
        assert!((stats.success_rate() - 2.0 / 3.0).abs() < 0.001);

        // 记录释放和续期
        stats.record_release();
        stats.record_renew();
        assert_eq!(stats.release_total(), 1);
        assert_eq!(stats.renew_total(), 1);

        // 平均等待时间
        let avg = stats.avg_wait_time();
        assert!(avg >= Duration::from_millis(149) && avg <= Duration::from_millis(151));

        // 最大等待时间
        let max = stats.max_wait_time();
        assert!(max >= Duration::from_millis(199) && max <= Duration::from_millis(201));
    }

    #[test]
    fn test_lock_stats_snapshot() {
        let stats = LockStats::new();
        stats.record_acquire(Duration::from_millis(50));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.acquire_total, 1);
        assert_eq!(snapshot.acquire_success, 1);
        assert!(!snapshot.to_readable_string().is_empty());
    }

    #[test]
    fn test_lock_metrics() {
        let metrics = LockMetrics::new();
        assert_eq!(metrics.active_locks, 0);
        assert_eq!(metrics.waiting_requests, 0);
    }
}
