//! Rate limiting middleware.
//!
//! Implements token bucket rate limiting per client IP / API key.
//! Uses L2 api contracts only, no direct L3/L4 dependencies.

use crate::config::RateLimitConfig;
use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Token bucket rate limiter.
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<parking_lot::Mutex<HashMap<String, TokenBucket>>>,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, amount: f64) -> bool {
        self.refill();
        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        let max_tokens = (config.max_requests + config.burst) as f64;
        let refill_rate = config.max_requests as f64 / config.window_secs as f64;
        Self {
            config,
            buckets: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    /// Check if a request from the given client should be allowed.
    pub fn check(&self, client_id: &str) -> bool {
        if !self.config.enabled { return true; }
        let mut buckets = self.buckets.lock();
        let bucket = buckets.entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(
                (self.config.max_requests + self.config.burst) as f64,
                self.config.max_requests as f64 / self.config.window_secs as f64,
            ));
        bucket.try_consume(1.0)
    }

    /// Get the current token count for a client.
    pub fn remaining(&self, client_id: &str) -> f64 {
        let buckets = self.buckets.lock();
        buckets.get(client_id).map(|b| b.tokens).unwrap_or(0.0)
    }

    /// Reset rate limit for a client.
    pub fn reset(&self, client_id: &str) {
        let mut buckets = self.buckets.lock();
        buckets.remove(client_id);
    }

    /// Get rate limit statistics.
    pub fn stats(&self) -> RateLimitStats {
        let buckets = self.buckets.lock();
        RateLimitStats {
            total_clients: buckets.len(),
            enabled: self.config.enabled,
            max_requests: self.config.max_requests,
            window_secs: self.config.window_secs,
        }
    }
}

/// Rate limit statistics.
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStats {
    pub total_clients: usize,
    pub enabled: bool,
    pub max_requests: u32,
    pub window_secs: u64,
}

use serde::Serialize;

/// Extract client identifier from request (IP or API key).
fn extract_client_id(request: &Request) -> String {
    // Try API key first
    if let Some(api_key) = request.headers().get("X-API-Key") {
        return format!("api:{}", api_key.to_str().unwrap_or("unknown"));
    }
    // Try X-Forwarded-For
    if let Some(xff) = request.headers().get("X-Forwarded-For") {
        return xff.to_str().unwrap_or("unknown").split(',').next().unwrap_or("unknown").trim().to_string();
    }
    // Fallback to remote addr
    request.extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Axum middleware for rate limiting.
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_id = extract_client_id(&request);
    if limiter.check(&client_id) {
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-remaining"),
            header::HeaderValue::from_str(&limiter.remaining(&client_id).to_string()).unwrap_or(header::HeaderValue::from_static("0")),
        );
        Ok(response)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_basic() {
        let config = RateLimitConfig {
            enabled: true,
            max_requests: 5,
            window_secs: 60,
            burst: 0,
        };
        let limiter = RateLimiter::new(config);
        for i in 0..5 {
            assert!(limiter.check(&format!("client-{}", i)), "request {} should pass", i);
        }
        // Same client should be limited
        assert!(limiter.check("client-0"), "first request passes");
        assert!(limiter.check("client-0"), "second request passes");
        assert!(limiter.check("client-0"), "third request passes");
        assert!(limiter.check("client-0"), "fourth request passes");
        assert!(limiter.check("client-0"), "fifth request passes");
        assert!(!limiter.check("client-0"), "sixth request should be limited");
    }

    #[test]
    fn test_rate_limit_disabled() {
        let config = RateLimitConfig { enabled: false, ..Default::default() };
        let limiter = RateLimiter::new(config);
        for _ in 0..1000 {
            assert!(limiter.check("client"));
        }
    }

    #[test]
    fn test_rate_limit_reset() {
        let config = RateLimitConfig { enabled: true, max_requests: 2, window_secs: 60, burst: 0 };
        let limiter = RateLimiter::new(config);
        assert!(limiter.check("client"));
        assert!(limiter.check("client"));
        assert!(!limiter.check("client"));
        limiter.reset("client");
        assert!(limiter.check("client"));
    }
}
