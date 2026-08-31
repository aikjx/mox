// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 结果融合引擎
//!
//! 实现多专家协作结果的融合策略，包括：
//! - 加权投票融合
//! - 置信度加权融合
//! - 择优融合
//! - 辩论融合
//! - 迭代精炼融合
//!
//! 每个融合策略可以独立配置，支持动态切换。

use mox_alliance_common_proto::{AllianceResult, FusionStrategy};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

use crate::dag_engine::NodeExecutionResult;

/// 融合输入
#[derive(Debug, Clone)]
pub struct FusionInput {
    /// 节点执行结果列表
    pub results: Vec<NodeExecutionResult>,
    /// 各专家的权重 (expert_id -> weight)
    pub expert_weights: HashMap<String, f64>,
    /// 融合策略
    pub strategy: FusionStrategy,
    /// 任务描述（用于上下文）
    pub task_description: String,
}

/// 融合输出
#[derive(Debug, Clone)]
pub struct FusionOutput {
    /// 融合后的结果内容
    pub content: Value,
    /// 融合置信度
    pub confidence: f64,
    /// 参与融合的专家数量
    pub expert_count: usize,
    /// 使用的融合策略
    pub strategy: FusionStrategy,
    /// 各专家的贡献度
    pub contributions: HashMap<String, f64>,
    /// 融合摘要说明
    pub summary: String,
}

/// 结果融合引擎
pub struct FusionEngine {
    /// 默认融合策略
    default_strategy: FusionStrategy,
}

impl FusionEngine {
    /// 创建融合引擎
    pub fn new() -> Self {
        Self {
            default_strategy: FusionStrategy::Weighted,
        }
    }

    /// 配置默认融合策略
    pub fn with_default_strategy(mut self, strategy: FusionStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    /// 执行结果融合
    pub fn fuse(&self, input: FusionInput) -> AllianceResult<FusionOutput> {
        let strategy = input.strategy;
        debug!(
            "Fusing {} expert results with strategy {:?}",
            input.results.len(),
            strategy
        );

        // 过滤掉失败的结果
        let successful_results: Vec<&NodeExecutionResult> =
            input.results.iter().filter(|r| r.success).collect();

        if successful_results.is_empty() {
            return Ok(FusionOutput {
                content: Value::Null,
                confidence: 0.0,
                expert_count: 0,
                strategy,
                contributions: HashMap::new(),
                summary: "No successful results to fuse".to_string(),
            });
        }

        let output = match strategy {
            FusionStrategy::Voting => Self::fuse_voting(&successful_results, &input.expert_weights),
            FusionStrategy::Weighted => {
                Self::fuse_weighted(&successful_results, &input.expert_weights)
            }
            FusionStrategy::ConfidenceWeighted => {
                Self::fuse_confidence_weighted(&successful_results, &input.expert_weights)
            }
            FusionStrategy::BestOf => {
                Self::fuse_best_of(&successful_results, &input.expert_weights)
            }
            FusionStrategy::Concatenation => {
                Self::fuse_concatenation(&successful_results, &input.expert_weights)
            }
            FusionStrategy::Debate => {
                Self::fuse_debate(&successful_results, &input.expert_weights)
            }
            FusionStrategy::Iterative => {
                Self::fuse_iterative(&successful_results, &input.expert_weights)
            }
            FusionStrategy::MapReduce => {
                Self::fuse_map_reduce(&successful_results, &input.expert_weights)
            }
            FusionStrategy::Stacking => {
                Self::fuse_stacking(&successful_results, &input.expert_weights)
            }
        };

        Ok(output)
    }

    // === 融合策略实现 ===

    /// 加权投票融合
    ///
    /// 适用于分类/决策类任务，每个专家"投票"，按权重计票。
    fn fuse_voting(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        // 简化实现：统计各结果的"方向"，按权重投票
        let mut votes: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0;

        for result in results {
            let weight = weights
                .get(&result.node_id)
                .copied()
                .unwrap_or(1.0);
            total_weight += weight;

            // 使用输出摘要作为投票选项（简化版）
            if let Some(ref summary) = result.output_summary {
                // 提取关键词作为投票选项
                let key = summary.clone();
                *votes.entry(key).or_insert(0.0) += weight;
            }
        }

        // 找出得票最高的选项
        let (winner, max_votes) = votes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.clone(), *v))
            .unwrap_or(("none".to_string(), 0.0));

        let confidence = if total_weight > 0.0 {
            max_votes / total_weight
        } else {
            0.0
        };

        FusionOutput {
            content: serde_json::json!({
                "winner": winner,
                "votes": votes,
                "total_weight": total_weight,
            }),
            confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Voting,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Voting fusion: {} experts, winner confidence {:.1}%",
                results.len(),
                confidence * 100.0
            ),
        }
    }

    /// 加权融合
    ///
    /// 适用于数值型结果，按权重加权平均。
    fn fuse_weighted(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        let mut total_weight = 0.0;
        let mut weighted_confidence = 0.0;

        for result in results {
            let weight = weights
                .get(&result.node_id)
                .copied()
                .unwrap_or(1.0);
            total_weight += weight;
            weighted_confidence += result.confidence.unwrap_or(0.5) * weight;
        }

        let avg_confidence = if total_weight > 0.0 {
            weighted_confidence / total_weight
        } else {
            0.0
        };

        // 收集所有结果摘要
        let summaries: Vec<String> = results
            .iter()
            .filter_map(|r| r.output_summary.clone())
            .collect();

        FusionOutput {
            content: serde_json::json!({
                "results": summaries,
                "weighted_confidence": avg_confidence,
                "total_weight": total_weight,
            }),
            confidence: avg_confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Weighted,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Weighted fusion: {} experts, avg confidence {:.1}%",
                results.len(),
                avg_confidence * 100.0
            ),
        }
    }

    /// 置信度加权融合
    ///
    /// 基于各专家输出的动态置信度进行加权，置信度高的专家权重更大。
    fn fuse_confidence_weighted(
        results: &[&NodeExecutionResult],
        base_weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        let mut total_adjusted_weight = 0.0;
        let mut weighted_confidence = 0.0;
        let mut adjusted_weights: HashMap<String, f64> = HashMap::new();

        for result in results {
            let base_weight = base_weights
                .get(&result.node_id)
                .copied()
                .unwrap_or(1.0);
            let confidence = result.confidence.unwrap_or(0.5);

            // 调整后的权重 = 基础权重 * 置信度
            let adjusted_weight = base_weight * confidence;
            adjusted_weights.insert(result.node_id.clone(), adjusted_weight);

            total_adjusted_weight += adjusted_weight;
            weighted_confidence += confidence * adjusted_weight;
        }

        let final_confidence = if total_adjusted_weight > 0.0 {
            weighted_confidence / total_adjusted_weight
        } else {
            0.0
        };

        let summaries: Vec<String> = results
            .iter()
            .filter_map(|r| r.output_summary.clone())
            .collect();

        FusionOutput {
            content: serde_json::json!({
                "results": summaries,
                "confidence_weighted": final_confidence,
                "adjusted_weights": adjusted_weights,
            }),
            confidence: final_confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::ConfidenceWeighted,
            contributions: Self::calculate_contributions(results, &adjusted_weights),
            summary: format!(
                "Confidence-weighted fusion: {} experts, final confidence {:.1}%",
                results.len(),
                final_confidence * 100.0
            ),
        }
    }

    /// 择优融合
    ///
    /// 选择置信度最高的专家结果作为最终输出。
    fn fuse_best_of(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        let best = results
            .iter()
            .max_by(|a, b| {
                let score_a =
                    a.confidence.unwrap_or(0.5) * weights.get(&a.node_id).copied().unwrap_or(1.0);
                let score_b =
                    b.confidence.unwrap_or(0.5) * weights.get(&b.node_id).copied().unwrap_or(1.0);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap();

        let confidence = best.confidence.unwrap_or(0.5);

        let mut contributions = HashMap::new();
        contributions.insert(best.node_id.clone(), 1.0);

        FusionOutput {
            content: serde_json::json!({
                "best_expert": best.node_id,
                "output": best.output_summary,
                "confidence": confidence,
            }),
            confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::BestOf,
            contributions,
            summary: format!(
                "Best-of fusion: selected {} with confidence {:.1}%",
                best.node_id,
                confidence * 100.0
            ),
        }
    }

    /// 拼接融合
    ///
    /// 将所有专家的结果拼接在一起，适用于需要综合多方观点的场景。
    fn fuse_concatenation(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        let summaries: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "expert": r.node_id,
                    "output": r.output_summary,
                    "confidence": r.confidence.unwrap_or(0.5),
                    "weight": weights.get(&r.node_id).copied().unwrap_or(1.0),
                })
            })
            .collect();

        let avg_confidence: f64 = results
            .iter()
            .map(|r| r.confidence.unwrap_or(0.5))
            .sum::<f64>()
            / results.len() as f64;

        FusionOutput {
            content: serde_json::json!({
                "expert_outputs": summaries,
                "count": results.len(),
            }),
            confidence: avg_confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Concatenation,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Concatenation fusion: {} expert outputs combined",
                results.len()
            ),
        }
    }

    /// 辩论融合
    ///
    /// 模拟多轮辩论，最终由"裁判"裁决。
    fn fuse_debate(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        // 简化版：前两个作为正反方，最后一个作为裁判
        let (debaters, judge) = if results.len() >= 3 {
            (&results[..results.len() - 1], Some(results.last().unwrap()))
        } else {
            (results, None)
        };

        let debater_summaries: Vec<String> = debaters
            .iter()
            .filter_map(|r| r.output_summary.clone())
            .collect();

        let verdict = judge
            .and_then(|j| j.output_summary.clone())
            .unwrap_or_else(|| "No judge verdict".to_string());

        let confidence = if let Some(j) = judge {
            j.confidence.unwrap_or(0.5)
        } else {
            // 没有裁判时，取各辩论者的平均置信度
            debaters.iter().map(|r| r.confidence.unwrap_or(0.5)).sum::<f64>()
                / debaters.len() as f64
        };

        FusionOutput {
            content: serde_json::json!({
                "debaters": debater_summaries,
                "verdict": verdict,
                "judge": judge.map(|j| j.node_id.clone()),
            }),
            confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Debate,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Debate fusion: {} debaters, 1 judge",
                debaters.len()
            ),
        }
    }

    /// 迭代精炼融合
    ///
    /// 多轮迭代，逐步优化结果。
    fn fuse_iterative(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        // 简化版：按顺序取最后一个结果作为最终精炼结果
        let final_result = results.last().unwrap();
        let confidence = final_result.confidence.unwrap_or(0.5);

        let iterations: Vec<String> = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "Iteration {}: {} (confidence: {:.1}%)",
                    i + 1,
                    r.output_summary.as_deref().unwrap_or("N/A"),
                    r.confidence.unwrap_or(0.5) * 100.0
                )
            })
            .collect();

        FusionOutput {
            content: serde_json::json!({
                "iterations": iterations,
                "final_output": final_result.output_summary,
                "final_confidence": confidence,
                "total_iterations": results.len(),
            }),
            confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Iterative,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Iterative fusion: {} iterations, final confidence {:.1}%",
                results.len(),
                confidence * 100.0
            ),
        }
    }

    /// Map-Reduce 融合
    ///
    /// 分治式融合，适用于大规模任务。
    fn fuse_map_reduce(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        // 简化版：将结果分为"映射"和"归约"两部分
        let mid = results.len() / 2;
        let map_results = &results[..mid.max(1)];
        let reduce_results = &results[mid.max(1)..];

        let map_summaries: Vec<String> = map_results
            .iter()
            .filter_map(|r| r.output_summary.clone())
            .collect();

        let reduce_summaries: Vec<String> = reduce_results
            .iter()
            .filter_map(|r| r.output_summary.clone())
            .collect();

        let avg_confidence: f64 = results
            .iter()
            .map(|r| r.confidence.unwrap_or(0.5))
            .sum::<f64>()
            / results.len() as f64;

        FusionOutput {
            content: serde_json::json!({
                "map_phase": map_summaries,
                "reduce_phase": reduce_summaries,
                "map_count": map_results.len(),
                "reduce_count": reduce_results.len(),
            }),
            confidence: avg_confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::MapReduce,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Map-Reduce fusion: {} mappers, {} reducers",
                map_results.len(),
                reduce_results.len()
            ),
        }
    }

    /// 堆叠融合（元学习器）
    ///
    /// 使用元学习器组合多个模型的输出。
    fn fuse_stacking(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> FusionOutput {
        // 简化版：加权平均（实际中应该用元模型）
        let mut total_weight = 0.0;
        let mut weighted_confidence = 0.0;
        let mut base_predictions: Vec<serde_json::Value> = Vec::new();

        for result in results {
            let weight = weights
                .get(&result.node_id)
                .copied()
                .unwrap_or(1.0);
            total_weight += weight;
            weighted_confidence += result.confidence.unwrap_or(0.5) * weight;

            base_predictions.push(serde_json::json!({
                "expert": result.node_id,
                "confidence": result.confidence.unwrap_or(0.5),
                "weight": weight,
                "output": result.output_summary,
            }));
        }

        let meta_confidence = if total_weight > 0.0 {
            weighted_confidence / total_weight
        } else {
            0.0
        };

        FusionOutput {
            content: serde_json::json!({
                "base_learners": base_predictions,
                "meta_learner_output": {
                    "confidence": meta_confidence,
                    "method": "weighted_average",
                },
            }),
            confidence: meta_confidence,
            expert_count: results.len(),
            strategy: FusionStrategy::Stacking,
            contributions: Self::calculate_contributions(results, weights),
            summary: format!(
                "Stacking fusion: {} base learners, meta confidence {:.1}%",
                results.len(),
                meta_confidence * 100.0
            ),
        }
    }

    // === 辅助方法 ===

    /// 计算各专家的贡献度（归一化权重）
    fn calculate_contributions(
        results: &[&NodeExecutionResult],
        weights: &HashMap<String, f64>,
    ) -> HashMap<String, f64> {
        let total_weight: f64 = results
            .iter()
            .map(|r| weights.get(&r.node_id).copied().unwrap_or(1.0))
            .sum();

        let mut contributions = HashMap::new();
        for result in results {
            let weight = weights
                .get(&result.node_id)
                .copied()
                .unwrap_or(1.0);
            let contribution = if total_weight > 0.0 {
                weight / total_weight
            } else {
                1.0 / results.len() as f64
            };
            contributions.insert(result.node_id.clone(), contribution);
        }

        contributions
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_result(node_id: &str, confidence: f64, summary: &str) -> NodeExecutionResult {
        NodeExecutionResult {
            node_id: node_id.to_string(),
            success: true,
            output_ref: Some(format!("output-{}", node_id)),
            duration_ms: 100,
            error_message: None,
            output_summary: Some(summary.to_string()),
            confidence: Some(confidence),
        }
    }

    #[test]
    fn weighted_fusion_correctness() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("expert-a", 0.8, "Result A"),
            make_test_result("expert-b", 0.6, "Result B"),
        ];

        let mut weights = HashMap::new();
        weights.insert("expert-a".to_string(), 2.0);
        weights.insert("expert-b".to_string(), 1.0);

        let input = FusionInput {
            results: results.clone(),
            expert_weights: weights,
            strategy: FusionStrategy::Weighted,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        assert_eq!(output.expert_count, 2);

        // 加权平均: (0.8 * 2 + 0.6 * 1) / (2 + 1) = 2.2 / 3 ≈ 0.733
        assert!((output.confidence - 0.733).abs() < 0.01);
    }

    #[test]
    fn best_of_selects_highest_confidence() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("expert-low", 0.5, "Low confidence result"),
            make_test_result("expert-high", 0.9, "High confidence result"),
            make_test_result("expert-mid", 0.7, "Medium confidence result"),
        ];

        let weights = HashMap::new(); // 等权重

        let input = FusionInput {
            results: results.clone(),
            expert_weights: weights,
            strategy: FusionStrategy::BestOf,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        assert_eq!(output.expert_count, 3);
        assert_eq!(output.confidence, 0.9);
    }

    #[test]
    fn confidence_weighted_adjusts_weights() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("expert-a", 0.9, "High confidence"), // 调整权重 = 1.0 * 0.9 = 0.9
            make_test_result("expert-b", 0.5, "Low confidence"),  // 调整权重 = 2.0 * 0.5 = 1.0
        ];

        let mut weights = HashMap::new();
        weights.insert("expert-a".to_string(), 1.0);
        weights.insert("expert-b".to_string(), 2.0);

        let input = FusionInput {
            results: results.clone(),
            expert_weights: weights,
            strategy: FusionStrategy::ConfidenceWeighted,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        assert_eq!(output.expert_count, 2);
        // 最终置信度应该在 0.5 和 0.9 之间
        assert!(output.confidence > 0.5);
        assert!(output.confidence < 0.9);
    }

    #[test]
    fn empty_results_returns_zero_confidence() {
        let engine = FusionEngine::new();

        let input = FusionInput {
            results: vec![],
            expert_weights: HashMap::new(),
            strategy: FusionStrategy::Weighted,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        assert_eq!(output.expert_count, 0);
        assert_eq!(output.confidence, 0.0);
    }

    #[test]
    fn concatenation_includes_all_results() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("e1", 0.8, "Result 1"),
            make_test_result("e2", 0.7, "Result 2"),
            make_test_result("e3", 0.9, "Result 3"),
        ];

        let input = FusionInput {
            results: results.clone(),
            expert_weights: HashMap::new(),
            strategy: FusionStrategy::Concatenation,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        assert_eq!(output.expert_count, 3);

        // 验证内容中包含所有 3 个结果
        let content = output.content;
        let expert_outputs = content["expert_outputs"].as_array().unwrap();
        assert_eq!(expert_outputs.len(), 3);
    }

    #[test]
    fn contributions_sum_to_one() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("e1", 0.8, "Result 1"),
            make_test_result("e2", 0.6, "Result 2"),
            make_test_result("e3", 0.7, "Result 3"),
        ];

        let mut weights = HashMap::new();
        weights.insert("e1".to_string(), 2.0);
        weights.insert("e2".to_string(), 1.0);
        weights.insert("e3".to_string(), 1.0);

        let input = FusionInput {
            results: results.clone(),
            expert_weights: weights,
            strategy: FusionStrategy::Weighted,
            task_description: "test".to_string(),
        };

        let output = engine.fuse(input).unwrap();
        let total_contribution: f64 = output.contributions.values().sum();
        assert!((total_contribution - 1.0).abs() < 0.001);
    }

    #[test]
    fn all_strategies_produce_valid_output() {
        let engine = FusionEngine::new();

        let results = vec![
            make_test_result("e1", 0.8, "Result 1"),
            make_test_result("e2", 0.7, "Result 2"),
            make_test_result("e3", 0.9, "Result 3"),
        ];

        let strategies = vec![
            FusionStrategy::Voting,
            FusionStrategy::Weighted,
            FusionStrategy::ConfidenceWeighted,
            FusionStrategy::BestOf,
            FusionStrategy::Concatenation,
            FusionStrategy::Debate,
            FusionStrategy::Iterative,
            FusionStrategy::MapReduce,
            FusionStrategy::Stacking,
        ];

        for strategy in strategies {
            let input = FusionInput {
                results: results.clone(),
                expert_weights: HashMap::new(),
                strategy,
                task_description: "test".to_string(),
            };

            let output = engine.fuse(input).unwrap();
            assert_eq!(output.expert_count, 3);
            assert!(output.confidence >= 0.0);
            assert!(output.confidence <= 1.0);
            assert_eq!(output.strategy, strategy);
        }
    }
}
