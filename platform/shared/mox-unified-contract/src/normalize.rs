// =============================================================================
// 归一化工具函数（分数clamp / 权重归一化 / 共识度计算 / 加权平均）
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的归一化算法。
// =============================================================================

use serde::{Deserialize, Serialize};

// =============================================================================
// 分数归一化
// =============================================================================

/// 将分数 clamp 到 [0.0, 1.0] 范围
///
/// 这是所有质量分、置信度、权重的统一归一化函数。
pub fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, 1.0)
}

/// 将分数归一化到 [0.0, 1.0] 范围（基于 min-max 归一化）
///
/// 如果 max == min，返回 0.5（中间值）。
pub fn normalize_min_max(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        0.5
    } else {
        clamp_score((value - min) / (max - min))
    }
}

/// Z-score 归一化（标准化）
///
/// 返回 (value - mean) / std_dev，结果不在 [0,1] 范围内。
pub fn z_score(value: f64, mean: f64, std_dev: f64) -> f64 {
    if std_dev.abs() < f64::EPSILON {
        0.0
    } else {
        (value - mean) / std_dev
    }
}

/// Sigmoid 归一化（将任意实数映射到 (0, 1)）
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Softmax 归一化（将向量归一化为概率分布）
///
/// 输入为空时返回空向量。
pub fn softmax(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exp_values: Vec<f64> = values.iter().map(|v| (v - max_val).exp()).collect();
    let sum: f64 = exp_values.iter().sum();
    if sum.abs() < f64::EPSILON {
        // 所有值相同，均匀分布
        vec![1.0 / values.len() as f64; values.len()]
    } else {
        exp_values.iter().map(|v| v / sum).collect()
    }
}

// =============================================================================
// 权重归一化
// =============================================================================

/// 将权重向量归一化为总和为 1.0
///
/// 所有负权重会被 clamp 到 0。如果所有权重为 0，返回均匀分布。
pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    if weights.is_empty() {
        return vec![];
    }
    let clamped: Vec<f64> = weights.iter().map(|w| w.max(0.0)).collect();
    let sum: f64 = clamped.iter().sum();
    if sum.abs() < f64::EPSILON {
        vec![1.0 / weights.len() as f64; weights.len()]
    } else {
        clamped.iter().map(|w| w / sum).collect()
    }
}

// =============================================================================
// 共识度计算
// =============================================================================

/// 计算专家观点的共识度
///
/// 共识度 = (1 - 标准差归一化) * 0.70 + 平均置信度 * 0.30
///
/// 其中标准差归一化 = std_dev / 0.5（因为分数范围是 [0,1]，最大标准差约为 0.5）
///
/// # 参数
/// - `scores`: 各专家的分数（0.0-1.0）
/// - `confidences`: 各专家的置信度（0.0-1.0）
///
/// # 返回
/// 共识度（0.0-1.0），值越高表示专家观点越一致
pub fn compute_consensus(scores: &[f64], confidences: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }

    // 计算分数标准差
    let mean_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance: f64 = scores
        .iter()
        .map(|s| (s - mean_score).powi(2))
        .sum::<f64>()
        / scores.len() as f64;
    let std_dev = variance.sqrt();

    // 标准差归一化（最大可能标准差约为 0.5）
    let std_norm = (std_dev / 0.5).min(1.0);

    // 平均置信度
    let avg_conf: f64 = if confidences.is_empty() {
        0.5
    } else {
        confidences.iter().sum::<f64>() / confidences.len() as f64
    };

    // 共识度公式
    let consensus = (1.0 - std_norm) * 0.70 + avg_conf * 0.30;
    clamp_score(consensus)
}

// =============================================================================
// 加权平均
// =============================================================================

/// 计算加权平均
///
/// 权重会自动归一化。如果权重和为 0，返回简单平均。
///
/// # 参数
/// - `values`: 值列表
/// - `weights`: 权重列表（长度必须与 values 相同）
pub fn weighted_average(values: &[f64], weights: &[f64]) -> f64 {
    if values.is_empty() || weights.is_empty() || values.len() != weights.len() {
        return 0.0;
    }

    let normalized_weights = normalize_weights(weights);
    let sum: f64 = values
        .iter()
        .zip(normalized_weights.iter())
        .map(|(v, w)| v * w)
        .sum();
    sum
}

// =============================================================================
// 合成权重计算
// =============================================================================

/// 计算专家观点的合成权重
///
/// 权重 = 0.50 * 分数 + 0.30 * 置信度 + 0.20 * 优先级归一化
///
/// # 参数
/// - `score`: 专家分数（0.0-1.0）
/// - `confidence`: 专家置信度（0.0-1.0）
/// - `priority`: 专家优先级（整数，值越大优先级越高）
/// - `max_priority`: 最大优先级（用于归一化）
pub fn synthesis_weight(score: f64, confidence: f64, priority: i32, max_priority: i32) -> f64 {
    let priority_norm = if max_priority > 0 {
        priority as f64 / max_priority as f64
    } else {
        0.5
    };
    let weight = 0.50 * clamp_score(score) + 0.30 * clamp_score(confidence) + 0.20 * clamp_score(priority_norm);
    clamp_score(weight)
}

// =============================================================================
// 质量门禁评分
// =============================================================================

/// 质量门禁综合评分
///
/// 总分 = 0.55 * 质量分 + 0.25 * 覆盖度 + 0.20 * 时效分
///
/// # 参数
/// - `quality`: 质量分（0.0-1.0）
/// - `coverage`: 覆盖度（0.0-1.0）
/// - `timeliness`: 时效分（0.0-1.0）
pub fn gate_score(quality: f64, coverage: f64, timeliness: f64) -> f64 {
    let score = 0.55 * clamp_score(quality) + 0.25 * clamp_score(coverage) + 0.20 * clamp_score(timeliness);
    clamp_score(score)
}

// =============================================================================
// 归一化配置（可配置权重）
// =============================================================================

/// 归一化配置（可配置各公式的权重）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// 共识度：标准差权重
    #[serde(default = "default_consensus_std_weight")]
    pub consensus_std_weight: f64,
    /// 共识度：置信度权重
    #[serde(default = "default_consensus_conf_weight")]
    pub consensus_conf_weight: f64,
    /// 合成权重：分数权重
    #[serde(default = "default_synthesis_score_weight")]
    pub synthesis_score_weight: f64,
    /// 合成权重：置信度权重
    #[serde(default = "default_synthesis_conf_weight")]
    pub synthesis_conf_weight: f64,
    /// 合成权重：优先级权重
    #[serde(default = "default_synthesis_priority_weight")]
    pub synthesis_priority_weight: f64,
    /// 门禁评分：质量权重
    #[serde(default = "default_gate_quality_weight")]
    pub gate_quality_weight: f64,
    /// 门禁评分：覆盖度权重
    #[serde(default = "default_gate_coverage_weight")]
    pub gate_coverage_weight: f64,
    /// 门禁评分：时效权重
    #[serde(default = "default_gate_timeliness_weight")]
    pub gate_timeliness_weight: f64,
}

fn default_consensus_std_weight() -> f64 { 0.70 }
fn default_consensus_conf_weight() -> f64 { 0.30 }
fn default_synthesis_score_weight() -> f64 { 0.50 }
fn default_synthesis_conf_weight() -> f64 { 0.30 }
fn default_synthesis_priority_weight() -> f64 { 0.20 }
fn default_gate_quality_weight() -> f64 { 0.55 }
fn default_gate_coverage_weight() -> f64 { 0.25 }
fn default_gate_timeliness_weight() -> f64 { 0.20 }

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            consensus_std_weight: default_consensus_std_weight(),
            consensus_conf_weight: default_consensus_conf_weight(),
            synthesis_score_weight: default_synthesis_score_weight(),
            synthesis_conf_weight: default_synthesis_conf_weight(),
            synthesis_priority_weight: default_synthesis_priority_weight(),
            gate_quality_weight: default_gate_quality_weight(),
            gate_coverage_weight: default_gate_coverage_weight(),
            gate_timeliness_weight: default_gate_timeliness_weight(),
        }
    }
}

impl NormalizationConfig {
    /// 验证权重之和为 1.0
    pub fn validate(&self) -> Result<(), String> {
        let consensus_sum = self.consensus_std_weight + self.consensus_conf_weight;
        if (consensus_sum - 1.0).abs() > 0.01 {
            return Err(format!("共识度权重之和应为 1.0，当前为 {}", consensus_sum));
        }
        let synthesis_sum = self.synthesis_score_weight + self.synthesis_conf_weight + self.synthesis_priority_weight;
        if (synthesis_sum - 1.0).abs() > 0.01 {
            return Err(format!("合成权重之和应为 1.0，当前为 {}", synthesis_sum));
        }
        let gate_sum = self.gate_quality_weight + self.gate_coverage_weight + self.gate_timeliness_weight;
        if (gate_sum - 1.0).abs() > 0.01 {
            return Err(format!("门禁权重之和应为 1.0，当前为 {}", gate_sum));
        }
        Ok(())
    }

    /// 使用自定义配置计算共识度
    pub fn compute_consensus(&self, scores: &[f64], confidences: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        let mean_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance: f64 = scores.iter().map(|s| (s - mean_score).powi(2)).sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt();
        let std_norm = (std_dev / 0.5).min(1.0);
        let avg_conf: f64 = if confidences.is_empty() { 0.5 } else { confidences.iter().sum::<f64>() / confidences.len() as f64 };
        let consensus = (1.0 - std_norm) * self.consensus_std_weight + avg_conf * self.consensus_conf_weight;
        clamp_score(consensus)
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_score_basic() {
        assert_eq!(clamp_score(0.5), 0.5);
        assert_eq!(clamp_score(1.5), 1.0);
        assert_eq!(clamp_score(-0.5), 0.0);
        assert_eq!(clamp_score(0.0), 0.0);
        assert_eq!(clamp_score(1.0), 1.0);
    }

    #[test]
    fn normalize_min_max_basic() {
        assert_eq!(normalize_min_max(5.0, 0.0, 10.0), 0.5);
        assert_eq!(normalize_min_max(0.0, 0.0, 10.0), 0.0);
        assert_eq!(normalize_min_max(10.0, 0.0, 10.0), 1.0);
        assert_eq!(normalize_min_max(5.0, 5.0, 5.0), 0.5); // max == min
    }

    #[test]
    fn z_score_basic() {
        assert_eq!(z_score(10.0, 10.0, 2.0), 0.0);
        assert_eq!(z_score(12.0, 10.0, 2.0), 1.0);
        assert_eq!(z_score(8.0, 10.0, 2.0), -1.0);
        assert_eq!(z_score(10.0, 10.0, 0.0), 0.0); // std_dev == 0
    }

    #[test]
    fn sigmoid_basic() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(100.0) > 0.99);
        assert!(sigmoid(-100.0) < 0.01);
    }

    #[test]
    fn softmax_basic() {
        let result = softmax(&[1.0, 2.0, 3.0]);
        assert!((result.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!(result[0] < result[1]);
        assert!(result[1] < result[2]);
    }

    #[test]
    fn softmax_empty() {
        let result = softmax(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn softmax_uniform() {
        let result = softmax(&[5.0, 5.0, 5.0]);
        assert!((result[0] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn normalize_weights_basic() {
        let result = normalize_weights(&[1.0, 2.0, 3.0]);
        assert!((result.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!((result[0] - 1.0 / 6.0).abs() < 1e-10);
        assert!((result[2] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn normalize_weights_negative() {
        let result = normalize_weights(&[-1.0, 2.0, 3.0]);
        assert!((result.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert_eq!(result[0], 0.0); // 负权重被 clamp 到 0
    }

    #[test]
    fn normalize_weights_all_zero() {
        let result = normalize_weights(&[0.0, 0.0, 0.0]);
        assert!((result[0] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn compute_consensus_high_agreement() {
        // 高分、高置信、低方差 → 高共识
        let scores = [0.9, 0.85, 0.92];
        let confidences = [0.9, 0.85, 0.95];
        let consensus = compute_consensus(&scores, &confidences);
        assert!(consensus > 0.8, "高一致应 > 0.8，实际 {}", consensus);
    }

    #[test]
    fn compute_consensus_low_agreement() {
        // 分数差异大 → 低共识
        let scores = [0.2, 0.8, 0.5];
        let confidences = [0.5, 0.5, 0.5];
        let consensus = compute_consensus(&scores, &confidences);
        assert!(consensus < 0.7, "低一致应 < 0.7，实际 {}", consensus);
    }

    #[test]
    fn compute_consensus_empty() {
        assert_eq!(compute_consensus(&[], &[]), 0.0);
    }

    #[test]
    fn weighted_average_basic() {
        let values = [10.0, 20.0, 30.0];
        let weights = [1.0, 1.0, 1.0];
        let result = weighted_average(&values, &weights);
        assert!((result - 20.0).abs() < 1e-10);
    }

    #[test]
    fn weighted_average_uneven() {
        let values = [10.0, 20.0, 30.0];
        let weights = [0.0, 0.0, 1.0];
        let result = weighted_average(&values, &weights);
        assert!((result - 30.0).abs() < 1e-10);
    }

    #[test]
    fn synthesis_weight_basic() {
        let weight = synthesis_weight(0.8, 0.9, 5, 10);
        assert!(weight > 0.0 && weight <= 1.0);
    }

    #[test]
    fn gate_score_basic() {
        let score = gate_score(0.8, 0.9, 0.7);
        assert!(score > 0.0 && score <= 1.0);
        // 0.55*0.8 + 0.25*0.9 + 0.20*0.7 = 0.44 + 0.225 + 0.14 = 0.805
        assert!((score - 0.805).abs() < 1e-10);
    }

    #[test]
    fn normalization_config_default_valid() {
        let config = NormalizationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn normalization_config_custom_consensus() {
        let config = NormalizationConfig::default();
        let scores = [0.9, 0.85, 0.92];
        let confidences = [0.9, 0.85, 0.95];
        let consensus = config.compute_consensus(&scores, &confidences);
        assert!(consensus > 0.8);
    }
}
