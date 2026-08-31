// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 置信度加权融合 (Confidence Weighting Fusion)
//!
//! # 原理
//! 置信度加权融合是加权平均的一种特殊形式，每个预测结果都附带一个置信度分数，
//! 表示该结果的可信程度。最终结果是所有预测值按置信度加权的平均值。
//!
//! 公式：
//! ```text
//! result = Σ (value_i * confidence_i) / Σ confidence_i
//! ```
//!
//! 与普通加权融合的区别：
//! - **加权融合**的权重是预先设定的（如专家职级），是静态的
//! - **置信度加权**的权重是每个结果自带的，是动态的，随具体任务变化
//!
//! # 适用场景
//! - **数值预测**：多模型回归预测结果融合
//! - **概率估计**：多概率分布的融合（如贝叶斯模型平均）
//! - **带置信区间的结果**：每个结果附带不确定性估计
//! - **自适应加权**：不同输入下各模型表现不同，置信度随之变化
//! - **排序融合**：结合相关性置信度的多路召回融合
//!
//! # 优点
//! - 直观且计算高效（O(n) 复杂度）
//! - 置信度提供了结果质量的量化指标
//! - 能自适应不同输入下各模型表现的差异
//! - 结果可解释：每个贡献者的影响与置信度成正比
//!
//! # 缺点
//! - 依赖置信度的准确性，置信度不可靠时结果会偏差
//! - 无法捕捉模型之间的相关性
//! - 对极端置信度敏感（单个极高置信度可能主导结果）
//! - 不适用于非数值型结果（需要额外编码）

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ScalarFusionStrategy;

/// 置信度加权融合器
///
/// 将多个带置信度的数值结果融合为一个最终结果。
/// 最终值 = sum(value * confidence) / sum(confidence)
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::ConfidenceWeightingFusion;
/// use mox_alliance_core::fusion::traits::ScalarFusionStrategy;
///
/// let fusion = ConfidenceWeightingFusion::new();
/// let values = vec![
///     (80.0, 0.9),   // 值=80，置信度=0.9
///     (90.0, 0.7),   // 值=90，置信度=0.7
///     (70.0, 0.5),   // 值=70，置信度=0.5
/// ];
/// let result = fusion.fuse_scalar(&values).unwrap();
/// // (80*0.9 + 90*0.7 + 70*0.5) / (0.9 + 0.7 + 0.5)
/// // = (72 + 63 + 35) / 2.1 = 170 / 2.1 ≈ 80.95
/// assert!((result - 80.952).abs() < 0.01);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConfidenceWeightingFusion {
    /// 置信度下限（低于此值的结果会被过滤）
    min_confidence: Option<f64>,
    /// 是否使用 softmax 归一化置信度（默认 false，使用线性归一化）
    use_softmax: bool,
    /// softmax 温度参数（仅当 use_softmax=true 时有效，默认 1.0）
    temperature: f64,
}

impl ConfidenceWeightingFusion {
    /// 创建一个新的置信度加权融合器
    pub fn new() -> Self {
        Self {
            min_confidence: None,
            use_softmax: false,
            temperature: 1.0,
        }
    }

    /// 设置最低置信度阈值
    ///
    /// 置信度低于此阈值的结果将被忽略。
    pub fn with_min_confidence(mut self, min_confidence: f64) -> Self {
        self.min_confidence = Some(min_confidence);
        self
    }

    /// 使用 softmax 归一化置信度
    ///
    /// 当 use_softmax = true 时，置信度将通过 softmax 函数归一化，
    /// 使得高置信度与低置信度之间的差距被放大。
    ///
    /// temperature 参数控制平滑度：
    /// - 温度 > 1：分布更平滑（差距缩小）
    /// - 温度 < 1：分布更尖锐（差距放大）
    pub fn with_softmax(mut self, temperature: f64) -> Self {
        self.use_softmax = true;
        self.temperature = temperature;
        self
    }

    /// 执行标量融合（带额外信息返回）
    ///
    /// 返回 (融合结果, 有效样本数, 总置信度)
    pub fn fuse_scalar_with_stats(
        &self,
        values: &[(f64, f64)],
    ) -> FusionResult<(f64, usize, f64)> {
        if values.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        // 验证并过滤数据
        let mut filtered: Vec<(f64, f64)> = Vec::with_capacity(values.len());
        for (i, (val, conf)) in values.iter().enumerate() {
            // 验证值的有效性
            if !val.is_finite() {
                return Err(FusionError::InvalidParameter {
                    param: "value",
                    reason: format!("value at index {} is not finite: {}", i, val),
                });
            }
            // 验证置信度的有效性
            if !conf.is_finite() {
                return Err(FusionError::InvalidConfidence {
                    index: i,
                    value: *conf,
                });
            }
            if *conf < 0.0 || *conf > 1.0 {
                return Err(FusionError::InvalidConfidence {
                    index: i,
                    value: *conf,
                });
            }
            // 应用最低置信度过滤
            if let Some(min_conf) = self.min_confidence {
                if *conf < min_conf {
                    continue;
                }
            }
            filtered.push((*val, *conf));
        }

        if filtered.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        // 计算权重
        let weights: Vec<f64> = if self.use_softmax {
            softmax(
                &filtered.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
                self.temperature,
            )
        } else {
            let total_conf: f64 = filtered.iter().map(|(_, c)| c).sum();
            if total_conf == 0.0 {
                return Err(FusionError::ZeroTotalWeight);
            }
            filtered.iter().map(|(_, c)| c / total_conf).collect()
        };

        // 计算加权平均
        let result: f64 = filtered
            .iter()
            .zip(weights.iter())
            .map(|((val, _), w)| val * w)
            .sum();

        let total_confidence: f64 = filtered.iter().map(|(_, c)| c).sum();

        Ok((result, filtered.len(), total_confidence))
    }
}

impl ScalarFusionStrategy for ConfidenceWeightingFusion {
    fn name(&self) -> &'static str {
        "Confidence Weighting Fusion"
    }

    fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64> {
        self.fuse_scalar_with_stats(values).map(|(r, _, _)| r)
    }
}

/// 计算 softmax
fn softmax(xs: &[f64], temperature: f64) -> Vec<f64> {
    if xs.is_empty() {
        return vec![];
    }
    let t = if temperature == 0.0 { 1.0 } else { temperature };

    // 减去最大值防止溢出
    let max_val = xs
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let exps: Vec<f64> = xs.iter().map(|x| ((x - max_val) / t).exp()).collect();
    let sum: f64 = exps.iter().sum();

    if sum == 0.0 {
        // 极端情况，均匀分布
        vec![1.0 / xs.len() as f64; xs.len()]
    } else {
        exps.iter().map(|e| e / sum).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ScalarFusionStrategy;

    #[test]
    fn test_basic_confidence_weighting() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, 0.9), (90.0, 0.7), (70.0, 0.5)];
        let result = fusion.fuse_scalar(&values).unwrap();
        // (80*0.9 + 90*0.7 + 70*0.5) / (0.9 + 0.7 + 0.5)
        // = (72 + 63 + 35) / 2.1 = 170 / 2.1 ≈ 80.952
        assert!((result - 80.952).abs() < 0.01);
    }

    #[test]
    fn test_equal_confidence() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, 0.5), (90.0, 0.5), (70.0, 0.5)];
        let result = fusion.fuse_scalar(&values).unwrap();
        // 等置信度时等同于算术平均
        assert!((result - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_single_value() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(42.0, 0.8)];
        let result = fusion.fuse_scalar(&values).unwrap();
        assert!((result - 42.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_input() {
        let fusion = ConfidenceWeightingFusion::new();
        let values: Vec<(f64, f64)> = vec![];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_invalid_confidence_above_one() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, 1.5)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(
            result,
            Err(FusionError::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn test_invalid_confidence_negative() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, -0.1)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(
            result,
            Err(FusionError::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn test_nan_value() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(f64::NAN, 0.8)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(
            result,
            Err(FusionError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_min_confidence_filter() {
        let fusion = ConfidenceWeightingFusion::new().with_min_confidence(0.6);
        let values = vec![
            (100.0, 0.5), // 被过滤
            (80.0, 0.9),  // 保留
            (90.0, 0.7),  // 保留
        ];
        let (result, count, total_conf) = fusion.fuse_scalar_with_stats(&values).unwrap();
        assert_eq!(count, 2);
        // (80*0.9 + 90*0.7) / (0.9 + 0.7) = (72 + 63) / 1.6 = 135 / 1.6 = 84.375
        assert!((result - 84.375).abs() < 0.001);
        assert!((total_conf - 1.6).abs() < 0.001);
    }

    #[test]
    fn test_all_filtered_out() {
        let fusion = ConfidenceWeightingFusion::new().with_min_confidence(0.9);
        let values = vec![(80.0, 0.5), (90.0, 0.7)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_softmax_normalization() {
        let fusion = ConfidenceWeightingFusion::new().with_softmax(0.1);
        let values = vec![(10.0, 0.9), (20.0, 0.1)];
        let result = fusion.fuse_scalar(&values).unwrap();
        // 低温度下 softmax 会让高置信度的权重大大增加
        // 结果应该更接近 10（高置信度的值）
        assert!(result < 15.0);
    }

    #[test]
    fn test_zero_confidence_total() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, 0.0), (90.0, 0.0)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::ZeroTotalWeight)));
    }

    #[test]
    fn test_with_stats() {
        let fusion = ConfidenceWeightingFusion::new();
        let values = vec![(80.0, 0.8), (90.0, 0.6)];
        let (result, count, total_conf) = fusion.fuse_scalar_with_stats(&values).unwrap();
        assert_eq!(count, 2);
        assert!((total_conf - 1.4).abs() < 0.001);
        assert!(result > 0.0);
    }

    #[test]
    fn test_name() {
        let fusion = ConfidenceWeightingFusion::new();
        assert_eq!(fusion.name(), "Confidence Weighting Fusion");
    }
}
