// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! T · AIOps 智能运维
//!
//! 核心能力：
//! - 异常检测：统计方法（3σ、EWMA、变化点检测）
//! - 根因分析：依赖图遍历、相关性分析、故障传播
//! - 预测性扩缩容：时间序列预测、负载预测、提前扩缩
//! - AIOps 仪表盘：异常汇总、根因报告、预测趋势、智能建议

pub mod root_cause;
pub mod predictive_scaler;
pub mod dashboard;

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

pub use root_cause::{RootCauseAnalyzer, RootCauseResult, FaultPropagationPath};
pub use predictive_scaler::{PredictiveScaler, ScalingRecommendation, ScalingDecision};
pub use dashboard::{AiopsDashboard, AiopsReport, IntelligentSuggestion};

/// 异常检测算法
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AnomalyAlgorithm {
    ThreeSigma,
    EWMA,
    ZScore,
    IQR,
    ChangePoint,
    Combined,
}

/// 异常严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// 异常类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AnomalyType {
    Spike,
    Drop,
    TrendChange,
    VarianceChange,
    LevelShift,
    SeasonalDeviation,
}

/// 异常事件
#[derive(Debug, Clone, Serialize)]
pub struct AnomalyEvent {
    pub id: String,
    pub metric_name: String,
    pub timestamp: String,
    pub value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub deviation_percentage: f64,
    pub severity: AnomalySeverity,
    pub anomaly_type: AnomalyType,
    pub algorithm: AnomalyAlgorithm,
    pub confidence: f64,
    pub description: String,
    pub acknowledged: bool,
    pub resolved: bool,
}

/// 指标数据点
#[derive(Debug, Clone, Copy)]
pub struct MetricPoint {
    pub timestamp: f64,
    pub value: f64,
}

/// 异常检测器
pub struct AnomalyDetector {
    config: RwLock<AnomalyConfig>,
    history: RwLock<std::collections::HashMap<String, VecDeque<MetricPoint>>>,
    anomalies: RwLock<Vec<AnomalyEvent>>,
    total_checks: std::sync::atomic::AtomicU64,
    total_anomalies: std::sync::atomic::AtomicU64,
}

/// 异常检测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    pub algorithm: AnomalyAlgorithm,
    pub window_size: usize,
    pub sigma_threshold: f64,
    pub ewma_alpha: f64,
    pub iqr_multiplier: f64,
    pub min_history_points: usize,
    pub cooldown_seconds: u64,
    pub critical_threshold: f64,
    pub high_threshold: f64,
    pub medium_threshold: f64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            algorithm: AnomalyAlgorithm::Combined,
            window_size: 100,
            sigma_threshold: 3.0,
            ewma_alpha: 0.3,
            iqr_multiplier: 1.5,
            min_history_points: 10,
            cooldown_seconds: 60,
            critical_threshold: 5.0,
            high_threshold: 3.5,
            medium_threshold: 2.0,
        }
    }
}

impl AnomalyDetector {
    /// 创建异常检测器
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            config: RwLock::new(config),
            history: RwLock::new(std::collections::HashMap::new()),
            anomalies: RwLock::new(Vec::new()),
            total_checks: std::sync::atomic::AtomicU64::new(0),
            total_anomalies: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 添加指标数据点
    pub fn add_metric(&self, metric_name: &str, value: f64, timestamp: Option<f64>) {
        let point = MetricPoint {
            timestamp: timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp_millis() as f64 / 1000.0),
            value,
        };

        let config = self.config.read().unwrap();
        let mut history = self.history.write().unwrap();
        let queue = history.entry(metric_name.to_string()).or_insert_with(VecDeque::new);
        queue.push_back(point);
        if queue.len() > config.window_size {
            queue.pop_front();
        }
    }

    /// 检测异常
    pub fn detect(&self, metric_name: &str, value: f64) -> Option<AnomalyEvent> {
        self.total_checks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let config = self.config.read().unwrap();
        let history = self.history.read().unwrap();
        let queue = history.get(metric_name)?;

        if queue.len() < config.min_history_points {
            return None;
        }

        let values: Vec<f64> = queue.iter().map(|p| p.value).collect();
        let mean = self.calculate_mean(&values);
        let std_dev = self.calculate_std_dev(&values, mean);

        match config.algorithm {
            AnomalyAlgorithm::ThreeSigma | AnomalyAlgorithm::Combined => {
                self.detect_three_sigma(metric_name, value, mean, std_dev, &config)
            }
            AnomalyAlgorithm::ZScore => {
                self.detect_zscore(metric_name, value, mean, std_dev, &config)
            }
            AnomalyAlgorithm::IQR => {
                self.detect_iqr(metric_name, value, &values, &config)
            }
            AnomalyAlgorithm::EWMA => {
                self.detect_ewma(metric_name, value, &values, &config)
            }
            AnomalyAlgorithm::ChangePoint => {
                self.detect_change_point(metric_name, value, &values, &config)
            }
        }
    }

    fn detect_three_sigma(&self, metric: &str, value: f64, mean: f64, std_dev: f64, config: &AnomalyConfig) -> Option<AnomalyEvent> {
        if std_dev == 0.0 { return None; }

        let z_score = (value - mean) / std_dev;
        if z_score.abs() < config.sigma_threshold {
            return None;
        }

        let severity = if z_score.abs() >= config.critical_threshold {
            AnomalySeverity::Critical
        } else if z_score.abs() >= config.high_threshold {
            AnomalySeverity::High
        } else if z_score.abs() >= config.medium_threshold {
            AnomalySeverity::Medium
        } else {
            AnomalySeverity::Low
        };

        let anomaly_type = if z_score > 0.0 { AnomalyType::Spike } else { AnomalyType::Drop };

        Some(self.create_anomaly(metric, value, mean, z_score, severity, anomaly_type, AnomalyAlgorithm::ThreeSigma))
    }

    fn detect_zscore(&self, metric: &str, value: f64, mean: f64, std_dev: f64, config: &AnomalyConfig) -> Option<AnomalyEvent> {
        self.detect_three_sigma(metric, value, mean, std_dev, config)
    }

    fn detect_iqr(&self, metric: &str, value: f64, values: &[f64], config: &AnomalyConfig) -> Option<AnomalyEvent> {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1 = sorted[sorted.len() / 4];
        let q3 = sorted[3 * sorted.len() / 4];
        let iqr = q3 - q1;
        let lower_bound = q1 - config.iqr_multiplier * iqr;
        let upper_bound = q3 + config.iqr_multiplier * iqr;

        if value >= lower_bound && value <= upper_bound {
            return None;
        }

        let median = sorted[sorted.len() / 2];
        let deviation = if value > upper_bound {
            value - upper_bound
        } else {
            lower_bound - value
        };

        let severity = if deviation > iqr * 2.0 {
            AnomalySeverity::Critical
        } else if deviation > iqr {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Medium
        };

        let anomaly_type = if value > upper_bound { AnomalyType::Spike } else { AnomalyType::Drop };

        Some(self.create_anomaly(metric, value, median, deviation / iqr, severity, anomaly_type, AnomalyAlgorithm::IQR))
    }

    fn detect_ewma(&self, metric: &str, value: f64, values: &[f64], config: &AnomalyConfig) -> Option<AnomalyEvent> {
        let mut ewma = values[0];
        for &v in &values[1..] {
            ewma = config.ewma_alpha * v + (1.0 - config.ewma_alpha) * ewma;
        }

        let variance: f64 = values.iter().map(|v| (v - ewma).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 { return None; }

        let deviation = (value - ewma) / std_dev;
        if deviation.abs() < config.sigma_threshold {
            return None;
        }

        let severity = if deviation.abs() >= config.critical_threshold {
            AnomalySeverity::Critical
        } else if deviation.abs() >= config.high_threshold {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Medium
        };

        let anomaly_type = if deviation > 0.0 { AnomalyType::Spike } else { AnomalyType::Drop };

        Some(self.create_anomaly(metric, value, ewma, deviation, severity, anomaly_type, AnomalyAlgorithm::EWMA))
    }

    fn detect_change_point(&self, metric: &str, value: f64, values: &[f64], config: &AnomalyConfig) -> Option<AnomalyEvent> {
        let mid = values.len() / 2;
        let first_half = &values[..mid];
        let second_half = &values[mid..];

        let mean1 = self.calculate_mean(first_half);
        let mean2 = self.calculate_mean(second_half);
        let std1 = self.calculate_std_dev(first_half, mean1);
        let std2 = self.calculate_std_dev(second_half, mean2);

        // CUSUM 变化点检测
        let mut cusum = 0.0;
        let threshold = config.sigma_threshold * ((std1 + std2) / 2.0);

        for &v in values {
            cusum += v - (mean1 + mean2) / 2.0;
            if cusum.abs() > threshold {
                let deviation = (value - mean2) / std2.max(0.001);
                if deviation.abs() > config.medium_threshold {
                    let severity = if deviation.abs() > config.critical_threshold {
                        AnomalySeverity::Critical
                    } else {
                        AnomalySeverity::High
                    };
                    return Some(self.create_anomaly(metric, value, mean2, deviation, severity, AnomalyType::LevelShift, AnomalyAlgorithm::ChangePoint));
                }
            }
        }

        None
    }

    fn create_anomaly(&self, metric: &str, value: f64, expected: f64, deviation: f64, severity: AnomalySeverity, anomaly_type: AnomalyType, algorithm: AnomalyAlgorithm) -> AnomalyEvent {
        let deviation_pct = if expected != 0.0 {
            (value - expected).abs() / expected.abs() * 100.0
        } else {
            100.0
        };

        let event = AnomalyEvent {
            id: uuid::Uuid::new_v4().to_string(),
            metric_name: metric.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            value,
            expected_value: expected,
            deviation,
            deviation_percentage: deviation_pct,
            severity,
            anomaly_type,
            algorithm,
            confidence: (deviation.abs() / 5.0).min(1.0) * 100.0,
            description: format!(
                "指标 {} 检测到异常: 当前值 {:.2}, 期望值 {:.2}, 偏离 {:.2}σ",
                metric, value, expected, deviation
            ),
            acknowledged: false,
            resolved: false,
        };

        self.anomalies.write().unwrap().push(event.clone());
        self.total_anomalies.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        event
    }

    fn calculate_mean(&self, values: &[f64]) -> f64 {
        if values.is_empty() { return 0.0; }
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn calculate_std_dev(&self, values: &[f64], mean: f64) -> f64 {
        if values.len() < 2 { return 0.0; }
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    }

    /// 确认异常
    pub fn acknowledge_anomaly(&self, anomaly_id: &str) -> bool {
        if let Some(mut anomalies) = self.anomalies.write().ok() {
            if let Some(anomaly) = anomalies.iter_mut().find(|a| a.id == anomaly_id) {
                anomaly.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// 解决异常
    pub fn resolve_anomaly(&self, anomaly_id: &str) -> bool {
        if let Some(mut anomalies) = self.anomalies.write().ok() {
            if let Some(anomaly) = anomalies.iter_mut().find(|a| a.id == anomaly_id) {
                anomaly.resolved = true;
                return true;
            }
        }
        false
    }

    /// 获取异常列表
    pub fn get_anomalies(&self, severity: Option<AnomalySeverity>, unresolved_only: bool) -> Vec<AnomalyEvent> {
        self.anomalies.read().unwrap()
            .iter()
            .filter(|a| severity.map_or(true, |s| a.severity == s))
            .filter(|a| !unresolved_only || !a.resolved)
            .cloned()
            .collect()
    }

    /// 更新配置
    pub fn update_config(&self, config: AnomalyConfig) {
        *self.config.write().unwrap() = config;
    }

    /// 获取统计
    pub fn stats(&self) -> AnomalyDetectorStats {
        let anomalies = self.anomalies.read().unwrap();
        AnomalyDetectorStats {
            config: self.config.read().unwrap().clone(),
            total_checks: self.total_checks.load(std::sync::atomic::Ordering::Relaxed),
            total_anomalies: self.total_anomalies.load(std::sync::atomic::Ordering::Relaxed),
            unresolved_anomalies: anomalies.iter().filter(|a| !a.resolved).count(),
            critical_anomalies: anomalies.iter().filter(|a| a.severity == AnomalySeverity::Critical && !a.resolved).count(),
            monitored_metrics: self.history.read().unwrap().len(),
        }
    }
}

/// 异常检测器统计
#[derive(Debug, Clone, Serialize)]
pub struct AnomalyDetectorStats {
    pub config: AnomalyConfig,
    pub total_checks: u64,
    pub total_anomalies: u64,
    pub unresolved_anomalies: usize,
    pub critical_anomalies: usize,
    pub monitored_metrics: usize,
}
