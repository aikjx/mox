// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 结果融合算法
//!
//! 提供多种结果融合策略的纯算法实现，覆盖六大核心融合策略：
//!
//! # 六大融合策略
//! - **加权投票融合 (Weighted Voting)** — 按权重投票，适用于分类/决策任务
//! - **置信度加权融合 (Confidence Weighting)** — 基于置信度的加权平均，适用于数值结果
//! - **堆叠融合 (Stacking / Meta-Learner)** — 元学习器组合多模型输出，适用于复杂任务
//! - **辩论融合 (Debate / Multi-Agent Debate)** — 多智能体辩论裁决，适用于争议性问题
//! - **Map-Reduce 融合 (Map-Reduce Fusion)** — 分治式融合，适用于大规模数据
//! - **迭代精炼融合 (Iterative Refinement)** — 多轮迭代优化，适用于需逐步求精的任务
//!
//! # 基础融合函数（保留向后兼容）
//! - RRF (Reciprocal Rank Fusion) — 多路召回融合
//! - 加权融合
//! - 投票融合
//! - 择优融合
//! - 文本拼接融合
//! - JSON 深度合并
//!
//! 所有函数都是纯函数，无 IO，可独立单测。

pub mod error;
pub mod traits;
pub mod strategies;
pub mod engine;

// ─── 重导出（方便下游使用） ────────────────────────────────────────────────

pub use error::{FusionError, FusionResult};
pub use traits::FusionStrategy;
pub use engine::FusionEngine;
pub use strategies::*;

// ─── 基础融合函数（保留向后兼容） ──────────────────────────────────────────

use serde_json::{json, Value};

/// RRF（Reciprocal Rank Fusion）融合
///
/// 将多路排序结果融合为一个排序。
/// formula: score(d) = sum(1 / (k + rank_i(d)))
///
/// # Arguments
/// * `ranked_lists` - 多路排序列表，每路是元素 ID 的有序数组
/// * `k` - RRF 常数，通常取 60
///
/// # Returns
/// 按融合分数降序排列的元素 ID 列表
pub fn rrf_fusion(ranked_lists: &[Vec<String>], k: usize) -> Vec<(String, f64)> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();

    for list in ranked_lists {
        for (rank, item) in list.iter().enumerate() {
            let score = 1.0 / (k as f64 + rank as f64 + 1.0);
            *scores.entry(item.clone()).or_insert(0.0) += score;
        }
    }

    let mut result: Vec<(String, f64)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// 加权融合
///
/// 将多个带权重的结果融合为一个最终结果。
/// 适用于数值型结果的加权平均。
///
/// # Arguments
/// * `weighted_results` - (结果值, 权重) 列表
///
/// # Returns
/// 加权平均后的结果
pub fn weighted_fusion(values: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = values.iter().map(|(_, w)| w).sum();
    if total_weight == 0.0 {
        return 0.0;
    }
    let weighted_sum: f64 = values.iter().map(|(v, w)| v * w).sum();
    weighted_sum / total_weight
}

/// 投票融合（多数决）
///
/// 从多个候选项中选出得票最多的。
/// 适用于分类/选择类结果。
///
/// # Arguments
/// * `votes` - 投票列表，每个元素是一个选项
///
/// # Returns
/// (胜出选项, 得票数, 总票数)
pub fn voting_fusion<T: Eq + std::hash::Hash + Clone>(votes: &[T]) -> Option<(T, usize, usize)> {
    use std::collections::HashMap;

    if votes.is_empty() {
        return None;
    }

    let mut counts: HashMap<&T, usize> = HashMap::new();
    for vote in votes {
        *counts.entry(vote).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(item, count)| (item.clone(), count, votes.len()))
}

/// 择优融合
///
/// 从多个结果中选择质量最高的。
/// 适用于有明确质量评分的结果。
///
/// # Arguments
/// * `scored_results` - (结果, 质量分) 列表
///
/// # Returns
/// 质量最高的结果及其分数
pub fn best_of_fusion<T: Clone>(scored_results: &[(T, f64)]) -> Option<(T, f64)> {
    scored_results
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(result, score)| (result.clone(), *score))
}

/// 文本结果拼接融合
///
/// 将多个文本结果按顺序拼接为一个结果。
/// 适用于报告类结果的合并。
pub fn concatenate_fusion(texts: &[String], separator: &str) -> String {
    texts.join(separator)
}

/// JSON 结果深度合并
///
/// 将多个 JSON 对象深度合并，后面的覆盖前面的。
/// 数组类型直接拼接。
pub fn merge_json_values(values: &[Value]) -> Value {
    if values.is_empty() {
        return json!({});
    }
    if values.len() == 1 {
        return values[0].clone();
    }

    let mut result = values[0].clone();
    for v in &values[1..] {
        merge_json_into(&mut result, v);
    }
    result
}

fn merge_json_into(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_obj), Value::Object(overlay_obj)) => {
            for (key, val) in overlay_obj {
                if let Some(base_val) = base_obj.get_mut(key) {
                    merge_json_into(base_val, val);
                } else {
                    base_obj.insert(key.clone(), val.clone());
                }
            }
        }
        (Value::Array(base_arr), Value::Array(overlay_arr)) => {
            base_arr.extend(overlay_arr.iter().cloned());
        }
        (base_val, overlay_val) => {
            *base_val = overlay_val.clone();
        }
    }
}

// ─── 单元测试（基础融合函数） ──────────────────────────────────────────────

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_rrf_fusion() {
        let list1 = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let list2 = vec!["B".to_string(), "A".to_string(), "D".to_string()];

        let result = rrf_fusion(&[list1, list2], 60);
        assert!(!result.is_empty());
        // A 和 B 都出现两次，应该排在前面
        assert!(result[0].0 == "A" || result[0].0 == "B");
    }

    #[test]
    fn test_weighted_fusion() {
        let values = vec![(80.0, 0.3), (90.0, 0.5), (70.0, 0.2)];
        let result = weighted_fusion(&values);
        // 80*0.3 + 90*0.5 + 70*0.2 = 24 + 45 + 14 = 83
        assert!((result - 83.0).abs() < 0.001);
    }

    #[test]
    fn test_voting_fusion() {
        let votes = vec!["A", "B", "A", "C", "A"];
        let (winner, count, total) = voting_fusion(&votes).unwrap();
        assert_eq!(winner, "A");
        assert_eq!(count, 3);
        assert_eq!(total, 5);
    }

    #[test]
    fn test_best_of_fusion() {
        let results = vec![
            ("result_a".to_string(), 0.7),
            ("result_b".to_string(), 0.95),
            ("result_c".to_string(), 0.8),
        ];
        let (best, score) = best_of_fusion(&results).unwrap();
        assert_eq!(best, "result_b");
        assert!((score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_concatenate_fusion() {
        let texts = vec!["Hello".to_string(), "World".to_string(), "!".to_string()];
        let result = concatenate_fusion(&texts, " ");
        assert_eq!(result, "Hello World !");
    }

    #[test]
    fn test_merge_json_values() {
        let v1 = json!({"a": 1, "b": {"c": 2}});
        let v2 = json!({"b": {"d": 3}, "e": 4});
        let result = merge_json_values(&[v1, v2]);
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"]["c"], 2);
        assert_eq!(result["b"]["d"], 3);
        assert_eq!(result["e"], 4);
    }
}
