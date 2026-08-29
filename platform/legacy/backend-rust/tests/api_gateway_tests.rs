// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! API 网关单元测试
//!
//! 覆盖：限流、熔断、重试、负载均衡

use mox_enterprise_backend::api_gateway::*;
use std::time::Duration;

// ==================== 限流器测试 ====================

#[tokio::test]
async fn test_token_bucket_basic() {
    let limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 10,
        burst_size: 10,
        window_seconds: 60,
        max_requests: 600,
    });

    // 初始应该有 burst_size 个令牌
    for _ in 0..10 {
        assert!(limiter.try_acquire("test").await, "应该能获取令牌");
    }
    // 第 11 个应该失败
    assert!(!limiter.try_acquire("test").await, "令牌耗尽应该失败");
}

#[tokio::test]
async fn test_token_bucket_refill() {
    let limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 100,
        burst_size: 10,
        window_seconds: 60,
        max_requests: 600,
    });

    // 耗尽令牌
    for _ in 0..10 {
        assert!(limiter.try_acquire("refill_test").await);
    }
    assert!(!limiter.try_acquire("refill_test").await);

    // 等待补充
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 应该能获取到新补充的令牌
    assert!(limiter.try_acquire("refill_test").await, "等待后应该能获取令牌");
}

#[tokio::test]
async fn test_sliding_window() {
    let limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::SlidingWindow,
        tokens_per_second: 10,
        burst_size: 10,
        window_seconds: 1,
        max_requests: 5,
    });

    // 5 次应该成功
    for i in 0..5 {
        assert!(limiter.try_acquire("window_test").await, "第 {} 次应该成功", i);
    }
    // 第 6 次应该失败
    assert!(!limiter.try_acquire("window_test").await, "超过窗口限制应该失败");
}

#[tokio::test]
async fn test_rate_limiter_independent_keys() {
    let limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 10,
        burst_size: 5,
        window_seconds: 60,
        max_requests: 600,
    });

    // key1 耗尽
    for _ in 0..5 {
        assert!(limiter.try_acquire("key1").await);
    }
    assert!(!limiter.try_acquire("key1").await);

    // key2 应该不受影响
    for _ in 0..5 {
        assert!(limiter.try_acquire("key2").await, "key2 应该独立计数");
    }
}

// ==================== 熔断器测试 ====================

#[tokio::test]
async fn test_circuit_breaker_initial_state() {
    let cb = CircuitBreaker::new(CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 10,
        open_duration_ms: 1000,
        half_open_max_requests: 3,
        window_size_ms: 10000,
    });

    assert_eq!(cb.state().await, CircuitState::Closed);
    assert!(cb.can_execute().await, "初始状态应该允许执行");
}

#[tokio::test]
async fn test_circuit_breaker_opens_after_failures() {
    let cb = CircuitBreaker::new(CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 4,
        open_duration_ms: 10000,
        half_open_max_requests: 3,
        window_size_ms: 10000,
    });

    // 记录 4 次失败（超过 50% 阈值）
    for _ in 0..4 {
        cb.record_failure().await;
    }

    // 应该进入 Open 状态
    assert_eq!(cb.state().await, CircuitState::Open);
    assert!(!cb.can_execute().await, "Open 状态应该拒绝请求");
}

#[tokio::test]
async fn test_circuit_breaker_half_open_after_timeout() {
    let cb = CircuitBreaker::new(CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 4,
        open_duration_ms: 50,
        half_open_max_requests: 3,
        window_size_ms: 10000,
    });

    // 触发熔断
    for _ in 0..4 {
        cb.record_failure().await;
    }
    assert_eq!(cb.state().await, CircuitState::Open);

    // 等待超时
    tokio::time::sleep(Duration::from_millis(60)).await;

    // 应该进入 HalfOpen
    assert!(cb.can_execute().await, "超时后应该允许探测请求");
}

#[tokio::test]
async fn test_circuit_breaker_recovery_on_success() {
    let cb = CircuitBreaker::new(CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 4,
        open_duration_ms: 50,
        half_open_max_requests: 3,
        window_size_ms: 10000,
    });

    // 触发熔断
    for _ in 0..4 {
        cb.record_failure().await;
    }

    // 等待进入 HalfOpen
    tokio::time::sleep(Duration::from_millis(60)).await;

    // 调用 can_execute 触发状态转换
    assert!(cb.can_execute().await, "超时后应该允许探测请求");

    // 半开状态下，需要 half_open_max_requests 次成功请求才能恢复
    for _ in 0..5 {
        assert!(cb.can_execute().await, "半开状态应该允许探测请求");
        cb.record_success().await;
    }

    // 应该恢复到 Closed
    assert_eq!(cb.state().await, CircuitState::Closed);
}

// ==================== 重试策略测试 ====================

#[tokio::test]
async fn test_retry_success_first_attempt() {
    let policy = RetryPolicy::new(RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        retry_on_status: vec![500, 502, 503],
    });

    let call_count = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
    let cc = call_count.clone();

    let result: Result<i32, String> = policy.execute(move || {
        cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async { Ok(42) }
    }).await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1, "第一次成功不应重试");
}

#[tokio::test]
async fn test_retry_eventually_succeeds() {
    let policy = RetryPolicy::new(RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        retry_on_status: vec![500],
    });

    let call_count = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
    let cc = call_count.clone();

    let result: Result<i32, String> = policy.execute(move || {
        let count = cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            if count < 2 {
                Err("临时错误".to_string())
            } else {
                Ok(99)
            }
        }
    }).await;

    assert_eq!(result.unwrap(), 99);
    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3, "应该重试到第 3 次成功");
}

#[tokio::test]
async fn test_retry_exhausted() {
    let policy = RetryPolicy::new(RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        retry_on_status: vec![500],
    });

    let result: Result<i32, String> = policy.execute(|| async {
        Err("持续错误".to_string())
    }).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "持续错误");
}

#[test]
fn test_retry_backoff_calculation() {
    let policy = RetryPolicy::new(RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        retry_on_status: vec![500],
    });

    // 第 0 次重试：100ms
    // 第 1 次重试：200ms
    // 第 2 次重试：400ms
    assert_eq!(policy.delay_for_attempt(0).as_millis(), 100);
    assert_eq!(policy.delay_for_attempt(1).as_millis(), 200);
    assert_eq!(policy.delay_for_attempt(2).as_millis(), 400);
}

// ==================== 负载均衡测试 ====================

#[test]
fn test_round_robin_selection() {
    let config = GatewayConfig {
        upstream_services: vec![UpstreamService {
            name: "test".to_string(),
            path_prefix: "/test".to_string(),
            targets: vec!["http://a:8080".to_string(), "http://b:8080".to_string(), "http://c:8080".to_string()],
            load_balance: LoadBalanceStrategy::RoundRobin,
            timeout_ms: 5000,
            retries: 3,
            rate_limit_per_second: 100,
            circuit_breaker_threshold: 0.5,
        }],
        ..Default::default()
    };

    let gateway = ApiGateway::new(config);
    let svc = gateway.match_upstream("/test/path").unwrap();

    // 轮询应该依次返回 a, b, c
    let t1 = gateway.select_target(&svc);
    let t2 = gateway.select_target(&svc);
    let t3 = gateway.select_target(&svc);
    let t4 = gateway.select_target(&svc);

    assert_eq!(t1, "http://a:8080");
    assert_eq!(t2, "http://b:8080");
    assert_eq!(t3, "http://c:8080");
    assert_eq!(t4, "http://a:8080", "应该循环回到第一个");
}

#[test]
fn test_upstream_matching() {
    let config = GatewayConfig {
        upstream_services: vec![
            UpstreamService {
                name: "storage".to_string(),
                path_prefix: "/api/storage".to_string(),
                targets: vec!["http://storage:8080".to_string()],
                load_balance: LoadBalanceStrategy::RoundRobin,
                timeout_ms: 5000,
                retries: 3,
                rate_limit_per_second: 100,
                circuit_breaker_threshold: 0.5,
            },
            UpstreamService {
                name: "metadata".to_string(),
                path_prefix: "/api/metadata".to_string(),
                targets: vec!["http://metadata:8080".to_string()],
                load_balance: LoadBalanceStrategy::RoundRobin,
                timeout_ms: 5000,
                retries: 3,
                rate_limit_per_second: 100,
                circuit_breaker_threshold: 0.5,
            },
        ],
        ..Default::default()
    };

    let gateway = ApiGateway::new(config);

    assert_eq!(gateway.match_upstream("/api/storage/file").unwrap().name, "storage");
    assert_eq!(gateway.match_upstream("/api/metadata/search").unwrap().name, "metadata");
    assert!(gateway.match_upstream("/api/other/path").is_none(), "未匹配的路径应该返回 None");
}
