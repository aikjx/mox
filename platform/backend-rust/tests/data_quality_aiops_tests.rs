//! 数据质量 + AIOps 单元测试（简化版，API 对齐）

use mox_enterprise_backend::aiops::*;
use mox_enterprise_backend::aiops::predictive_scaler::{PredictiveScalerConfig, PredictionAlgorithm, ScalingDecision};
use mox_enterprise_backend::data_quality::QualityDimension;

// ==================== AIOps 异常检测测试 ====================

#[test]
fn test_anomaly_detector_insufficient_data() {
    let detector = AnomalyDetector::new(AnomalyConfig::default());

    // 数据不足时不应该报警
    for i in 0..3 {
        detector.add_metric("metric2", i as f64, None);
    }
    let result = detector.detect("metric2", 100.0);
    assert!(result.is_none(), "数据不足时应该返回 None");
}

#[test]
fn test_anomaly_detector_basic_detection() {
    let detector = AnomalyDetector::new(AnomalyConfig::default());

    // 建立基线（使用 add_metric 添加数据点）
    for i in 0..15 {
        detector.add_metric("metric1", 50.0 + (i as f64 * 0.1), None);
    }

    // 正常点不应该报警
    let normal = detector.detect("metric1", 51.0);
    assert!(normal.is_none(), "正常值不应该报警");

    // 异常点应该报警
    let anomaly = detector.detect("metric1", 200.0);
    assert!(anomaly.is_some(), "异常值应该报警");
    if let Some(event) = anomaly {
        assert!(event.value == 200.0);
        assert!(event.metric_name == "metric1");
    }
}

// ==================== AIOps 预测性扩缩容测试 ====================

#[test]
fn test_predictive_scaler_moving_average() {
    let scaler = PredictiveScaler::new(PredictiveScalerConfig {
        algorithm: PredictionAlgorithm::MovingAverage,
        window_size: 20,
        prediction_horizon_seconds: 300,
        scale_up_threshold: 0.7,
        scale_down_threshold: 0.3,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 0,
        target_utilization: 0.6,
        safety_margin: 0.2,
        moving_average_window: 5,
        exponential_alpha: 0.3,
    });

    // 记录递增负载
    for i in 1..=10 {
        scaler.record_load("service1", i as f64 * 0.1, None);
    }

    let prediction = scaler.predict_load("service1");
    assert!(prediction.is_some(), "应该能预测负载");
}

#[test]
fn test_predictive_scaler_scale_up_decision() {
    let scaler = PredictiveScaler::new(PredictiveScalerConfig {
        algorithm: PredictionAlgorithm::MovingAverage,
        window_size: 20,
        prediction_horizon_seconds: 300,
        scale_up_threshold: 0.7,
        scale_down_threshold: 0.3,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 0,
        target_utilization: 0.6,
        safety_margin: 0.1,
        moving_average_window: 5,
        exponential_alpha: 0.3,
    });

    scaler.set_current_replicas("service2", 2);

    // 高负载
    for _ in 0..10 {
        scaler.record_load("service2", 0.9, None);
    }

    let recommendation = scaler.evaluate("service2");
    assert!(recommendation.is_some(), "应该生成扩缩容建议");
    let rec = recommendation.unwrap();
    assert!(rec.recommended_replicas >= 1 && rec.recommended_replicas <= 10);
}

#[test]
fn test_predictive_scaler_scale_down_decision() {
    let scaler = PredictiveScaler::new(PredictiveScalerConfig {
        algorithm: PredictionAlgorithm::MovingAverage,
        window_size: 20,
        prediction_horizon_seconds: 300,
        scale_up_threshold: 0.7,
        scale_down_threshold: 0.3,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 0,
        target_utilization: 0.6,
        safety_margin: 0.1,
        moving_average_window: 5,
        exponential_alpha: 0.3,
    });

    scaler.set_current_replicas("service3", 5);

    // 低负载
    for _ in 0..10 {
        scaler.record_load("service3", 0.1, None);
    }

    let recommendation = scaler.evaluate("service3");
    assert!(recommendation.is_some(), "应该生成扩缩容建议");
}

#[test]
fn test_predictive_scaler_cooldown() {
    let scaler = PredictiveScaler::new(PredictiveScalerConfig {
        algorithm: PredictionAlgorithm::MovingAverage,
        window_size: 20,
        prediction_horizon_seconds: 300,
        scale_up_threshold: 0.7,
        scale_down_threshold: 0.3,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 3600, // 1 小时冷却
        target_utilization: 0.6,
        safety_margin: 0.1,
        moving_average_window: 5,
        exponential_alpha: 0.3,
    });

    scaler.set_current_replicas("service4", 1);

    // 高负载
    for _ in 0..10 {
        scaler.record_load("service4", 0.9, None);
    }

    // 先执行一次扩容
    let rec1 = scaler.evaluate("service4").unwrap();
    assert_eq!(rec1.decision, ScalingDecision::ScaleUp, "高负载应该建议扩容");
    scaler.execute_scaling("service4", &rec1);

    // 冷却期内应该 Hold
    let rec2 = scaler.evaluate("service4").unwrap();
    assert_eq!(rec2.decision, ScalingDecision::Hold, "冷却期内应该保持");
    assert!(rec2.cooldown_remaining_seconds > 0, "应该有剩余冷却时间");
}

// ==================== 数据质量维度测试 ====================

#[test]
fn test_quality_dimensions() {
    let all = QualityDimension::all();
    assert_eq!(all.len(), 6, "应该有 6 个质量维度");

    assert_eq!(QualityDimension::Completeness.description(), "数据完整性：非空率、必填字段覆盖率");
    assert_eq!(QualityDimension::Accuracy.description(), "数据准确性：值范围、格式校验、参照完整性");
}
