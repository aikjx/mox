// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 加权投票融合 (Weighted Voting Fusion)
//!
//! # 原理
//! 加权投票是多数投票的推广形式。每个投票者（专家/模型）拥有不同的权重，
//! 最终结果不是简单统计票数，而是统计各选项获得的权重总和。
//!
//! 对于分类问题，公式为：
//! ```text
//! score(c) = Σ w_i * I(v_i = c)
//! ```
//! 其中 w_i 是第 i 个投票者的权重，I(·) 是指示函数。
//!
//! # 适用场景
//! - **分类/决策任务**：多专家对离散选项进行投票
//! - **专家能力差异**：不同专家的专业水平不同，需要赋予不同权重
//! - **集成学习**：多模型分类结果的集成
//! - **委员会决策**：模拟委员会投票，资深委员权重更高
//!
//! # 优点
//! - 实现简单，计算高效
//! - 直观易懂，结果可解释
//! - 能有效利用专家能力差异信息
//! - 对异常值有一定鲁棒性
//!
//! # 缺点
//! - 需要预先确定各专家权重，权重设定主观
//! - 对选项数量敏感，选项过多时效果下降
//! - 无法处理连续值输出（需配合离散化）
//! - 权重为静态的，不能根据任务动态调整

use std::collections::HashMap;
use std::hash::Hash;

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ClassificationFusionStrategy;

/// 加权投票融合器
///
/// 将多个带权重的投票融合为最终结果。
/// 每个投票者有一个权重，最终按各选项获得的权重总和排序。
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::WeightedVotingFusion;
/// use mox_alliance_core::fusion::traits::ClassificationFusionStrategy;
///
/// let fusion = WeightedVotingFusion::new();
/// let votes = vec![
///     ("A", 0.5),   // 专家1投A，权重0.5
///     ("B", 0.3),   // 专家2投B，权重0.3
///     ("A", 0.2),   // 专家3投A，权重0.2
/// ];
/// let (winner, score, total) = fusion.fuse_classification(&votes).unwrap();
/// assert_eq!(winner, "A");
/// assert!((score - 0.7).abs() < 1e-9);  // 0.5 + 0.2 = 0.7
/// assert!((total - 1.0).abs() < 1e-9);
/// ```
#[derive(Debug, Clone, Default)]
pub struct WeightedVotingFusion {
    /// 是否对权重进行归一化（默认 false）
    normalize_weights: bool,
}

impl WeightedVotingFusion {
    /// 创建一个新的加权投票融合器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否归一化权重
    ///
    /// 若为 true，则所有投票者的权重会被归一化到 [0, 1] 区间，
    /// 使得最终得分可以解释为"支持比例"。
    pub fn with_normalize_weights(mut self, normalize: bool) -> Self {
        self.normalize_weights = normalize;
        self
    }

    /// 执行加权投票融合，返回所有选项的得分排名
    ///
    /// # Arguments
    /// * `votes` - 投票列表，每个元素是 (选项, 权重)
    ///
    /// # Returns
    /// 按得分降序排列的 (选项, 得分) 列表
    ///
    /// # Errors
    /// - `FusionError::EmptyInput` — 投票列表为空
    /// - `FusionError::InvalidWeight` — 权重包含 NaN 或 Infinity
    /// - `FusionError::ZeroTotalWeight` — 权重总和为零（仅归一化模式）
    pub fn fuse_ranked<Category: Eq + Hash + Clone>(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<Vec<(Category, f64)>> {
        if votes.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        // 验证权重有效性
        for (i, (_, w)) in votes.iter().enumerate() {
            if !w.is_finite() {
                return Err(FusionError::InvalidWeight {
                    index: i,
                    value: format!("{}", w),
                });
            }
            if *w < 0.0 {
                return Err(FusionError::InvalidWeight {
                    index: i,
                    value: format!("{} (negative)", w),
                });
            }
        }

        // 计算各选项得分
        let mut scores: HashMap<&Category, f64> = HashMap::new();
        for (category, weight) in votes {
            *scores.entry(category).or_insert(0.0) += weight;
        }

        let total_weight: f64 = votes.iter().map(|(_, w)| w).sum();

        // 如果需要归一化
        let results: Vec<(Category, f64)> = if self.normalize_weights {
            if total_weight == 0.0 {
                return Err(FusionError::ZeroTotalWeight);
            }
            scores
                .into_iter()
                .map(|(cat, score)| (cat.clone(), score / total_weight))
                .collect()
        } else {
            scores
                .into_iter()
                .map(|(cat, score)| (cat.clone(), score))
                .collect()
        };

        // 按得分降序排序
        let mut sorted = results;
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(sorted)
    }
}

impl<Category: Eq + Hash + Clone> ClassificationFusionStrategy<Category>
    for WeightedVotingFusion
{
    fn name(&self) -> &'static str {
        "Weighted Voting Fusion"
    }

    fn fuse_classification(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<(Category, f64, f64)> {
        let ranked = self.fuse_ranked(votes)?;
        let total_weight: f64 = votes.iter().map(|(_, w)| w).sum();
        let (winner, score) = ranked
            .into_iter()
            .next()
            .ok_or(FusionError::EmptyInput)?;
        Ok((winner, score, total_weight))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ClassificationFusionStrategy;

    #[test]
    fn test_basic_weighted_voting() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 0.5), ("B", 0.3), ("A", 0.2)];
        let (winner, score, total) = fusion.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
        assert!((score - 0.7).abs() < 1e-9);
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_normalized_weights() {
        let fusion = WeightedVotingFusion::new().with_normalize_weights(true);
        let votes = vec![("A", 5.0), ("B", 3.0), ("A", 2.0)];
        let (winner, score, total) = fusion.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
        // 归一化后 A 的得分 = (5 + 2) / (5 + 3 + 2) = 0.7
        assert!((score - 0.7).abs() < 1e-9);
        assert!((total - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_tie_breaking() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 1.0), ("B", 1.0), ("C", 0.5)];
        let ranked = fusion.fuse_ranked(&votes).unwrap();
        assert_eq!(ranked.len(), 3);
        // 前两名并列
        assert!((ranked[0].1 - 1.0).abs() < 1e-9);
        assert!((ranked[1].1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_input() {
        let fusion = WeightedVotingFusion::new();
        let votes: Vec<(&str, f64)> = vec![];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_negative_weight() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 1.0), ("B", -0.5)];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::InvalidWeight { .. })));
    }

    #[test]
    fn test_nan_weight() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 1.0), ("B", f64::NAN)];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::InvalidWeight { .. })));
    }

    #[test]
    fn test_zero_total_weight_normalized() {
        let fusion = WeightedVotingFusion::new().with_normalize_weights(true);
        let votes = vec![("A", 0.0), ("B", 0.0)];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::ZeroTotalWeight)));
    }

    #[test]
    fn test_single_vote() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 1.0)];
        let (winner, score, total) = fusion.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
        assert!((score - 1.0).abs() < 1e-9);
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_different() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![("A", 0.3), ("B", 0.5), ("C", 0.2)];
        let ranked = fusion.fuse_ranked(&votes).unwrap();
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].0, "B");
        assert_eq!(ranked[1].0, "A");
        assert_eq!(ranked[2].0, "C");
    }

    #[test]
    fn test_fuse_ranked_ordering() {
        let fusion = WeightedVotingFusion::new();
        let votes = vec![
            ("X", 0.1),
            ("Y", 0.5),
            ("Z", 0.3),
            ("Y", 0.2),
            ("X", 0.4),
        ];
        let ranked = fusion.fuse_ranked(&votes).unwrap();
        // X: 0.1 + 0.4 = 0.5
        // Y: 0.5 + 0.2 = 0.7
        // Z: 0.3
        assert_eq!(ranked[0].0, "Y");
        assert!((ranked[0].1 - 0.7).abs() < 1e-9);
        assert_eq!(ranked[1].0, "X");
        assert!((ranked[1].1 - 0.5).abs() < 1e-9);
        assert_eq!(ranked[2].0, "Z");
        assert!((ranked[2].1 - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_name() {
        let fusion = WeightedVotingFusion::new();
        assert_eq!(<WeightedVotingFusion as ClassificationFusionStrategy<&str>>::name(&fusion), "Weighted Voting Fusion");
    }
}
