// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 三维扫描预算模块（时间 / IO / 容量）
//!
//! 为生命周期扫描和其他后台扫描任务提供统一的预算控制：
//! - **时间预算**：扫描窗口（每日时段）、单次扫描最大时长
//! - **IO 预算**：每秒最大扫描对象数（令牌桶限速）、每秒最大 IO 操作数、最大并发
//! - **容量预算**：单次扫描最大处理字节数、最大迁移字节数、最大对象数
//!
//! 算法参考：RustFS scanner 三维预算控制（rate / parallelism / bytes、
//! CancellationToken、断点续扫），Apache 2.0，`ais/RustFS/crates/scanner/src/`。
//! 本实现为完全自研重写：采用令牌桶（token bucket）限速 + 原子计数器统计，
//! 未直接复制 RustFS 源码。

use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

// ---------------------------------------------------------------------------
// 三维预算配置
// ---------------------------------------------------------------------------

/// 三维扫描预算配置（时间 / IO / 容量）
#[derive(Debug, Clone)]
pub struct ScanBudget {
    /// 时间预算
    pub time: TimeBudget,
    /// IO 预算
    pub io: IoBudget,
    /// 容量预算
    pub capacity: CapacityBudget,
}

/// 时间预算
#[derive(Debug, Clone, Default)]
pub struct TimeBudget {
    /// 单次扫描最大时长（None = 不限制）
    pub max_duration: Option<Duration>,
    /// 扫描窗口起始小时（0-23），None = 不限制窗口
    pub window_start_hour: Option<u8>,
    /// 扫描窗口结束小时（0-23）
    pub window_end_hour: Option<u8>,
}

/// IO 预算
#[derive(Debug, Clone)]
pub struct IoBudget {
    /// 每秒最大扫描对象数（0 = 不限制）
    pub max_objects_per_sec: u32,
    /// 每秒最大 IO 操作数（0 = 不限制）
    pub max_io_per_sec: u32,
    /// 最大并发扫描数
    pub max_parallelism: usize,
}

/// 容量预算
#[derive(Debug, Clone, Default)]
pub struct CapacityBudget {
    /// 单次扫描最大处理字节数（0 = 不限制）
    pub max_bytes_per_scan: u64,
    /// 单次扫描最大迁移字节数（0 = 不限制）
    pub max_migration_bytes: u64,
    /// 单次扫描最大对象数（0 = 不限制）
    pub max_objects_per_scan: u64,
}

impl Default for ScanBudget {
    fn default() -> Self {
        Self {
            time: TimeBudget { max_duration: None, window_start_hour: None, window_end_hour: None },
            io: IoBudget { max_objects_per_sec: 0, max_io_per_sec: 0, max_parallelism: 4 },
            capacity: CapacityBudget {
                max_bytes_per_scan: 0,
                max_migration_bytes: 0,
                max_objects_per_scan: 0,
            },
        }
    }
}


impl Default for IoBudget {
    fn default() -> Self {
        Self { max_objects_per_sec: 0, max_io_per_sec: 0, max_parallelism: 4 }
    }
}

// ---------------------------------------------------------------------------
// 扫描统计
// ---------------------------------------------------------------------------

/// 扫描运行时统计（可 JSON 序列化，供监控 / 日志输出）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanStats {
    pub objects_scanned: u64,
    pub bytes_scanned: u64,
    pub bytes_migrated: u64,
    pub io_ops: u64,
    pub elapsed_ms: u64,
    /// 被限速的次数（令牌桶不足触发 sleep 的次数）
    pub throttled_count: u64,
    /// 是否因预算超限而提前终止
    pub budget_exceeded: bool,
}

// ---------------------------------------------------------------------------
// 预算追踪器
// ---------------------------------------------------------------------------

/// 三维预算追踪器
///
/// 每次扫描创建一个实例，在扫描循环中调用：
/// - [`can_continue`](Self::can_continue)：检查时间 / 容量预算是否超限
/// - [`record_object`](Self::record_object)：记录扫描对象（含字节数），触发令牌桶限速
/// - [`record_io`](Self::record_io)：记录一次 IO 操作（含 IO 限速）
/// - [`record_migration`](Self::record_migration)：记录迁移字节数
/// - [`stats`](Self::stats)：获取当前统计快照
///
/// 所有计数器均为原子变量，可安全跨线程共享（`Arc<ScanBudgetTracker>`）。
pub struct ScanBudgetTracker {
    budget: ScanBudget,
    start_time: Instant,
    objects_scanned: AtomicU64,
    bytes_scanned: AtomicU64,
    bytes_migrated: AtomicU64,
    io_ops: AtomicU64,
    throttled_count: AtomicU64,
    budget_exceeded: AtomicU64, // 0 = false, 1 = true
    // 对象限速令牌桶
    obj_tokens: Mutex<f64>,
    obj_last_time: Mutex<Instant>,
    // IO 限速令牌桶
    io_tokens: Mutex<f64>,
    io_last_time: Mutex<Instant>,
}

impl ScanBudgetTracker {
    /// 创建新的预算追踪器（立即开始计时）
    pub fn new(budget: ScanBudget) -> Self {
        let now = Instant::now();
        let obj_rate = budget.io.max_objects_per_sec as f64;
        let io_rate = budget.io.max_io_per_sec as f64;
        Self {
            budget,
            start_time: now,
            objects_scanned: AtomicU64::new(0),
            bytes_scanned: AtomicU64::new(0),
            bytes_migrated: AtomicU64::new(0),
            io_ops: AtomicU64::new(0),
            throttled_count: AtomicU64::new(0),
            budget_exceeded: AtomicU64::new(0),
            // 令牌桶初始填满（桶容量 = 速率，即允许 1 秒突发）
            obj_tokens: Mutex::new(obj_rate),
            obj_last_time: Mutex::new(now),
            io_tokens: Mutex::new(io_rate),
            io_last_time: Mutex::new(now),
        }
    }

    /// 获取预算配置引用
    pub fn budget(&self) -> &ScanBudget {
        &self.budget
    }

    /// 检查是否可以继续扫描（时间 / 容量预算）
    ///
    /// 返回 `false` 表示至少一项预算已超限，调用方应终止扫描。
    /// 首次返回 `false` 时会设置 `budget_exceeded` 标志。
    pub fn can_continue(&self) -> bool {
        // 1. 时间预算：最大时长
        if let Some(max_dur) = self.budget.time.max_duration {
            if self.start_time.elapsed() >= max_dur {
                self.mark_exceeded();
                return false;
            }
        }

        // 2. 时间预算：扫描窗口
        if !self.is_in_window() {
            self.mark_exceeded();
            return false;
        }

        // 3. 容量预算：最大对象数
        let max_objs = self.budget.capacity.max_objects_per_scan;
        if max_objs > 0 && self.objects_scanned.load(Ordering::Relaxed) >= max_objs {
            self.mark_exceeded();
            return false;
        }

        // 4. 容量预算：最大处理字节数
        let max_bytes = self.budget.capacity.max_bytes_per_scan;
        if max_bytes > 0 && self.bytes_scanned.load(Ordering::Relaxed) >= max_bytes {
            self.mark_exceeded();
            return false;
        }

        // 5. 容量预算：最大迁移字节数
        let max_mig = self.budget.capacity.max_migration_bytes;
        if max_mig > 0 && self.bytes_migrated.load(Ordering::Relaxed) >= max_mig {
            self.mark_exceeded();
            return false;
        }

        true
    }

    /// 记录扫描了一个对象（含字节数），如果 IO 预算超限则 sleep 限速
    ///
    /// 令牌桶算法：
    /// - 每秒补充 `max_objects_per_sec` 个令牌
    /// - 每记录一个对象消耗 1 个令牌
    /// - 令牌不足时 sleep 到下一个令牌补充时间
    /// - `max_objects_per_sec == 0` 时不限速
    pub fn record_object(&self, object_size: u64) {
        self.objects_scanned.fetch_add(1, Ordering::Relaxed);
        self.bytes_scanned.fetch_add(object_size, Ordering::Relaxed);

        let rate = self.budget.io.max_objects_per_sec as f64;
        if rate <= 0.0 {
            return; // 不限速
        }

        let mut tokens = self.obj_tokens.lock().expect("obj_tokens mutex poisoned");
        let mut last_time = self.obj_last_time.lock().expect("obj_last_time mutex poisoned");

        let now = Instant::now();
        let elapsed = now.duration_since(*last_time).as_secs_f64();
        *last_time = now;

        // 补充令牌（桶容量 = rate，即允许最多 1 秒的突发）
        *tokens = (*tokens + elapsed * rate).min(rate);

        // 消耗 1 个令牌
        *tokens -= 1.0;

        if *tokens < 0.0 {
            // 令牌不足：计算需要 sleep 的时间以恢复 1 个令牌
            let sleep_secs = (-*tokens) / rate;
            self.throttled_count.fetch_add(1, Ordering::Relaxed);
            // 释放锁后再 sleep，避免持有锁阻塞
            drop(tokens);
            drop(last_time);
            std::thread::sleep(Duration::from_secs_f64(sleep_secs));
        }
    }

    /// 记录一次 IO 操作（含 IO 限速）
    ///
    /// 与 [`record_object`](Self::record_object) 类似，但使用 `max_io_per_sec` 速率。
    pub fn record_io(&self) {
        self.io_ops.fetch_add(1, Ordering::Relaxed);

        let rate = self.budget.io.max_io_per_sec as f64;
        if rate <= 0.0 {
            return;
        }

        let mut tokens = self.io_tokens.lock().expect("io_tokens mutex poisoned");
        let mut last_time = self.io_last_time.lock().expect("io_last_time mutex poisoned");

        let now = Instant::now();
        let elapsed = now.duration_since(*last_time).as_secs_f64();
        *last_time = now;

        *tokens = (*tokens + elapsed * rate).min(rate);
        *tokens -= 1.0;

        if *tokens < 0.0 {
            let sleep_secs = (-*tokens) / rate;
            self.throttled_count.fetch_add(1, Ordering::Relaxed);
            drop(tokens);
            drop(last_time);
            std::thread::sleep(Duration::from_secs_f64(sleep_secs));
        }
    }

    /// 记录迁移字节数
    pub fn record_migration(&self, bytes: u64) {
        self.bytes_migrated.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 获取当前统计快照
    pub fn stats(&self) -> ScanStats {
        ScanStats {
            objects_scanned: self.objects_scanned.load(Ordering::Relaxed),
            bytes_scanned: self.bytes_scanned.load(Ordering::Relaxed),
            bytes_migrated: self.bytes_migrated.load(Ordering::Relaxed),
            io_ops: self.io_ops.load(Ordering::Relaxed),
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            throttled_count: self.throttled_count.load(Ordering::Relaxed),
            budget_exceeded: self.budget_exceeded.load(Ordering::Relaxed) != 0,
        }
    }

    /// 检查是否在扫描窗口内
    ///
    /// 窗口规则：
    /// - `window_start_hour` 和 `window_end_hour` 均为 `None` → 始终在窗口内
    /// - 正常窗口（start <= end）：当前小时 ∈ [start, end)
    /// - 跨午夜窗口（start > end，如 22:00-06:00）：当前小时 >= start 或 < end
    pub fn is_in_window(&self) -> bool {
        let (Some(start), Some(end)) =
            (self.budget.time.window_start_hour, self.budget.time.window_end_hour)
        else {
            return true; // 未配置窗口，不限制
        };

        let current_hour = current_hour_24();
        if start <= end {
            // 正常窗口：[start, end)
            current_hour >= start && current_hour < end
        } else {
            // 跨午夜窗口：>= start 或 < end
            current_hour >= start || current_hour < end
        }
    }

    /// 标记预算已超限（幂等）
    fn mark_exceeded(&self) {
        self.budget_exceeded.store(1, Ordering::Relaxed);
    }
}

/// 获取当前小时（0-23，本地时间）
fn current_hour_24() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // 简化实现：基于 UNIX 时间戳 + 8 小时（UTC+8，中国时区）计算小时
    // 生产环境应使用 chrono::Local，但此处避免额外依赖
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // UTC+8
    let hours_utc8 = (secs / 3600 + 8) % 24;
    hours_utc8 as u8
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1：默认预算配置值正确
    #[test]
    fn test_scan_budget_default() {
        let b = ScanBudget::default();
        assert!(b.time.max_duration.is_none());
        assert!(b.time.window_start_hour.is_none());
        assert!(b.time.window_end_hour.is_none());
        assert_eq!(b.io.max_objects_per_sec, 0);
        assert_eq!(b.io.max_io_per_sec, 0);
        assert_eq!(b.io.max_parallelism, 4);
        assert_eq!(b.capacity.max_bytes_per_scan, 0);
        assert_eq!(b.capacity.max_migration_bytes, 0);
        assert_eq!(b.capacity.max_objects_per_scan, 0);
    }

    /// 测试 2：can_continue — 容量预算未超限时 true，超限时 false
    #[test]
    fn test_budget_tracker_can_continue() {
        // 无限制预算 → 始终可以继续
        let t = ScanBudgetTracker::new(ScanBudget::default());
        assert!(t.can_continue());

        // 设置 max_objects_per_scan = 2
        let budget = ScanBudget {
            capacity: CapacityBudget { max_objects_per_scan: 2, ..Default::default() },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);
        assert!(t.can_continue());
        t.record_object(100); // 1 个
        assert!(t.can_continue());
        t.record_object(200); // 2 个 → 达到上限
        assert!(!t.can_continue(), "should stop after reaching max_objects_per_scan");
        assert!(t.stats().budget_exceeded);

        // 设置 max_bytes_per_scan = 500
        let budget2 = ScanBudget {
            capacity: CapacityBudget { max_bytes_per_scan: 500, ..Default::default() },
            ..Default::default()
        };
        let t2 = ScanBudgetTracker::new(budget2);
        t2.record_object(300);
        assert!(t2.can_continue());
        t2.record_object(250); // 累计 550 >= 500
        assert!(!t2.can_continue(), "should stop after reaching max_bytes_per_scan");
    }

    /// 测试 3：令牌桶限速生效
    #[test]
    fn test_budget_tracker_rate_limit() {
        // 设置极低速率：2 对象/秒，桶容量 = 2
        let budget = ScanBudget {
            io: IoBudget { max_objects_per_sec: 2, max_io_per_sec: 0, max_parallelism: 1 },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);

        let start = Instant::now();
        // 前 2 个消耗初始令牌（桶满 = 2），不应 sleep
        t.record_object(1);
        t.record_object(1);
        let elapsed_first_two = start.elapsed();
        assert!(
            elapsed_first_two < Duration::from_millis(100),
            "first two should not throttle, elapsed={:?}",
            elapsed_first_two
        );

        // 第 3 个：令牌不足，应 sleep 约 0.5s（恢复 1 个令牌需要 1/2 秒）
        t.record_object(1);
        let elapsed_total = start.elapsed();
        assert!(
            elapsed_total >= Duration::from_millis(300),
            "third object should be throttled, elapsed={:?}",
            elapsed_total
        );

        let stats = t.stats();
        assert!(stats.throttled_count >= 1, "should have at least 1 throttle event");
        assert_eq!(stats.objects_scanned, 3);
    }

    /// 测试 4：统计正确
    #[test]
    fn test_budget_tracker_stats() {
        let t = ScanBudgetTracker::new(ScanBudget::default());
        t.record_object(100);
        t.record_object(200);
        t.record_io();
        t.record_io();
        t.record_io();
        t.record_migration(150);

        let s = t.stats();
        assert_eq!(s.objects_scanned, 2);
        assert_eq!(s.bytes_scanned, 300);
        assert_eq!(s.io_ops, 3);
        assert_eq!(s.bytes_migrated, 150);
        assert!(!s.budget_exceeded);
        assert_eq!(s.throttled_count, 0);
    }

    /// 测试 5：时间预算 — max_duration 超限后 can_continue 返回 false
    #[test]
    fn test_budget_tracker_time_duration() {
        let budget = ScanBudget {
            time: TimeBudget {
                max_duration: Some(Duration::from_millis(50)),
                window_start_hour: None,
                window_end_hour: None,
            },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);
        assert!(t.can_continue());
        std::thread::sleep(Duration::from_millis(80));
        assert!(!t.can_continue(), "should stop after max_duration elapsed");
        assert!(t.stats().budget_exceeded);
    }

    /// 测试 6：is_in_window 逻辑（正常窗口 + 跨午夜窗口）
    #[test]
    fn test_scan_budget_window_logic() {
        // 未配置窗口 → 始终 true
        let t = ScanBudgetTracker::new(ScanBudget::default());
        assert!(t.is_in_window());

        // 正常窗口 0-23（全天）→ true
        let budget = ScanBudget {
            time: TimeBudget {
                max_duration: None,
                window_start_hour: Some(0),
                window_end_hour: Some(23),
            },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);
        // 当前小时应在 0-23 范围内（除非是 23 点整，边界情况）
        let h = current_hour_24();
        if h < 23 {
            assert!(t.is_in_window());
        }

        // 跨午夜窗口 22-6（夜间窗口）
        let budget_night = ScanBudget {
            time: TimeBudget {
                max_duration: None,
                window_start_hour: Some(22),
                window_end_hour: Some(6),
            },
            ..Default::default()
        };
        let t_night = ScanBudgetTracker::new(budget_night);
        let h = current_hour_24();
        let expected = !(6..22).contains(&h);
        assert_eq!(t_night.is_in_window(), expected, "hour={}", h);
    }

    /// 测试 7：IO 限速生效
    #[test]
    fn test_budget_tracker_io_rate_limit() {
        let budget = ScanBudget {
            io: IoBudget { max_objects_per_sec: 0, max_io_per_sec: 2, max_parallelism: 1 },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);
        let start = Instant::now();
        t.record_io(); // 1
        t.record_io(); // 2 (consumes full bucket)
        t.record_io(); // 3 → throttled
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(300),
            "IO rate limit should throttle, elapsed={:?}",
            elapsed
        );
        assert_eq!(t.stats().io_ops, 3);
        assert!(t.stats().throttled_count >= 1);
    }
}


#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_scan_stats_default() {
        let s = ScanStats {
            objects_scanned: 0,
            bytes_scanned: 0,
            bytes_migrated: 0,
            io_ops: 0,
            elapsed_ms: 0,
            throttled_count: 0,
            budget_exceeded: false,
        };
        assert_eq!(s.objects_scanned, 0);
        assert!(!s.budget_exceeded);
    }

    #[test]
    fn test_scan_stats_serialization() {
        let s = ScanStats {
            objects_scanned: 10,
            bytes_scanned: 1024,
            bytes_migrated: 512,
            io_ops: 20,
            elapsed_ms: 100,
            throttled_count: 3,
            budget_exceeded: true,
        };
        let json = format!("{:?}", s);
        assert!(json.contains("objects_scanned"));
        assert!(json.contains("budget_exceeded"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_time_budget_default() {
        let t = TimeBudget::default();
        assert!(t.max_duration.is_none());
        assert!(t.window_start_hour.is_none());
        assert!(t.window_end_hour.is_none());
    }

    #[test]
    fn test_capacity_budget_default() {
        let c = CapacityBudget::default();
        assert_eq!(c.max_bytes_per_scan, 0);
        assert_eq!(c.max_migration_bytes, 0);
        assert_eq!(c.max_objects_per_scan, 0);
    }

    #[test]
    fn test_io_budget_default() {
        let io = IoBudget::default();
        assert_eq!(io.max_objects_per_sec, 0);
        assert_eq!(io.max_io_per_sec, 0);
        assert_eq!(io.max_parallelism, 4);
    }

    #[test]
    fn test_scan_budget_clone() {
        let b = ScanBudget::default();
        let b2 = b.clone();
        assert_eq!(b.io.max_parallelism, b2.io.max_parallelism);
    }

    #[test]
    fn test_budget_tracker_budget_accessor() {
        let b = ScanBudget::default();
        let t = ScanBudgetTracker::new(b);
        assert_eq!(t.budget().io.max_parallelism, 4);
    }

    #[test]
    fn test_budget_tracker_max_migration_bytes() {
        let budget = ScanBudget {
            capacity: CapacityBudget { max_migration_bytes: 100, ..Default::default() },
            ..Default::default()
        };
        let t = ScanBudgetTracker::new(budget);
        assert!(t.can_continue());
        t.record_migration(50);
        assert!(t.can_continue());
        t.record_migration(60); // 110 >= 100
        assert!(!t.can_continue());
        assert!(t.stats().budget_exceeded);
    }

    #[test]
    fn test_budget_tracker_zero_budget_continues() {
        // All budgets 0 = unlimited
        let t = ScanBudgetTracker::new(ScanBudget::default());
        for _ in 0..100 {
            t.record_object(1000);
            t.record_io();
            t.record_migration(1000);
            assert!(t.can_continue());
        }
    }

    #[test]
    fn test_budget_tracker_elapsed_ms_increases() {
        let t = ScanBudgetTracker::new(ScanBudget::default());
        let e1 = t.stats().elapsed_ms;
        std::thread::sleep(Duration::from_millis(10));
        let e2 = t.stats().elapsed_ms;
        assert!(e2 >= e1);
    }
}
