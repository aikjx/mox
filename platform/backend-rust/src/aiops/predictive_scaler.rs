//! 预测性扩缩容
//!
//! 核心能力：
//! - 时间序列预测（移动平均、线性回归、指数平滑）
//! - 负载趋势预测
//! - 提前扩缩容决策
//! - 扩缩容冷却时间管理
//! - 扩缩容历史与效果评估

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

/// 扩缩容决策
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ScalingDecision {
    ScaleUp,
    ScaleDown,
    Hold,
}

/// 扩缩容建议
#[derive(Debug, Clone, Serialize)]
pub struct ScalingRecommendation {
    pub id: String,
    pub resource: String,
    pub current_replicas: u32,
    pub recommended_replicas: u32,
    pub decision: ScalingDecision,
    pub reason: String,
    pub predicted_load: f64,
    pub current_load: f64,
    pub confidence: f64,
    pub cooldown_remaining_seconds: u64,
    pub created_at: String,
    pub executed: bool,
}

/// 预测算法
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PredictionAlgorithm {
    MovingAverage,
    ExponentialSmoothing,
    LinearRegression,
    ARIMA,
    Combined,
}

/// 预测性扩缩容配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveScalerConfig {
    pub algorithm: PredictionAlgorithm,
    pub window_size: usize,
    pub prediction_horizon_seconds: u64,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub cooldown_seconds: u64,
    pub target_utilization: f64,
    pub safety_margin: f64,
    pub moving_average_window: usize,
    pub exponential_alpha: f64,
}

impl Default for PredictiveScalerConfig {
    fn default() -> Self {
        Self {
            algorithm: PredictionAlgorithm::Combined,
            window_size: 60,
            prediction_horizon_seconds: 300,
            scale_up_threshold: 0.7,
            scale_down_threshold: 0.3,
            min_replicas: 1,
            max_replicas: 100,
            cooldown_seconds: 300,
            target_utilization: 0.6,
            safety_margin: 0.2,
            moving_average_window: 10,
            exponential_alpha: 0.3,
        }
    }
}

/// 负载数据点
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct LoadPoint {
    timestamp: f64,
    value: f64,
}

/// 预测性扩缩容器
pub struct PredictiveScaler {
    config: RwLock<PredictiveScalerConfig>,
    load_history: RwLock<std::collections::HashMap<String, VecDeque<LoadPoint>>>,
    current_replicas: RwLock<std::collections::HashMap<String, u32>>,
    last_scale_time: RwLock<std::collections::HashMap<String, Instant>>,
    recommendations: RwLock<Vec<ScalingRecommendation>>,
    total_predictions: std::sync::atomic::AtomicU64,
    total_scale_ups: std::sync::atomic::AtomicU64,
    total_scale_downs: std::sync::atomic::AtomicU64,
}

impl PredictiveScaler {
    /// 创建预测性扩缩容器
    pub fn new(config: PredictiveScalerConfig) -> Self {
        Self {
            config: RwLock::new(config),
            load_history: RwLock::new(std::collections::HashMap::new()),
            current_replicas: RwLock::new(std::collections::HashMap::new()),
            last_scale_time: RwLock::new(std::collections::HashMap::new()),
            recommendations: RwLock::new(Vec::new()),
            total_predictions: std::sync::atomic::AtomicU64::new(0),
            total_scale_ups: std::sync::atomic::AtomicU64::new(0),
            total_scale_downs: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 记录负载数据
    pub fn record_load(&self, resource: &str, load: f64, timestamp: Option<f64>) {
        let point = LoadPoint {
            timestamp: timestamp.unwrap_or_else(|| Instant::now().elapsed().as_secs_f64()),
            value: load.max(0.0),
        };

        let config = self.config.read().unwrap();
        let mut history = self.load_history.write().unwrap();
        let queue = history.entry(resource.to_string()).or_insert_with(VecDeque::new);
        queue.push_back(point);
        if queue.len() > config.window_size {
            queue.pop_front();
        }
    }

    /// 设置当前副本数
    pub fn set_current_replicas(&self, resource: &str, replicas: u32) {
        self.current_replicas.write().unwrap().insert(resource.to_string(), replicas);
    }

    /// 预测负载
    pub fn predict_load(&self, resource: &str) -> Option<f64> {
        self.total_predictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let config = self.config.read().unwrap();
        let history = self.load_history.read().unwrap();
        let queue = history.get(resource)?;

        if queue.len() < 3 {
            return None;
        }

        let values: Vec<f64> = queue.iter().map(|p| p.value).collect();

        match config.algorithm {
            PredictionAlgorithm::MovingAverage => Some(self.predict_moving_average(&values, &config)),
            PredictionAlgorithm::ExponentialSmoothing => Some(self.predict_exponential_smoothing(&values, &config)),
            PredictionAlgorithm::LinearRegression => Some(self.predict_linear_regression(&values)),
            PredictionAlgorithm::ARIMA => Some(self.predict_arima(&values, &config)),
            PredictionAlgorithm::Combined => {
                let ma = self.predict_moving_average(&values, &config);
                let es = self.predict_exponential_smoothing(&values, &config);
                let lr = self.predict_linear_regression(&values);
                Some((ma + es + lr) / 3.0)
            }
        }
    }

    fn predict_moving_average(&self, values: &[f64], config: &PredictiveScalerConfig) -> f64 {
        let window = config.moving_average_window.min(values.len());
        let recent: &[f64] = &values[values.len() - window..];
        recent.iter().sum::<f64>() / window as f64
    }

    fn predict_exponential_smoothing(&self, values: &[f64], config: &PredictiveScalerConfig) -> f64 {
        let mut smoothed = values[0];
        for &v in &values[1..] {
            smoothed = config.exponential_alpha * v + (1.0 - config.exponential_alpha) * smoothed;
        }
        smoothed
    }

    fn predict_linear_regression(&self, values: &[f64]) -> f64 {
        let n = values.len() as f64;
        let xs: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();

        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = xs.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = xs.iter().map(|x| x * x).sum();

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.abs() < 0.0001 {
            return sum_y / n;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        // 预测下一个时间点
        intercept + slope * n
    }

    fn predict_arima(&self, values: &[f64], _config: &PredictiveScalerConfig) -> f64 {
        // 简化的 ARIMA(1,1,1) 实现
        if values.len() < 3 {
            return values.last().copied().unwrap_or(0.0);
        }

        // 差分
        let diff: Vec<f64> = values.windows(2).map(|w| w[1] - w[0]).collect();

        // AR(1) 系数
        let ar_coef = if diff.len() > 1 {
            let mean: f64 = diff.iter().sum::<f64>() / diff.len() as f64;
            let mut num = 0.0;
            let mut den = 0.0;
            for i in 1..diff.len() {
                num += (diff[i] - mean) * (diff[i-1] - mean);
                den += (diff[i-1] - mean).powi(2);
            }
            if den.abs() > 0.0001 { num / den } else { 0.0 }
        } else {
            0.0
        };

        // 预测差分值
        let last_diff = diff.last().copied().unwrap_or(0.0);
        let predicted_diff = ar_coef * last_diff;

        // 还原
        values.last().copied().unwrap_or(0.0) + predicted_diff
    }

    /// 评估扩缩容决策
    pub fn evaluate(&self, resource: &str) -> Option<ScalingRecommendation> {
        let config = self.config.read().unwrap();
        let predicted_load = self.predict_load(resource)?;
        let current_load = self.load_history.read().unwrap()
            .get(resource)
            .and_then(|q| q.back().map(|p| p.value))
            .unwrap_or(0.0);

        let current_replicas = *self.current_replicas.read().unwrap()
            .get(resource)
            .unwrap_or(&config.min_replicas);

        // 检查冷却时间
        let cooldown_remaining = self.last_scale_time.read().unwrap()
            .get(resource)
            .map(|t| {
                let elapsed = t.elapsed().as_secs();
                config.cooldown_seconds.saturating_sub(elapsed)
            })
            .unwrap_or(0);

        // 计算所需副本数
        let target_load = config.target_utilization * (1.0 + config.safety_margin);
        let required_replicas = if target_load > 0.0 {
            ((predicted_load / target_load).ceil() as u32)
                .max(config.min_replicas)
                .min(config.max_replicas)
        } else {
            current_replicas
        };

        let (decision, reason) = if cooldown_remaining > 0 {
            (ScalingDecision::Hold, format!("冷却中，剩余 {} 秒", cooldown_remaining))
        } else if required_replicas > current_replicas && predicted_load > config.scale_up_threshold {
            (ScalingDecision::ScaleUp, format!(
                "预测负载 {:.0}% 超过扩容阈值 {:.0}%，建议从 {} 扩容到 {}",
                predicted_load * 100.0, config.scale_up_threshold * 100.0, current_replicas, required_replicas
            ))
        } else if required_replicas < current_replicas && predicted_load < config.scale_down_threshold {
            (ScalingDecision::ScaleDown, format!(
                "预测负载 {:.0}% 低于缩容阈值 {:.0}%，建议从 {} 缩容到 {}",
                predicted_load * 100.0, config.scale_down_threshold * 100.0, current_replicas, required_replicas
            ))
        } else {
            (ScalingDecision::Hold, format!(
                "预测负载 {:.0}% 在稳定区间，当前 {} 副本合适",
                predicted_load * 100.0, current_replicas
            ))
        };

        let confidence = if predicted_load > 0.0 {
            (1.0 - (predicted_load - current_load).abs() / predicted_load).max(0.0).min(1.0) * 100.0
        } else {
            50.0
        };

        let recommendation = ScalingRecommendation {
            id: Uuid::new_v4().to_string(),
            resource: resource.to_string(),
            current_replicas,
            recommended_replicas: required_replicas,
            decision,
            reason,
            predicted_load,
            current_load,
            confidence,
            cooldown_remaining_seconds: cooldown_remaining,
            created_at: chrono::Utc::now().to_rfc3339(),
            executed: false,
        };

        self.recommendations.write().unwrap().push(recommendation.clone());
        Some(recommendation)
    }

    /// 执行扩缩容
    pub fn execute_scaling(&self, resource: &str, recommendation: &ScalingRecommendation) -> bool {
        if recommendation.executed {
            return false;
        }

        match recommendation.decision {
            ScalingDecision::ScaleUp => {
                self.total_scale_ups.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.current_replicas.write().unwrap().insert(resource.to_string(), recommendation.recommended_replicas);
                self.last_scale_time.write().unwrap().insert(resource.to_string(), Instant::now());
                true
            }
            ScalingDecision::ScaleDown => {
                self.total_scale_downs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.current_replicas.write().unwrap().insert(resource.to_string(), recommendation.recommended_replicas);
                self.last_scale_time.write().unwrap().insert(resource.to_string(), Instant::now());
                true
            }
            ScalingDecision::Hold => false,
        }
    }

    /// 获取扩缩容历史
    pub fn get_history(&self, resource: Option<&str>, limit: usize) -> Vec<ScalingRecommendation> {
        self.recommendations.read().unwrap()
            .iter()
            .filter(|r| resource.map_or(true, |res| r.resource == res))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 更新配置
    pub fn update_config(&self, config: PredictiveScalerConfig) {
        *self.config.write().unwrap() = config;
    }

    /// 获取统计
    pub fn stats(&self) -> PredictiveScalerStats {
        let recommendations = self.recommendations.read().unwrap();
        PredictiveScalerStats {
            config: self.config.read().unwrap().clone(),
            monitored_resources: self.load_history.read().unwrap().len(),
            total_predictions: self.total_predictions.load(std::sync::atomic::Ordering::Relaxed),
            total_scale_ups: self.total_scale_ups.load(std::sync::atomic::Ordering::Relaxed),
            total_scale_downs: self.total_scale_downs.load(std::sync::atomic::Ordering::Relaxed),
            total_recommendations: recommendations.len(),
            executed_recommendations: recommendations.iter().filter(|r| r.executed).count(),
        }
    }
}

/// 预测性扩缩容统计
#[derive(Debug, Clone, Serialize)]
pub struct PredictiveScalerStats {
    pub config: PredictiveScalerConfig,
    pub monitored_resources: usize,
    pub total_predictions: u64,
    pub total_scale_ups: u64,
    pub total_scale_downs: u64,
    pub total_recommendations: usize,
    pub executed_recommendations: usize,
}
