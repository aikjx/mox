// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 迭代精炼融合 (Iterative Refinement Fusion)
//!
//! # 原理
//! 迭代精炼融合模拟人类"反复推敲、逐步完善"的思维过程。
//! 从初始融合结果出发，通过多轮迭代不断修正和优化，
//! 直到结果收敛或达到最大迭代次数。
//!
//! 核心迭代过程：
//! 1. **初始估计**：用简单融合策略得到初始结果
//! 2. **偏差校正**：计算每个专家结果与当前融合结果的偏差，
//!    识别并降低异常值的权重
//! 3. **重新加权**：根据偏差大小重新分配权重（类似迭代再加权最小二乘）
//! 4. **重新融合**：用新的权重重新计算融合结果
//! 5. **收敛检查**：如果结果变化小于阈值，则停止迭代
//!
//! 权重更新公式（Huber 风格的鲁棒加权）：
//! ```text
//! new_weight_i = old_weight_i * w(|value_i - current_result|)
//! w(d) = 1 / (1 + d/scale)  // 距离越远权重越小
//! ```
//!
//! # 适用场景
//! - **异常值较多**：数据中存在异常值，需要鲁棒融合
//! - **高精度要求**：需要逐步求精得到最优解
//! - **初值不准**：初始权重/置信度不够准确
//! - **迭代优化**：类似 EM 算法的迭代优化过程
//! - **数据质量不一**：各数据源质量差异大且未知
//!
//! # 优点
//! - 对异常值鲁棒性强（自动降低异常值权重）
//! - 能自适应调整各数据源的权重
//! - 不依赖预先设定的精确权重
//! - 理论上收敛到鲁棒估计
//! - 可解释性好：每轮都在修正偏差
//!
//! # 缺点
//! - 计算成本较高（多轮迭代）
//! - 可能收敛到局部最优
//! - 需要调整收敛阈值和最大迭代次数
//! - 初始结果影响最终收敛点
//! - 对收敛条件敏感

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ScalarFusionStrategy;

/// 迭代精炼融合器
///
/// 通过多轮迭代不断优化融合结果，自动调整各数据源的权重，
/// 降低异常值的影响，最终收敛到鲁棒的融合结果。
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::IterativeRefinementFusion;
/// use mox_alliance_core::fusion::traits::ScalarFusionStrategy;
///
/// let fusion = IterativeRefinementFusion::new()
///     .with_max_iterations(20)
///     .with_tolerance(1e-6);
///
/// // 大部分值在 10 附近，有一个 100 的异常值
/// let values = vec![
///     (10.0, 0.8),
///     (10.5, 0.7),
///     (9.8, 0.9),
///     (10.2, 0.6),
///     (100.0, 0.5),  // 异常值
/// ];
///
/// let result = fusion.fuse_scalar(&values).unwrap();
/// // 结果应该更接近 10 而不是被拉向 100
/// assert!(result < 20.0);
/// ```
#[derive(Debug, Clone)]
pub struct IterativeRefinementFusion {
    /// 最大迭代次数
    max_iterations: usize,
    /// 收敛阈值（结果变化小于此值时停止）
    tolerance: f64,
    /// 尺度参数（控制权重衰减速度，越大衰减越慢）
    scale: f64,
    /// 权重下限（防止权重变为0）
    min_weight: f64,
}

impl Default for IterativeRefinementFusion {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            tolerance: 1e-6,
            scale: 1.0,
            min_weight: 1e-6,
        }
    }
}

impl IterativeRefinementFusion {
    /// 创建一个新的迭代精炼融合器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大迭代次数（默认 50）
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// 设置收敛阈值（默认 1e-6）
    ///
    /// 当相邻两次迭代结果的绝对差小于此阈值时，迭代提前终止。
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// 设置尺度参数（默认 1.0）
    ///
    /// 尺度参数控制权重随偏差衰减的速度：
    /// - 较大值：权重衰减慢，对异常值容忍度高
    /// - 较小值：权重衰减快，对异常值更敏感
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// 设置权重下限（默认 1e-6）
    ///
    /// 防止任何数据点的权重被降为零，确保所有数据都有最低限度的贡献。
    pub fn with_min_weight(mut self, min_weight: f64) -> Self {
        self.min_weight = min_weight;
        self
    }

    /// 执行迭代精炼融合，返回详细信息
    ///
    /// # Returns
    /// * `result` - 最终融合结果
    /// * `iterations` - 实际迭代次数
    /// * `final_delta` - 最后一次迭代的变化量
    /// * `final_weights` - 最终的权重分布
    pub fn refine(
        &self,
        values: &[(f64, f64)],
    ) -> FusionResult<(f64, usize, f64, Vec<f64>)> {
        // 验证输入
        if values.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        if self.max_iterations == 0 {
            return Err(FusionError::invalid_param(
                "max_iterations",
                "must be at least 1",
            ));
        }

        if self.tolerance <= 0.0 {
            return Err(FusionError::invalid_param(
                "tolerance",
                "must be positive",
            ));
        }

        if self.scale <= 0.0 {
            return Err(FusionError::invalid_param(
                "scale",
                "must be positive",
            ));
        }

        // 验证值和权重
        for (i, (val, weight)) in values.iter().enumerate() {
            if !val.is_finite() {
                return Err(FusionError::InvalidParameter {
                    param: "value",
                    reason: format!("non-finite value at index {}: {}", i, val),
                });
            }
            if !weight.is_finite() {
                return Err(FusionError::InvalidWeight {
                    index: i,
                    value: format!("{}", weight),
                });
            }
            if *weight < 0.0 {
                return Err(FusionError::InvalidWeight {
                    index: i,
                    value: format!("{} (negative)", weight),
                });
            }
        }

        // 提取值和初始权重
        let n = values.len();
        let xs: Vec<f64> = values.iter().map(|(v, _)| *v).collect();
        let mut weights: Vec<f64> = values.iter().map(|(_, w)| *w).collect();

        // 初始融合：加权平均
        let mut current = weighted_mean(&xs, &weights)?;
        let mut final_delta = 0.0;
        let mut actual_iterations = 0;

        for iter in 0..self.max_iterations {
            actual_iterations = iter + 1;

            // 计算每个点的偏差
            let deviations: Vec<f64> = xs.iter().map(|x| (x - current).abs()).collect();

            // 更新权重：偏差越大权重越小
            // 使用 Huber 风格的加权函数：w(d) = 1 / (1 + d/scale)
            for i in 0..n {
                let d = deviations[i];
                let weight_factor = 1.0 / (1.0 + d / self.scale);
                let base_weight = values[i].1;
                weights[i] = (base_weight * weight_factor).max(self.min_weight);
            }

            // 重新计算加权平均
            let next = weighted_mean(&xs, &weights)?;

            // 检查收敛
            let delta = (next - current).abs();
            final_delta = delta;
            current = next;

            if delta < self.tolerance {
                break;
            }
        }

        Ok((current, actual_iterations, final_delta, weights))
    }

    /// 获取最大迭代次数
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// 获取收敛阈值
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
}

/// 计算加权平均值
fn weighted_mean(values: &[f64], weights: &[f64]) -> FusionResult<f64> {
    let total_weight: f64 = weights.iter().sum();
    if total_weight == 0.0 {
        return Err(FusionError::ZeroTotalWeight);
    }
    let sum: f64 = values.iter().zip(weights.iter()).map(|(v, w)| v * w).sum();
    Ok(sum / total_weight)
}

impl ScalarFusionStrategy for IterativeRefinementFusion {
    fn name(&self) -> &'static str {
        "Iterative Refinement Fusion"
    }

    fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64> {
        self.refine(values).map(|(r, _, _, _)| r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ScalarFusionStrategy;

    #[test]
    fn test_basic_iterative_refinement() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(20)
            .with_tolerance(1e-8);

        let values = vec![
            (10.0, 0.8),
            (10.5, 0.7),
            (9.8, 0.9),
            (10.2, 0.6),
            (100.0, 0.5), // 异常值
        ];

        let result = fusion.fuse_scalar(&values).unwrap();
        // 迭代精炼应该降低异常值的权重，结果更接近真实值 10
        assert!(result < 20.0);
        assert!(result > 9.0 && result < 11.0);
    }

    #[test]
    fn test_convergence() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(100)
            .with_tolerance(1e-10);

        let values = vec![(10.0, 1.0), (10.1, 1.0), (9.9, 1.0)];
        let (_, iterations, delta, _) = fusion.refine(&values).unwrap();

        // 数据接近时应该很快收敛
        assert!(iterations < 100);
        assert!(delta < 1e-10);
    }

    #[test]
    fn test_empty_input() {
        let fusion = IterativeRefinementFusion::new();
        let values: Vec<(f64, f64)> = vec![];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_zero_max_iterations() {
        let fusion = IterativeRefinementFusion::new().with_max_iterations(0);
        let values = vec![(10.0, 1.0)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_negative_tolerance() {
        let fusion = IterativeRefinementFusion::new().with_tolerance(-0.1);
        let values = vec![(10.0, 1.0)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_single_value() {
        let fusion = IterativeRefinementFusion::new();
        let values = vec![(42.0, 1.0)];
        let result = fusion.fuse_scalar(&values).unwrap();
        assert!((result - 42.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_same_value() {
        let fusion = IterativeRefinementFusion::new().with_max_iterations(10);
        let values = vec![(5.0, 1.0), (5.0, 2.0), (5.0, 0.5)];
        let (result, iterations, _, _) = fusion.refine(&values).unwrap();
        assert!((result - 5.0).abs() < 1e-9);
        // 所有值相同，应该一次迭代就收敛
        assert_eq!(iterations, 1);
    }

    #[test]
    fn test_outlier_downweighted() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(50)
            .with_scale(0.5); // 较小 scale = 更快衰减

        let values = vec![
            (10.0, 1.0),
            (10.5, 1.0),
            (9.5, 1.0),
            (1000.0, 1.0), // 极端异常值
        ];

        let (result, _, _, final_weights) = fusion.refine(&values).unwrap();

        // 异常值的最终权重应该远低于正常值
        let normal_weight = final_weights[0];
        let outlier_weight = final_weights[3];
        assert!(outlier_weight < normal_weight * 0.1);

        // 结果应该接近正常值，而不是异常值
        assert!(result < 20.0);
    }

    #[test]
    fn test_scale_parameter_effect() {
        let values = vec![
            (10.0, 1.0),
            (10.0, 1.0),
            (20.0, 1.0), // 偏差 10
        ];

        let small_scale = IterativeRefinementFusion::new()
            .with_max_iterations(20)
            .with_scale(0.1); // 小 scale，对偏差敏感

        let large_scale = IterativeRefinementFusion::new()
            .with_max_iterations(20)
            .with_scale(10.0); // 大 scale，对偏差不敏感

        let r_small = small_scale.fuse_scalar(&values).unwrap();
        let r_large = large_scale.fuse_scalar(&values).unwrap();

        // 小 scale 时，异常值权重下降更多，结果更偏向正常值
        assert!(r_small < r_large);
    }

    #[test]
    fn test_min_weight() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(100)
            .with_min_weight(0.01)
            .with_scale(0.01); // 极小 scale，让权重快速下降

        let values = vec![
            (10.0, 1.0),
            (100.0, 1.0), // 极端异常值
        ];

        let (_, _, _, final_weights) = fusion.refine(&values).unwrap();

        // 所有权重都不低于 min_weight
        for w in &final_weights {
            assert!(*w >= 0.01 - 1e-9);
        }
    }

    #[test]
    fn test_invalid_weight() {
        let fusion = IterativeRefinementFusion::new();
        let values = vec![(10.0, -1.0)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::InvalidWeight { .. })));
    }

    #[test]
    fn test_nan_value() {
        let fusion = IterativeRefinementFusion::new();
        let values = vec![(f64::NAN, 1.0)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_name() {
        let fusion = IterativeRefinementFusion::new();
        assert_eq!(fusion.name(), "Iterative Refinement Fusion");
    }

    #[test]
    fn test_accessors() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(42)
            .with_tolerance(1e-5);
        assert_eq!(fusion.max_iterations(), 42);
        assert!((fusion.tolerance() - 1e-5).abs() < 1e-12);
    }

    #[test]
    fn test_iteration_count() {
        let fusion = IterativeRefinementFusion::new()
            .with_max_iterations(5)
            .with_tolerance(1e-20) // 极小阈值，确保达到最大迭代次数
            .with_scale(0.01); // 小 scale 让权重变化更剧烈

        let values = vec![
            (10.0, 2.0),
            (10.1, 2.0),
            (9.9, 2.0),
            (100.0, 1.0), // 单个异常值
        ];

        let (_, iterations, _, _) = fusion.refine(&values).unwrap();
        // 应该执行多次迭代（直到达到最大迭代次数）
        assert!(iterations >= 2);
        assert!(iterations <= 5);
    }
}
