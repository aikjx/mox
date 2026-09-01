// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一融合引擎 (FusionEngine)
//!
//! 提供统一的融合入口，根据配置选择不同的融合策略。
//! 封装了六大融合策略的创建和调度逻辑。

use mox_alliance_common_proto::FusionStrategy as FusionStrategyType;

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::strategies::*;
use crate::fusion::traits::{ClassificationFusionStrategy, ScalarFusionStrategy};

/// 融合引擎配置
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// 融合策略类型
    pub strategy: FusionStrategyType,
    /// 权重列表（可选，用于加权类策略）
    pub weights: Option<Vec<f64>>,
    /// 最大迭代/轮次数（用于迭代/辩论类策略）
    pub max_rounds: Option<usize>,
    /// 收敛阈值
    pub tolerance: Option<f64>,
    /// 分组数量（用于 Map-Reduce）
    pub num_partitions: Option<usize>,
    /// 学习率（用于辩论）
    pub learning_rate: Option<f64>,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            strategy: FusionStrategyType::Weighted,
            weights: None,
            max_rounds: None,
            tolerance: None,
            num_partitions: None,
            learning_rate: None,
        }
    }
}

impl FusionConfig {
    /// 创建指定策略的配置
    pub fn new(strategy: FusionStrategyType) -> Self {
        Self {
            strategy,
            ..Default::default()
        }
    }

    /// 设置权重
    pub fn with_weights(mut self, weights: Vec<f64>) -> Self {
        self.weights = Some(weights);
        self
    }

    /// 设置最大轮次
    pub fn with_max_rounds(mut self, rounds: usize) -> Self {
        self.max_rounds = Some(rounds);
        self
    }

    /// 设置收敛阈值
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = Some(tolerance);
        self
    }

    /// 设置分组数量
    pub fn with_num_partitions(mut self, n: usize) -> Self {
        self.num_partitions = Some(n);
        self
    }

    /// 设置学习率
    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.learning_rate = Some(rate);
        self
    }
}

/// 统一融合引擎
///
/// 根据配置创建并调度不同的融合策略。
/// 支持标量融合和分类融合两种模式。
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::FusionEngine;
/// use mox_alliance_common_proto::FusionStrategy;
///
/// // 标量融合
/// let engine = FusionEngine::from_strategy(FusionStrategy::Weighted);
/// let values = vec![(80.0, 0.3), (90.0, 0.5), (70.0, 0.2)];
/// let result = engine.fuse_scalar(&values).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct FusionEngine {
    config: FusionConfig,
}

impl FusionEngine {
    /// 从策略类型创建融合引擎
    pub fn from_strategy(strategy: FusionStrategyType) -> Self {
        Self {
            config: FusionConfig::new(strategy),
        }
    }

    /// 从配置创建融合引擎
    pub fn from_config(config: FusionConfig) -> Self {
        Self { config }
    }

    /// 获取当前配置
    pub fn config(&self) -> &FusionConfig {
        &self.config
    }

    /// 获取当前策略类型
    pub fn strategy(&self) -> FusionStrategyType {
        self.config.strategy
    }

    /// 执行标量融合
    ///
    /// 将多个 (值, 权重/置信度) 对融合为单个标量结果。
    ///
    /// # Arguments
    /// * `values` - (值, 权重) 列表
    ///
    /// # Returns
    /// 融合后的标量值
    pub fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64> {
        match self.config.strategy {
            FusionStrategyType::Weighted | FusionStrategyType::Voting => {
                // 加权/投票策略对标量场景使用置信度加权融合
                let fusion = self.build_confidence_weighting();
                fusion.fuse_scalar(values)
            }
            FusionStrategyType::ConfidenceWeighted => {
                let fusion = self.build_confidence_weighting();
                fusion.fuse_scalar(values)
            }
            FusionStrategyType::BestOf => {
                // 择优：选权重最高的值
                if values.is_empty() {
                    return Err(FusionError::EmptyInput);
                }
                values
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(v, _)| *v)
                    .ok_or(FusionError::EmptyInput)
            }
            FusionStrategyType::Concatenation => {
                // 拼接对标量场景降级为求和
                Ok(values.iter().map(|(v, _)| v).sum())
            }
            FusionStrategyType::Stacking => {
                // 堆叠融合：使用简单的平均权重作为 baseline
                // 完整的 stacking 需要训练数据，此处降级为置信度加权
                let fusion = self.build_confidence_weighting();
                fusion.fuse_scalar(values)
            }
            FusionStrategyType::Debate => {
                // 辩论对标量场景降级为迭代精炼
                let fusion = self.build_iterative_refinement();
                fusion.fuse_scalar(values)
            }
            FusionStrategyType::MapReduce => {
                let fusion = self.build_map_reduce();
                fusion.fuse_scalar(values)
            }
            FusionStrategyType::Iterative => {
                let fusion = self.build_iterative_refinement();
                fusion.fuse_scalar(values)
            }
        }
    }

    /// 执行分类融合
    ///
    /// 将多个 (类别, 权重) 对融合为胜出的类别。
    ///
    /// # Arguments
    /// * `votes` - (类别, 权重) 列表
    ///
    /// # Returns
    /// (胜出类别, 最终得分, 总权重)
    pub fn fuse_classification<Category: Eq + std::hash::Hash + Clone>(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<(Category, f64, f64)> {
        match self.config.strategy {
            FusionStrategyType::Voting | FusionStrategyType::Weighted => {
                let fusion = self.build_weighted_voting();
                fusion.fuse_classification(votes)
            }
            FusionStrategyType::ConfidenceWeighted => {
                // 置信度加权对分类场景使用加权投票
                let fusion = self.build_weighted_voting();
                fusion.fuse_classification(votes)
            }
            FusionStrategyType::Debate => {
                let fusion = self.build_debate();
                fusion.fuse_classification(votes)
            }
            FusionStrategyType::BestOf => {
                // 择优：选权重最高的类别
                if votes.is_empty() {
                    return Err(FusionError::EmptyInput);
                }
                let best = votes
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .ok_or(FusionError::EmptyInput)?;
                let total: f64 = votes.iter().map(|(_, w)| w).sum();
                Ok((best.0.clone(), best.1, total))
            }
            FusionStrategyType::Stacking
            | FusionStrategyType::MapReduce
            | FusionStrategyType::Concatenation
            | FusionStrategyType::Iterative => {
                // 这些策略对分类场景降级为加权投票
                let fusion = self.build_weighted_voting();
                fusion.fuse_classification(votes)
            }
        }
    }

    // ─── 策略构建器 ────────────────────────────────────────────────────────

    fn build_weighted_voting(&self) -> WeightedVotingFusion {
        WeightedVotingFusion::new()
    }

    fn build_confidence_weighting(&self) -> ConfidenceWeightingFusion {
        let mut fusion = ConfidenceWeightingFusion::new();
        if let Some(min_conf) = self.config.tolerance {
            // 复用 tolerance 作为最低置信度阈值的近似配置
            if min_conf > 0.0 && min_conf < 1.0 {
                fusion = fusion.with_min_confidence(min_conf);
            }
        }
        fusion
    }

    fn build_debate(&self) -> DebateFusion {
        let mut fusion = DebateFusion::new();
        if let Some(rounds) = self.config.max_rounds {
            fusion = fusion.with_max_rounds(rounds);
        }
        if let Some(rate) = self.config.learning_rate {
            fusion = fusion.with_learning_rate(rate);
        }
        if let Some(tol) = self.config.tolerance {
            fusion = fusion.with_convergence_threshold(tol);
        }
        fusion
    }

    fn build_iterative_refinement(&self) -> IterativeRefinementFusion {
        let mut fusion = IterativeRefinementFusion::new();
        if let Some(iterations) = self.config.max_rounds {
            fusion = fusion.with_max_iterations(iterations);
        }
        if let Some(tol) = self.config.tolerance {
            fusion = fusion.with_tolerance(tol);
        }
        fusion
    }

    fn build_map_reduce(&self) -> MapReduceFusion {
        let n = self.config.num_partitions.unwrap_or(3);
        MapReduceFusion::new(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_weighted_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Weighted);
        let values = vec![(80.0, 0.3), (90.0, 0.5), (70.0, 0.2)];
        let result = engine.fuse_scalar(&values).unwrap();
        assert!((result - 83.0).abs() < 0.001);
    }

    #[test]
    fn test_engine_voting_classification() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Voting);
        let votes = vec![("A", 0.5), ("B", 0.3), ("A", 0.2)];
        let (winner, score, total) = engine.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
        assert!((score - 0.7).abs() < 1e-9);
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_engine_debate_classification() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Debate);
        let votes = vec![("支持", 0.8), ("反对", 0.3), ("支持", 0.6)];
        let (winner, _, _) = engine.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "支持");
    }

    #[test]
    fn test_engine_best_of_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::BestOf);
        let values = vec![(10.0, 0.5), (20.0, 0.9), (15.0, 0.7)];
        let result = engine.fuse_scalar(&values).unwrap();
        // 权重最高的是 20.0（权重 0.9）
        assert_eq!(result, 20.0);
    }

    #[test]
    fn test_engine_best_of_classification() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::BestOf);
        let votes = vec![("A", 0.3), ("B", 0.8), ("C", 0.5)];
        let (winner, score, total) = engine.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "B");
        assert!((score - 0.8).abs() < 1e-9);
        assert!((total - 1.6).abs() < 1e-9);
    }

    #[test]
    fn test_engine_iterative_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Iterative);
        let values = vec![
            (10.0, 0.8),
            (10.5, 0.7),
            (9.8, 0.9),
            (100.0, 0.5), // 异常值
        ];
        let result = engine.fuse_scalar(&values).unwrap();
        // 迭代精炼应该降低异常值影响
        assert!(result < 20.0);
    }

    #[test]
    fn test_engine_concatenation_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Concatenation);
        let values = vec![(1.0, 1.0), (2.0, 1.0), (3.0, 1.0)];
        let result = engine.fuse_scalar(&values).unwrap();
        // 拼接对标量降级为求和
        assert_eq!(result, 6.0);
    }

    #[test]
    fn test_engine_empty_input() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Weighted);
        let values: Vec<(f64, f64)> = vec![];
        let result = engine.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_engine_with_config() {
        let config = FusionConfig::new(FusionStrategyType::Debate)
            .with_max_rounds(3)
            .with_learning_rate(0.5);
        let engine = FusionEngine::from_config(config);
        assert_eq!(engine.strategy(), FusionStrategyType::Debate);
        assert_eq!(engine.config().max_rounds, Some(3));
    }

    #[test]
    fn test_engine_config_default() {
        let config = FusionConfig::default();
        assert_eq!(config.strategy, FusionStrategyType::Weighted);
        assert!(config.weights.is_none());
        assert!(config.max_rounds.is_none());
    }

    #[test]
    fn test_fusion_config_builder() {
        let config = FusionConfig::new(FusionStrategyType::Iterative)
            .with_weights(vec![0.5, 0.3, 0.2])
            .with_max_rounds(10)
            .with_tolerance(1e-5)
            .with_num_partitions(5)
            .with_learning_rate(0.3);

        assert_eq!(config.strategy, FusionStrategyType::Iterative);
        assert_eq!(config.weights.as_ref().unwrap(), &vec![0.5, 0.3, 0.2]);
        assert_eq!(config.max_rounds, Some(10));
        assert!((config.tolerance.unwrap() - 1e-5).abs() < 1e-12);
        assert_eq!(config.num_partitions, Some(5));
        assert!((config.learning_rate.unwrap() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_engine_confidence_weighted_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::ConfidenceWeighted);
        let values = vec![(80.0, 0.9), (90.0, 0.7), (70.0, 0.5)];
        let result = engine.fuse_scalar(&values).unwrap();
        // (80*0.9 + 90*0.7 + 70*0.5) / (0.9 + 0.7 + 0.5) = 170 / 2.1 ≈ 80.95
        assert!((result - 80.95).abs() < 0.01);
    }

    #[test]
    fn test_engine_confidence_weighted_classification() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::ConfidenceWeighted);
        let votes = vec![("A", 0.6), ("B", 0.4), ("A", 0.3)];
        let (winner, _, _) = engine.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
    }

    #[test]
    fn test_engine_map_reduce_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::MapReduce);
        let values = vec![
            (10.0, 0.8),
            (20.0, 0.7),
            (30.0, 0.9),
            (40.0, 0.6),
        ];
        let result = engine.fuse_scalar(&values).unwrap();
        assert!(result > 0.0);
        assert!((10.0..=40.0).contains(&result));
    }

    #[test]
    fn test_engine_map_reduce_with_config() {
        let config = FusionConfig::new(FusionStrategyType::MapReduce)
            .with_num_partitions(2);
        let engine = FusionEngine::from_config(config);
        let values = vec![(10.0, 0.8), (20.0, 0.7), (30.0, 0.9), (40.0, 0.6)];
        let result = engine.fuse_scalar(&values).unwrap();
        assert!(result > 0.0);
    }

    #[test]
    fn test_engine_stacking_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Stacking);
        let values = vec![(10.0, 0.8), (20.0, 0.7)];
        let result = engine.fuse_scalar(&values).unwrap();
        // Stacking 在引擎中降级为置信度加权
        assert!(result > 0.0);
    }

    #[test]
    fn test_engine_stacking_classification() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Stacking);
        let votes = vec![("A", 0.6), ("B", 0.4), ("A", 0.5)];
        let (winner, _, _) = engine.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
    }

    #[test]
    fn test_engine_debate_scalar() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::Debate);
        let values = vec![
            (10.0, 0.8),
            (10.5, 0.7),
            (9.5, 0.9),
            (50.0, 0.3), // 异常值
        ];
        let result = engine.fuse_scalar(&values).unwrap();
        // 辩论对标量降级为迭代精炼，应该降低异常值影响
        assert!(result < 20.0);
    }

    #[test]
    fn test_engine_strategy_accessor() {
        let engine = FusionEngine::from_strategy(FusionStrategyType::MapReduce);
        assert_eq!(engine.strategy(), FusionStrategyType::MapReduce);
    }

    #[test]
    fn test_all_strategies_scalar_fusion() {
        // 验证所有支持标量融合的策略都能正常工作
        let strategies = [
            FusionStrategyType::Weighted,
            FusionStrategyType::ConfidenceWeighted,
            FusionStrategyType::BestOf,
            FusionStrategyType::Stacking,
            FusionStrategyType::MapReduce,
            FusionStrategyType::Iterative,
            FusionStrategyType::Debate,
            FusionStrategyType::Voting,
            FusionStrategyType::Concatenation,
        ];

        let values = vec![(10.0, 0.8), (20.0, 0.7), (15.0, 0.9)];

        for strategy in &strategies {
            let engine = FusionEngine::from_strategy(*strategy);
            let result = engine.fuse_scalar(&values);
            assert!(result.is_ok(), "Strategy {:?} failed: {:?}", strategy, result.err());
        }
    }

    #[test]
    fn test_all_strategies_classification_fusion() {
        // 验证所有支持分类融合的策略都能正常工作
        let strategies = [
            FusionStrategyType::Voting,
            FusionStrategyType::Weighted,
            FusionStrategyType::ConfidenceWeighted,
            FusionStrategyType::BestOf,
            FusionStrategyType::Debate,
            FusionStrategyType::Iterative,
            FusionStrategyType::Stacking,
            FusionStrategyType::MapReduce,
            FusionStrategyType::Concatenation,
        ];

        let votes = vec![("A", 0.5), ("B", 0.3), ("A", 0.4)];

        for strategy in &strategies {
            let engine = FusionEngine::from_strategy(*strategy);
            let result = engine.fuse_classification(&votes);
            assert!(
                result.is_ok(),
                "Strategy {:?} classification failed: {:?}",
                strategy,
                result.err()
            );
        }
    }
}
