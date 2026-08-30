// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 算法集合模块
//!
//! 按类别组织的算法实现，所有算法均遵循统一 Trait 规范。
//! 通过 feature flag 控制编译，按需启用。

/// 相似度计算算法
#[cfg(feature = "similarity")]
pub mod similarity;

/// 排序与评分算法
#[cfg(feature = "ranking")]
pub mod ranking;

/// 聚类算法（占位）
#[cfg(feature = "clustering")]
pub mod clustering {
    //! 聚类算法（开发中）
}

/// 激活传播算法（占位）
#[cfg(feature = "activation")]
pub mod activation {
    //! 激活传播算法（开发中）
}

/// 图算法（通过 graph 模块对外暴露）
#[cfg(feature = "graph-algo")]
pub mod graph {
    //! 图算法在根模块 graph.rs 中实现，此处重新导出
    pub use crate::graph::UnifiedGraphEngine;
}

/// 嵌入算法（占位）
#[cfg(feature = "embedding")]
pub mod embedding {
    //! 向量嵌入算法（开发中）
}

/// 统计算法（占位）
#[cfg(feature = "stats")]
pub mod stats {
    //! 统计算法（开发中）
}
