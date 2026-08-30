// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 算法注册表
//!
//! 统一的算法注册与发现机制，支持运行时动态查询可用算法。
//! 三大业务域通过注册表获取算法实例，实现解耦。

use std::collections::HashMap;
use std::sync::RwLock;

/// 算法元信息
#[derive(Debug, Clone)]
pub struct AlgoInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub category: AlgoCategory,
    pub description: &'static str,
    pub tags: &'static [&'static str],
}

/// 算法分类
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlgoCategory {
    Graph,
    Similarity,
    Ranking,
    Clustering,
    Activation,
    Embedding,
    Stats,
    Fusion,
}

impl AlgoCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlgoCategory::Graph => "graph",
            AlgoCategory::Similarity => "similarity",
            AlgoCategory::Ranking => "ranking",
            AlgoCategory::Clustering => "clustering",
            AlgoCategory::Activation => "activation",
            AlgoCategory::Embedding => "embedding",
            AlgoCategory::Stats => "stats",
            AlgoCategory::Fusion => "fusion",
        }
    }
}

/// 全局算法注册表
pub struct AlgoRegistry {
    algorithms: RwLock<HashMap<&'static str, AlgoInfo>>,
}

impl AlgoRegistry {
    pub fn new() -> Self {
        Self {
            algorithms: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, info: AlgoInfo) {
        if let Ok(mut map) = self.algorithms.write() {
            map.insert(info.id, info);
        }
    }

    pub fn get(&self, id: &str) -> Option<AlgoInfo> {
        self.algorithms
            .read()
            .ok()
            .and_then(|map| map.get(id).cloned())
    }

    pub fn list_all(&self) -> Vec<AlgoInfo> {
        self.algorithms
            .read()
            .map(|map| map.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn list_by_category(&self, category: &AlgoCategory) -> Vec<AlgoInfo> {
        self.algorithms
            .read()
            .map(|map| {
                map.values()
                    .filter(|a| &a.category == category)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search_by_tag(&self, tag: &str) -> Vec<AlgoInfo> {
        self.algorithms
            .read()
            .map(|map| {
                map.values()
                    .filter(|a| a.tags.contains(&tag))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.algorithms.read().map(|m| m.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AlgoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 内置算法定义（编译期静态数组）
const BUILTIN_ALGORITHMS: &[AlgoInfo] = &[
    // 图算法
    AlgoInfo {
        id: "graph.pagerank",
        name: "PageRank",
        version: "1.0.0",
        category: AlgoCategory::Graph,
        description: "基于幂法迭代的 PageRank 算法，支持个性化 PageRank",
        tags: &["ranking", "graph", "centrality"],
    },
    AlgoInfo {
        id: "graph.centrality",
        name: "中心性分析",
        version: "1.0.0",
        category: AlgoCategory::Graph,
        description: "度中心性、介数中心性、接近中心性、特征向量中心性",
        tags: &["centrality", "graph", "analysis"],
    },
    AlgoInfo {
        id: "graph.community",
        name: "社区发现",
        version: "1.0.0",
        category: AlgoCategory::Graph,
        description: "Louvain / CNM 社区发现算法，支持模块度优化",
        tags: &["clustering", "graph", "community"],
    },
    AlgoInfo {
        id: "graph.shortest_path",
        name: "最短路径",
        version: "1.0.0",
        category: AlgoCategory::Graph,
        description: "Dijkstra / A* / Bellman-Ford 最短路径算法",
        tags: &["pathfinding", "graph"],
    },
    // 相似度算法
    AlgoInfo {
        id: "sim.cosine",
        name: "余弦相似度",
        version: "1.0.0",
        category: AlgoCategory::Similarity,
        description: "向量余弦相似度计算，支持稀疏/稠密向量",
        tags: &["similarity", "vector", "embedding"],
    },
    AlgoInfo {
        id: "sim.jaccard",
        name: "Jaccard 相似度",
        version: "1.0.0",
        category: AlgoCategory::Similarity,
        description: "集合 Jaccard 相似度（交集/并集）",
        tags: &["similarity", "set"],
    },
    // 排序算法
    AlgoInfo {
        id: "rank.weighted",
        name: "加权评分",
        version: "1.0.0",
        category: AlgoCategory::Ranking,
        description: "多因子加权评分排序，支持动态权重调整",
        tags: &["ranking", "scoring"],
    },
    AlgoInfo {
        id: "rank.borda",
        name: "Borda 计数融合排序",
        version: "1.0.0",
        category: AlgoCategory::Ranking,
        description: "多排名融合的 Borda Count 方法",
        tags: &["ranking", "fusion"],
    },
    // 聚类算法
    AlgoInfo {
        id: "cluster.kmeans",
        name: "K-Means 聚类",
        version: "1.0.0",
        category: AlgoCategory::Clustering,
        description: "K-Means 聚类算法，支持 K-Means++ 初始化",
        tags: &["clustering", "vector"],
    },
    AlgoInfo {
        id: "cluster.dbscan",
        name: "DBSCAN 聚类",
        version: "1.0.0",
        category: AlgoCategory::Clustering,
        description: "基于密度的 DBSCAN 聚类算法",
        tags: &["clustering", "density"],
    },
    // 激活传播
    AlgoInfo {
        id: "act.ppr",
        name: "个性化 PageRank 激活传播",
        version: "1.0.0",
        category: AlgoCategory::Activation,
        description: "从种子节点出发的 PPR 激活扩散算法",
        tags: &["activation", "pagerank", "propagation"],
    },
    // 融合算法
    AlgoInfo {
        id: "fusion.weighted_vote",
        name: "加权投票融合",
        version: "1.0.0",
        category: AlgoCategory::Fusion,
        description: "基于专家权重的加权投票融合算法",
        tags: &["fusion", "voting", "expert"],
    },
    AlgoInfo {
        id: "fusion.debate",
        name: "多轮辩论融合",
        version: "1.0.0",
        category: AlgoCategory::Fusion,
        description: "多专家多轮辩论后融合输出",
        tags: &["fusion", "debate", "expert"],
    },
];

/// 全局单例注册表（延迟初始化，首次访问时填充内置算法）
use std::sync::OnceLock;

fn global_registry_instance() -> &'static AlgoRegistry {
    static REGISTRY: OnceLock<AlgoRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let registry = AlgoRegistry::new();
        for algo in BUILTIN_ALGORITHMS {
            registry.register(algo.clone());
        }
        registry
    })
}

/// 获取全局算法注册表
pub fn global_algo_registry() -> &'static AlgoRegistry {
    global_registry_instance()
}

/// 注册内置算法（保留 API 兼容，实际通过 OnceLock 自动初始化）
pub fn register_builtin_algorithms() {
    // 触发一次初始化
    let _ = global_algo_registry();
}
