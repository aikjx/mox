//! MOX 企业级后端性能基准测试
//!
//! 使用 criterion 进行微基准测试，覆盖核心模块的性能特征。
//!
//! 运行方式：cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mox_enterprise_backend::aiops::{AnomalyConfig, AnomalyDetector, PredictiveScaler};
use mox_enterprise_backend::aiops::predictive_scaler::PredictiveScalerConfig;
use mox_enterprise_backend::api_gateway::circuit_breaker::{CircuitBreaker, CircuitConfig};
use mox_enterprise_backend::api_gateway::rate_limiter::{RateLimiter, RateLimitConfig, RateLimitAlgorithm};
use std::time::Duration;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// ============================================================================
// 限流器基准
// ============================================================================

fn bench_rate_limiter_token_bucket(c: &mut Criterion) {
    let config = RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 100000,
        burst_size: 200000,
        window_seconds: 1,
        max_requests: 100000,
    };
    let limiter = RateLimiter::new(config);
    let runtime = rt();

    c.bench_function("rate_limiter_token_bucket_try_acquire", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ = black_box(limiter.try_acquire("bench-client").await);
            });
        });
    });
}

fn bench_rate_limiter_sliding_window(c: &mut Criterion) {
    let config = RateLimitConfig {
        algorithm: RateLimitAlgorithm::SlidingWindow,
        tokens_per_second: 100000,
        burst_size: 200000,
        window_seconds: 1,
        max_requests: 100000,
    };
    let limiter = RateLimiter::new(config);
    let runtime = rt();

    c.bench_function("rate_limiter_sliding_window_try_acquire", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ = black_box(limiter.try_acquire("bench-client").await);
            });
        });
    });
}

// ============================================================================
// 熔断器基准
// ============================================================================

fn bench_circuit_breaker_can_execute(c: &mut Criterion) {
    let config = CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 1000,
        open_duration_ms: 30000,
        half_open_max_requests: 5,
        window_size_ms: 10000,
    };
    let cb = CircuitBreaker::new(config);
    let runtime = rt();

    c.bench_function("circuit_breaker_can_execute_closed", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ = black_box(cb.can_execute().await);
            });
        });
    });
}

fn bench_circuit_breaker_record_success(c: &mut Criterion) {
    let config = CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 100000,
        open_duration_ms: 30000,
        half_open_max_requests: 5,
        window_size_ms: 10000,
    };
    let cb = CircuitBreaker::new(config);
    let runtime = rt();

    c.bench_function("circuit_breaker_record_success", |b| {
        b.iter(|| {
            runtime.block_on(async {
                black_box(cb.record_success().await);
            });
        });
    });
}

// ============================================================================
// 异常检测基准
// ============================================================================

fn bench_anomaly_detector_detect_normal(c: &mut Criterion) {
    let detector = AnomalyDetector::new(AnomalyConfig::default());

    // 预热：添加历史数据
    for i in 0..100 {
        detector.add_metric("bench-metric", 50.0 + (i as f64 * 0.01), None);
    }

    c.bench_function("anomaly_detector_detect_normal", |b| {
        b.iter(|| {
            let _ = black_box(detector.detect("bench-metric", 51.0));
        });
    });
}

fn bench_anomaly_detector_add_metric(c: &mut Criterion) {
    let detector = AnomalyDetector::new(AnomalyConfig::default());

    c.bench_function("anomaly_detector_add_metric", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            black_box(detector.add_metric("bench-metric", 50.0, Some(i as f64)));
        });
    });
}

// ============================================================================
// 预测性扩缩容基准
// ============================================================================

fn bench_predictive_scaler_evaluate(c: &mut Criterion) {
    let scaler = PredictiveScaler::new(PredictiveScalerConfig::default());

    // 预热：添加历史负载
    for i in 0..50 {
        scaler.record_load("bench-service", 0.5 + (i as f64 * 0.001), None);
    }

    c.bench_function("predictive_scaler_evaluate", |b| {
        b.iter(|| {
            let _ = black_box(scaler.evaluate("bench-service"));
        });
    });
}

// ============================================================================
// 基准组配置
// ============================================================================

criterion_group!(
    name = rate_limiter;
    config = Criterion::default().measurement_time(Duration::from_secs(3));
    targets = bench_rate_limiter_token_bucket, bench_rate_limiter_sliding_window
);

criterion_group!(
    name = circuit_breaker;
    config = Criterion::default().measurement_time(Duration::from_secs(3));
    targets = bench_circuit_breaker_can_execute, bench_circuit_breaker_record_success
);

criterion_group!(
    name = anomaly_detector;
    config = Criterion::default().measurement_time(Duration::from_secs(3));
    targets = bench_anomaly_detector_detect_normal, bench_anomaly_detector_add_metric
);

criterion_group!(
    name = predictive_scaler;
    config = Criterion::default().measurement_time(Duration::from_secs(3));
    targets = bench_predictive_scaler_evaluate
);

criterion_main!(rate_limiter, circuit_breaker, anomaly_detector, predictive_scaler);
