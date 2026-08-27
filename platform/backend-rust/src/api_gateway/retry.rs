// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 重试策略
//!
//! 支持：
//! - 指数退避（Exponential Backoff）
//! - 抖动（Jitter）防止惊群
//! - 可配置的最大重试次数
//! - 基于状态码的重试条件
//! - 基于异常类型的重试条件

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
    pub retry_on_status: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            retry_on_status: vec![500, 502, 503, 504],
        }
    }
}

/// 重试策略
pub struct RetryPolicy {
    config: RetryConfig,
    total_retries: std::sync::atomic::AtomicU64,
    total_success_on_retry: std::sync::atomic::AtomicU64,
    total_exhausted: std::sync::atomic::AtomicU64,
}

impl RetryPolicy {
    /// 创建重试策略
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            total_retries: std::sync::atomic::AtomicU64::new(0),
            total_success_on_retry: std::sync::atomic::AtomicU64::new(0),
            total_exhausted: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 最大重试次数
    pub fn max_attempts(&self) -> u32 {
        self.config.max_attempts
    }

    /// 计算第 n 次重试的延迟（指数退避 + 抖动）
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.config.initial_delay_ms as f64
            * self.config.backoff_multiplier.powi(attempt as i32);
        let capped = base.min(self.config.max_delay_ms as f64);

        // 添加抖动
        let jitter = if self.config.jitter_factor > 0.0 {
            let jitter_range = capped * self.config.jitter_factor;
            rand::thread_rng().gen_range(-jitter_range..jitter_range)
        } else {
            0.0
        };

        let delay_ms = (capped + jitter).max(1.0) as u64;
        Duration::from_millis(delay_ms)
    }

    /// 执行带重试的异步闭包
    pub async fn execute<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: Send + Clone,
    {
        let mut last_error: Option<E> = None;

        for attempt in 0..=self.config.max_attempts {
            if attempt > 0 {
                self.total_retries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let delay = self.delay_for_attempt(attempt - 1);
                sleep(delay).await;
            }

            match f().await {
                Ok(value) => {
                    if attempt > 0 {
                        self.total_success_on_retry.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    return Ok(value);
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    // 如果是最后一次尝试，不再重试
                    if attempt == self.config.max_attempts {
                        break;
                    }
                }
            }
        }

        self.total_exhausted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Err(last_error.expect("至少有一次错误"))
    }

    /// 执行带重试的 HTTP 请求（基于状态码判断）
    pub async fn execute_http<F, Fut>(&self, f: F) -> Result<reqwest::Response, reqwest::Error>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + Send,
    {
        let mut last_error: Option<reqwest::Error> = None;

        for attempt in 0..=self.config.max_attempts {
            if attempt > 0 {
                self.total_retries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let delay = self.delay_for_attempt(attempt - 1);
                sleep(delay).await;
            }

            match f().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    // 检查是否需要重试
                    if self.config.retry_on_status.contains(&status) && attempt < self.config.max_attempts {
                        continue;
                    }
                    if attempt > 0 && !self.config.retry_on_status.contains(&status) {
                        self.total_success_on_retry.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt == self.config.max_attempts {
                        break;
                    }
                }
            }
        }

        self.total_exhausted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Err(last_error.expect("至少有一次错误"))
    }

    /// 获取统计
    pub fn stats(&self) -> RetryStats {
        RetryStats {
            max_attempts: self.config.max_attempts,
            initial_delay_ms: self.config.initial_delay_ms,
            max_delay_ms: self.config.max_delay_ms,
            backoff_multiplier: self.config.backoff_multiplier,
            total_retries: self.total_retries.load(std::sync::atomic::Ordering::Relaxed),
            total_success_on_retry: self.total_success_on_retry.load(std::sync::atomic::Ordering::Relaxed),
            total_exhausted: self.total_exhausted.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// 重试统计
#[derive(Debug, Clone, Serialize)]
pub struct RetryStats {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub total_retries: u64,
    pub total_success_on_retry: u64,
    pub total_exhausted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy::new(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
            retry_on_status: vec![],
        });

        let d0 = policy.delay_for_attempt(0);
        assert!(d0.as_millis() >= 100 && d0.as_millis() <= 100);

        let d1 = policy.delay_for_attempt(1);
        assert!(d1.as_millis() >= 200 && d1.as_millis() <= 200);

        let d2 = policy.delay_for_attempt(2);
        assert!(d2.as_millis() >= 400 && d2.as_millis() <= 400);
    }

    #[tokio::test]
    async fn test_retry_success() {
        let policy = RetryPolicy::new(RetryConfig::default());
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, String> = policy.execute(move || {
            let count = attempts_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err("transient error".to_string())
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let policy = RetryPolicy::new(RetryConfig {
            max_attempts: 2,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 1.0,
            jitter_factor: 0.0,
            retry_on_status: vec![],
        });

        let result: Result<i32, String> = policy.execute(|| async {
            Err("always fail".to_string())
        }).await;

        assert!(result.is_err());
    }
}
