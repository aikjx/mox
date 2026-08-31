// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Map-Reduce 融合 (Map-Reduce Fusion)
//!
//! # 原理
//! Map-Reduce 融合借鉴了 MapReduce 编程模型的思想，将大规模融合任务分解为：
//!
//! 1. **Map 阶段**：将输入数据分成多个组（Shard/Partition），
//!    每个组独立执行局部融合。
//!    - 可以按专家领域分组
//!    - 可以按数据特征分组
//!    - 可以随机分组（用于集成多样性）
//!
//! 2. **Shuffle 阶段**（可选）：将 Map 阶段的输出重新分组，
//!    为 Reduce 阶段做准备。
//!
//! 3. **Reduce 阶段**：将各组的融合结果再次融合，得到最终结果。
//!
//! 这种分治策略的核心优势在于：
//! - **可扩展性**：支持大规模数据的并行处理
//! - **容错性**：单个分组失败不影响整体（可降级处理）
//! - **多样性**：不同分组可采用不同策略，增加结果鲁棒性
//!
//! # 适用场景
//! - **大规模数据融合**：专家数量或数据量非常大时
//! - **分层决策**：先小组内讨论，再汇总各组意见
//! - **分布式系统**：融合任务需要分布到多个节点执行
//! - **分层专家架构**：专家按领域/层级组织
//! - **容错要求高**：部分节点/专家故障时仍需给出结果
//!
//! # 优点
//! - 天然支持并行计算，可扩展性好
//! - 容错能力强，局部失败不影响全局
//! - 可以结合多种融合策略，取长补短
//! - 适合分层/分布式架构
//! - 计算复杂度可控
//!
//! # 缺点
//! - 分组策略会影响最终结果
//! - 信息损失：组间信息交互有限
//! - 实现复杂度高于简单融合策略
//! - 小数据场景下优势不明显

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ScalarFusionStrategy;
use crate::fusion::strategies::confidence_weighting::ConfidenceWeightingFusion;

/// 分组策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionStrategy {
    /// 平均切分（按输入顺序）
    Sequential,
    /// 随机分组（提高多样性）
    Random,
    /// 按权重分层（高权重与低权重混合）
    WeightBalanced,
}

impl Default for PartitionStrategy {
    fn default() -> Self {
        Self::Sequential
    }
}

/// Map-Reduce 融合器
///
/// 将输入数据分成多个组，每组独立融合（Map 阶段），
/// 然后将各组结果再次融合（Reduce 阶段）。
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::MapReduceFusion;
/// use mox_alliance_core::fusion::traits::ScalarFusionStrategy;
/// use mox_alliance_core::fusion::strategies::map_reduce::PartitionStrategy;
///
/// let fusion = MapReduceFusion::new(3)  // 分成3组
///     .with_partition_strategy(PartitionStrategy::WeightBalanced);
///
/// let values = vec![
///     (10.0, 0.8),
///     (20.0, 0.7),
///     (30.0, 0.9),
///     (40.0, 0.6),
///     (50.0, 0.5),
///     (60.0, 0.8),
/// ];
///
/// let result = fusion.fuse_scalar(&values).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct MapReduceFusion {
    /// 分组数量
    num_partitions: usize,
    /// 分组策略
    partition_strategy: PartitionStrategy,
    /// 组内融合策略（Map 阶段）
    /// 默认为置信度加权融合
    map_fusion: ConfidenceWeightingFusion,
    /// 组间融合策略（Reduce 阶段）
    /// 默认为置信度加权融合
    reduce_fusion: ConfidenceWeightingFusion,
    /// 随机种子（仅 Random 分组策略使用）
    random_seed: Option<u64>,
}

impl MapReduceFusion {
    /// 创建一个新的 Map-Reduce 融合器
    ///
    /// # Arguments
    /// * `num_partitions` - 分组数量
    pub fn new(num_partitions: usize) -> Self {
        Self {
            num_partitions,
            partition_strategy: PartitionStrategy::default(),
            map_fusion: ConfidenceWeightingFusion::new(),
            reduce_fusion: ConfidenceWeightingFusion::new(),
            random_seed: None,
        }
    }

    /// 设置分组策略
    pub fn with_partition_strategy(mut self, strategy: PartitionStrategy) -> Self {
        self.partition_strategy = strategy;
        self
    }

    /// 设置随机种子（用于 Random 分组策略的可重复性）
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// 设置 Map 阶段的最小置信度阈值
    pub fn with_map_min_confidence(mut self, min_conf: f64) -> Self {
        self.map_fusion = self.map_fusion.with_min_confidence(min_conf);
        self
    }

    /// 设置 Reduce 阶段的最小置信度阈值
    pub fn with_reduce_min_confidence(mut self, min_conf: f64) -> Self {
        self.reduce_fusion = self.reduce_fusion.with_min_confidence(min_conf);
        self
    }

    /// 执行 Map-Reduce 融合，返回中间结果
    ///
    /// # Returns
    /// * `final_result` - 最终融合结果
    /// * `partition_results` - 各分组的融合结果 (组内结果, 组内有效样本数)
    pub fn fuse_with_partitions(
        &self,
        values: &[(f64, f64)],
    ) -> FusionResult<(f64, Vec<(f64, usize)>)> {
        // 验证输入
        if values.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        if self.num_partitions == 0 {
            return Err(FusionError::invalid_param(
                "num_partitions",
                "must be at least 1",
            ));
        }

        // 验证值
        for (i, (val, conf)) in values.iter().enumerate() {
            if !val.is_finite() {
                return Err(FusionError::InvalidParameter {
                    param: "value",
                    reason: format!("non-finite value at index {}: {}", i, val),
                });
            }
            if !conf.is_finite() || *conf < 0.0 || *conf > 1.0 {
                return Err(FusionError::InvalidConfidence {
                    index: i,
                    value: *conf,
                });
            }
        }

        // 分区
        let partitions = self.partition(values);

        // Map 阶段：每个分区独立融合
        let mut partition_results: Vec<(f64, usize)> = Vec::new();
        let mut reduce_inputs: Vec<(f64, f64)> = Vec::new();

        for partition in &partitions {
            if partition.is_empty() {
                continue;
            }

            match self.map_fusion.fuse_scalar_with_stats(partition) {
                Ok((result, count, total_conf)) => {
                    partition_results.push((result, count));
                    // Reduce 阶段的置信度 = 该组的平均置信度
                    let avg_conf = if count > 0 {
                        total_conf / count as f64
                    } else {
                        0.0
                    };
                    reduce_inputs.push((result, avg_conf));
                }
                Err(FusionError::EmptyInput) => {
                    // 该组全部被过滤，跳过
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        if reduce_inputs.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        // Reduce 阶段：融合各分区结果
        let final_result = self.reduce_fusion.fuse_scalar(&reduce_inputs)?;

        Ok((final_result, partition_results))
    }

    /// 将输入数据分组
    fn partition(&self, values: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
        let n = values.len();
        let k = self.num_partitions.min(n);

        match self.partition_strategy {
            PartitionStrategy::Sequential => {
                // 顺序切分
                let mut partitions: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
                let chunk_size = (n + k - 1) / k; // 向上取整

                for (i, &item) in values.iter().enumerate() {
                    let p = (i / chunk_size).min(k - 1);
                    partitions[p].push(item);
                }

                partitions
            }
            PartitionStrategy::Random => {
                // 随机分组（使用确定性伪随机）
                let mut rng_state = self.random_seed.unwrap_or(42);
                let mut indices: Vec<usize> = (0..n).collect();

                // Fisher-Yates shuffle
                for i in (1..n).rev() {
                    // 简单的线性同余生成器
                    rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let j = (rng_state % (i as u64 + 1)) as usize;
                    indices.swap(i, j);
                }

                let mut partitions: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
                for (i, &idx) in indices.iter().enumerate() {
                    let p = i % k;
                    partitions[p].push(values[idx]);
                }

                partitions
            }
            PartitionStrategy::WeightBalanced => {
                // 按权重排序后蛇形分配，使各组总权重尽量均衡
                let mut indexed: Vec<(usize, f64)> =
                    values.iter().enumerate().map(|(i, (_, w))| (i, *w)).collect();

                // 按权重降序排序
                indexed.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                });

                let mut partitions: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
                let mut partition_weights = vec![0.0f64; k];

                for (idx, weight) in indexed {
                    // 分配到当前总权重最小的组
                    let mut min_p = 0;
                    let mut min_w = partition_weights[0];
                    for p in 1..k {
                        if partition_weights[p] < min_w {
                            min_w = partition_weights[p];
                            min_p = p;
                        }
                    }
                    partitions[min_p].push(values[idx]);
                    partition_weights[min_p] += weight;
                }

                partitions
            }
        }
    }

    /// 获取分组数量
    pub fn num_partitions(&self) -> usize {
        self.num_partitions
    }

    /// 获取分组策略
    pub fn partition_strategy(&self) -> PartitionStrategy {
        self.partition_strategy
    }
}

impl ScalarFusionStrategy for MapReduceFusion {
    fn name(&self) -> &'static str {
        "Map-Reduce Fusion"
    }

    fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64> {
        self.fuse_with_partitions(values).map(|(r, _)| r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ScalarFusionStrategy;

    #[test]
    fn test_basic_map_reduce() {
        let fusion = MapReduceFusion::new(2);
        let values = vec![
            (10.0, 0.8),
            (20.0, 0.6),
            (30.0, 0.9),
            (40.0, 0.7),
        ];

        let result = fusion.fuse_scalar(&values).unwrap();
        assert!(result > 0.0);
        // 结果应该在输入值范围内
        assert!(result >= 10.0 && result <= 40.0);
    }

    #[test]
    fn test_single_partition() {
        // 单分组应该退化为普通置信度加权
        let fusion = MapReduceFusion::new(1);
        let values = vec![(10.0, 0.8), (20.0, 0.6)];

        let result = fusion.fuse_scalar(&values).unwrap();
        let expected = (10.0 * 0.8 + 20.0 * 0.6) / (0.8 + 0.6);
        assert!((result - expected).abs() < 0.001);
    }

    #[test]
    fn test_empty_input() {
        let fusion = MapReduceFusion::new(2);
        let values: Vec<(f64, f64)> = vec![];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_zero_partitions() {
        let fusion = MapReduceFusion::new(0);
        let values = vec![(10.0, 0.8)];
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_random_partition_reproducible() {
        let values = vec![
            (1.0, 0.5),
            (2.0, 0.6),
            (3.0, 0.7),
            (4.0, 0.8),
            (5.0, 0.9),
            (6.0, 0.5),
        ];

        let fusion1 = MapReduceFusion::new(3)
            .with_partition_strategy(PartitionStrategy::Random)
            .with_random_seed(12345);
        let fusion2 = MapReduceFusion::new(3)
            .with_partition_strategy(PartitionStrategy::Random)
            .with_random_seed(12345);

        let r1 = fusion1.fuse_scalar(&values).unwrap();
        let r2 = fusion2.fuse_scalar(&values).unwrap();

        // 相同种子应该得到相同结果
        assert!((r1 - r2).abs() < 1e-9);
    }

    #[test]
    fn test_weight_balanced_partition() {
        // 构造权重差异大的数据（置信度范围 [0, 1]）
        let values = vec![
            (100.0, 0.95), // 高置信度
            (1.0, 0.1),
            (2.0, 0.1),
            (3.0, 0.1),
            (4.0, 0.1),
            (5.0, 0.1),
        ];

        let fusion = MapReduceFusion::new(2)
            .with_partition_strategy(PartitionStrategy::WeightBalanced);

        let (_, partitions) = fusion.fuse_with_partitions(&values).unwrap();
        assert_eq!(partitions.len(), 2);
        // 两组都应该有数据
        assert!(partitions[0].1 > 0);
        assert!(partitions[1].1 > 0);
    }

    #[test]
    fn test_with_partition_results() {
        let fusion = MapReduceFusion::new(3);
        let values = vec![
            (10.0, 0.8),
            (20.0, 0.7),
            (30.0, 0.9),
            (40.0, 0.6),
            (50.0, 0.5),
            (60.0, 0.8),
        ];

        let (final_result, partitions) = fusion.fuse_with_partitions(&values).unwrap();
        assert_eq!(partitions.len(), 3);
        assert!(final_result > 0.0);
        // 所有样本数之和应等于输入数量
        let total_count: usize = partitions.iter().map(|(_, c)| *c).sum();
        assert_eq!(total_count, 6);
    }

    #[test]
    fn test_more_partitions_than_items() {
        let fusion = MapReduceFusion::new(10); // 10个分组，但只有3个数据
        let values = vec![(10.0, 0.8), (20.0, 0.7), (30.0, 0.9)];

        let (_, partitions) = fusion.fuse_with_partitions(&values).unwrap();
        // 实际有效分组数不超过数据量
        assert!(partitions.len() <= 3);
    }

    #[test]
    fn test_invalid_confidence() {
        let fusion = MapReduceFusion::new(2);
        let values = vec![(10.0, 1.5)]; // 置信度超出范围
        let result = fusion.fuse_scalar(&values);
        assert!(matches!(
            result,
            Err(FusionError::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn test_min_confidence_filtering() {
        let fusion = MapReduceFusion::new(2).with_map_min_confidence(0.5);
        let values = vec![
            (100.0, 0.1), // 会被过滤
            (50.0, 0.9),  // 保留
            (60.0, 0.8),  // 保留
            (200.0, 0.2), // 会被过滤
        ];

        let (_, partitions) = fusion.fuse_with_partitions(&values).unwrap();
        let total_count: usize = partitions.iter().map(|(_, c)| *c).sum();
        // 应该只有2个有效样本通过过滤
        assert_eq!(total_count, 2);
    }

    #[test]
    fn test_name() {
        let fusion = MapReduceFusion::new(2);
        assert_eq!(fusion.name(), "Map-Reduce Fusion");
    }

    #[test]
    fn test_partition_strategy_default() {
        assert_eq!(
            PartitionStrategy::default(),
            PartitionStrategy::Sequential
        );
    }

    #[test]
    fn test_accessors() {
        let fusion = MapReduceFusion::new(5)
            .with_partition_strategy(PartitionStrategy::Random);
        assert_eq!(fusion.num_partitions(), 5);
        assert_eq!(fusion.partition_strategy(), PartitionStrategy::Random);
    }
}
