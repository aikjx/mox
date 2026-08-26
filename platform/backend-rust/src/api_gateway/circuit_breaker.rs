//! 熔断器
//!
//! 三态状态机：
//! - Closed（关闭）：正常放行，统计失败率
//! - Open（打开）：快速失败，不调用下游
//! - HalfOpen（半开）：放行少量请求探测，成功则关闭，失败则重新打开
//!
//! 基于滑动窗口统计失败率

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 熔断器状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitConfig {
    pub failure_threshold: f64,
    pub minimum_requests: u64,
    pub open_duration_ms: u64,
    pub half_open_max_requests: u64,
    pub window_size_ms: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 0.5,
            minimum_requests: 20,
            open_duration_ms: 30000,
            half_open_max_requests: 5,
            window_size_ms: 60000,
        }
    }
}

/// 滑动窗口中的请求记录
#[derive(Debug, Clone, Copy)]
struct WindowEntry {
    timestamp: Instant,
    success: bool,
}

/// 熔断器内部状态
struct CircuitInner {
    state: CircuitState,
    window: Vec<WindowEntry>,
    open_since: Option<Instant>,
    half_open_requests: u64,
    half_open_successes: u64,
    half_open_failures: u64,
}

/// 熔断器
pub struct CircuitBreaker {
    config: CircuitConfig,
    inner: RwLock<CircuitInner>,
    total_calls: AtomicU64,
    total_successes: AtomicU64,
    total_failures: AtomicU64,
    state_transitions: AtomicU64,
}

impl CircuitBreaker {
    /// 创建熔断器
    pub fn new(config: CircuitConfig) -> Self {
        Self {
            config,
            inner: RwLock::new(CircuitInner {
                state: CircuitState::Closed,
                window: Vec::new(),
                open_since: None,
                half_open_requests: 0,
                half_open_successes: 0,
                half_open_failures: 0,
            }),
            total_calls: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            state_transitions: AtomicU64::new(0),
        }
    }

    /// 当前状态
    pub async fn state(&self) -> CircuitState {
        self.inner.read().await.state
    }

    /// 是否可以执行请求
    pub async fn can_execute(&self) -> bool {
        let mut inner = self.inner.write().await;
        let now = Instant::now();

        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时，可以进入半开
                if let Some(open_since) = inner.open_since {
                    if now.duration_since(open_since) >= Duration::from_millis(self.config.open_duration_ms) {
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_requests = 0;
                        inner.half_open_successes = 0;
                        inner.half_open_failures = 0;
                        self.state_transitions.fetch_add(1, Ordering::Relaxed);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                if inner.half_open_requests < self.config.half_open_max_requests {
                    inner.half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 记录成功
    pub async fn record_success(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_successes.fetch_add(1, Ordering::Relaxed);

        let mut inner = self.inner.write().await;
        let now = Instant::now();

        match inner.state {
            CircuitState::Closed => {
                inner.window.push(WindowEntry { timestamp: now, success: true });
                self.prune_window(&mut inner, now);
            }
            CircuitState::HalfOpen => {
                inner.half_open_successes += 1;
                // 半开状态下所有探测请求都成功，则关闭熔断器
                if inner.half_open_requests >= self.config.half_open_max_requests
                    && inner.half_open_failures == 0
                {
                    inner.state = CircuitState::Closed;
                    inner.window.clear();
                    inner.open_since = None;
                    inner.half_open_requests = 0;
                    inner.half_open_successes = 0;
                    inner.half_open_failures = 0;
                    self.state_transitions.fetch_add(1, Ordering::Relaxed);
                }
            }
            _ => {}
        }
    }

    /// 记录失败
    pub async fn record_failure(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);

        let mut inner = self.inner.write().await;
        let now = Instant::now();

        match inner.state {
            CircuitState::Closed => {
                inner.window.push(WindowEntry { timestamp: now, success: false });
                self.prune_window(&mut inner, now);

                // 检查失败率是否超过阈值
                let total = inner.window.len() as u64;
                if total >= self.config.minimum_requests {
                    let failures = inner.window.iter().filter(|e| !e.success).count() as f64;
                    let failure_rate = failures / total as f64;

                    if failure_rate >= self.config.failure_threshold {
                        inner.state = CircuitState::Open;
                        inner.open_since = Some(now);
                        self.state_transitions.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            CircuitState::HalfOpen => {
                inner.half_open_failures += 1;
                // 半开状态下任何失败都重新打开熔断器
                inner.state = CircuitState::Open;
                inner.open_since = Some(now);
                inner.half_open_requests = 0;
                inner.half_open_successes = 0;
                inner.half_open_failures = 0;
                self.state_transitions.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// 执行带熔断保护的闭包
    pub async fn execute<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: From<String>,
    {
        if !self.can_execute().await {
            return Err(E::from("circuit_breaker_open".to_string()));
        }

        let result = f().await;

        match &result {
            Ok(_) => self.record_success().await,
            Err(_) => self.record_failure().await,
        }

        result
    }

    fn prune_window(&self, inner: &mut CircuitInner, now: Instant) {
        let window = Duration::from_millis(self.config.window_size_ms);
        inner.window.retain(|e| now.duration_since(e.timestamp) < window);
    }

    /// 获取当前失败率
    pub async fn failure_rate(&self) -> f64 {
        let inner = self.inner.read().await;
        let total = inner.window.len();
        if total == 0 {
            return 0.0;
        }
        let failures = inner.window.iter().filter(|e| !e.success).count();
        failures as f64 / total as f64
    }

    /// 重置熔断器
    pub async fn reset(&self) {
        let mut inner = self.inner.write().await;
        inner.state = CircuitState::Closed;
        inner.window.clear();
        inner.open_since = None;
        inner.half_open_requests = 0;
        inner.half_open_successes = 0;
        inner.half_open_failures = 0;
    }

    /// 获取统计
    pub async fn stats(&self) -> CircuitBreakerStats {
        let inner = self.inner.read().await;
        CircuitBreakerStats {
            state: inner.state,
            failure_threshold: self.config.failure_threshold,
            current_failure_rate: if !inner.window.is_empty() {
                inner.window.iter().filter(|e| !e.success).count() as f64 / inner.window.len() as f64
            } else { 0.0 },
            window_size: inner.window.len(),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            state_transitions: self.state_transitions.load(Ordering::Relaxed),
            open_duration_ms: self.config.open_duration_ms,
        }
    }
}

/// 熔断器统计
#[derive(Debug, Clone, Serialize)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_threshold: f64,
    pub current_failure_rate: f64,
    pub window_size: usize,
    pub total_calls: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub state_transitions: u64,
    pub open_duration_ms: u64,
}
