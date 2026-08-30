// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 排序与评分算法模块
//!
//! 跨域共享的排名算法：
//! - **加权评分排序**：多因子加权（专家匹配 / 文档推荐）
//! - **Borda 计数融合**：多排名结果融合（多专家推荐融合）
//! - **加权多数投票**：多意见投票融合（专家决策）
//!
//! 三大业务域共享：
//! - KG 域：节点重要性排名、推荐结果排序
//! - EA 域：专家匹配排名、多专家意见融合
//! - Cloud 域：搜索结果排序、文档推荐排序

use crate::traits::*;
use crate::types::*;
use std::collections::HashMap;

// ============================================================================
// 加权评分排序
// ============================================================================

/// 加权评分排序算法
///
/// 支持配置多个评分因子及其权重，对条目进行综合评分排名。
/// 广泛应用于专家匹配、文档推荐、搜索排序等场景。
#[derive(Debug, Clone)]
pub struct WeightedRanker {
    /// 评分因子配置：(因子名称, 权重)
    pub factors: Vec<(String, f64)>,
    /// 是否归一化得分到 0~1
    pub normalize: bool,
}

impl Default for WeightedRanker {
    fn default() -> Self {
        Self {
            factors: vec![],
            normalize: true,
        }
    }
}

impl WeightedRanker {
    pub fn new(factors: Vec<(String, f64)>) -> Self {
        Self {
            factors,
            normalize: true,
        }
    }

    /// 对条目进行加权评分排名
    ///
    /// - `items`: 条目列表，每个条目包含各因子的原始得分
    /// - 返回: 按综合得分降序排列的排名结果
    pub fn rank<K: Clone + serde::Serialize>(
        &self,
        items: &[(K, HashMap<String, f64>)],
    ) -> RankingResult<K> {
        let mut scored: Vec<ScoredItem<K>> = items
            .iter()
            .map(|(key, scores)| {
                let mut total = 0.0;
                let mut weight_sum = 0.0;
                let mut breakdown = HashMap::new();

                for (factor_name, weight) in &self.factors {
                    if let Some(&score) = scores.get(factor_name) {
                        let weighted = score * weight;
                        total += weighted;
                        weight_sum += weight;
                        breakdown.insert(factor_name.clone(), weighted);
                    }
                }

                let final_score = if weight_sum > 0.0 && self.normalize {
                    total / weight_sum
                } else {
                    total
                };

                ScoredItem {
                    key: key.clone(),
                    score: final_score,
                    rank: 0, // 后面统一填充
                    confidence: weight_sum,
                    score_breakdown: Some(breakdown),
                }
            })
            .collect();

        // 按得分降序排序
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 填充排名
        for (i, item) in scored.iter_mut().enumerate() {
            item.rank = i + 1;
        }

        let total = scored.len();
        RankingResult {
            items: scored,
            total,
            method: "weighted".to_string(),
        }
    }
}

impl Algorithm for WeightedRanker {
    fn id(&self) -> &str {
        "rank.weighted"
    }
    fn name(&self) -> &str {
        "加权评分排序"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "多因子加权评分排序，支持动态权重调整和得分分解"
    }
}

// ============================================================================
// Borda 计数融合排序
// ============================================================================

/// Borda Count 融合排序算法
///
/// 将多个排名列表融合为一个最终排名。
/// 每个排名列表中的条目获得 (N - rank) 分，总分最高者排名最前。
/// 适用于多专家推荐融合、多检索结果融合等场景。
#[derive(Debug, Clone, Default)]
pub struct BordaFusion;

impl BordaFusion {
    /// 融合多个排名列表
    ///
    /// - `rankings`: 多个排名列表，每个列表是按排名顺序的条目 key
    /// - 返回: 融合后的排名结果
    pub fn fuse<K: Clone + serde::Serialize + Eq + std::hash::Hash>(
        &self,
        rankings: &[Vec<K>],
    ) -> RankingResult<K> {
        let mut scores: HashMap<K, f64> = HashMap::new();
        let n_rankings = rankings.len() as f64;

        for ranking in rankings {
            let n = ranking.len();
            for (i, item) in ranking.iter().enumerate() {
                let borda_score = (n - i) as f64;
                *scores.entry(item.clone()).or_insert(0.0) += borda_score;
            }
        }

        // 归一化：除以最大可能得分 (n_rankings * max_n)
        let max_possible = rankings.iter().map(|r| r.len() as f64).sum::<f64>();

        let mut items: Vec<ScoredItem<K>> = scores
            .into_iter()
            .map(|(key, score)| ScoredItem {
                key,
                score: if max_possible > 0.0 {
                    score / max_possible
                } else {
                    0.0
                },
                rank: 0,
                confidence: n_rankings as f64,
                score_breakdown: None,
            })
            .collect();

        items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        for (i, item) in items.iter_mut().enumerate() {
            item.rank = i + 1;
        }

        let total = items.len();
        RankingResult {
            items,
            total,
            method: "borda_fusion".to_string(),
        }
    }
}

impl Algorithm for BordaFusion {
    fn id(&self) -> &str {
        "rank.borda"
    }
    fn name(&self) -> &str {
        "Borda 计数融合排序"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "多排名融合的 Borda Count 方法，适用于多专家推荐融合"
    }
}

// ============================================================================
// 加权投票融合（专家决策）
// ============================================================================

/// 加权投票融合算法
///
/// 每个专家有一个权重，投票结果按权重加权计算最终得分。
/// 适用于多专家决策、多模型输出融合等场景。
#[derive(Debug, Clone)]
pub struct WeightedVotingFusion;

impl WeightedVotingFusion {
    /// 对分类/离散选项进行加权投票
    ///
    /// - `votes`: 投票列表 (投票者ID, 选项, 权重, 置信度)
    /// - 返回: 各选项的加权得分，按降序排列
    pub fn vote_options<T: Clone + serde::Serialize + Eq + std::hash::Hash>(
        &self,
        votes: &[(String, T, f64, f64)],
    ) -> Vec<(T, f64)> {
        let mut scores: HashMap<T, f64> = HashMap::new();

        for (_voter, option, weight, confidence) in votes {
            let weighted = weight * confidence;
            *scores.entry(option.clone()).or_insert(0.0) += weighted;
        }

        let mut result: Vec<(T, f64)> = scores.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// 对连续值进行加权平均融合
    pub fn fuse_continuous(&self, values: &[(String, f64, f64, f64)]) -> f64 {
        let mut total = 0.0;
        let mut weight_sum = 0.0;

        for (_voter, value, weight, confidence) in values {
            let w = weight * confidence;
            total += value * w;
            weight_sum += w;
        }

        if weight_sum > 0.0 {
            total / weight_sum
        } else {
            0.0
        }
    }
}

impl Algorithm for WeightedVotingFusion {
    fn id(&self) -> &str {
        "fusion.weighted_vote"
    }
    fn name(&self) -> &str {
        "加权投票融合"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "基于权重和置信度的加权投票融合算法"
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_ranker() {
        let ranker = WeightedRanker::new(vec![
            ("skill".to_string(), 0.5),
            ("experience".to_string(), 0.3),
            ("availability".to_string(), 0.2),
        ]);

        let items: Vec<(String, HashMap<String, f64>)> = vec![
            (
                "expert_a".to_string(),
                HashMap::from([
                    ("skill".to_string(), 0.9),
                    ("experience".to_string(), 0.7),
                    ("availability".to_string(), 0.8),
                ]),
            ),
            (
                "expert_b".to_string(),
                HashMap::from([
                    ("skill".to_string(), 0.7),
                    ("experience".to_string(), 0.9),
                    ("availability".to_string(), 0.6),
                ]),
            ),
        ];

        let result = ranker.rank(&items);
        assert_eq!(result.total, 2);
        assert_eq!(result.items[0].rank, 1);
        assert_eq!(result.items[1].rank, 2);

        // expert_a: 0.9*0.5 + 0.7*0.3 + 0.8*0.2 = 0.45 + 0.21 + 0.16 = 0.82
        // expert_b: 0.7*0.5 + 0.9*0.3 + 0.6*0.2 = 0.35 + 0.27 + 0.12 = 0.74
        assert!((result.items[0].score - 0.82).abs() < 1e-6);
        assert_eq!(result.items[0].key, "expert_a");
        assert!((result.items[1].score - 0.74).abs() < 1e-6);
    }

    #[test]
    fn test_borda_fusion() {
        let fusion = BordaFusion;
        let rankings: Vec<Vec<String>> = vec![
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["B".to_string(), "A".to_string(), "C".to_string()],
            vec!["A".to_string(), "C".to_string(), "B".to_string()],
        ];

        let result = fusion.fuse(&rankings);
        // A: 3+2+3 = 8, B: 2+3+1 = 6, C: 1+1+2 = 4
        // max_possible = 3+3+3 = 9 (每个排名列表的长度)
        assert_eq!(result.items[0].key, "A");
        assert_eq!(result.items[1].key, "B");
        assert_eq!(result.items[2].key, "C");
    }

    #[test]
    fn test_weighted_voting() {
        let fusion = WeightedVotingFusion;
        let votes = vec![
            ("expert1".to_string(), "option_a".to_string(), 0.8, 0.9),
            ("expert2".to_string(), "option_b".to_string(), 0.6, 0.95),
            ("expert3".to_string(), "option_a".to_string(), 0.7, 0.85),
        ];

        let result = fusion.vote_options(&votes);
        // option_a: 0.8*0.9 + 0.7*0.85 = 0.72 + 0.595 = 1.315
        // option_b: 0.6*0.95 = 0.57
        assert_eq!(result[0].0, "option_a");
        assert!((result[0].1 - 1.315).abs() < 1e-6);
    }

    #[test]
    fn test_fuse_continuous() {
        let fusion = WeightedVotingFusion;
        let values = vec![
            ("expert1".to_string(), 80.0, 0.8, 0.9),
            ("expert2".to_string(), 90.0, 0.6, 0.95),
            ("expert3".to_string(), 70.0, 0.7, 0.85),
        ];

        let result = fusion.fuse_continuous(&values);
        // (80*0.72 + 90*0.57 + 70*0.595) / (0.72 + 0.57 + 0.595)
        // = (57.6 + 51.3 + 41.65) / 1.885
        // = 150.55 / 1.885 ≈ 79.87
        assert!((result - 79.87).abs() < 0.1);
    }
}
