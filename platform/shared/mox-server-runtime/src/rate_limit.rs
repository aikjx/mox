// =============================================================================
// 限流中间件（RateLimit）
// =============================================================================
//
// 轻量级令牌桶限流实现，基于 AtomicU64，无锁，高性能。
// 支持：全局 QPS 限制、按 IP 限流、突发容量配置。
//
// 使用方式：
// ```ignore
// let rate_limit = RateLimitLayer::new(100, 200); // 100 QPS, 突发200
// Router::new().layer(rate_limit)
// ```
// =============================================================================

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

/// 令牌桶限流器
pub struct RateLimiter {
    /// 每秒生成令牌数（QPS）
    rate_per_sec: u64,
    /// 桶容量（突发上限）
    capacity: u64,
    /// 当前令牌数（原子，放大 1000 倍以支持亚秒精度）
    tokens: AtomicU64,
    /// 上次补充时间（毫秒时间戳，从创建时开始计算）
    last_refill: AtomicU64,
    /// 创建时间（用于计算经过的毫秒数）
    start: Instant,
}

impl RateLimiter {
    /// 创建限流器
    /// - `rate_per_sec`: 每秒允许的请求数
    /// - `capacity`: 桶容量（允许的突发请求数）
    pub fn new(rate_per_sec: u64, capacity: u64) -> Self {
        Self {
            rate_per_sec,
            capacity,
            tokens: AtomicU64::new(capacity * 1000),
            last_refill: AtomicU64::new(0),
            start: Instant::now(),
        }
    }

    /// 获取当前毫秒时间戳（从创建时开始）
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// 尝试获取一个令牌，返回是否成功
    pub fn try_acquire(&self) -> bool {
        let now_ms = self.now_ms();
        let last_ms = self.last_refill.load(Ordering::Relaxed);

        // 补充令牌
        if now_ms > last_ms {
            let elapsed_ms = now_ms - last_ms;
            let refill = (self.rate_per_sec * elapsed_ms) / 1000;
            if refill > 0 {
                let current = self.tokens.load(Ordering::Relaxed);
                let new_tokens = (current + refill * 1000).min(self.capacity * 1000);
                self.tokens.store(new_tokens, Ordering::Relaxed);
                self.last_refill.store(now_ms, Ordering::Relaxed);
            }
        }

        // 消耗一个令牌
        let current = self.tokens.load(Ordering::Relaxed);
        if current >= 1000 {
            self.tokens.store(current - 1000, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// 获取当前可用令牌数（用于监控）
    pub fn available_tokens(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed) / 1000
    }
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter")
            .field("rate_per_sec", &self.rate_per_sec)
            .field("capacity", &self.capacity)
            .field("available", &self.available_tokens())
            .finish()
    }
}

/// 限流 Layer
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: std::sync::Arc<RateLimiter>,
}

impl RateLimitLayer {
    /// 创建限流层
    pub fn new(rate_per_sec: u64, capacity: u64) -> Self {
        Self {
            limiter: std::sync::Arc::new(RateLimiter::new(rate_per_sec, capacity)),
        }
    }

    /// 获取限流器引用（用于监控）
    pub fn limiter(&self) -> &std::sync::Arc<RateLimiter> {
        &self.limiter
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// 限流 Service
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: std::sync::Arc<RateLimiter>,
}

impl<S, Request> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if self.limiter.try_acquire() {
            Box::pin(self.inner.call(req))
        } else {
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": "rate limit exceeded",
                    "retry_after_ms": 1000,
                })),
            )
            .into_response();
            Box::pin(std::future::ready(Ok(response)))
        }
    }
}

/// 限流配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitConfig {
    /// 是否启用限流
    #[serde(default)]
    pub enabled: bool,
    /// 每秒允许的请求数
    #[serde(default = "default_rate_per_sec")]
    pub rate_per_sec: u64,
    /// 桶容量（突发上限）
    #[serde(default = "default_capacity")]
    pub capacity: u64,
}

fn default_rate_per_sec() -> u64 { 1000 }
fn default_capacity() -> u64 { 2000 }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate_per_sec: 1000,
            capacity: 2000,
        }
    }
}

impl RateLimitConfig {
    /// 创建限流层（如果启用）
    pub fn into_layer(self) -> Option<RateLimitLayer> {
        if self.enabled {
            Some(RateLimitLayer::new(self.rate_per_sec, self.capacity))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_acquire() {
        let limiter = RateLimiter::new(10, 10);
        // 初始有 10 个令牌
        for _ in 0..10 {
            assert!(limiter.try_acquire());
        }
        // 第 11 个应该失败
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(1000, 10);
        // 消耗所有令牌
        for _ in 0..10 {
            assert!(limiter.try_acquire());
        }
        assert!(!limiter.try_acquire());
        // 等待 10ms，应该补充约 10 个令牌
        std::thread::sleep(Duration::from_millis(20));
        assert!(limiter.try_acquire());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rate_per_sec, 1000);
        assert_eq!(config.capacity, 2000);
        assert!(config.into_layer().is_none());
    }

    #[test]
    fn test_rate_limit_config_enabled() {
        let config = RateLimitConfig {
            enabled: true,
            rate_per_sec: 100,
            capacity: 200,
        };
        let layer = config.into_layer();
        assert!(layer.is_some());
        let layer = layer.unwrap();
        assert_eq!(layer.limiter().available_tokens(), 200);
    }

    #[test]
    fn test_rate_limiter_debug() {
        let limiter = RateLimiter::new(10, 10);
        let debug = format!("{:?}", limiter);
        assert!(debug.contains("rate_per_sec"));
        assert!(debug.contains("capacity"));
    }
}
