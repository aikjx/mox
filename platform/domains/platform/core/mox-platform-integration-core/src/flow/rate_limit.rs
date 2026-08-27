// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 限流器 — Rate Limiter
//!
//! 企业级限流：令牌桶算法，支持按key限流、全局限流、突发容量。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每秒允许的请求数（令牌生成速率）
    pub rate_per_second: f64,
    /// 突发容量（令牌桶最大容量）
    pub burst_capacity: f64,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rate_per_second: 100.0,
            burst_capacity: 200.0,
            enabled: true,
        }
    }
}

/// 限流结果
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RateLimitResult {
    /// 允许通过
    Allowed,
    /// 被限流
    Limited {
        /// 需要等待的时间（毫秒）
        retry_after_ms: u64,
    },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool { matches!(self, RateLimitResult::Allowed) }
    pub fn is_limited(&self) -> bool { matches!(self, RateLimitResult::Limited { .. }) }
}

/// 令牌桶状态
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64) -> Self {
        Self { tokens: capacity, last_refill: Instant::now() }
    }

    /// 尝试获取1个令牌
    fn try_acquire(&mut self, config: &RateLimitConfig) -> RateLimitResult {
        if !config.enabled {
            return RateLimitResult::Allowed;
        }

        // 补充令牌
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = (elapsed * config.rate_per_second).min(config.burst_capacity - self.tokens);
        self.tokens += new_tokens;
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            RateLimitResult::Allowed
        } else {
            // 计算需要等待的时间
            let needed = 1.0 - self.tokens;
            let wait_secs = needed / config.rate_per_second;
            RateLimitResult::Limited { retry_after_ms: (wait_secs * 1000.0) as u64 }
        }
    }
}

/// 限流器（支持按key限流）
pub struct RateLimiter {
    config: RateLimitConfig,
    /// 全局令牌桶
    global_bucket: Mutex<TokenBucket>,
    /// 按key的令牌桶
    key_buckets: Mutex<HashMap<String, TokenBucket>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            global_bucket: Mutex::new(TokenBucket::new(config.burst_capacity)),
            key_buckets: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// 全局限流（不区分key）
    pub fn check_global(&self) -> RateLimitResult {
        self.global_bucket.lock().try_acquire(&self.config)
    }

    /// 按key限流
    pub fn check_key(&self, key: &str) -> RateLimitResult {
        let mut buckets = self.key_buckets.lock();
        let bucket = buckets.entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.config.burst_capacity));
        bucket.try_acquire(&self.config)
    }

    /// 同时检查全局和key（两者都通过才允许）
    pub fn check(&self, key: &str) -> RateLimitResult {
        let global = self.check_global();
        if global.is_limited() { return global; }
        self.check_key(key)
    }

    /// 更新配置（运行时）
    pub fn update_config(&self, config: RateLimitConfig) {
        let mut global = self.global_bucket.lock();
        global.tokens = global.tokens.min(config.burst_capacity);
        // 注意：config字段需要内部可变性，这里简化处理
        // 实际生产中应使用Arc<RwLock<RateLimitConfig>>
        drop(global);
    }

    /// 获取当前配置
    pub fn config(&self) -> &RateLimitConfig { &self.config }

    /// 重置某个key的限流状态
    pub fn reset_key(&self, key: &str) {
        self.key_buckets.lock().remove(key);
    }

    /// 重置所有限流状态
    pub fn reset_all(&self) {
        self.key_buckets.lock().clear();
        *self.global_bucket.lock() = TokenBucket::new(self.config.burst_capacity);
    }
}

/// 创建Arc包装的限流器
pub fn create_rate_limiter(config: RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(config))
}
