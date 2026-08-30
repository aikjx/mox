// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 相似度计算模块（对外统一入口）
//!
//! 重新导出 algorithms/similarity 中的算法类型，保持模块路径清晰。

pub use crate::algorithms::similarity::{
    CosineSimilarity, JaccardSimilarity, LevenshteinDistance, WeightedHybridSimilarity,
};
