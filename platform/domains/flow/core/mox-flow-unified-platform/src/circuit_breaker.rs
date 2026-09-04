// Copyright (c) 2026 璇玑 RelGraph · mox 模块化系统架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 企业级治理：限流与熔断
//!
//! 提供令牌桶限流 + 熔断器模式，保障平台稳定性：
//! - 全局限流、租户限流、用户限流三级
//! - 熔断器（Closed/Open/Half-Open 三态）
//! - 降级策略支持

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::error::{PlatformError, PlatformResult};

/// 限流结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    /// 允许通过
    Allowed,
    /// 被限流
    Throttled {
        /// 需要等待的时间（毫秒）
        retry_after_ms: u64,
    },
}

/// 令牌桶限流器
pub struct TokenBucket {
    /// 每秒生成令牌数
    rate_per_second: f64,
    /// 桶容量
    capacity: f64,
    /// 当前令牌数
    tokens: RwLock<f64>,
    /// 上次补充时间
    last_refill: RwLock<Instant>,
}

impl TokenBucket {
    /// 创建令牌桶
    pub fn new(rate_per_second: f64, capacity: f64) -> Self {
        Self {
            rate_per_second,
            capacity,
            tokens: RwLock::new(capacity),
            last_refill: RwLock::new(Instant::now()),
        }
    }

    /// 尝试获取令牌
    pub fn try_acquire(&self, tokens: f64) -> RateLimitResult {
        let now = Instant::now();
        let mut current_tokens = self.tokens.write();
        let mut last_refill = self.last_refill.write();

        // 补充令牌
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.rate_per_second;
        *current_tokens = (*current_tokens + new_tokens).min(self.capacity);
        *last_refill = now;

        if *current_tokens >= tokens {
            *current_tokens -= tokens;
            RateLimitResult::Allowed
        } else {
            let needed = tokens - *current_tokens;
            let retry_after_ms = (needed / self.rate_per_second * 1000.0) as u64;
            RateLimitResult::Throttled { retry_after_ms }
        }
    }

    /// 获取当前令牌数
    pub fn current_tokens(&self) -> f64 {
        // 先补充再读取
        let now = Instant::now();
        let mut tokens = self.tokens.write();
        let mut last_refill = self.last_refill.write();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.rate_per_second;
        *tokens = (*tokens + new_tokens).min(self.capacity);
        *last_refill = now;
        *tokens
    }

    /// 获取速率
    pub fn rate(&self) -> f64 {
        self.rate_per_second
    }

    /// 获取容量
    pub fn capacity(&self) -> f64 {
        self.capacity
    }
}

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// 关闭（正常运行）
    Closed,
    /// 打开（熔断，拒绝请求）
    Open,
    /// 半开（尝试恢复）
    HalfOpen,
}

impl CircuitState {
    pub fn name(&self) -> &'static str {
        match self {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        }
    }
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败率阈值（0-1），超过则熔断
    pub failure_rate_threshold: f64,
    /// 滑动窗口大小（请求数）
    pub window_size: usize,
    /// 最小请求数（低于此数量不熔断）
    pub min_requests: usize,
    /// 熔断持续时间（毫秒）
    pub open_duration_ms: u64,
    /// 半开状态允许通过的请求数
    pub half_open_max_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 0.5,
            window_size: 100,
            min_requests: 20,
            open_duration_ms: 30000,
            half_open_max_requests: 5,
        }
    }
}

/// 滑动窗口统计
struct CircuitWindow {
    /// 窗口内的结果（true=成功, false=失败）
    results: Vec<bool>,
    /// 失败数
    failures: usize,
}

impl CircuitWindow {
    fn new(size: usize) -> Self {
        Self {
            results: Vec::with_capacity(size),
            failures: 0,
        }
    }

    fn record(&mut self, success: bool, max_size: usize) {
        if self.results.len() >= max_size {
            let removed = self.results.remove(0);
            if !removed {
                self.failures -= 1;
            }
        }
        self.results.push(success);
        if !success {
            self.failures += 1;
        }
    }

    fn len(&self) -> usize {
        self.results.len()
    }

    fn failure_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        self.failures as f64 / self.results.len() as f64
    }
}

/// 熔断器
pub struct CircuitBreaker {
    /// 配置
    config: CircuitBreakerConfig,
    /// 当前状态
    state: RwLock<CircuitState>,
    /// 滑动窗口
    window: RwLock<CircuitWindow>,
    /// 状态变更时间
    state_changed_at: RwLock<Instant>,
    /// 半开状态通过数
    half_open_passes: AtomicU64,
    /// 总请求数
    total_requests: AtomicU64,
    /// 总拒绝数
    total_rejected: AtomicU64,
}

impl CircuitBreaker {
    /// 创建熔断器
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let window_size = config.window_size;
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            window: RwLock::new(CircuitWindow::new(window_size)),
            state_changed_at: RwLock::new(Instant::now()),
            half_open_passes: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
        }
    }

    /// 尝试通过熔断器
    pub fn try_acquire(&self) -> bool {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let state = *self.state.read();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否到期，到期则进入半开
                let elapsed = self.state_changed_at.read().elapsed().as_millis() as u64;
                if elapsed >= self.config.open_duration_ms {
                    *self.state.write() = CircuitState::HalfOpen;
                    *self.state_changed_at.write() = Instant::now();
                    self.half_open_passes.store(1, Ordering::Relaxed);
                    true
                } else {
                    self.total_rejected.fetch_add(1, Ordering::Relaxed);
                    false
                }
            }
            CircuitState::HalfOpen => {
                let passes = self.half_open_passes.fetch_add(1, Ordering::Relaxed);
                if passes < self.config.half_open_max_requests as u64 {
                    true
                } else {
                    self.total_rejected.fetch_add(1, Ordering::Relaxed);
                    false
                }
            }
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                let mut window = self.window.write();
                window.record(true, self.config.window_size);
            }
            CircuitState::HalfOpen => {
                let passes = self.half_open_passes.load(Ordering::Relaxed);
                // 如果半开状态的请求都成功了，关闭熔断器
                if passes >= self.config.half_open_max_requests as u64 {
                    *self.state.write() = CircuitState::Closed;
                    *self.state_changed_at.write() = Instant::now();
                    let mut window = self.window.write();
                    *window = CircuitWindow::new(self.config.window_size);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// 记录失败
    pub fn record_failure(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                let mut window = self.window.write();
                window.record(false, self.config.window_size);

                // 检查是否达到熔断条件
                if window.len() >= self.config.min_requests
                    && window.failure_rate() >= self.config.failure_rate_threshold
                {
                    *self.state.write() = CircuitState::Open;
                    *self.state_changed_at.write() = Instant::now();
                }
            }
            CircuitState::HalfOpen => {
                // 半开状态有失败，立即回到打开
                *self.state.write() = CircuitState::Open;
                *self.state_changed_at.write() = Instant::now();
            }
            CircuitState::Open => {}
        }
    }

    /// 获取当前状态
    /// 注意：如果熔断超时，会自动转换到 HalfOpen 状态
    pub fn state(&self) -> CircuitState {
        let state = *self.state.read();
        if state == CircuitState::Open {
            let elapsed = self.state_changed_at.read().elapsed().as_millis() as u64;
            if elapsed >= self.config.open_duration_ms {
                // 自动转换到半开状态
                let mut state_w = self.state.write();
                // 双重检查
                if *state_w == CircuitState::Open {
                    let elapsed2 = self.state_changed_at.read().elapsed().as_millis() as u64;
                    if elapsed2 >= self.config.open_duration_ms {
                        *state_w = CircuitState::HalfOpen;
                        *self.state_changed_at.write() = Instant::now();
                        self.half_open_passes.store(0, Ordering::Relaxed);
                        return CircuitState::HalfOpen;
                    }
                }
                return *state_w;
            }
        }
        state
    }

    /// 获取失败率
    pub fn failure_rate(&self) -> f64 {
        self.window.read().failure_rate()
    }

    /// 获取总请求数
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// 获取总拒绝数
    pub fn total_rejected(&self) -> u64 {
        self.total_rejected.load(Ordering::Relaxed)
    }

    /// 手动重置
    pub fn reset(&self) {
        *self.state.write() = CircuitState::Closed;
        *self.state_changed_at.write() = Instant::now();
        let mut window = self.window.write();
        *window = CircuitWindow::new(self.config.window_size);
        self.half_open_passes.store(0, Ordering::Relaxed);
    }
}

/// 多级限流管理器（全局/租户/用户）
pub struct RateLimitManager {
    /// 全局限流器
    global: TokenBucket,
    /// 租户级限流器
    tenant_buckets: RwLock<HashMap<String, TokenBucket>>,
    /// 用户级限流器
    user_buckets: RwLock<HashMap<String, TokenBucket>>,
    /// 默认租户限流速率
    tenant_default_rate: f64,
    /// 默认用户限流速率
    user_default_rate: f64,
}

impl RateLimitManager {
    /// 创建限流管理器
    pub fn new(
        global_rate: f64,
        global_capacity: f64,
        tenant_default_rate: f64,
        user_default_rate: f64,
    ) -> Self {
        Self {
            global: TokenBucket::new(global_rate, global_capacity),
            tenant_buckets: RwLock::new(HashMap::new()),
            user_buckets: RwLock::new(HashMap::new()),
            tenant_default_rate,
            user_default_rate,
        }
    }

    /// 全局限流检查
    pub fn check_global(&self, tokens: f64) -> RateLimitResult {
        self.global.try_acquire(tokens)
    }

    /// 租户限流检查
    pub fn check_tenant(&self, tenant_id: &str, tokens: f64) -> RateLimitResult {
        let mut tenants = self.tenant_buckets.write();
        let bucket = tenants
            .entry(tenant_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.tenant_default_rate, self.tenant_default_rate * 2.0));
        bucket.try_acquire(tokens)
    }

    /// 用户限流检查
    pub fn check_user(&self, user_id: &str, tokens: f64) -> RateLimitResult {
        let mut users = self.user_buckets.write();
        let bucket = users
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.user_default_rate, self.user_default_rate * 2.0));
        bucket.try_acquire(tokens)
    }

    /// 三级限流检查（全局→租户→用户，全部通过才算通过）
    pub fn check_all(&self, tenant_id: &str, user_id: &str, tokens: f64) -> RateLimitResult {
        // 全局
        match self.global.try_acquire(tokens) {
            RateLimitResult::Throttled { retry_after_ms } => {
                return RateLimitResult::Throttled { retry_after_ms };
            }
            RateLimitResult::Allowed => {}
        }

        // 租户
        match self.check_tenant(tenant_id, tokens) {
            RateLimitResult::Throttled { retry_after_ms } => {
                return RateLimitResult::Throttled { retry_after_ms };
            }
            RateLimitResult::Allowed => {}
        }

        // 用户
        self.check_user(user_id, tokens)
    }

    /// 设置租户特定限流
    pub fn set_tenant_limit(&self, tenant_id: &str, rate: f64, capacity: f64) {
        self.tenant_buckets
            .write()
            .insert(tenant_id.to_string(), TokenBucket::new(rate, capacity));
    }

    /// 设置用户特定限流
    pub fn set_user_limit(&self, user_id: &str, rate: f64, capacity: f64) {
        self.user_buckets
            .write()
            .insert(user_id.to_string(), TokenBucket::new(rate, capacity));
    }

    /// 获取租户数量
    pub fn tenant_count(&self) -> usize {
        self.tenant_buckets.read().len()
    }

    /// 获取用户数量
    pub fn user_count(&self) -> usize {
        self.user_buckets.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TokenBucket 测试 =====

    #[test]
    fn test_token_bucket_allow() {
        let bucket = TokenBucket::new(100.0, 100.0);
        assert_eq!(bucket.try_acquire(1.0), RateLimitResult::Allowed);
        assert_eq!(bucket.try_acquire(50.0), RateLimitResult::Allowed);
    }

    #[test]
    fn test_token_bucket_throttle() {
        let bucket = TokenBucket::new(10.0, 10.0);
        // 先取完所有令牌
        assert_eq!(bucket.try_acquire(10.0), RateLimitResult::Allowed);
        // 再取应该被限流
        let result = bucket.try_acquire(1.0);
        assert!(matches!(result, RateLimitResult::Throttled { .. }));
    }

    #[test]
    fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(1000.0, 10.0);
        assert_eq!(bucket.try_acquire(10.0), RateLimitResult::Allowed);

        // 等待一小段时间让令牌补充
        std::thread::sleep(std::time::Duration::from_millis(20));

        // 应该有一些令牌补充了
        let tokens = bucket.current_tokens();
        assert!(tokens > 0.0);
    }

    // ===== CircuitBreaker 测试 =====

    #[test]
    fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 5,
            open_duration_ms: 1000,
            half_open_max_requests: 3,
        });

        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.try_acquire());

        // 记录一些成功
        for _ in 0..5 {
            cb.record_success();
        }
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_trips() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 4,
            open_duration_ms: 1000,
            half_open_max_requests: 3,
        });

        // 3 成功 + 3 失败 = 50%，达到阈值
        for _ in 0..3 {
            cb.record_success();
        }
        for _ in 0..3 {
            cb.record_failure();
        }

        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_open_rejects() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 4,
            open_duration_ms: 10000, // 长时间打开
            half_open_max_requests: 3,
        });

        // 触发熔断
        for _ in 0..3 {
            cb.record_success();
        }
        for _ in 0..3 {
            cb.record_failure();
        }

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.try_acquire());
        assert!(cb.total_rejected() > 0);
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 4,
            open_duration_ms: 10, // 非常短
            half_open_max_requests: 2,
        });

        // 触发熔断
        for _ in 0..3 { cb.record_success(); }
        for _ in 0..3 { cb.record_failure(); }
        assert_eq!(cb.state(), CircuitState::Open);

        // 等待熔断超时
        std::thread::sleep(std::time::Duration::from_millis(20));

        // 应该进入半开
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // 半开状态成功请求
        assert!(cb.try_acquire());
        cb.record_success();
        assert!(cb.try_acquire());
        cb.record_success();

        // 应该恢复关闭
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_fails() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 4,
            open_duration_ms: 10,
            half_open_max_requests: 5,
        });

        // 触发熔断
        for _ in 0..3 { cb.record_success(); }
        for _ in 0..3 { cb.record_failure(); }

        // 等待超时
        std::thread::sleep(std::time::Duration::from_millis(20));

        // 半开状态失败
        cb.try_acquire();
        cb.record_failure();

        // 应该回到打开状态
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_min_requests() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 10, // 需要10个请求才熔断
            open_duration_ms: 1000,
            half_open_max_requests: 3,
        });

        // 只有3个失败请求，未达到最小请求数
        for _ in 0..3 {
            cb.record_failure();
        }

        // 不应熔断
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 10,
            min_requests: 4,
            open_duration_ms: 10000,
            half_open_max_requests: 3,
        });

        for _ in 0..3 { cb.record_success(); }
        for _ in 0..3 { cb.record_failure(); }
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_rate(), 0.0);
    }

    // ===== RateLimitManager 测试 =====

    #[test]
    fn test_rate_limit_manager_global() {
        let rlm = RateLimitManager::new(100.0, 100.0, 50.0, 10.0);
        assert_eq!(rlm.check_global(50.0), RateLimitResult::Allowed);
    }

    #[test]
    fn test_rate_limit_manager_tenant() {
        let rlm = RateLimitManager::new(100.0, 100.0, 10.0, 5.0);
        assert_eq!(rlm.check_tenant("t1", 5.0), RateLimitResult::Allowed);
        assert_eq!(rlm.tenant_count(), 1);
    }

    #[test]
    fn test_rate_limit_manager_user() {
        let rlm = RateLimitManager::new(100.0, 100.0, 50.0, 10.0);
        assert_eq!(rlm.check_user("u1", 5.0), RateLimitResult::Allowed);
        assert_eq!(rlm.user_count(), 1);
    }

    #[test]
    fn test_rate_limit_manager_check_all() {
        let rlm = RateLimitManager::new(100.0, 100.0, 50.0, 10.0);
        let result = rlm.check_all("t1", "u1", 5.0);
        assert_eq!(result, RateLimitResult::Allowed);
    }

    #[test]
    fn test_set_custom_limits() {
        let rlm = RateLimitManager::new(100.0, 100.0, 50.0, 10.0);

        rlm.set_tenant_limit("t-vip", 1000.0, 2000.0);
        rlm.set_user_limit("u-vip", 500.0, 1000.0);

        assert_eq!(rlm.check_tenant("t-vip", 500.0), RateLimitResult::Allowed);
        assert_eq!(rlm.check_user("u-vip", 100.0), RateLimitResult::Allowed);
    }

    // ===== 集成测试 =====

    #[test]
    fn test_circuit_breaker_with_rate_limit() {
        let rlm = RateLimitManager::new(1000.0, 1000.0, 500.0, 100.0);
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            window_size: 20,
            min_requests: 10,
            open_duration_ms: 1000,
            half_open_max_requests: 3,
        });

        // 模拟正常请求
        for _ in 0..15 {
            assert_eq!(rlm.check_global(1.0), RateLimitResult::Allowed);
            assert!(cb.try_acquire());
            cb.record_success();
        }
        assert_eq!(cb.state(), CircuitState::Closed);

        // 模拟大量失败
        for _ in 0..15 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
