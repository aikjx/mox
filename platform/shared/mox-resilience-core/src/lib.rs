//! mox-resilience-core: 企业级弹性容错核心
//!
//! 提供三大高可用保障机制：
//! - **重试（Retry）**：固定间隔/指数退避/带抖动指数退避，自动重试临时性错误
//! - **熔断（Circuit Breaker）**：Closed/Open/HalfOpen 三态，失败率超阈值快速失败
//! - **降级（Fallback）**：静态值/自定义函数/缓存，服务不可用时保障核心功能
//!
//! 三者可组合使用：重试解决瞬时故障，熔断防止级联失败，降级保障用户体验。

pub mod circuit_breaker;
pub mod fallback;
pub mod retry;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState,
};
pub use fallback::{
    Fallback, FallbackExecutor, FallbackResult, FunctionFallback, NoFallback, StaticFallback,
};
pub use retry::{BackoffStrategy, RetryPolicy, Retryable};

use std::time::{Duration, Instant};

/// 弹性执行器：组合重试 + 熔断 + 降级
pub struct ResilienceExecutor<T: Clone + Send + Sync + 'static> {
    retry_policy: Option<RetryPolicy>,
    circuit_breaker: Option<CircuitBreaker>,
    fallback: Option<Arc<dyn Fallback<T>>>,
}

impl<T: Clone + Send + Sync + 'static> ResilienceExecutor<T> {
    /// 创建弹性执行器构建器
    pub fn builder() -> ResilienceExecutorBuilder<T> {
        ResilienceExecutorBuilder::new()
    }

    /// 执行异步操作，自动应用重试、熔断、降级
    pub async fn execute<Fut, F>(&self, operation: F) -> Result<T, String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let start = Instant::now();
        let max_retries = self.retry_policy.as_ref().map(|p| p.max_retries).unwrap_or(0);

        for attempt in 0..=max_retries {
            // 熔断检查
            if let Some(cb) = &self.circuit_breaker {
                if !cb.allow_request() {
                    tracing::warn!("Circuit breaker open, rejecting request (attempt {})", attempt);
                    return self.try_fallback("Circuit breaker open").await;
                }
            }

            // 执行操作
            let result = operation().await;

            match &result {
                Ok(value) => {
                    // 记录成功
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_success();
                    }
                    return Ok(value.clone());
                }
                Err(error) => {
                    // 记录失败
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_failure();
                    }

                    // 判断是否重试
                    if attempt < max_retries {
                        if let Some(policy) = &self.retry_policy {
                            if policy.should_retry(attempt, start.elapsed()) {
                                let delay = policy.retry_delay(attempt);
                                tracing::warn!(
                                    "Operation failed (attempt {}), retrying in {:?}: {}",
                                    attempt + 1,
                                    delay,
                                    error
                                );
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                        }
                    }

                    // 重试耗尽，尝试降级
                    return self.try_fallback(error).await;
                }
            }
        }

        Err("Max retries exceeded".to_string())
    }

    async fn try_fallback(&self, error: &str) -> Result<T, String> {
        if let Some(fallback) = &self.fallback {
            match fallback.fallback(error).await {
                FallbackResult::Fallback(value) => {
                    tracing::warn!("Using fallback value for error: {}", error);
                    return Ok(value);
                }
                FallbackResult::Propagate => {}
            }
        }
        Err(error.to_string())
    }
}

use std::sync::Arc;

/// 弹性执行器构建器
pub struct ResilienceExecutorBuilder<T: Clone + Send + Sync + 'static> {
    retry_policy: Option<RetryPolicy>,
    circuit_breaker: Option<CircuitBreaker>,
    fallback: Option<Arc<dyn Fallback<T>>>,
}

impl<T: Clone + Send + Sync + 'static> ResilienceExecutorBuilder<T> {
    fn new() -> Self {
        Self {
            retry_policy: None,
            circuit_breaker: None,
            fallback: None,
        }
    }

    /// 设置重试策略
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// 设置熔断器
    pub fn with_circuit_breaker(mut self, cb: CircuitBreaker) -> Self {
        self.circuit_breaker = Some(cb);
        self
    }

    /// 设置降级策略
    pub fn with_fallback(mut self, fallback: Arc<dyn Fallback<T>>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// 构建弹性执行器
    pub fn build(self) -> ResilienceExecutor<T> {
        ResilienceExecutor {
            retry_policy: self.retry_policy,
            circuit_breaker: self.circuit_breaker,
            fallback: self.fallback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resilience_executor_success() {
        let executor: ResilienceExecutor<i32> = ResilienceExecutor::builder().build();
        let result = executor.execute(|| async { Ok(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_resilience_executor_with_retry() {
        let policy = RetryPolicy::fixed(2, Duration::from_millis(10));
        let executor: ResilienceExecutor<i32> = ResilienceExecutor::builder()
            .with_retry(policy)
            .build();

        let attempts = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let attempts_clone = attempts.clone();

        let result = executor
            .execute(move || {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count < 1 {
                        Err("temporary error".to_string())
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_resilience_executor_with_fallback() {
        let fallback = StaticFallback::new(-1);
        let executor: ResilienceExecutor<i32> = ResilienceExecutor::builder()
            .with_fallback(Arc::new(fallback))
            .build();

        let result = executor
            .execute(|| async { Err("fatal error".to_string()) })
            .await;
        assert_eq!(result.unwrap(), -1);
    }

    #[tokio::test]
    async fn test_resilience_executor_with_circuit_breaker() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new("test", config);
        let executor: ResilienceExecutor<i32> = ResilienceExecutor::builder()
            .with_circuit_breaker(cb.clone())
            .build();

        // 触发熔断
        let _ = executor.execute(|| async { Err("error".to_string()) }).await;
        let _ = executor.execute(|| async { Err("error".to_string()) }).await;

        assert_eq!(cb.state(), CircuitState::Open);

        // 熔断打开后，请求应被拒绝
        let result = executor.execute(|| async { Ok(42) }).await;
        assert!(result.is_err());
    }
}
