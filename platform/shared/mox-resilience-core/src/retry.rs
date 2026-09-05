//! 重试策略模块
//!
//! 支持固定间隔、指数退避、带抖动的指数退避三种重试策略，
//! 可配置最大重试次数、最大间隔、可重试错误判断。

use std::time::Duration;

/// 退避策略
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed(Duration),
    /// 指数退避（初始间隔 * 2^attempt）
    Exponential {
        initial: Duration,
        max: Duration,
    },
    /// 带抖动的指数退避（防止惊群效应）
    ExponentialWithJitter {
        initial: Duration,
        max: Duration,
        jitter_factor: f64,
    },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::ExponentialWithJitter {
            initial: Duration::from_millis(100),
            max: Duration::from_secs(10),
            jitter_factor: 0.5,
        }
    }
}

impl BackoffStrategy {
    /// 计算第 n 次重试的等待时间（attempt 从 0 开始）
    pub fn delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed(d) => *d,
            Self::Exponential { initial, max } => {
                let delay = initial.mul_f64(2f64.powi(attempt as i32));
                delay.min(*max)
            }
            Self::ExponentialWithJitter {
                initial,
                max,
                jitter_factor,
            } => {
                let base = initial.mul_f64(2f64.powi(attempt as i32));
                let base = base.min(*max);
                // 抖动范围：[base * (1-jitter), base]
                let jitter = base.mul_f64(rand_factor() * jitter_factor);
                base.saturating_sub(jitter)
            }
        }
    }
}

/// 简单的伪随机因子（0.0 ~ 1.0），避免引入 rand 依赖
fn rand_factor() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    (hasher.finish() % 10000) as f64 / 10000.0
}

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数（不含首次执行）
    pub max_retries: u32,
    /// 退避策略
    pub backoff: BackoffStrategy,
    /// 总超时时间（可选）
    pub total_timeout: Option<Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffStrategy::default(),
            total_timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl RetryPolicy {
    /// 创建固定间隔重试策略
    pub fn fixed(max_retries: u32, interval: Duration) -> Self {
        Self {
            max_retries,
            backoff: BackoffStrategy::Fixed(interval),
            total_timeout: None,
        }
    }

    /// 创建指数退避重试策略
    pub fn exponential(max_retries: u32, initial: Duration, max: Duration) -> Self {
        Self {
            max_retries,
            backoff: BackoffStrategy::Exponential { initial, max },
            total_timeout: None,
        }
    }

    /// 判断是否应该重试
    pub fn should_retry(&self, attempt: u32, elapsed: Duration) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        if let Some(timeout) = self.total_timeout {
            if elapsed >= timeout {
                return false;
            }
        }
        true
    }

    /// 获取第 n 次重试的等待时间
    pub fn retry_delay(&self, attempt: u32) -> Duration {
        self.backoff.delay(attempt)
    }
}

/// 可重试错误判断 trait
pub trait Retryable {
    /// 判断此错误是否可重试（临时性错误 vs 永久性错误）
    fn is_retryable(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_backoff() {
        let strategy = BackoffStrategy::Fixed(Duration::from_millis(500));
        assert_eq!(strategy.delay(0), Duration::from_millis(500));
        assert_eq!(strategy.delay(5), Duration::from_millis(500));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = BackoffStrategy::Exponential {
            initial: Duration::from_millis(100),
            max: Duration::from_secs(5),
        };
        assert_eq!(strategy.delay(0), Duration::from_millis(100));
        assert_eq!(strategy.delay(1), Duration::from_millis(200));
        assert_eq!(strategy.delay(2), Duration::from_millis(400));
        assert_eq!(strategy.delay(3), Duration::from_millis(800));
        // 超过最大值
        assert_eq!(strategy.delay(10), Duration::from_secs(5));
    }

    #[test]
    fn test_exponential_with_jitter() {
        let strategy = BackoffStrategy::ExponentialWithJitter {
            initial: Duration::from_millis(100),
            max: Duration::from_secs(5),
            jitter_factor: 0.5,
        };
        // 抖动后的延迟应在 [base*0.5, base] 范围内
        for attempt in 0..5 {
            let delay = strategy.delay(attempt);
            let base = Duration::from_millis(100).mul_f64(2f64.powi(attempt as i32));
            let base = base.min(Duration::from_secs(5));
            assert!(delay <= base, "delay {:?} should <= base {:?}", delay, base);
            assert!(delay >= base.mul_f64(0.5), "delay {:?} should >= base*0.5 {:?}", delay, base.mul_f64(0.5));
        }
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let policy = RetryPolicy {
            max_retries: 3,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(100)),
            total_timeout: Some(Duration::from_secs(10)),
        };
        assert!(policy.should_retry(0, Duration::from_secs(1)));
        assert!(policy.should_retry(2, Duration::from_secs(1)));
        assert!(!policy.should_retry(3, Duration::from_secs(1))); // 超过最大重试次数
        assert!(!policy.should_retry(1, Duration::from_secs(15))); // 超过总超时
    }

    #[test]
    fn test_retry_policy_retry_delay() {
        let policy = RetryPolicy::exponential(3, Duration::from_millis(100), Duration::from_secs(5));
        assert_eq!(policy.retry_delay(0), Duration::from_millis(100));
        assert_eq!(policy.retry_delay(1), Duration::from_millis(200));
    }

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert!(policy.total_timeout.is_some());
    }
}
