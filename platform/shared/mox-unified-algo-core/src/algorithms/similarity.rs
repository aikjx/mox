// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 相似度计算算法模块
//!
//! 跨域共享的相似度算法集合：
//! - **余弦相似度**：向量空间模型（专家画像匹配 / 文档向量检索）
//! - **Jaccard 相似度**：集合相似度（标签重叠 / 共同邻居）
//! - **欧氏距离**：向量空间距离（转相似度）
//! - **编辑距离**：字符串相似度（名称匹配 / 模糊搜索）
//!
//! 三大业务域共享：
//! - KG 域：节点相似度、子图匹配
//! - EA 域：专家-任务匹配、专家画像相似度
//! - Cloud 域：文档向量检索、标签推荐

use crate::traits::*;
use crate::types::*;
use crate::SIMILARITY_PRECISION;

// ============================================================================
// 余弦相似度计算器
// ============================================================================

/// 余弦相似度算法
#[derive(Debug, Clone)]
pub struct CosineSimilarity {
    precision: f64,
}

impl Default for CosineSimilarity {
    fn default() -> Self {
        Self {
            precision: SIMILARITY_PRECISION,
        }
    }
}

impl Algorithm for CosineSimilarity {
    fn id(&self) -> &str {
        "sim.cosine"
    }
    fn name(&self) -> &str {
        "余弦相似度"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "向量余弦相似度计算，支持稠密向量和稀疏向量"
    }
}

impl VectorSimilarity for CosineSimilarity {}

impl SimilarityAlgo<Vec<f64>> for CosineSimilarity {
    fn similarity(&self, a: &Vec<f64>, b: &Vec<f64>) -> f64 {
        self.cosine_similarity(a, b)
    }
}

impl SimilarityAlgo<DenseVector> for CosineSimilarity {
    fn similarity(&self, a: &DenseVector, b: &DenseVector) -> f64 {
        self.cosine_similarity(&a.values, &b.values)
    }
}

// ============================================================================
// Jaccard 相似度
// ============================================================================

/// Jaccard 相似度算法（集合相似度）
#[derive(Debug, Clone, Default)]
pub struct JaccardSimilarity;

impl Algorithm for JaccardSimilarity {
    fn id(&self) -> &str {
        "sim.jaccard"
    }
    fn name(&self) -> &str {
        "Jaccard 相似度"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "集合 Jaccard 相似度 = |A ∩ B| / |A ∪ B|"
    }
}

impl<T: Eq + std::hash::Hash> SimilarityAlgo<Vec<T>> for JaccardSimilarity
where
    T: Clone,
{
    fn similarity(&self, a: &Vec<T>, b: &Vec<T>) -> f64 {
        use std::collections::HashSet;
        let set_a: HashSet<&T> = a.iter().collect();
        let set_b: HashSet<&T> = b.iter().collect();
        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;
        if union < 1e-10 {
            0.0
        } else {
            intersection / union
        }
    }
}

// ============================================================================
// 编辑距离（Levenshtein）
// ============================================================================

/// Levenshtein 编辑距离算法
#[derive(Debug, Clone, Default)]
pub struct LevenshteinDistance;

impl LevenshteinDistance {
    /// 计算编辑距离
    pub fn distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let n = a_chars.len();
        let m = b_chars.len();

        if n == 0 {
            return m;
        }
        if m == 0 {
            return n;
        }

        let mut d = vec![vec![0usize; m + 1]; n + 1];
        for i in 0..=n {
            d[i][0] = i;
        }
        for j in 0..=m {
            d[0][j] = j;
        }

        for j in 1..=m {
            for i in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                d[i][j] = *[
                    d[i - 1][j] + 1,      // 删除
                    d[i][j - 1] + 1,      // 插入
                    d[i - 1][j - 1] + cost, // 替换
                ]
                .iter()
                .min()
                .unwrap();
            }
        }

        d[n][m]
    }

    /// 计算归一化相似度（0.0 ~ 1.0）
    pub fn normalized_similarity(a: &str, b: &str) -> f64 {
        let dist = Self::distance(a, b) as f64;
        let max_len = a.chars().count().max(b.chars().count()) as f64;
        if max_len < 1e-10 {
            1.0
        } else {
            1.0 - dist / max_len
        }
    }
}

impl Algorithm for LevenshteinDistance {
    fn id(&self) -> &str {
        "sim.levenshtein"
    }
    fn name(&self) -> &str {
        "Levenshtein 编辑距离"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "字符串编辑距离及归一化相似度"
    }
}

// ============================================================================
// 加权混合相似度（多维度融合）
// ============================================================================

/// 多维度加权混合相似度
///
/// 用于专家匹配等复杂场景，融合多个维度的相似度：
/// - 技能向量余弦相似度（权重高）
/// - 标签 Jaccard 相似度（权重中）
/// - 名称/描述文本相似度（权重低）
#[derive(Debug, Clone)]
pub struct WeightedHybridSimilarity {
    pub weights: Vec<(SimilarityMethod, f64)>,
}

impl Default for WeightedHybridSimilarity {
    fn default() -> Self {
        Self {
            weights: vec![
                (SimilarityMethod::Cosine, 0.6),
                (SimilarityMethod::Jaccard, 0.25),
                (SimilarityMethod::Levenshtein, 0.15),
            ],
        }
    }
}

impl Algorithm for WeightedHybridSimilarity {
    fn id(&self) -> &str {
        "sim.weighted_hybrid"
    }
    fn name(&self) -> &str {
        "加权混合相似度"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "多维度加权融合相似度，适用于专家匹配等复杂场景"
    }
}

impl WeightedHybridSimilarity {
    /// 计算混合相似度
    /// - vec_sim: 向量相似度（余弦）
    /// - set_sim: 集合相似度（Jaccard）
    /// - text_sim: 文本相似度（编辑距离归一化）
    pub fn compute(&self, vec_sim: f64, set_sim: f64, text_sim: f64) -> f64 {
        let sims = [vec_sim, set_sim, text_sim];
        let mut total = 0.0;
        let mut weight_sum = 0.0;
        for (i, (_, w)) in self.weights.iter().enumerate() {
            if i < sims.len() {
                total += sims[i] * w;
                weight_sum += w;
            }
        }
        if weight_sum < 1e-10 {
            0.0
        } else {
            total / weight_sum
        }
    }
}

// ============================================================================
// 单元测试（内联）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let sim = CosineSimilarity::default();
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let s = sim.similarity(&a, &b);
        assert!((s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let sim = CosineSimilarity::default();
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let s = sim.similarity(&a, &b);
        assert!(s.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let sim = CosineSimilarity::default();
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let s = sim.similarity(&a, &b);
        assert!((s + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_similarity() {
        let sim = JaccardSimilarity;
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let s = sim.similarity(&a, &b);
        // 交集: {3,4} = 2, 并集: {1,2,3,4,5,6} = 6, Jaccard = 2/6 = 1/3
        assert!((s - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_identical() {
        let sim = JaccardSimilarity;
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let s = sim.similarity(&a, &b);
        assert!((s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let sim = JaccardSimilarity;
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let s = sim.similarity(&a, &b);
        assert!(s.abs() < 1e-6);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(LevenshteinDistance::distance("kitten", "sitting"), 3);
        assert_eq!(LevenshteinDistance::distance("flaw", "lawn"), 2);
        assert_eq!(LevenshteinDistance::distance("", "abc"), 3);
        assert_eq!(LevenshteinDistance::distance("abc", ""), 3);
        assert_eq!(LevenshteinDistance::distance("abc", "abc"), 0);
    }

    #[test]
    fn test_levenshtein_normalized() {
        let s = LevenshteinDistance::normalized_similarity("abc", "abc");
        assert!((s - 1.0).abs() < 1e-6);

        let s = LevenshteinDistance::normalized_similarity("abc", "xyz");
        assert!(s.abs() < 1e-6);
    }

    #[test]
    fn test_weighted_hybrid() {
        let hybrid = WeightedHybridSimilarity::default();
        let s = hybrid.compute(0.8, 0.6, 0.9);
        // 0.8*0.6 + 0.6*0.25 + 0.9*0.15 = 0.48 + 0.15 + 0.135 = 0.765
        assert!((s - 0.765).abs() < 1e-6);
    }
}
