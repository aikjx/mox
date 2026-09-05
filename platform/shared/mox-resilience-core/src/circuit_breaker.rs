//! 熔断器模块
//!
//! 实现经典的熔断器模式（Circuit Breaker），包含三种状态：
//! - Closed（关闭）：正常执行，统计失败率
//! - Open（打开）：直接拒绝请求，快速失败
//! - HalfOpen（半开）：允许少量请求通过，探测服务是否恢复
//!
//! 当失败率超过阈值且请求量达到最小样本数时，熔断器从 Closed 变为 Open；
//! Open 状态持续一段时间后自动变为 HalfOpen；
//! HalfOpen 状态下允许一定数量的探测请求，全部成功则变为 Closed，任一失败则变回 Open。

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭状态：正常执行
    Closed,
    /// 打开状态：快速失败
    Open,
    /// 半开状态：探测恢复
    HalfOpen,
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败率阈值（0.0 ~ 1.0），超过此值触发熔断
    pub failure_rate_threshold: f64,
    /// 最小请求样本数，低于此数不触发熔断（避免小样本误判）
    pub minimum_requests: u64,
    /// 滑动窗口大小（统计最近 N 个请求）
    pub window_size: u64,
    /// Open 状态持续时间，之后进入 HalfOpen
    pub open_duration: Duration,
    /// HalfOpen 状态允许的探测请求数
    pub half_open_max_requests: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 0.5,
            minimum_requests: 10,
            window_size: 100,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 5,
        }
    }
}

/// 滑动窗口统计
#[derive(Debug)]
struct WindowStats {
    /// 环形缓冲区：true=成功，false=失败
    results: Vec<bool>,
    /// 写入位置
    head: usize,
    /// 当前窗口内的请求数
    count: u64,
    /// 当前窗口内的失败数
    failures: u64,
}

impl WindowStats {
    fn new(size: u64) -> Self {
        Self {
            results: vec![false; size as usize],
            head: 0,
            count: 0,
            failures: 0,
        }
    }

    fn record(&mut self, success: bool) {
        let size = self.results.len();
        if self.count >= size as u64 {
            // 覆盖旧值
            let old = self.results[self.head];
            if !old {
                self.failures -= 1;
            }
        } else {
            self.count += 1;
        }
        self.results[self.head] = success;
        if !success {
            self.failures += 1;
        }
        self.head = (self.head + 1) % size;
    }

    fn failure_rate(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.failures as f64 / self.count as f64
        }
    }

    fn reset(&mut self) {
        self.head = 0;
        self.count = 0;
        self.failures = 0;
    }
}

/// 熔断器内部状态
struct CircuitBreakerInner {
    config: CircuitBreakerConfig,
    state: CircuitState,
    stats: WindowStats,
    /// 状态变更时间
    state_changed_at: Instant,
    /// HalfOpen 状态下已通过的探测请求数
    half_open_requests: u64,
    /// HalfOpen 状态下的成功数
    half_open_successes: u64,
}

impl CircuitBreakerInner {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            stats: WindowStats::new(config.window_size),
            state: CircuitState::Closed,
            state_changed_at: Instant::now(),
            half_open_requests: 0,
            half_open_successes: 0,
            config,
        }
    }

    /// 判断是否允许请求通过
    fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时，超时则进入 HalfOpen
                if self.state_changed_at.elapsed() >= self.config.open_duration {
                    self.transition_to(CircuitState::HalfOpen);
                    self.half_open_requests = 1; // 第一个探测请求
                    self.half_open_successes = 0;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                if self.half_open_requests < self.config.half_open_max_requests {
                    self.half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 记录请求结果
    fn record_result(&mut self, success: bool) {
        match self.state {
            CircuitState::Closed => {
                self.stats.record(success);
                // 检查是否需要熔断
                if self.stats.count >= self.config.minimum_requests
                    && self.stats.failure_rate() >= self.config.failure_rate_threshold
                {
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                if success {
                    self.half_open_successes += 1;
                }
                // 检查 HalfOpen 是否结束
                if self.half_open_requests >= self.config.half_open_max_requests {
                    if self.half_open_successes == self.config.half_open_max_requests {
                        // 全部成功，恢复 Closed
                        self.stats.reset();
                        self.transition_to(CircuitState::Closed);
                    } else {
                        // 有失败，回到 Open
                        self.transition_to(CircuitState::Open);
                    }
                } else if !success {
                    // 任一失败，立即回到 Open
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::Open => {
                // Open 状态不记录
            }
        }
    }

    fn transition_to(&mut self, new_state: CircuitState) {
        if self.state != new_state {
            tracing::info!(
                "Circuit breaker state transition: {:?} -> {:?}",
                self.state,
                new_state
            );
            self.state = new_state;
            self.state_changed_at = Instant::now();
        }
    }
}

/// 熔断器执行错误
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// 熔断器打开，请求被拒绝
    CircuitOpen(CircuitOpenError),
    /// 原始操作错误
    Operation(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen(e) => write!(f, "{}", e),
            CircuitBreakerError::Operation(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {}

/// 熔断器
#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<Mutex<CircuitBreakerInner>>,
    name: String,
}

impl CircuitBreaker {
    /// 创建熔断器
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CircuitBreakerInner::new(config))),
            name: name.into(),
        }
    }

    /// 创建默认配置的熔断器
    pub fn with_default(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// 获取熔断器名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitState {
        self.inner.lock().state
    }

    /// 判断是否允许请求通过
    pub fn allow_request(&self) -> bool {
        self.inner.lock().allow_request()
    }

    /// 记录成功
    pub fn record_success(&self) {
        self.inner.lock().record_result(true);
    }

    /// 记录失败
    pub fn record_failure(&self) {
        self.inner.lock().record_result(false);
    }

    /// 执行闭包，自动记录结果
    pub fn execute<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if !self.allow_request() {
            return Err(CircuitBreakerError::CircuitOpen(CircuitOpenError {
                message: format!("Circuit breaker '{}' is open, request rejected", self.name),
            }));
        }
        let result = f();
        match &result {
            Ok(_) => self.record_success(),
            Err(_) => self.record_failure(),
        }
        result.map_err(CircuitBreakerError::Operation)
    }

    /// 重置熔断器（用于测试或手动恢复）
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        inner.stats.reset();
        inner.transition_to(CircuitState::Closed);
        inner.half_open_requests = 0;
        inner.half_open_successes = 0;
    }
}

/// 熔断器打开错误
#[derive(Debug, Clone)]
pub struct CircuitOpenError {
    pub message: String,
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CircuitOpenError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::with_default("test");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_opens_on_high_failure_rate() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 5,
            window_size: 10,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);

        // 5个请求，4个失败 -> 失败率80% > 50%
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_not_open_below_minimum_requests() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 10,
            window_size: 20,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);

        // 只有3个请求，全部失败，但未达到最小样本数
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_millis(50),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);

        // 触发熔断
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // 等待超时
        std::thread::sleep(Duration::from_millis(60));

        // 应该进入 HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_recovers_on_half_open_success() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_millis(50),
            half_open_max_requests: 2,
        };
        let cb = CircuitBreaker::new("test", config);

        // 触发熔断
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // 等待超时
        std::thread::sleep(Duration::from_millis(60));

        // HalfOpen 状态下2个成功请求
        assert!(cb.allow_request());
        cb.record_success();
        assert!(cb.allow_request());
        cb.record_success();

        // 应该恢复 Closed
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reopens_on_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_millis(50),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);

        // 触发熔断
        cb.record_failure();
        cb.record_failure();

        // 等待超时
        std::thread::sleep(Duration::from_millis(60));

        // HalfOpen 状态下第一个请求失败
        assert!(cb.allow_request());
        cb.record_failure();

        // 应该立即回到 Open
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_execute() {
        let cb = CircuitBreaker::with_default("test");

        // 成功执行
        let result: Result<i32, CircuitBreakerError<String>> = cb.execute(|| Ok(42));
        assert_eq!(result.unwrap(), 42);

        // 失败执行
        let result: Result<i32, CircuitBreakerError<String>> =
            cb.execute(|| Err("test error".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CircuitBreakerError::Operation(_)));
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }
}
