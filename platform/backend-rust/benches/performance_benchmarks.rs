//! MOX Enterprise 性能基准测试
//!
//! 覆盖：限流器 QPS、熔断器切换、异常检测吞吐量、数据血缘遍历

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use mox_enterprise_backend::api_gateway::*;
use mox_enterprise_backend::aiops::*;
use mox_enterprise_backend::data_quality::*;
use std::time::Duration;

// ==================== 限流器基准测试 ====================

fn benchmark_rate_limiter(c: &mut Criterion) {
    let mut group = c.benchmark_group("rate_limiter");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    // 令牌桶单 key 高并发
    let limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 100000,
        burst_size: 100000,
        window_seconds: 60,
        max_requests: 600000,
    });

    group.bench_function("token_bucket_single_key", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                limiter.try_acquire("bench_key").await
            })
    });

    // 滑动窗口
    let sw_limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::SlidingWindow,
        tokens_per_second: 100000,
        burst_size: 100000,
        window_seconds: 1,
        max_requests: 100000,
    });

    group.bench_function("sliding_window_single_key", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                sw_limiter.try_acquire("bench_sw").await
            })
    });

    // 多 key 场景
    let multi_limiter = RateLimiter::new(RateLimitConfig {
        algorithm: RateLimitAlgorithm::TokenBucket,
        tokens_per_second: 100000,
        burst_size: 100000,
        window_seconds: 60,
        max_requests: 600000,
    });

    let keys: Vec<String> = (0..100).map(|i| format!("key_{}", i)).collect();
    let mut key_idx = 0usize;

    group.bench_function("token_bucket_multi_key_100", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let key = &keys[key_idx % 100];
                key_idx += 1;
                multi_limiter.try_acquire(key).await
            })
    });

    group.finish();
}

// ==================== 熔断器基准测试 ====================

fn benchmark_circuit_breaker(c: &mut Criterion) {
    let mut group = c.benchmark_group("circuit_breaker");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(5));

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Closed 状态 can_execute
    let cb_closed = CircuitBreaker::new(CircuitConfig {
        failure_threshold: 0.5,
        minimum_requests: 100,
        open_duration_ms: 10000,
        half_open_max_requests: 10,
        window_size_ms: 60000,
    });

    group.bench_function("can_execute_closed", |b| {
        b.to_async(&rt).iter(|| async {
            cb_closed.can_execute().await
        })
    });

    // record_success
    group.bench_function("record_success", |b| {
        b.to_async(&rt).iter(|| async {
            cb_closed.record_success().await
        })
    });

    // record_failure
    group.bench_function("record_failure", |b| {
        b.to_async(&rt).iter(|| async {
            cb_closed.record_failure().await
        })
    });

    group.finish();
}

// ==================== 异常检测基准测试 ====================

fn benchmark_anomaly_detector(c: &mut Criterion) {
    let mut group = c.benchmark_group("anomaly_detector");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    // 3σ 算法
    let detector = AnomalyDetector::new(AnomalyConfig {
        algorithm: AnomalyAlgorithm::ThreeSigma,
        window_size: 100,
        threshold: 3.0,
        sensitivity: 1.0,
        min_data_points: 10,
    });

    // 预热数据
    for i in 0..50 {
        detector.detect("metric", 50.0 + (i as f64 * 0.1), None);
    }

    group.bench_function("three_sigma_detection", |b| {
        let mut val = 50.0f64;
        b.iter(|| {
            val += 0.01;
            detector.detect("metric", val, None)
        })
    });

    // EWMA 算法
    let ewma_detector = AnomalyDetector::new(AnomalyConfig {
        algorithm: AnomalyAlgorithm::EWMA,
        window_size: 100,
        threshold: 3.0,
        sensitivity: 1.0,
        min_data_points: 10,
    });

    for i in 0..50 {
        ewma_detector.detect("metric", 50.0 + (i as f64 * 0.1), None);
    }

    group.bench_function("ewma_detection", |b| {
        let mut val = 50.0f64;
        b.iter(|| {
            val += 0.01;
            ewma_detector.detect("metric", val, None)
        })
    });

    // 多指标并发
    let multi_detector = AnomalyDetector::new(AnomalyConfig {
        algorithm: AnomalyAlgorithm::ThreeSigma,
        window_size: 100,
        threshold: 3.0,
        sensitivity: 1.0,
        min_data_points: 10,
    });

    for m in 0..10 {
        for i in 0..50 {
            multi_detector.detect(&format!("metric_{}", m), 50.0 + (i as f64 * 0.1), None);
        }
    }

    group.bench_function("multi_metric_10_detection", |b| {
        let mut idx = 0usize;
        b.iter(|| {
            idx += 1;
            let metric = format!("metric_{}", idx % 10);
            multi_detector.detect(&metric, 50.5, None)
        })
    });

    group.finish();
}

// ==================== 预测性扩缩容基准测试 ====================

fn benchmark_predictive_scaler(c: &mut Criterion) {
    let mut group = c.benchmark_group("predictive_scaler");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(5));

    let scaler = PredictiveScaler::new(PredictiveScalerConfig {
        algorithm: PredictionAlgorithm::Combined,
        window_size: 100,
        prediction_horizon_seconds: 300,
        scale_up_threshold: 0.7,
        scale_down_threshold: 0.3,
        min_replicas: 1,
        max_replicas: 100,
        cooldown_seconds: 0,
        target_utilization: 0.6,
        safety_margin: 0.2,
        moving_average_window: 10,
        exponential_alpha: 0.3,
    });

    // 预热数据
    for i in 0..80 {
        scaler.record_load("service", 0.5 + (i as f64 * 0.005), None);
    }
    scaler.set_current_replicas("service", 5);

    // 负载预测
    group.bench_function("predict_load_combined", |b| {
        b.iter(|| scaler.predict_load("service"))
    });

    // 扩缩容评估
    group.bench_function("evaluate_scaling", |b| {
        b.iter(|| scaler.evaluate("service"))
    });

    // 单算法对比
    for algo in [PredictionAlgorithm::MovingAverage, PredictionAlgorithm::ExponentialSmoothing, PredictionAlgorithm::LinearRegression] {
        let s = PredictiveScaler::new(PredictiveScalerConfig {
            algorithm: algo,
            window_size: 100,
            prediction_horizon_seconds: 300,
            scale_up_threshold: 0.7,
            scale_down_threshold: 0.3,
            min_replicas: 1,
            max_replicas: 100,
            cooldown_seconds: 0,
            target_utilization: 0.6,
            safety_margin: 0.2,
            moving_average_window: 10,
            exponential_alpha: 0.3,
        });
        for i in 0..80 {
            s.record_load("svc", 0.5 + (i as f64 * 0.005), None);
        }

        let name = format!("predict_{:?}", algo);
        group.bench_with_input(BenchmarkId::from_parameter(name), &s, |b, s| {
            b.iter(|| s.predict_load("svc"))
        });
    }

    group.finish();
}

// ==================== 数据质量规则引擎基准测试 ====================

fn benchmark_quality_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("quality_engine");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(5));

    let engine = QualityRuleEngine::new();

    // 添加 100 条规则
    for i in 0..100 {
        engine.add_rule(QualityRule {
            id: String::new(),
            name: format!("rule_{}", i),
            description: String::new(),
            asset_id: format!("asset_{}", i % 10),
            dimension: QualityDimension::all()[i % 6],
            rule_type: QualityRuleType::NotNull,
            field_name: None,
            expression: None,
            threshold: 0.9,
            severity: QualitySeverity::Medium,
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        });
    }

    // 按资产查询
    group.bench_function("get_rules_by_asset", |b| {
        b.iter(|| engine.get_rules_by_asset("asset_5"))
    });

    // 按维度查询
    group.bench_function("get_rules_by_dimension", |b| {
        b.iter(|| engine.get_rules_by_dimension(QualityDimension::Completeness))
    });

    // 生成报告
    let results: Vec<QualityResult> = (0..50).map(|i| QualityResult {
        rule_id: format!("r{}", i),
        asset_id: "asset_0".to_string(),
        dimension: QualityDimension::all()[i % 6],
        score: 0.8 + (i as f64 * 0.003),
        passed: i % 5 != 0,
        details: String::new(),
        evaluated_at: String::new(),
    }).collect();

    group.bench_function("generate_report_50_rules", |b| {
        b.iter(|| engine.generate_report("asset_0", &results))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_rate_limiter,
    benchmark_circuit_breaker,
    benchmark_anomaly_detector,
    benchmark_predictive_scaler,
    benchmark_quality_engine,
);
criterion_main!(benches);
