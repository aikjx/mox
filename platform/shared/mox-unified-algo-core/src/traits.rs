// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 统一算法 Trait 定义
//!
//! 所有算法必须实现的标准接口，确保跨域调用时行为一致。
//! 通过泛型和关联类型适配不同领域的数据结构。

use crate::types::*;

// ============================================================================
// 核心算法 Trait
// ============================================================================

/// 基础算法 Trait —— 所有算法的根 Trait
pub trait Algorithm {
    /// 算法唯一标识
    fn id(&self) -> &str;

    /// 算法名称
    fn name(&self) -> &str;

    /// 算法版本
    fn version(&self) -> &str;

    /// 算法描述
    fn description(&self) -> &str;
}

/// 同步执行算法
pub trait SyncAlgorithm<Input, Output>: Algorithm {
    /// 同步执行算法
    fn run(&self, input: &Input) -> AlgoResult<Output>;
}

/// 异步执行算法（长耗时场景）
/// （暂存定义，启用 async-trait 依赖后生效）
pub trait AsyncAlgorithm<Input, Output>: Algorithm {
    /// 异步执行算法
    fn run_async<'a>(&'a self, input: &'a Input) -> std::pin::Pin<Box<dyn std::future::Future<Output = AlgoResult<Output>> + Send + 'a>>;
}

// ============================================================================
// 评分/排序类算法 Trait
// ============================================================================

/// 可评分算法 —— 对一组条目给出排名
pub trait RankingAlgo<K: Clone + serde::Serialize>: Algorithm {
    /// 输入类型
    type Input;

    /// 对输入进行评分排名
    fn rank(&self, input: &Self::Input) -> RankingResult<K>;
}

/// 可增量更新的排名算法
pub trait IncrementalRanking<K: Clone + serde::Serialize>: RankingAlgo<K> {
    /// 添加一个条目并更新排名
    fn add_item(&mut self, key: K, features: Vec<f64>);

    /// 移除一个条目
    fn remove_item(&mut self, key: &K);

    /// 更新条目的特征
    fn update_item(&mut self, key: &K, features: Vec<f64>);
}

// ============================================================================
// 相似度类算法 Trait
// ============================================================================

/// 相似度计算算法
pub trait SimilarityAlgo<T>: Algorithm {
    /// 计算两个对象的相似度
    fn similarity(&self, a: &T, b: &T) -> f64;

    /// 计算与目标最相似的 Top-K 个对象
    fn top_k_similar(&self, target: &T, candidates: &[T], k: usize) -> Vec<(usize, f64)>
    where
        T: Clone,
    {
        let mut scored: Vec<(usize, f64)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.similarity(target, c)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
}

/// 向量相似度算法特化
pub trait VectorSimilarity: Algorithm {
    /// 计算两个向量的余弦相似度
    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        debug_assert_eq!(a.len(), b.len(), "向量维度不一致");
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }

    /// 计算两个向量的欧氏距离
    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        debug_assert_eq!(a.len(), b.len(), "向量维度不一致");
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    }
}

// ============================================================================
// 聚类类算法 Trait
// ============================================================================

/// 聚类算法
pub trait ClusteringAlgo<K: Clone + serde::Serialize>: Algorithm {
    /// 输入类型
    type Input;

    /// 执行聚类
    fn cluster(&self, input: &Self::Input) -> ClusteringResult<K>;
}

// ============================================================================
// 图算法 Trait
// ============================================================================

/// 图算法基类
pub trait GraphAlgorithm: Algorithm {
    /// 图节点类型
    type Node: Clone + serde::Serialize + Eq + std::hash::Hash;
    /// 图边权重类型
    type EdgeWeight: Clone + Default;
}

/// 中心性计算算法
pub trait CentralityAlgo: GraphAlgorithm {
    /// 计算节点中心性
    fn centrality(
        &self,
        graph: &petgraph::Graph<Self::Node, Self::EdgeWeight>,
        ctype: CentralityType,
    ) -> Vec<ScoredItem<Self::Node>>;
}

/// PageRank 算法
pub trait PageRankAlgo: GraphAlgorithm {
    /// 计算 PageRank
    fn pagerank(
        &self,
        graph: &petgraph::Graph<Self::Node, Self::EdgeWeight>,
        damping: f64,
        max_iter: usize,
        tolerance: f64,
    ) -> Vec<ScoredItem<Self::Node>>;
}

/// 社区发现算法
pub trait CommunityDetectionAlgo: GraphAlgorithm {
    /// 检测社区
    fn detect_communities(
        &self,
        graph: &petgraph::Graph<Self::Node, Self::EdgeWeight>,
    ) -> ClusteringResult<Self::Node>;
}

/// 最短路径算法
pub trait ShortestPathAlgo: GraphAlgorithm {
    /// 单源最短路径
    fn shortest_path(
        &self,
        graph: &petgraph::Graph<Self::Node, Self::EdgeWeight>,
        source: Self::Node,
        target: Self::Node,
    ) -> PathResult<Self::Node>;
}

// ============================================================================
// 融合/投票类算法 Trait（专家联盟专用）
// ============================================================================

/// 融合算法 —— 多专家意见融合
pub trait FusionAlgo<T>: Algorithm {
    /// 融合多个专家输出
    fn fuse(&self, expert_outputs: &[ExpertOutput<T>]) -> FusionResult<T>;
}

/// 单专家输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpertOutput<T> {
    pub expert_id: String,
    pub expert_weight: f64,
    pub content: T,
    pub confidence: f64,
}

/// 融合结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FusionResult<T> {
    pub result: T,
    pub consensus_score: f64,
    pub contributing_experts: Vec<String>,
    pub method: String,
}
