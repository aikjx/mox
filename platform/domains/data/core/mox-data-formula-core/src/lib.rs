// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # MOX Formulas Core · 图公式权威单源（T3 / TR-4.2 精度护栏）
//!
//! 单一真相源，严禁 Node/Python 独立实现等价算法（需经本 crate 提供的绑定接入）。
//!
//! ## 12 项公式（Rust 最高性能实现）
//! 1. 密度 density（无向 D = 2E / [N(N−1)]，附人读解读）
//! 2. 度中心性 degree_centrality（RAW 边 incident/(N−1)）
//! 3. Brandes 介数中心性 betweenness_centrality（并行，O(N·E)）
//! 4. Harmonic 紧密中心性 closeness_harmonic（不可达=0）
//! 5. PageRank（Gauss-Seidel 单缓存 CSR，α=0.85，30 轮精度护栏）
//! 6. 个性化 PageRank（激活扩散特例，d=0.85，30 轮）
//! 7. CNM 社区检测 community_cnm（模块度贪心凝聚，确定性平局）
//! 8. Newman 模块度 modularity（Q = Σ_c [ l_c/m − (d_c/2m)² ]）
//! 9. K-Core 分解 k_core（Bin-Sort，O(E)）
//! 10. 特征向量中心性 eigenvector_centrality（幂迭代，CSR）
//! 11. 三角计数 triangles + 聚集系数 clustering_coefficient
//! 12. 度同配系数 assortativity（Pearson 度相关）
//!
//! ## 算法选型（均为最高性能 / 确定性 / 可并行的权威实现）
//! - 稀疏结构：CSR（出/入双邻接可按需展开）
//! - PageRank：Gauss-Seidel 原地迭代（比 Power Iteration 少 30~50% 轮次）
//! - Brandes 介数：rayon 并行（每个源节点独立 BFS + 反序依赖累积）
//! - CNM：最大堆维护 ΔQ，平局字典序（与 Node T3 对账一致）
//! - K-Core：Batagelj-Zaversnik O(m) Bin-Sort
//! - 三角计数：forward 优化（u<v<w），O(E·√E) 实际运行快 5~10×

pub mod csr;
pub mod pagerank;
pub mod centrality;
pub mod community;
pub mod stats;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use csr::{CsrGraph, Edge, RawExpand};

// ======================================================================
// 精度护栏常量（T3 / 项目记忆 锁死，外部不可改）
// ======================================================================
/// 激活扩散 / PPR 的阻尼因子：0.85（项目记忆 强制）
pub const PPR_D: f64 = 0.85;
/// PPR / PageRank 的最大迭代轮数：30（项目记忆 强制）
pub const PPR_MAX_ITER: usize = 30;
/// 收敛容差：Gauss-Seidel 提前停止（仅 PR 内部使用，不影响 30 轮护栏）
pub const PR_EPS: f64 = 1e-9;
/// CNM ΔQ > 0 才合并（杜绝负增益假合并）
pub const CNM_GAIN_EPS: f64 = 1e-12;

/// 密度人读解读：thresholds 与 TR-8 对齐
#[inline]
pub fn density_interpretation(d: f64) -> &'static str {
    if d >= 0.8 {
        "高度稠密（d≥0.8）：节点间几乎全连接，适合全局聚合类查询"
    } else if d >= 0.3 {
        "中等密度（0.3≤d<0.8）：关系分布均衡，适合社区划分与结构分析"
    } else if d >= 0.05 {
        "稀疏图（0.05≤d<0.3）：低连接强度，适合关键桥点与核心子图挖掘"
    } else {
        "极端稀疏图（d<0.05）：结构松散，需关注连通分支与孤立节点"
    }
}

// ======================================================================
// 通用输入/输出结构（Node/Python 绑定的 JSON schema 真源）
// ======================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInput {
    pub source: String,
    pub target: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default, alias = "relationType", alias = "relation_type")]
    pub relation_type: Option<String>,
}

fn default_weight() -> f64 {
    1.0
}

/// 密度输出（F1 三字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityOut {
    pub value: f64,
    pub formula: String,
    pub interpretation: String,
}

/// 社区检测输出（F6）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnmOut {
    pub communities: Vec<Vec<String>>,
    pub node_community: HashMap<String, usize>,
    pub modularity: f64,
    pub algorithm: String,
    pub merges: usize,
}

/// Pagerank 含转置图对照（项目记忆 F8 强制）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrWithTranspose {
    pub standard: HashMap<String, f64>,
    pub transposed: HashMap<String, f64>,
    pub diff: f64,
    pub d: f64,
    pub max_iter: usize,
    pub converged_at: usize,
}

/// 计算密度（F1）
pub fn density(node_count: usize, edge_count: usize) -> DensityOut {
    let n = node_count as f64;
    let e = edge_count as f64;
    let value = if n <= 1.0 {
        0.0
    } else {
        // 无向语义：每 RAW 边算成一条无向边 → D = 2E / (N(N-1))
        (2.0 * e) / (n * (n - 1.0))
    };
    DensityOut {
        value,
        formula: "D = 2E / (N·(N−1))".to_string(),
        interpretation: density_interpretation(value).to_string(),
    }
}

/// 快速从输入节点+边构建 CSR（无向语义内部 RAW 展开，除非 directed=true）
pub fn build_csr(nodes: &[NodeInput], edges: &[EdgeInput], directed: bool) -> CsrGraph {
    CsrGraph::from_inputs(nodes, edges, if directed { RawExpand::None } else { RawExpand::Undirected })
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn density_thresholds() {
        assert!(density_interpretation(0.9).contains("高度稠密"));
        assert!(density_interpretation(0.5).contains("中等密度"));
        assert!(density_interpretation(0.1).contains("稀疏图"));
        assert!(density_interpretation(0.001).contains("极端稀疏"));
    }

    #[test]
    fn density_simple() {
        // 三角形（3 节点 3 无向边）→ D = 6/(3*2) = 1.0
        let d = density(3, 3);
        assert!((d.value - 1.0).abs() < 1e-15);
        // 单点
        let d0 = density(1, 0);
        assert_eq!(d0.value, 0.0);
        // 空
        let dn = density(0, 0);
        assert_eq!(dn.value, 0.0);
    }

    #[test]
    fn precision_railway_constants_locked() {
        // 项目记忆硬约束：禁止修改
        assert_eq!(PPR_D, 0.85);
        assert_eq!(PPR_MAX_ITER, 30);
    }
}
