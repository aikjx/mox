//! 弹性容错 — 限流/熔断/降级/重试/超时/舱壁，零配置默认启用

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // 正常
    Open,     // 熔断（拒绝请求）
    HalfOpen, // 半开（试探性放行）
}

/// 熔断器
pub struct CircuitBreaker {
    state: Arc<AtomicUsize>, // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    last_failure: Arc<parking_lot::Mutex<Option<Instant>>>,
    threshold: f64,
    min_requests: u64,
    reset_timeout: Duration,
    half_open_max: u64,
}

impl CircuitBreaker {
    pub fn new(threshold: f64, min_requests: u64, reset_timeout: Duration) -> Self {
        Self {
            state: Arc::new(AtomicUsize::new(0)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            last_failure: Arc::new(parking_lot::Mutex::new(None)),
            threshold,
            min_requests,
            reset_timeout,
            half_open_max: 3,
        }
    }

    pub fn state(&self) -> CircuitState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            _ => CircuitState::HalfOpen,
        }
    }

    /// 检查是否允许请求通过
    pub fn allow(&self) -> bool {
        let state = self.state();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时可以进入半开
                let last = *self.last_failure.lock();
                if let Some(t) = last {
                    if t.elapsed() >= self.reset_timeout {
                        self.state.store(2, Ordering::Relaxed); // HalfOpen
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                // 半开状态只允许有限请求
                self.success_count.load(Ordering::Relaxed) < self.half_open_max
            }
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        if self.state() == CircuitState::HalfOpen {
            // 半开状态连续成功则关闭
            if self.success_count.load(Ordering::Relaxed) >= self.half_open_max {
                self.state.store(0, Ordering::Relaxed); // Closed
                self.failure_count.store(0, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
            }
        }
    }

    /// 记录失败
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.lock() = Some(Instant::now());
        let total = self.failure_count.load(Ordering::Relaxed) + self.success_count.load(Ordering::Relaxed);
        if total >= self.min_requests {
            let rate = self.failure_count.load(Ordering::Relaxed) as f64 / total as f64;
            if rate >= self.threshold {
                self.state.store(1, Ordering::Relaxed); // Open
            }
        }
        if self.state() == CircuitState::HalfOpen {
            self.state.store(1, Ordering::Relaxed); // 半开失败立即熔断
        }
    }
}

/// 限流器（令牌桶）
pub struct RateLimiter {
    tokens: Arc<AtomicU64>,
    capacity: u64,
    refill_per_sec: u64,
    last_refill: Arc<parking_lot::Mutex<Instant>>,
}

impl RateLimiter {
    pub fn new(capacity: u64, refill_per_sec: u64) -> Self {
        Self {
            tokens: Arc::new(AtomicU64::new(capacity)),
            capacity,
            refill_per_sec,
            last_refill: Arc::new(parking_lot::Mutex::new(Instant::now())),
        }
    }

    /// 尝试获取一个令牌
    pub fn try_acquire(&self) -> bool {
        self.refill();
        let current = self.tokens.load(Ordering::Relaxed);
        if current > 0 {
            self.tokens.store(current - 1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.lock();
        let elapsed = last.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            let refill = (elapsed * self.refill_per_sec as f64) as u64;
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + refill).min(self.capacity);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last = Instant::now();
        }
    }
}

/// 舱壁隔离（限制并发数）
pub struct Bulkhead {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl Bulkhead {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// 获取并发许可
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit, tokio::sync::AcquireError> {
        self.semaphore.acquire().await
    }

    /// 当前可用许可数
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

/// 重试策略
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// 计算第n次重试的延迟（指数退避+抖动）
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let base = self.base_delay.as_millis() as f64;
        let delay = base * self.multiplier.powi(attempt as i32);
        let jitter = delay * 0.1 * (rand_like() as f64);
        Duration::from_millis((delay + jitter).min(self.max_delay.as_millis() as f64) as u64)
    }
}

/// 简单的随机数（0-1），避免引入rand依赖
fn rand_like() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}
