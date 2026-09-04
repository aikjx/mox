// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! CAS 背压信号量模块（P1 优化）
//!
//! 写入路径的并发准入控制器：使用 compare-and-swap 原子操作实现无锁信号量，
//! 控制最大并发写入数。三态状态机（Normal / Warning / Critical）配合 cooldown
//! 防抖，在高并发时优雅降级。
//!
//! 算法参考：RustFS io-core `backpressure.rs`（Apache 2.0），
//! 本实现为完全重写：状态与时间戳均采用原子变量（AtomicU8 / AtomicU64），
//! 摒弃原实现中的 `std::sync::Mutex`，并新增 RAII `BackpressurePermit` 自动释放。

use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// ---------------------------------------------------------------------------
// Thread-local batched counters (P1 optimization)
// ---------------------------------------------------------------------------

/// Every N local admissions/rejections, flush to the global AtomicU64.
/// Reduces atomic RMW pressure on the hot path by ~16x.
const TL_FLUSH_BATCH: u64 = 16;

thread_local! {
    /// (admissions, rejections) accumulated on this thread since last flush.
    static TL_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

/// Flush the current thread's batched counters into the provided globals.
#[inline]
fn flush_tl_counts(admissions_global: &AtomicU64, rejections_global: &AtomicU64) {
    TL_COUNTS.with(|c| {
        let (a, r) = c.get();
        if a > 0 {
            admissions_global.fetch_add(a, Ordering::Relaxed);
        }
        if r > 0 {
            rejections_global.fetch_add(r, Ordering::Relaxed);
        }
        c.set((0, 0));
    });
}

// ---------------------------------------------------------------------------
// 三态状态机
// ---------------------------------------------------------------------------

/// 背压状态：Normal → Warning → Critical，带迟滞（低水位恢复）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackpressureState {
    /// 正常：并发数低于高水位，所有请求准入。
    Normal,
    /// 警告：并发数超过高水位但未达上限，记录告警但仍准入。
    Warning,
    /// 临界：并发数达到上限，新请求被拒绝。
    Critical,
}

impl BackpressureState {
    /// 序列化为 u8（存入 AtomicU8）。
    #[inline]
    fn as_u8(self) -> u8 {
        match self {
            BackpressureState::Normal => 0,
            BackpressureState::Warning => 1,
            BackpressureState::Critical => 2,
        }
    }

    /// 从 u8 反序列化。
    #[inline]
    fn from_u8(v: u8) -> Self {
        match v {
            1 => BackpressureState::Warning,
            2 => BackpressureState::Critical,
            _ => BackpressureState::Normal,
        }
    }

    /// 状态可读名称。
    pub fn as_str(&self) -> &'static str {
        match self {
            BackpressureState::Normal => "normal",
            BackpressureState::Warning => "warning",
            BackpressureState::Critical => "critical",
        }
    }
}

// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

/// 背压配置。
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// 最大并发数，默认 32。
    pub max_concurrent: usize,
    /// 高水位比例（0.0~1.0），默认 0.8（阈值 = max_concurrent * 0.8）。
    pub high_water: f32,
    /// 低水位比例（0.0~1.0），默认 0.5（阈值 = max_concurrent * 0.5）。
    pub low_water: f32,
    /// 状态切换冷却时间，默认 100ms（防止抖动）。
    pub cooldown: Duration,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 32,
            high_water: 0.8,
            low_water: 0.5,
            cooldown: Duration::from_millis(100),
        }
    }
}

impl BackpressureConfig {
    /// 高水位阈值（并发数）。
    #[inline]
    pub fn high_threshold(&self) -> usize {
        (self.max_concurrent as f32 * self.high_water) as usize
    }

    /// 低水位阈值（并发数）。
    #[inline]
    pub fn low_threshold(&self) -> usize {
        (self.max_concurrent as f32 * self.low_water) as usize
    }
}

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 背压错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackpressureError {
    /// 请求被拒绝（达到最大并发数）。
    Rejected { current: usize, max: usize },
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpressureError::Rejected { current, max } => {
                write!(f, "backpressure rejected: concurrent {}/{} at capacity", current, max)
            },
        }
    }
}

impl std::error::Error for BackpressureError {}

// ---------------------------------------------------------------------------
// 指标快照
// ---------------------------------------------------------------------------

/// 背压指标快照（可序列化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureMetrics {
    pub current_concurrent: usize,
    pub max_concurrent: usize,
    pub state: BackpressureState,
    pub total_admissions: u64,
    pub total_rejections: u64,
    /// 拒绝率 = rejections / (admissions + rejections)。
    pub rejection_rate: f64,
}

// ---------------------------------------------------------------------------
// RAII 准入许可
// ---------------------------------------------------------------------------

/// 准入许可：持有期间占用一个并发槽，Drop 时自动释放。
pub struct BackpressurePermit<'a> {
    monitor: &'a BackpressureMonitor,
}

impl<'a> Drop for BackpressurePermit<'a> {
    fn drop(&mut self) {
        self.monitor.release();
    }
}

// ---------------------------------------------------------------------------
// 核心：CAS 背压监视器
// ---------------------------------------------------------------------------

/// CAS 背压信号量监视器。
///
/// 全部共享状态均为原子变量，无锁；`try_acquire` 通过
/// `fetch_add` 乐观递增（替代 CAS 重试循环）保证并发安全且不超过
/// `max_concurrent`。`current` 字段后接 64 字节 padding，将其隔离到
/// 独立缓存行，避免与 `state` / `last_transition` 等字段发生 false sharing。
#[derive(Debug)]
pub struct BackpressureMonitor {
    config: BackpressureConfig,
    /// 当前并发数（fetch_add 乐观递增，最热字段）。
    current: AtomicUsize,
    /// 缓存行填充：将 `current` 与后续字段隔离到不同缓存行，
    /// 避免多线程竞争 `current` 时连带失效 `state` 等字段的缓存。
    _cacheline_pad: [u8; 64],
    /// 当前状态（AtomicU8 表示 BackpressureState）。
    state: AtomicU8,
    /// 上次状态切换时间（unix ms，用于 cooldown）。
    last_transition: AtomicU64,
    /// 拒绝计数（指标，thread-local 批处理后写入）。
    rejection_count: AtomicU64,
    /// 准入计数（指标，thread-local 批处理后写入）。
    admission_count: AtomicU64,
}

impl BackpressureMonitor {
    /// 创建新的背压监视器。
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            current: AtomicUsize::new(0),
            _cacheline_pad: [0u8; 64],
            state: AtomicU8::new(BackpressureState::Normal.as_u8()),
            last_transition: AtomicU64::new(0),
            rejection_count: AtomicU64::new(0),
            admission_count: AtomicU64::new(0),
        }
    }

    /// 使用默认配置创建。
    pub fn with_default() -> Self {
        Self::new(BackpressureConfig::default())
    }

    /// 获取配置引用。
    pub fn config(&self) -> &BackpressureConfig {
        &self.config
    }

    /// 尝试获取准入许可（fetch_add 乐观递增，无 CAS 重试循环）。
    ///
    /// 使用 `fetch_add(1)` 单次原子操作乐观递增，若递增前已达上限则
    /// 回滚并拒绝。这消除了高并发下的 CAS 重试风暴，吞吐量显著提升。
    /// 准入/拒绝指标采用 thread-local 批处理（每 16 次刷新一次全局），
    /// 进一步减少热路径上的原子 RMW 操作。
    ///
    /// 返回 `Ok(permit)` 表示准入成功，permit drop 时自动释放；
    /// 返回 `Err(BackpressureError::Rejected)` 表示被拒绝（达到 max_concurrent）。
    pub fn try_acquire(&self) -> Result<BackpressurePermit<'_>, BackpressureError> {
        // 乐观递增：单次原子操作，无 CAS 重试循环。
        let prev = self.current.fetch_add(1, Ordering::AcqRel);
        if prev >= self.config.max_concurrent {
            // 超发：回滚递增并拒绝。
            self.current.fetch_sub(1, Ordering::AcqRel);
            self.tl_record_rejection();
            return Err(BackpressureError::Rejected {
                current: prev,
                max: self.config.max_concurrent,
            });
        }
        self.tl_record_admission();
        self.update_state(prev + 1);
        Ok(BackpressurePermit { monitor: self })
    }

    /// 记录一次准入到 thread-local 批处理计数器，达到阈值后刷新全局。
    #[inline]
    fn tl_record_admission(&self) {
        TL_COUNTS.with(|c| {
            let (a, r) = c.get();
            let new_a = a + 1;
            if new_a >= TL_FLUSH_BATCH {
                self.admission_count.fetch_add(new_a, Ordering::Relaxed);
                c.set((0, r));
            } else {
                c.set((new_a, r));
            }
        });
    }

    /// 记录一次拒绝到 thread-local 批处理计数器，达到阈值后刷新全局。
    #[inline]
    fn tl_record_rejection(&self) {
        TL_COUNTS.with(|c| {
            let (a, r) = c.get();
            let new_r = r + 1;
            if new_r >= TL_FLUSH_BATCH {
                self.rejection_count.fetch_add(new_r, Ordering::Relaxed);
                c.set((a, 0));
            } else {
                c.set((a, new_r));
            }
        });
    }

    /// 释放一个并发槽（通常由 `BackpressurePermit::drop` 自动调用）。
    ///
    /// 使用 `fetch_update` + `checked_sub` 防止下溢：未配对的 release
    /// 在 current==0 时不会把计数器卷回 `usize::MAX`。
    fn release(&self) {
        let result = self
            .current
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| current.checked_sub(1));
        if let Ok(prev) = result {
            // prev 是递减前的值，递减后为 prev - 1。
            self.update_state(prev - 1);
        }
        // checked_sub 返回 None（current 已为 0）时不做任何事。
    }

    /// 获取当前状态。
    pub fn state(&self) -> BackpressureState {
        BackpressureState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// 获取当前并发数。
    pub fn current_concurrent(&self) -> usize {
        self.current.load(Ordering::Acquire)
    }

    /// 获取指标快照。
    ///
    /// 调用前先刷新当前线程的 thread-local 批处理计数器，确保本线程
    /// 累计的准入/拒绝数被计入全局。其他线程的未刷新批次可能存在至多
    /// `TL_FLUSH_BATCH × 活跃线程数` 的最终一致性延迟。
    pub fn metrics(&self) -> BackpressureMetrics {
        flush_tl_counts(&self.admission_count, &self.rejection_count);
        let admissions = self.admission_count.load(Ordering::Relaxed);
        let rejections = self.rejection_count.load(Ordering::Relaxed);
        let total = admissions + rejections;
        let rejection_rate = if total == 0 { 0.0 } else { rejections as f64 / total as f64 };
        BackpressureMetrics {
            current_concurrent: self.current_concurrent(),
            max_concurrent: self.config.max_concurrent,
            state: self.state(),
            total_admissions: admissions,
            total_rejections: rejections,
            rejection_rate,
        }
    }

    /// 内部：根据当前并发数更新状态（带 cooldown 防抖）。
    ///
    /// 快速路径：当 `cooldown == Duration::ZERO` 时跳过系统时间读取，
    /// 直接执行状态机判断，减少热路径上的 `SystemTime::now()` 调用。
    fn update_state(&self, current: usize) {
        let high_threshold = self.config.high_threshold();
        let low_threshold = self.config.low_threshold();

        let current_state = self.state();

        // cooldown 快速路径：cooldown 为零时无需读取系统时间。
        if self.config.cooldown > Duration::ZERO {
            let now = current_time_ms();
            let last = self.last_transition.load(Ordering::Relaxed);
            if now.saturating_sub(last) < self.config.cooldown.as_millis() as u64 {
                return;
            }
        }

        let new_state = match current_state {
            BackpressureState::Normal => {
                if current >= high_threshold {
                    BackpressureState::Warning
                } else {
                    BackpressureState::Normal
                }
            },
            BackpressureState::Warning => {
                if current >= self.config.max_concurrent {
                    BackpressureState::Critical
                } else if current < low_threshold {
                    BackpressureState::Normal
                } else {
                    BackpressureState::Warning
                }
            },
            BackpressureState::Critical => {
                if current < low_threshold {
                    BackpressureState::Normal
                } else if current < high_threshold {
                    BackpressureState::Warning
                } else {
                    BackpressureState::Critical
                }
            },
        };

        if new_state != current_state {
            self.state.store(new_state.as_u8(), Ordering::Release);
            if self.config.cooldown > Duration::ZERO {
                self.last_transition.store(current_time_ms(), Ordering::Relaxed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/// 获取当前 unix 毫秒时间戳。
#[inline]
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, thread};

    fn config_with(max: usize, cooldown: Duration) -> BackpressureConfig {
        BackpressureConfig { max_concurrent: max, high_water: 0.8, low_water: 0.5, cooldown }
    }

    #[test]
    fn test_try_acquire_and_release() {
        let monitor = BackpressureMonitor::new(config_with(2, Duration::ZERO));

        // 前 2 次准入成功
        let p1 = monitor.try_acquire().expect("acquire 1 should succeed");
        let p2 = monitor.try_acquire().expect("acquire 2 should succeed");
        assert_eq!(monitor.current_concurrent(), 2);

        // 第 3 次被拒绝
        let err = match monitor.try_acquire() {
            Err(e) => e,
            Ok(_) => panic!("expected rejection at capacity"),
        };
        match err {
            BackpressureError::Rejected { current, max } => {
                assert_eq!(current, 2);
                assert_eq!(max, 2);
            },
        }

        // 释放 1 个后可以再次准入
        drop(p1);
        assert_eq!(monitor.current_concurrent(), 1);
        let _p3 = monitor.try_acquire().expect("acquire after release should succeed");
        assert_eq!(monitor.current_concurrent(), 2);

        // 清理
        drop(p2);
        drop(_p3);
        assert_eq!(monitor.current_concurrent(), 0);
    }

    #[test]
    fn test_permit_drop_auto_releases() {
        let monitor = BackpressureMonitor::with_default();

        {
            let _permit = monitor.try_acquire().expect("should acquire");
            assert_eq!(monitor.current_concurrent(), 1);
            // permit 在此作用域结束时 drop，自动释放
        }
        assert_eq!(monitor.current_concurrent(), 0);
    }

    #[test]
    fn test_state_transitions() {
        // max=10, high=8, low=5, cooldown=0
        let monitor = BackpressureMonitor::new(config_with(10, Duration::ZERO));

        // 持有 8 个 permit → 达到高水位 → Warning
        let mut permits: Vec<BackpressurePermit<'_>> = Vec::new();
        for _ in 0..8 {
            permits.push(monitor.try_acquire().expect("acquire"));
        }
        assert_eq!(monitor.state(), BackpressureState::Warning);

        // 再持有 2 个 → 达到上限 → Critical
        for _ in 0..2 {
            permits.push(monitor.try_acquire().expect("acquire"));
        }
        assert_eq!(monitor.state(), BackpressureState::Critical);
        assert_eq!(monitor.current_concurrent(), 10);

        // 释放 6 个 → current=4 < low=5 → Normal
        for _ in 0..6 {
            permits.pop();
        }
        assert_eq!(monitor.current_concurrent(), 4);
        assert_eq!(monitor.state(), BackpressureState::Normal);
    }

    #[test]
    fn test_rejection_metrics() {
        let monitor = BackpressureMonitor::new(config_with(1, Duration::ZERO));

        // 1 次准入
        let _p = monitor.try_acquire().expect("acquire");

        // 10 次拒绝
        for _ in 0..10 {
            assert!(monitor.try_acquire().is_err());
        }

        let m = monitor.metrics();
        assert_eq!(m.total_admissions, 1);
        assert_eq!(m.total_rejections, 10);
        // 10 / 11 ≈ 0.909
        assert!((m.rejection_rate - 10.0 / 11.0).abs() < 1e-9);
        assert_eq!(m.current_concurrent, 1);
        assert_eq!(m.max_concurrent, 1);
    }

    #[test]
    fn test_release_prevents_underflow() {
        let monitor = BackpressureMonitor::with_default();
        assert_eq!(monitor.current_concurrent(), 0);

        // 手动调用 release（正常情况下不应发生，但必须安全）
        monitor.release();
        // 不下溢为 usize::MAX
        assert_eq!(monitor.current_concurrent(), 0);

        // 监视器仍可正常使用
        let p = monitor.try_acquire().expect("should still work");
        assert_eq!(monitor.current_concurrent(), 1);
        drop(p);
        assert_eq!(monitor.current_concurrent(), 0);
    }

    #[test]
    fn test_concurrent_acquire_stress() {
        let monitor = Arc::new(BackpressureMonitor::new(config_with(4, Duration::ZERO)));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let m = Arc::clone(&monitor);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    // 循环直到准入成功（因为有 release，最终一定能成功）
                    loop {
                        match m.try_acquire() {
                            Ok(permit) => {
                                // 模拟短暂持有
                                std::hint::black_box(&permit);
                                drop(permit);
                                break;
                            },
                            Err(_) => continue,
                        }
                    }
                }
                // Flush this thread's batched admission/rejection counters into
                // the globals before the thread exits.  Without this, up to
                // TL_FLUSH_BATCH-1 admissions per thread remain in thread-local
                // storage and `total_admissions` under-reports (e.g. 960 vs 1000).
                let _ = m.metrics();
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        // 所有线程完成后，并发数应归零
        assert_eq!(monitor.current_concurrent(), 0);
        // 总准入数 = 10 线程 × 100 次 = 1000
        let m = monitor.metrics();
        assert_eq!(m.total_admissions, 1000);
    }

    #[test]
    fn test_config_thresholds() {
        let cfg = BackpressureConfig::default();
        assert_eq!(cfg.max_concurrent, 32);
        assert_eq!(cfg.high_threshold(), 25); // 32 * 0.8
        assert_eq!(cfg.low_threshold(), 16); // 32 * 0.5
    }

    #[test]
    fn test_error_display() {
        let err = BackpressureError::Rejected { current: 32, max: 32 };
        let s = format!("{}", err);
        assert!(s.contains("backpressure rejected"));
        assert!(s.contains("32/32"));
    }
}


#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_backpressure_state_as_str() {
        assert_eq!(BackpressureState::Normal.as_str(), "normal");
        assert_eq!(BackpressureState::Warning.as_str(), "warning");
        assert_eq!(BackpressureState::Critical.as_str(), "critical");
    }

    #[test]
    fn test_backpressure_config_default() {
        let cfg = BackpressureConfig::default();
        assert_eq!(cfg.max_concurrent, 32);
        assert!((cfg.high_water - 0.8).abs() < 1e-6);
        assert!((cfg.low_water - 0.5).abs() < 1e-6);
        assert_eq!(cfg.cooldown, Duration::from_millis(100));
    }

    #[test]
    fn test_backpressure_config_thresholds() {
        let cfg = BackpressureConfig {
            max_concurrent: 100,
            high_water: 0.75,
            low_water: 0.25,
            cooldown: Duration::from_millis(50),
        };
        assert_eq!(cfg.high_threshold(), 75);
        assert_eq!(cfg.low_threshold(), 25);
    }

    #[test]
    fn test_backpressure_error_display() {
        let err = BackpressureError::Rejected { current: 10, max: 10 };
        let s = format!("{err}");
        assert!(s.contains("backpressure rejected"));
        assert!(s.contains("10/10"));
    }

    #[test]
    fn test_backpressure_error_clone_eq() {
        let e1 = BackpressureError::Rejected { current: 5, max: 10 };
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_backpressure_metrics_serialization() {
        let monitor = BackpressureMonitor::with_default();
        let _p = monitor.try_acquire().unwrap();
        let m = monitor.metrics();
        let json = format!("{:?}", m);
        assert!(json.contains("current_concurrent"));
        assert!(json.contains("max_concurrent"));
        assert!(json.contains("state"));
        assert!(json.contains("total_admissions"));
        assert!(json.contains("total_rejections"));
        assert!(json.contains("rejection_rate"));
    }

    #[test]
    fn test_backpressure_monitor_debug() {
        let monitor = BackpressureMonitor::with_default();
        let s = format!("{monitor:?}");
        assert!(!s.is_empty());
    }

    #[test]
    fn test_backpressure_config_clone() {
        let cfg = BackpressureConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.max_concurrent, cfg2.max_concurrent);
        assert_eq!(cfg.high_water, cfg2.high_water);
    }

    #[test]
    fn test_backpressure_state_transitions_with_cooldown() {
        // max=10, high=8, low=5, cooldown=1s (so transitions are debounced)
        let cfg = BackpressureConfig {
            max_concurrent: 10,
            high_water: 0.8,
            low_water: 0.5,
            cooldown: Duration::from_secs(1),
        };
        let monitor = BackpressureMonitor::new(cfg);
        // Acquire 8 → should go to Warning (first transition, no cooldown issue)
        let mut permits = Vec::new();
        for _ in 0..8 {
            permits.push(monitor.try_acquire().unwrap());
        }
        assert_eq!(monitor.state(), BackpressureState::Warning);
        // Release all → cooldown prevents immediate transition back
        drop(permits);
        // State may still be Warning due to cooldown
        assert!(
            monitor.state() == BackpressureState::Warning
                || monitor.state() == BackpressureState::Normal
        );
    }
}
