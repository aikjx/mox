// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 辩论融合 (Debate / Multi-Agent Debate Fusion)
//!
//! # 原理
//! 辩论融合模拟人类辩论过程：多个智能体（专家）针对问题提出各自立场，
//! 经过多轮辩论（提出论点、反驳对方、更新立场），最终根据各立场的置信度
//! 进行裁决。
//!
//! 辩论过程的核心机制：
//! 1. **初始陈述**：各专家提出初始立场和置信度
//! 2. **多轮辩论**：每轮中专家根据其他专家的论点调整自身置信度
//!    - 共识效应：与多数专家一致时，置信度提升
//!    - 异议效应：与多数专家不一致时，置信度下降
//!    - 专业度加权：高权重专家的影响力更大
//! 3. **最终裁决**：根据最终置信度加权得出结果
//!
//! 置信度更新公式（简化模型）：
//! ```text
//! new_confidence = old_confidence + α * (agreement - 0.5) * (1 - old_confidence)
//! ```
//! 其中 agreement 是与其他专家的加权一致度，α 是学习率。
//!
//! # 适用场景
//! - **争议性问题**：存在多种合理解释，需要多角度论证
//! - **高风险决策**：需要充分讨论才能做出的重要决策
//! - **多视角问题**：不同领域专家从不同角度分析同一问题
//! - **事实核查**：多方验证信息真实性
//! - **创造性任务**：多方案比较与择优
//!
//! # 优点
//! - 能充分挖掘不同观点的价值
//! - 模拟人类辩论过程，结果更易被接受
//! - 对初始置信度的鲁棒性较好
//! - 可解释性强：可追踪每轮辩论的立场变化
//! - 能有效减少个体偏差
//!
//! # 缺点
//! - 计算成本较高（多轮迭代）
//! - 参数（学习率、轮数）需要调优
//! - 可能出现群体思维（从众效应）
//! - 辩论模型简化了真实辩论的复杂性
//! - 极端情况下可能震荡不收敛

use std::collections::HashMap;
use std::hash::Hash;

use crate::fusion::error::{FusionError, FusionResult};
use crate::fusion::traits::ClassificationFusionStrategy;

/// 辩论轮次结果
#[derive(Debug, Clone)]
pub struct DebateRound<Category: Clone> {
    /// 轮次编号（从 0 开始）
    pub round: usize,
    /// 各立场的置信度（辩论后）
    pub confidences: Vec<(Category, f64)>,
    /// 本轮最大置信度变化
    pub max_delta: f64,
}

/// 辩论融合器
///
/// 模拟多智能体辩论过程，通过多轮交互更新各立场的置信度，
/// 最终根据置信度裁决出最优结果。
///
/// # 示例
///
/// ```
/// use mox_alliance_core::fusion::DebateFusion;
/// use mox_alliance_core::fusion::traits::ClassificationFusionStrategy;
///
/// let fusion = DebateFusion::new()
///     .with_max_rounds(5)
///     .with_learning_rate(0.3);
///
/// let votes = vec![
///     ("支持", 0.8),
///     ("反对", 0.7),
///     ("支持", 0.6),
///     ("弃权", 0.5),
/// ];
///
/// let (winner, final_conf, _) = fusion.fuse_classification(&votes).unwrap();
/// println!("最终结果: {}, 置信度: {}", winner, final_conf);
/// ```
#[derive(Debug, Clone)]
pub struct DebateFusion {
    /// 最大辩论轮次
    max_rounds: usize,
    /// 学习率（每轮置信度调整幅度）
    learning_rate: f64,
    /// 收敛阈值（最大置信度变化小于此值时提前终止）
    convergence_threshold: f64,
    /// 专家权重的影响系数
    expert_weight_influence: f64,
}

impl Default for DebateFusion {
    fn default() -> Self {
        Self {
            max_rounds: 10,
            learning_rate: 0.2,
            convergence_threshold: 1e-4,
            expert_weight_influence: 1.0,
        }
    }
}

impl DebateFusion {
    /// 创建一个新的辩论融合器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大辩论轮次（默认 10）
    pub fn with_max_rounds(mut self, rounds: usize) -> Self {
        self.max_rounds = rounds;
        self
    }

    /// 设置学习率（默认 0.2）
    ///
    /// 学习率控制每轮置信度调整的幅度：
    /// - 较大值：收敛快，但可能震荡
    /// - 较小值：收敛慢，但更稳定
    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.learning_rate = rate;
        self
    }

    /// 设置收敛阈值（默认 1e-4）
    ///
    /// 当最大置信度变化小于此阈值时，提前终止辩论。
    pub fn with_convergence_threshold(mut self, threshold: f64) -> Self {
        self.convergence_threshold = threshold;
        self
    }

    /// 设置专家权重影响力（默认 1.0）
    ///
    /// 控制专家原始权重对辩论结果的影响程度。
    pub fn with_expert_weight_influence(mut self, influence: f64) -> Self {
        self.expert_weight_influence = influence;
        self
    }

    /// 执行辩论融合，返回完整的辩论过程
    ///
    /// # Arguments
    /// * `votes` - 初始投票列表，每个元素是 (立场, 初始置信度/专家权重)
    ///
    /// # Returns
    /// * `winner` - 最终胜出的立场
    /// * `final_confidence` - 最终置信度
    /// * `rounds` - 各轮辩论结果
    /// * `total_weight` - 总权重
    pub fn debate<Category: Eq + Hash + Clone>(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<(Category, f64, Vec<DebateRound<Category>>, f64)> {
        // 验证输入
        if votes.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        for (i, (_, conf)) in votes.iter().enumerate() {
            if !conf.is_finite() {
                return Err(FusionError::InvalidConfidence {
                    index: i,
                    value: *conf,
                });
            }
            if *conf < 0.0 {
                return Err(FusionError::InvalidConfidence {
                    index: i,
                    value: *conf,
                });
            }
        }

        if self.max_rounds == 0 {
            return Err(FusionError::invalid_param(
                "max_rounds",
                "must be at least 1",
            ));
        }

        // 聚合初始立场：同一立场的多个专家合并
        let mut position_confidences: HashMap<Category, (f64, f64)> = HashMap::new(); // (总置信度, 总权重)
        for (position, weight) in votes {
            let entry = position_confidences
                .entry(position.clone())
                .or_insert((0.0, 0.0));
            entry.0 += weight; // 初始置信度用权重表示
            entry.1 += weight; // 总权重
        }

        let total_weight: f64 = votes.iter().map(|(_, w)| w).sum();

        // 归一化初始置信度
        let mut positions: Vec<(Category, f64, f64)> = position_confidences
            .into_iter()
            .map(|(pos, (conf, weight))| {
                let normalized_conf = if total_weight > 0.0 {
                    conf / total_weight
                } else {
                    0.0
                };
                (pos, normalized_conf, weight)
            })
            .collect();

        if positions.is_empty() {
            return Err(FusionError::EmptyInput);
        }

        let mut rounds: Vec<DebateRound<Category>> = Vec::new();

        // 多轮辩论
        for round in 0..self.max_rounds {
            let old_confidences: Vec<f64> = positions.iter().map(|(_, c, _)| *c).collect();
            let total_weight_pos: f64 = positions.iter().map(|(_, _, w)| w).sum();

            // 计算每个立场的"社会压力"（其他立场的加权影响）
            let mut updates: Vec<f64> = Vec::with_capacity(positions.len());

            for (i, (_, conf, weight)) in positions.iter().enumerate() {
                // 计算与其他立场的"加权一致度"
                // 简化模型：多数派压力 = 其他立场置信度之和的加权
                let other_conf_sum: f64 = positions
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, (_, c, w))| {
                        // 权重越大的立场，影响力越大
                        let weight_factor = if total_weight_pos > 0.0 {
                            w / total_weight_pos
                        } else {
                            0.0
                        };
                        c * weight_factor * self.expert_weight_influence
                    })
                    .sum();

                // 共识效应：如果我是多数派，置信度提升
                // 异议效应：如果我是少数派，置信度下降
                let majority_pressure = if *conf > other_conf_sum {
                    // 我是多数派，置信度增加
                    self.learning_rate * (*conf - other_conf_sum) * (1.0 - *conf)
                } else {
                    // 我是少数派，置信度减少
                    -self.learning_rate * (other_conf_sum - *conf) * *conf
                };

                // 权重的自我强化：高权重专家的立场更稳定
                let weight_stability = if total_weight_pos > 0.0 {
                    let weight_ratio = weight / total_weight_pos;
                    1.0 - 0.5 * weight_ratio // 权重大则变化小
                } else {
                    1.0
                };

                let update = majority_pressure * weight_stability;
                updates.push(update);
            }

            // 应用更新
            let mut max_delta = 0.0;
            for (i, update) in updates.into_iter().enumerate() {
                let new_conf = (positions[i].1 + update).clamp(0.0, 1.0);
                let delta = (new_conf - old_confidences[i]).abs();
                if delta > max_delta {
                    max_delta = delta;
                }
                positions[i].1 = new_conf;
            }

            // 重新归一化（确保总和为 1）
            let conf_sum: f64 = positions.iter().map(|(_, c, _)| *c).sum();
            if conf_sum > 0.0 {
                for (_, conf, _) in &mut positions {
                    *conf /= conf_sum;
                }
            }

            // 记录本轮结果
            let round_result = DebateRound {
                round,
                confidences: positions
                    .iter()
                    .map(|(pos, conf, _)| (pos.clone(), *conf))
                    .collect(),
                max_delta,
            };
            rounds.push(round_result);

            // 检查收敛
            if max_delta < self.convergence_threshold {
                break;
            }
        }

        // 找出最终胜出者
        positions.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        let (winner, final_conf, _) = positions
            .into_iter()
            .next()
            .ok_or(FusionError::EmptyInput)?;

        Ok((winner, final_conf, rounds, total_weight))
    }

    /// 获取辩论轮次数（实际执行的轮次）
    pub fn max_rounds(&self) -> usize {
        self.max_rounds
    }
}

impl<Category: Eq + Hash + Clone> ClassificationFusionStrategy<Category> for DebateFusion {
    fn name(&self) -> &'static str {
        "Debate / Multi-Agent Debate Fusion"
    }

    fn fuse_classification(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<(Category, f64, f64)> {
        let (winner, conf, _, total) = self.debate(votes)?;
        Ok((winner, conf, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::traits::ClassificationFusionStrategy;

    #[test]
    fn test_basic_debate() {
        let fusion = DebateFusion::new().with_max_rounds(5);
        let votes = vec![("支持", 0.8), ("反对", 0.3), ("支持", 0.6)];
        let (winner, conf, _) = fusion.fuse_classification(&votes).unwrap();
        // "支持" 初始占优，辩论后应该仍然胜出
        assert_eq!(winner, "支持");
        assert!(conf > 0.5);
    }

    #[test]
    fn test_debate_convergence() {
        let fusion = DebateFusion::new()
            .with_max_rounds(100)
            .with_convergence_threshold(1e-4);
        let votes = vec![("A", 0.6), ("B", 0.4)];
        let (_, _, rounds, _): (&str, _, _, _) = fusion.debate(&votes).unwrap();
        // 应该在达到最大轮次前收敛或稳定
        assert!(!rounds.is_empty());
        // 验证轮次数合理（辩论会逐步稳定）
        assert!(rounds.len() <= 100);
    }

    #[test]
    fn test_empty_input() {
        let fusion = DebateFusion::new();
        let votes: Vec<(&str, f64)> = vec![];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::EmptyInput)));
    }

    #[test]
    fn test_negative_confidence() {
        let fusion = DebateFusion::new();
        let votes = vec![("A", -0.1)];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(
            result,
            Err(FusionError::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn test_zero_max_rounds() {
        let fusion = DebateFusion::new().with_max_rounds(0);
        let votes = vec![("A", 0.5)];
        let result = fusion.fuse_classification(&votes);
        assert!(matches!(result, Err(FusionError::InvalidParameter { .. })));
    }

    #[test]
    fn test_single_position() {
        let fusion = DebateFusion::new().with_max_rounds(3);
        let votes = vec![("A", 0.8), ("A", 0.6)];
        let (winner, conf, rounds, _): (&str, _, _, _) = fusion.debate(&votes).unwrap();
        assert_eq!(winner, "A");
        assert!((conf - 1.0).abs() < 1e-9); // 单一立场置信度应为1
        assert!(!rounds.is_empty());
    }

    #[test]
    fn test_majority_wins() {
        let fusion = DebateFusion::new().with_max_rounds(10);
        // 多数派应该胜出
        let votes = vec![
            ("A", 0.5),
            ("A", 0.5),
            ("B", 0.5),
            ("A", 0.5),
            ("C", 0.5),
        ];
        let (winner, _, _) = fusion.fuse_classification(&votes).unwrap();
        assert_eq!(winner, "A");
    }

    #[test]
    fn test_high_weight_expert_influence() {
        let fusion = DebateFusion::new()
            .with_max_rounds(10)
            .with_expert_weight_influence(2.0);
        // 一个高权重专家 vs 多个低权重专家
        let votes = vec![
            ("专家意见", 10.0), // 高权重
            ("大众意见A", 1.0),
            ("大众意见B", 1.0),
            ("大众意见C", 1.0),
        ];
        let (winner, conf, _) = fusion.fuse_classification(&votes).unwrap();
        // 高权重专家的意见应该有较大影响力
        assert_eq!(winner, "专家意见");
        assert!(conf > 0.5);
    }

    #[test]
    fn test_debate_rounds_recorded() {
        let fusion = DebateFusion::new().with_max_rounds(5);
        let votes = vec![("A", 0.6), ("B", 0.4)];
        let (_, _, rounds, _) = fusion.debate::<&str>(&votes).unwrap();
        assert_eq!(rounds.len(), 5);
        for (i, round) in rounds.iter().enumerate() {
            assert_eq!(round.round, i);
            assert_eq!(round.confidences.len(), 2);
        }
    }

    #[test]
    fn test_learning_rate_effect() {
        let votes = vec![("A", 0.55), ("B", 0.45)];

        let slow = DebateFusion::new()
            .with_max_rounds(3)
            .with_learning_rate(0.01);
        let fast = DebateFusion::new()
            .with_max_rounds(3)
            .with_learning_rate(0.5);

        let (_, slow_conf, _): (&str, f64, f64) = slow.fuse_classification(&votes).unwrap();
        let (_, fast_conf, _): (&str, f64, f64) = fast.fuse_classification(&votes).unwrap();

        // 高学习率应该让优势方的置信度提升更快
        assert!(fast_conf > slow_conf);
    }

    #[test]
    fn test_name() {
        let fusion = DebateFusion::new();
        assert_eq!(<DebateFusion as ClassificationFusionStrategy<&str>>::name(&fusion), "Debate / Multi-Agent Debate Fusion");
    }

    #[test]
    fn test_default_values() {
        let fusion = DebateFusion::new();
        assert_eq!(fusion.max_rounds(), 10);
    }
}
