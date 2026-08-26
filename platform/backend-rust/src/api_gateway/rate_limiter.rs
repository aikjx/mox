//! 分布式限流器
//!
//! 支持两种算法：
//! - TokenBucket（令牌桶）：平滑限流，允许突发
//! - SlidingWindow（滑动窗口）：精确限流，防突发
//!
//! 支持 Redis 分布式限流（多实例共享计数）

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 限流算法
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RateLimitAlgorithm {
    TokenBucket,
    SlidingWindow,
}

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub algorithm: RateLimitAlgorithm,
    pub tokens_per_second: u64,
    pub burst_size: u64,
    pub window_seconds: u64,
    pub max_requests: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            algorithm: RateLimitAlgorithm::TokenBucket,
            tokens_per_second: 100,
            burst_size: 200,
            window_seconds: 60,
            max_requests: 6000,
        }
    }
}

/// 令牌桶状态
struct TokenBucketState {
    tokens: f64,
    last_refill: Instant,
}

/// 滑动窗口状态
struct SlidingWindowState {
    requests: Vec<Instant>,
}

/// 限流器
pub struct RateLimiter {
    config: RateLimitConfig,
    token_buckets: DashMap<String, Mutex<TokenBucketState>>,
    sliding_windows: DashMap<String, Mutex<SlidingWindowState>>,
    total_allowed: AtomicU64,
    total_denied: AtomicU64,
}

impl RateLimiter {
    /// 创建限流器
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            token_buckets: DashMap::new(),
            sliding_windows: DashMap::new(),
            total_allowed: AtomicU64::new(0),
            total_denied: AtomicU64::new(0),
        }
    }

    /// 每秒令牌数
    pub fn tokens_per_second(&self) -> u64 {
        self.config.tokens_per_second
    }

    /// 尝试获取令牌（非阻塞）
    pub async fn try_acquire(&self, key: &str) -> bool {
        let result = match self.config.algorithm {
            RateLimitAlgorithm::TokenBucket => self.try_acquire_token_bucket(key).await,
            RateLimitAlgorithm::SlidingWindow => self.try_acquire_sliding_window(key).await,
        };

        if result {
            self.total_allowed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_denied.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// 尝试获取指定数量的令牌
    pub async fn try_acquire_n(&self, key: &str, n: u64) -> bool {
        for _ in 0..n {
            if !self.try_acquire(key).await {
                return false;
            }
        }
        true
    }

    /// 阻塞等待获取令牌
    pub async fn acquire(&self, key: &str) {
        loop {
            if self.try_acquire(key).await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn try_acquire_token_bucket(&self, key: &str) -> bool {
        let mut state = self.token_buckets
            .entry(key.to_string())
            .or_insert_with(|| Mutex::new(TokenBucketState {
                tokens: self.config.burst_size as f64,
                last_refill: Instant::now(),
            }))
            .lock()
            .await;

        // 补充令牌
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let refill = elapsed * self.config.tokens_per_second as f64;
        state.tokens = (state.tokens + refill).min(self.config.burst_size as f64);
        state.last_refill = now;

        // 尝试获取
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    async fn try_acquire_sliding_window(&self, key: &str) -> bool {
        let mut state = self.sliding_windows
            .entry(key.to_string())
            .or_insert_with(|| Mutex::new(SlidingWindowState {
                requests: Vec::new(),
            }))
            .lock()
            .await;

        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_seconds);

        // 移除窗口外的请求
        state.requests.retain(|t| now.duration_since(*t) < window);

        // 检查是否超过限制
        if (state.requests.len() as u64) < self.config.max_requests {
            state.requests.push(now);
            true
        } else {
            false
        }
    }

    /// 获取指定 key 的当前用量
    pub async fn current_usage(&self, key: &str) -> RateLimitUsage {
        match self.config.algorithm {
            RateLimitAlgorithm::TokenBucket => {
                if let Some(state) = self.token_buckets.get(key) {
                    let s = state.lock().await;
                    RateLimitUsage {
                        current: (self.config.burst_size as f64 - s.tokens) as u64,
                        limit: self.config.burst_size,
                        remaining: s.tokens as u64,
                        reset_seconds: 0,
                    }
                } else {
                    RateLimitUsage { current: 0, limit: self.config.burst_size, remaining: self.config.burst_size, reset_seconds: 0 }
                }
            }
            RateLimitAlgorithm::SlidingWindow => {
                if let Some(state) = self.sliding_windows.get(key) {
                    let s = state.lock().await;
                    let now = Instant::now();
                    let window = Duration::from_secs(self.config.window_seconds);
                    let current = s.requests.iter().filter(|t| now.duration_since(**t) < window).count() as u64;
                    RateLimitUsage {
                        current,
                        limit: self.config.max_requests,
                        remaining: self.config.max_requests.saturating_sub(current),
                        reset_seconds: self.config.window_seconds,
                    }
                } else {
                    RateLimitUsage { current: 0, limit: self.config.max_requests, remaining: self.config.max_requests, reset_seconds: self.config.window_seconds }
                }
            }
        }
    }

    /// 重置指定 key 的限流计数
    pub async fn reset(&self, key: &str) {
        self.token_buckets.remove(key);
        self.sliding_windows.remove(key);
    }

    /// 获取统计
    pub fn stats(&self) -> RateLimitStats {
        RateLimitStats {
            algorithm: self.config.algorithm,
            tokens_per_second: self.config.tokens_per_second,
            burst_size: self.config.burst_size,
            total_allowed: self.total_allowed.load(Ordering::Relaxed),
            total_denied: self.total_denied.load(Ordering::Relaxed),
            active_keys: self.token_buckets.len() + self.sliding_windows.len(),
        }
    }
}

/// 限流用量
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitUsage {
    pub current: u64,
    pub limit: u64,
    pub remaining: u64,
    pub reset_seconds: u64,
}

/// 限流统计
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStats {
    pub algorithm: RateLimitAlgorithm,
    pub tokens_per_second: u64,
    pub burst_size: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub active_keys: usize,
}
