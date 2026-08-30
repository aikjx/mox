// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 统一算法类型定义
//!
//! 所有算法的输入输出类型标准化，确保跨域调用时数据格式一致。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// 通用结果类型
// ============================================================================

/// 算法执行结果统一包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoResult<T> {
    /// 算法唯一标识
    pub algo_id: String,
    /// 算法版本
    pub algo_version: String,
    /// 执行状态
    pub status: AlgoStatus,
    /// 结果数据
    pub data: Option<T>,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 性能指标
    pub metrics: AlgoMetrics,
}

/// 算法执行状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlgoStatus {
    /// 执行成功
    Success,
    /// 部分完成（有结果但未完全收敛）
    Partial,
    /// 执行失败
    Failed,
    /// 执行中（异步场景）
    Running,
}

/// 算法性能指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlgoMetrics {
    /// 总耗时
    pub duration_ms: u64,
    /// 内存使用峰值（字节）
    pub peak_memory_bytes: Option<u64>,
    /// 迭代次数
    pub iterations: Option<usize>,
    /// 收敛误差
    pub convergence_error: Option<f64>,
    /// 处理节点数
    pub nodes_processed: Option<usize>,
    /// 处理边数
    pub edges_processed: Option<usize>,
}

// ============================================================================
// 评分与排名类型
// ============================================================================

/// 带评分的条目（统一排名结果格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredItem<K: Clone + Serialize> {
    /// 条目键（节点ID / 专家ID / 文档ID 等）
    pub key: K,
    /// 得分（0.0 ~ 1.0 标准化）
    pub score: f64,
    /// 排名（从 1 开始）
    pub rank: usize,
    /// 置信度（0.0 ~ 1.0）
    pub confidence: f64,
    /// 得分来源标签（多因子融合时记录各因子贡献）
    pub score_breakdown: Option<HashMap<String, f64>>,
}

/// 排名结果列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingResult<K: Clone + Serialize> {
    /// 排名列表（按得分降序）
    pub items: Vec<ScoredItem<K>>,
    /// 总条目数
    pub total: usize,
    /// 排名方法
    pub method: String,
}

// ============================================================================
// 相似度类型
// ============================================================================

/// 相似度计算方法
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimilarityMethod {
    /// 余弦相似度
    Cosine,
    /// Jaccard 相似度
    Jaccard,
    /// 欧氏距离（转换为相似度）
    Euclidean,
    /// 曼哈顿距离（转换为相似度）
    Manhattan,
    /// 皮尔逊相关系数
    Pearson,
    /// 编辑距离（字符串）
    Levenshtein,
    /// 汉明距离
    Hamming,
}

/// 相似度结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult<K: Clone + Serialize> {
    /// 源条目
    pub source: K,
    /// 目标条目
    pub target: K,
    /// 相似度得分（0.0 ~ 1.0）
    pub similarity: f64,
    /// 计算方法
    pub method: SimilarityMethod,
}

// ============================================================================
// 聚类类型
// ============================================================================

/// 聚类算法类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusteringMethod {
    /// K-Means
    KMeans,
    /// 层次聚类
    Hierarchical,
    /// DBSCAN
    Dbscan,
    /// 谱聚类
    Spectral,
    /// 社区发现（图聚类）
    Community,
}

/// 单个聚类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster<K: Clone + Serialize> {
    /// 聚类ID
    pub cluster_id: usize,
    /// 成员列表
    pub members: Vec<K>,
    /// 聚类中心（向量表示，可选）
    pub centroid: Option<Vec<f64>>,
    /// 聚类内平均相似度
    pub intra_similarity: Option<f64>,
    /// 聚类大小
    pub size: usize,
}

/// 聚类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringResult<K: Clone + Serialize> {
    /// 聚类列表
    pub clusters: Vec<Cluster<K>>,
    /// 聚类数量
    pub n_clusters: usize,
    /// 噪声点数量（DBSCAN 等）
    pub noise_count: Option<usize>,
    /// 方法
    pub method: ClusteringMethod,
}

// ============================================================================
// 图算法类型
// ============================================================================

/// 图算法类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphAlgoType {
    /// PageRank
    PageRank,
    /// 中心性分析
    Centrality(CentralityType),
    /// 社区发现
    CommunityDetection,
    /// 最短路径
    ShortestPath,
    /// 激活传播
    ActivationPropagation,
    /// 图嵌入
    GraphEmbedding,
}

/// 中心性类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CentralityType {
    /// 度中心性
    Degree,
    /// 介数中心性
    Betweenness,
    /// 接近中心性
    Closeness,
    /// 特征向量中心性
    Eigenvector,
    /// Katz 中心性
    Katz,
    /// PageRank 中心性
    PageRank,
}

/// 路径结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult<K: Clone + Serialize> {
    /// 路径节点序列
    pub nodes: Vec<K>,
    /// 路径总权重/距离
    pub distance: f64,
    /// 路径长度（边数）
    pub path_length: usize,
    /// 是否找到路径
    pub found: bool,
}

// ============================================================================
// 算法配置基类
// ============================================================================

/// 通用算法配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub tolerance: f64,
    /// 随机种子（可重复性）
    pub random_seed: Option<u64>,
    /// 是否并行执行
    pub parallel: bool,
    /// 结果 Top-K（0 表示全部）
    pub top_k: usize,
}

impl Default for AlgoConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            random_seed: Some(42),
            parallel: false,
            top_k: 0,
        }
    }
}

// ============================================================================
// 向量类型
// ============================================================================

/// 稠密向量（用于嵌入表示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseVector {
    pub dim: usize,
    pub values: Vec<f64>,
}

impl DenseVector {
    pub fn new(values: Vec<f64>) -> Self {
        let dim = values.len();
        Self { dim, values }
    }

    pub fn zeros(dim: usize) -> Self {
        Self {
            dim,
            values: vec![0.0; dim],
        }
    }

    /// L2 范数
    pub fn norm(&self) -> f64 {
        self.values.iter().map(|v| v * v).sum::<f64>().sqrt()
    }

    /// 归一化（单位向量）
    pub fn normalize(&self) -> Self {
        let n = self.norm();
        if n < 1e-10 {
            return self.clone();
        }
        Self {
            dim: self.dim,
            values: self.values.iter().map(|v| v / n).collect(),
        }
    }
}

impl From<Vec<f64>> for DenseVector {
    fn from(values: Vec<f64>) -> Self {
        DenseVector::new(values)
    }
}

// ============================================================================
// 辅助方法
// ============================================================================

impl<T> AlgoResult<T> {
    /// 创建成功结果
    pub fn success(algo_id: &str, data: T, metrics: AlgoMetrics) -> Self {
        Self {
            algo_id: algo_id.to_string(),
            algo_version: env!("CARGO_PKG_VERSION").to_string(),
            status: AlgoStatus::Success,
            data: Some(data),
            error: None,
            metrics,
        }
    }

    /// 创建失败结果
    pub fn failed(algo_id: &str, error: &str, metrics: AlgoMetrics) -> Self {
        Self {
            algo_id: algo_id.to_string(),
            algo_version: env!("CARGO_PKG_VERSION").to_string(),
            status: AlgoStatus::Failed,
            data: None,
            error: Some(error.to_string()),
            metrics,
        }
    }
}

impl AlgoMetrics {
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            duration_ms: duration.as_millis() as u64,
            ..Default::default()
        }
    }
}
