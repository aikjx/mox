//! 降级策略模块
//!
//! 当服务不可用或超时时，提供降级机制保障核心功能可用：
//! - StaticFallback：返回静态默认值
//! - CacheFallback：从缓存读取上次成功结果
//! - FunctionFallback：执行自定义降级函数

use std::sync::Arc;

/// 降级结果
#[derive(Debug, Clone)]
pub enum FallbackResult<T> {
    /// 使用降级值
    Fallback(T),
    /// 无降级值，传播错误
    Propagate,
}

/// 降级策略 trait
#[async_trait::async_trait]
pub trait Fallback<T>: Send + Sync {
    /// 获取降级值
    async fn fallback(&self, error: &str) -> FallbackResult<T>;
}

/// 静态降级：返回固定默认值
pub struct StaticFallback<T: Clone + Send + Sync> {
    value: T,
}

impl<T: Clone + Send + Sync> StaticFallback<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

#[async_trait::async_trait]
impl<T: Clone + Send + Sync + 'static> Fallback<T> for StaticFallback<T> {
    async fn fallback(&self, _error: &str) -> FallbackResult<T> {
        FallbackResult::Fallback(self.value.clone())
    }
}

/// 函数降级：执行自定义降级函数
pub struct FunctionFallback<T, F>
where
    F: Fn(&str) -> FallbackResult<T> + Send + Sync,
{
    func: Arc<F>,
    _marker: std::marker::PhantomData<T>,
}

impl<T, F> FunctionFallback<T, F>
where
    F: Fn(&str) -> FallbackResult<T> + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func: Arc::new(func),
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T, F> Fallback<T> for FunctionFallback<T, F>
where
    F: Fn(&str) -> FallbackResult<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn fallback(&self, error: &str) -> FallbackResult<T> {
        (self.func)(error)
    }
}

/// 无降级：直接传播错误
pub struct NoFallback<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for NoFallback<T> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync + 'static> Fallback<T> for NoFallback<T> {
    async fn fallback(&self, _error: &str) -> FallbackResult<T> {
        FallbackResult::Propagate
    }
}

/// 带降级的执行器
pub struct FallbackExecutor<T> {
    fallback: Arc<dyn Fallback<T>>,
}

impl<T: Send + Sync + 'static> FallbackExecutor<T> {
    pub fn new(fallback: Arc<dyn Fallback<T>>) -> Self {
        Self { fallback }
    }

    /// 执行异步操作，失败时尝试降级
    pub async fn execute<Fut>(&self, operation: Fut) -> Result<T, String>
    where
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        match operation.await {
            Ok(value) => Ok(value),
            Err(error) => match self.fallback.fallback(&error).await {
                FallbackResult::Fallback(value) => {
                    tracing::warn!("Operation failed, using fallback: {}", error);
                    Ok(value)
                }
                FallbackResult::Propagate => Err(error),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_fallback() {
        let fallback = StaticFallback::new(42);
        let executor = FallbackExecutor::new(Arc::new(fallback));

        // 成功操作
        let result = executor.execute(async { Ok(100) }).await;
        assert_eq!(result.unwrap(), 100);

        // 失败操作，使用降级值
        let result = executor.execute(async { Err("error".to_string()) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_function_fallback() {
        let fallback = FunctionFallback::new(|error: &str| {
            if error.contains("retryable") {
                FallbackResult::Fallback(0)
            } else {
                FallbackResult::Propagate
            }
        });
        let executor = FallbackExecutor::new(Arc::new(fallback));

        // 可重试错误，使用降级值
        let result = executor
            .execute(async { Err("retryable error".to_string()) })
            .await;
        assert_eq!(result.unwrap(), 0);

        // 不可重试错误，传播错误
        let result = executor
            .execute(async { Err("fatal error".to_string()) })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_fallback() {
        let fallback: NoFallback<i32> = NoFallback::default();
        let executor = FallbackExecutor::new(Arc::new(fallback));

        let result = executor
            .execute(async { Err("error".to_string()) })
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_result_variants() {
        let r1: FallbackResult<i32> = FallbackResult::Fallback(42);
        let r2: FallbackResult<i32> = FallbackResult::Propagate;
        assert!(matches!(r1, FallbackResult::Fallback(_)));
        assert!(matches!(r2, FallbackResult::Propagate));
    }
}
