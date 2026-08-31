// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 堆叠融合 (Stacking / Meta-Learner Fusion)
//!
//! # 原理
//! 堆叠融合（Stacked Generalization，简称 Stacking）是一种层级式集成学习方法。
//! 它包含两层（或多层）模型：
//!
//! - **第一层（Base Learners）**：多个基础模型/专家各自独立产生预测结果
//! - **第二层（Meta-Learner）**：将第一层的输出作为输入特征，学习如何最优地组合
//!   这些基础预测，产生最终结果
//!
//! 本实现提供了简化版的堆叠融合，支持以下元学习器：
//! - **线性加权**：学习最优权重组合（闭式解，最小二乘）
//! - **平均值**：所有基础结果等权平均（作为 baseline）
//! - **加权中位数**：鲁棒性更强的融合策略
//!
//! # 适用场景
//! - **高精度要求任务**：当简单加权不够好，需要学习最优组合时
//! - **多模型异质性强**：各模型在不同数据子集上表现差异大
//! - **有标注数据可用**：需要训练数据来训练元学习器
//! - **模型相关性低**：基础模型之间相关性越低，堆叠增益越大
//! - **竞赛场景**：Kaggle 等数据科学竞赛的常用策略
//!
//! # 优点
//! - 能自动学习最优组合权重，无需人工调参
//! - 可以捕捉模型之间的复杂关系
//! - 通常比单一模型或简单加权效果更好
//! - 理论基础扎实，有统计学习理论支撑
//!
//! # 缺点
//! - 需要训练数据来训练元学习器
//! - 实现复杂度较高
//! - 容易过拟合（需要交叉验证等技术）
//! - 计算成本高于简单融合策略
//! - 可解释性较差（黑盒特性）

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ScalarFusionStrategy;

/// 元学习器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaLearnerType {
    /// 简单平均（作为 baseline）
    Mean,
    /// 线性加权（最小二乘求解最优权重）
    LinearWeighted,
    /// 加权中位数（鲁棒性强）
    WeightedMedian,
}

impl Default for MetaLearnerType {
    fn default() -> Self {
        Self::Mean
    }
}

/// 堆叠融合器
///
/// 使用元学习器将多个基础模型的输出融合为最终结果。
///
/// # 训练与预测分离
///
/// 堆叠融合分为两个阶段：
/// 1. **训练阶段**：使用训练数据（基础预测 + 真实值）训练元学习器
/// 2. **预测阶段**：使用训练好的元学习器对新的基础预测进行融合
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::StackingFusion;
/// use mox_alliance_core::fusion::strategies::stacking::MetaLearnerType;
///
/// // 创建融合器并训练（3个样本，2个模型）
/// let base_predictions = vec![
///     vec![1.0, 2.0],  // 样本1：模型A=1.0, 模型B=2.0
///     vec![3.0, 5.0],  // 样本2：模型A=3.0, 模型B=5.0
///     vec![5.0, 6.0],  // 样本3：模型A=5.0, 模型B=6.0
/// ];
/// let true_values = vec![1.8, 4.2, 5.5];  // 真实值
///
/// let fusion = StackingFusion::train(
///     &base_predictions,
///     &true_values,
///     MetaLearnerType::LinearWeighted,
/// ).unwrap();
///
/// // 预测
/// let new_prediction = vec![2.0, 3.0];
/// let _result = fusion.predict(&new_prediction).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct StackingFusion {
    /// 元学习器类型
    meta_learner: MetaLearnerType,
    /// 学习到的权重（LinearWeighted 模式使用）
    weights: Vec<f64>,
    /// 偏置项（LinearWeighted 模式使用）
    bias: f64,
    /// 基础模型数量
    n_models: usize,
}

impl StackingFusion {
    /// 训练堆叠融合器
    ///
    /// # Arguments
    /// * `base_predictions` - 基础模型预测矩阵，每行一个样本，每列一个模型
    /// * `true_values` - 真实值列表，与样本一一对应
    /// * `meta_learner` - 元学习器类型
    ///
    /// # Returns
    /// 训练好的 `StackingFusion` 实例
    ///
    /// # Errors
    /// - `FusionError::EmptyInput` — 训练数据为空
    /// - `FusionError::DimensionMismatch` — 预测矩阵与真实值维度不匹配
    pub fn train(
        base_predictions: &[Vec<f64>],
        true_values: &[f64],
        meta_learner: MetaLearnerType,
    ) -> FusionResult<Self> {
        if base_predictions.is_empty() {
            return Err(FusionError::EmptyInput);
        }
        if base_predictions.len() != true_values.len() {
            return Err(FusionError::DimensionMismatch {
                expected: true_values.len(),
                actual: base_predictions.len(),
            });
        }

        let n_models = base_predictions[0].len();
        if n_models == 0 {
            return Err(FusionError::EmptyInput);
        }

        // 验证所有样本维度一致
        for (i, row) in base_predictions.iter().enumerate() {
            if row.len() != n_models {
                return Err(FusionError::DimensionMismatch {
                    expected: n_models,
                    actual: row.len(),
                });
            }
            // 验证值有限
            for (j, &val) in row.iter().enumerate() {
                if !val.is_finite() {
                    return Err(FusionError::InvalidParameter {
                        param: "base_predictions",
                        reason: format!(
                            "non-finite value at sample {}, model {}: {}",
                            i, j, val
                        ),
                    });
                }
            }
        }

        // 验证真实值
        for (i, &val) in true_values.iter().enumerate() {
            if !val.is_finite() {
                return Err(FusionError::InvalidParameter {
                    param: "true_values",
                    reason: format!("non-finite value at index {}: {}", i, val),
                });
            }
        }

        let (weights, bias) = match meta_learner {
            MetaLearnerType::Mean => {
                // 平均：所有权重 = 1/n，偏置 = 0
                let w = 1.0 / n_models as f64;
                (vec![w; n_models], 0.0)
            }
            MetaLearnerType::LinearWeighted => {
                // 线性加权：用最小二乘求解
                solve_linear_weights(base_predictions, true_values)?
            }
            MetaLearnerType::WeightedMedian => {
                // 加权中位数：使用权重 = 各模型的逆 MAE
                let weights = compute_inverse_mae_weights(base_predictions, true_values);
                (weights, 0.0)
            }
        };

        Ok(Self {
            meta_learner,
            weights,
            bias,
            n_models,
        })
    }

    /// 使用训练好的元学习器进行预测
    ///
    /// # Arguments
    /// * `base_outputs` - 基础模型的输出向量，每个元素对应一个模型
    ///
    /// # Returns
    /// 融合后的预测值
    ///
    /// # Errors
    /// - `FusionError::DimensionMismatch` — 输入维度与训练时不一致
    pub fn predict(&self, base_outputs: &[f64]) -> FusionResult<f64> {
        if base_outputs.len() != self.n_models {
            return Err(FusionError::DimensionMismatch {
                expected: self.n_models,
                actual: base_outputs.len(),
            });
        }

        // 验证输入值
        for (i, &val) in base_outputs.iter().enumerate() {
            if !val.is_finite() {
                return Err(FusionError::InvalidParameter {
                    param: "base_outputs",
                    reason: format!("non-finite value at index {}: {}", i, val),
                });
            }
        }

        match self.meta_learner {
            MetaLearnerType::Mean | MetaLearnerType::LinearWeighted => {
                // 线性组合
                let result: f64 = self
                    .weights
                    .iter()
                    .zip(base_outputs.iter())
                    .map(|(w, x)| w * x)
                    .sum::<f64>()
                    + self.bias;
                Ok(result)
            }
            MetaLearnerType::WeightedMedian => {
                // 加权中位数
                Ok(weighted_median(base_outputs, &self.weights))
            }
        }
    }

    /// 获取学习到的权重
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// 获取偏置项
    pub fn bias(&self) -> f64 {
        self.bias
    }

    /// 获取基础模型数量
    pub fn n_models(&self) -> usize {
        self.n_models
    }

    /// 获取元学习器类型
    pub fn meta_learner_type(&self) -> MetaLearnerType {
        self.meta_learner
    }
}

impl ScalarFusionStrategy for StackingFusion {
    fn name(&self) -> &'static str {
        "Stacking / Meta-Learner Fusion"
    }

    fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64> {
        // 对于 ScalarFusionStrategy trait，我们将第一个元素作为值，
        // 按顺序排列各模型的输出（忽略第二个元素，因为权重已由训练决定）
        let outputs: Vec<f64> = values.iter().map(|(v, _)| *v).collect();
        self.predict(&outputs)
    }
}

/// 使用最小二乘求解线性权重
///
/// 求解 w, b 使得 ||Xw + b - y||² 最小。
/// 使用高斯消元法求解正规方程。
fn solve_linear_weights(
    x: &[Vec<f64>],
    y: &[f64],
) -> FusionResult<(Vec<f64>, f64)> {
    let n_samples = x.len();
    let n_features = x[0].len();
    let n_vars = n_features + 1; // +1 for bias

    // 构建增广矩阵 (X^T X | X^T y)
    let mut a = vec![vec![0.0; n_vars + 1]; n_vars];

    for i in 0..n_samples {
        // xi 包含偏置项（最后一个元素=1）
        let mut xi = vec![0.0; n_vars];
        for j in 0..n_features {
            xi[j] = x[i][j];
        }
        xi[n_features] = 1.0; // 偏置项

        // 累加 X^T X
        for row in 0..n_vars {
            for col in 0..n_vars {
                a[row][col] += xi[row] * xi[col];
            }
            // 累加 X^T y
            a[row][n_vars] += xi[row] * y[i];
        }
    }

    // 高斯消元
    for col in 0..n_vars {
        // 选主元
        let mut max_row = col;
        let mut max_val = a[col][col].abs();
        for row in col + 1..n_vars {
            if a[row][col].abs() > max_val {
                max_val = a[row][col].abs();
                max_row = row;
            }
        }

        if max_val < 1e-12 {
            return Err(FusionError::invalid_param(
                "base_predictions",
                "singular matrix: base predictions are linearly dependent",
            ));
        }

        // 交换行
        if max_row != col {
            a.swap(col, max_row);
        }

        // 消元
        for row in 0..n_vars {
            if row != col && a[row][col].abs() > 1e-15 {
                let factor = a[row][col] / a[col][col];
                for k in col..=n_vars {
                    a[row][k] -= factor * a[col][k];
                }
            }
        }
    }

    // 回代求解
    let mut solution = vec![0.0; n_vars];
    for i in 0..n_vars {
        solution[i] = a[i][n_vars] / a[i][i];
    }

    let weights = solution[..n_features].to_vec();
    let bias = solution[n_features];

    Ok((weights, bias))
}

/// 计算基于逆 MAE 的权重
fn compute_inverse_mae_weights(x: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let n_models = x[0].len();
    let mut maes = vec![0.0; n_models];

    for i in 0..x.len() {
        for j in 0..n_models {
            maes[j] += (x[i][j] - y[i]).abs();
        }
    }

    // 转换为权重（MAE 越小权重越大）
    let mut weights: Vec<f64> = maes
        .iter()
        .map(|&mae| {
            if mae < 1e-12 {
                1e12 // 避免除零
            } else {
                1.0 / mae
            }
        })
        .collect();

    // 归一化
    let sum: f64 = weights.iter().sum();
    if sum > 0.0 {
        for w in &mut weights {
            *w /= sum;
        }
    }

    weights
}

/// 加权中位数
fn weighted_median(values: &[f64], weights: &[f64]) -> f64 {
    let mut pairs: Vec<(f64, f64)> = values.iter().zip(weights.iter()).map(|(v, w)| (*v, *w)).collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let total_weight: f64 = weights.iter().sum();
    let half = total_weight / 2.0;

    let mut cumulative = 0.0;
    for (val, w) in &pairs {
        cumulative += w;
        if cumulative >= half {
            return *val;
        }
    }

    // 理论上不会到达这里
    pairs.last().map(|(v, _)| *v).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ScalarFusionStrategy;

    #[test]
    fn test_mean_stacking() {
        let base_preds = vec![vec![1.0, 3.0], vec![2.0, 4.0]];
        let true_vals = vec![2.0, 3.0];

        let fusion = StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::Mean).unwrap();
        assert_eq!(fusion.n_models(), 2);
        assert_eq!(fusion.weights().len(), 2);
        assert!((fusion.weights()[0] - 0.5).abs() < 1e-9);
        assert!((fusion.weights()[1] - 0.5).abs() < 1e-9);
        assert!((fusion.bias() - 0.0).abs() < 1e-9);

        let result = fusion.predict(&[3.0, 5.0]).unwrap();
        assert!((result - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_linear_weighted_stacking() {
        // 构造完美线性关系的数据：y = 0.3*x1 + 0.7*x2
        let base_preds = vec![
            vec![10.0, 20.0],
            vec![20.0, 10.0],
            vec![30.0, 30.0],
            vec![40.0, 10.0],
        ];
        let true_vals: Vec<f64> = base_preds
            .iter()
            .map(|x| 0.3 * x[0] + 0.7 * x[1])
            .collect();

        let fusion =
            StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::LinearWeighted).unwrap();

        // 学习到的权重应该接近真实权重
        let w = fusion.weights();
        assert!((w[0] - 0.3).abs() < 0.01);
        assert!((w[1] - 0.7).abs() < 0.01);

        // 预测应该准确
        let result = fusion.predict(&[100.0, 200.0]).unwrap();
        let expected = 0.3 * 100.0 + 0.7 * 200.0;
        assert!((result - expected).abs() < 0.01);
    }

    #[test]
    fn test_weighted_median_stacking() {
        let base_preds = vec![
            vec![1.0, 100.0],  // 模型1准确，模型2离谱
            vec![2.0, 200.0],
            vec![3.0, 300.0],
        ];
        let true_vals = vec![1.0, 2.0, 3.0];

        let fusion =
            StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::WeightedMedian).unwrap();

        // 模型1的权重应该远高于模型2（因为模型2 MAE大）
        let w = fusion.weights();
        assert!(w[0] > w[1]);

        // 加权中位数应该更接近模型1的预测
        let result = fusion.predict(&[5.0, 100.0]).unwrap();
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_empty_input() {
        let result = StackingFusion::train(&[], &[], MetaLearnerType::Mean);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_dimension_mismatch() {
        let base_preds = vec![vec![1.0, 2.0]];
        let true_vals = vec![1.5, 2.5];
        let result = StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::Mean);
        assert!(matches!(result, Err(FusionError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_predict_dimension_mismatch() {
        let base_preds = vec![vec![1.0, 2.0]];
        let true_vals = vec![1.5];
        let fusion = StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::Mean).unwrap();
        let result = fusion.predict(&[1.0]);
        assert!(matches!(result, Err(FusionError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_nan_in_training_data() {
        let base_preds = vec![vec![f64::NAN, 2.0]];
        let true_vals = vec![1.5];
        let result = StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::Mean);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_scalar_fusion_strategy() {
        // 使用足够多且独立的样本以避免矩阵奇异
        let base_preds = vec![
            vec![1.0, 4.0],
            vec![2.0, 5.0],
            vec![3.0, 8.0],
            vec![4.0, 6.0],
            vec![5.0, 7.0],
        ];
        let true_vals: Vec<f64> = base_preds
            .iter()
            .map(|x| 0.7 * x[0] + 0.3 * x[1] + 0.5)
            .collect();

        let fusion =
            StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::LinearWeighted).unwrap();

        let values = vec![(10.0, 0.0), (20.0, 0.0)];
        let result = fusion.fuse_scalar(&values).unwrap();
        // 0.7 * 10 + 0.3 * 20 + 0.5 = 7 + 6 + 0.5 = 13.5
        assert!((result - 13.5).abs() < 0.5);
    }

    #[test]
    fn test_name() {
        let base_preds = vec![vec![1.0, 2.0]];
        let true_vals = vec![1.5];
        let fusion = StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::Mean).unwrap();
        assert_eq!(fusion.name(), "Stacking / Meta-Learner Fusion");
    }

    #[test]
    fn test_meta_learner_type_default() {
        assert_eq!(MetaLearnerType::default(), MetaLearnerType::Mean);
    }

    #[test]
    fn test_single_model() {
        let base_preds = vec![vec![1.0], vec![2.0], vec![3.0]];
        let true_vals = vec![1.0, 2.0, 3.0];
        let fusion =
            StackingFusion::train(&base_preds, &true_vals, MetaLearnerType::LinearWeighted).unwrap();
        assert_eq!(fusion.n_models(), 1);
        let result = fusion.predict(&[5.0]).unwrap();
        // 单模型完美拟合，权重接近1，偏置接近0
        assert!((result - 5.0).abs() < 0.01);
    }
}
