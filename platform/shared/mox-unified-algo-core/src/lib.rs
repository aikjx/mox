// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 统一算法核心库 · mox-unified-algo-core
//!
//! 跨域算法归一化基础设施，为三大业务域提供共享算法能力：
//! - **KG 域**：知识图谱图算法（中心性 / 社区发现 / 路径 / 推荐）
//! - **EA 域**：专家联盟匹配算法（相似度 / 排序 / 聚类 / 融合）
//! - **Cloud 域**：知识库云盘检索算法（向量相似度 / 语义搜索 / 标签推荐）
//!
//! ## 设计原则
//! 1. **算法归一化**：同一类算法（如相似度计算）统一接口，多领域复用
//! 2. **领域适配**：通过泛型和 trait 适配不同领域的数据结构
//! 3. **性能分层**：提供基础版和优化版，按场景选择
//! 4. **结果标准化**：统一的算法结果格式（得分 / 排名 / 置信度）
//! 5. **零耦合**：不依赖任何业务域，纯算法库
//!
//! ## 算法分类
//! - `graph` - 图算法（PageRank / 中心性 / 社区发现 / 最短路径）
//! - `similarity` - 相似度计算（余弦 / Jaccard / 编辑距离 / 向量相似度）
//! - `ranking` - 排序算法（加权评分 / 学习排序 / 融合排序）
//! - `clustering` - 聚类算法（K-Means / 层次聚类 / DBSCAN）
//! - `activation` - 激活传播（PPR / 热扩散 / 影响力最大化）
//! - `embedding` - 嵌入算法（向量表示 / 降维 / 相似度检索）
//! - `stats` - 统计算法（分布 / 相关性 / 显著性检验）

pub const CRATE_ID: &str = "ua1c0r3a-41cd-5274-be2f-a1g0n0rmc0re";
pub const ENGINE_NAME: &str = "mox::unified_algo";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "expert-alliance",
};

pub use mox_platform_operator_core::Result;

// ============================================================================
// 全局算法参数（单源真相，跨域一致性保证）
// ============================================================================

/// PageRank / 激活传播阻尼因子（跨域统一：d=0.85）
pub const PPR_DAMPING: f64 = 0.85;
/// PageRank / 激活传播最大迭代次数（跨域统一：30 轮）
pub const PPR_MAX_ITER: usize = 30;
/// PageRank 收敛阈值
pub const PPR_TOLERANCE: f64 = 1e-6;

/// 相似度计算默认精度
pub const SIMILARITY_PRECISION: f64 = 1e-8;
/// 余弦相似度最小值阈值
pub const COSINE_SIM_MIN_THRESHOLD: f64 = 0.01;

/// 社区发现模块度精度
pub const COMMUNITY_MODULARITY_PRECISION: f64 = 1e-7;
/// Louvain 算法最大迭代次数
pub const LOUVAIN_MAX_ITER: usize = 100;

/// 向量维度默认值（用于知识嵌入 / 专家画像）
pub const DEFAULT_EMBEDDING_DIM: usize = 384;

// ============================================================================
// 模块声明
// ============================================================================

/// 核心类型定义（统一算法输入/输出格式）
pub mod types;

/// 统一算法 trait 定义（所有算法必须实现的标准接口）
pub mod traits;

/// 算法工厂与注册表
pub mod registry;

/// 算法集合（按类别组织）
pub mod algorithms;

/// 图算法封装
#[cfg(feature = "graph-algo")]
pub mod graph;

/// 相似度计算算法
#[cfg(feature = "similarity")]
pub mod similarity;

/// 排序与评分算法
#[cfg(feature = "ranking")]
pub mod ranking;

/// 聚类算法（存根占位，后续扩展）
#[cfg(feature = "clustering")]
pub mod clustering {
    //! 聚类算法模块（开发中）
    //! 后续将实现 K-Means / DBSCAN / 层次聚类等
}

/// 算法性能基准工具
pub mod benchmark;

/// 通用工具函数
pub mod utils;
