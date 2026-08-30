// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 图算法模块集合
//!
//! 按算法类别组织，所有方法均以 `impl KnowledgeGraph` 形式扩展，
//! 对外通过根模块 `pub use` 统一暴露。
//!
//! # 核心算法（7 个）
//! - `pagerank` - PageRank / 个性化 PageRank（增强版：死端/蜘蛛陷阱/幂法加速）
//! - `centrality` - 度中心性 / 介数中心性 / 接近中心性
//! - `community` - CNM 社区发现（模块度贪心）
//! - `pathfinding` - 最短路径（Dijkstra）
//! - `activation` - 激活扩散
//! - `recommendation` - 智能推荐
//! - `stats` - 图统计信息
//!
//! # 扩展算法（新增）
//! - `distributed_pagerank` - 分布式 PageRank（分片 / 同步异步 / 检查点）
//! - `shortest_path` - 最短路径算法集（Dijkstra/Bellman-Ford/Floyd-Warshall/A*/双向BFS/k最短路径）
//! - `centrality_extended` - 扩展中心性（特征向量/Katz/HITS/信息/局部）
//! - `community_extended` - 扩展社区发现（Louvain/标签传播/谱聚类/Girvan-Newman/SLPA）
//! - `graph_embedding` - 图嵌入（DeepWalk/Node2Vec/LINE/GraphSAGE/节点相似度）

// 核心算法模块
pub mod centrality;
pub mod community;
pub mod pagerank;
pub mod pathfinding;
pub mod activation;
pub mod recommendation;
pub mod stats;

// 扩展算法模块
pub mod distributed_pagerank;
pub mod shortest_path;
pub mod centrality_extended;
pub mod community_extended;
pub mod graph_embedding;
