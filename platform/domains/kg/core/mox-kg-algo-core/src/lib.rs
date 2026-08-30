// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 知识图谱模块 - AI驱动关系网引擎
//!
//! 实现公理3：关联关系加权有向图
//! 基于petgraph实现加权有向图，支持邻接矩阵、关联度计算、图拉普拉斯、
//! 中心性分析、社区发现、最短路径、智能推荐等AI驱动功能

pub const CRATE_ID: &str = "fbd31c6a-41cd-5274-be2f-2a28066eaf0a";
pub const ENGINE_NAME: &str = "mox::graph_algorithms";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
};

pub use mox_flow_operator_core::Result;

// ============================================================================
// T3 单源真相参数：锁死 7 算法的精度护栏（严禁修改）
// ============================================================================
/// personalizedPageRank / 激活扩散 阻尼因子（与 Node 项目记忆硬性一致：d=0.85）
pub const PPR_D: f64 = 0.85;
/// personalizedPageRank / 激活扩散 最大迭代轮数（与 Node 项目记忆硬性一致：30 轮）
pub const PPR_MAX_ITER: usize = 30;

// ============================================================================
// 模块声明
// ============================================================================

/// 数据类型定义
pub mod types;

/// CSR 稀疏邻接结构（内部工具，pub(crate) 级别）
pub(crate) mod csr;

/// KnowledgeGraph 核心结构与基本方法
pub mod graph;

/// 矩阵运算（邻接矩阵、度矩阵、拉普拉斯矩阵、关联度）
pub mod matrices;

/// AI 流程图谱引擎
pub mod flow_graph;

/// 图算法集合（按类别组织）
pub mod algorithms;

/// 图谱构建器
pub mod builder;

/// 工具函数
pub mod utils;

// ============================================================================
// 公开 API 重导出（保持向后兼容）
// ============================================================================

pub use types::{
    CentralityMetrics, Community, GraphStats, KnowledgeEdge, KnowledgeNode, NodeRecommendation,
    PathResult,
};

pub use graph::KnowledgeGraph;
pub use builder::KnowledgeGraphBuilder;
pub use utils::raw_bidirectional_expand;

pub use flow_graph::{AIFlowGraph, CapabilityMeta, FlowGraphStats, IntentResult, IntentRule};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::{rank_vec_to_map, CsrAdj};
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;
    use std::collections::HashMap;

    #[test]
    fn test_graph_creation() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(KnowledgeNode {
            id: "a".to_string(),
            label: "A".to_string(),
            node_type: "test".to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        graph.add_node(KnowledgeNode {
            id: "b".to_string(),
            label: "B".to_string(),
            node_type: "test".to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_adjacency_matrix() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let adj = graph.adjacency_matrix();
        assert_relative_eq!(adj[(0, 1)], 1.0);
        assert_relative_eq!(adj[(1, 0)], 0.0);
    }

    #[test]
    fn test_laplacian() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let lap = graph.laplacian_matrix();
        assert_relative_eq!(lap[(0, 0)], 1.0);
        assert_relative_eq!(lap[(0, 1)], -1.0);
    }

    #[test]
    fn test_pagerank() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "a", 1.0)
            .build();

        let pr = graph.pagerank(100);
        assert!(pr.len() == 3);
        for score in pr.values() {
            assert!(*score > 0.0);
        }
    }

    #[test]
    fn test_communities() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "group1")
            .add_node("b", "B", "group1")
            .add_node("c", "C", "group2")
            .add_node("d", "D", "group2")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "a", 1.0)
            .add_edge("c", "d", 1.0)
            .add_edge("d", "c", 1.0)
            .build();

        let communities = graph.detect_communities(10);
        assert!(communities.len() >= 2);
    }

    // —— CSR vs Dense 等价性回归 ——
    fn deterministic_graph(n: usize, edge_p: f64, seed: u64) -> KnowledgeGraph {
        let mut rng = seed;
        let mut xorshift = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng as f64) / (u64::MAX as f64)
        };
        let mut g = KnowledgeGraph::new();
        for i in 0..n {
            g.add_node(KnowledgeNode {
                id: format!("n{i}"),
                label: format!("N{i}"),
                node_type: "t".to_string(),
                properties: serde_json::json!({}),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
        }
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                if xorshift() < edge_p {
                    let w = 0.5 + xorshift() * 2.0;
                    let _ = g.add_edge(KnowledgeEdge {
                        source: format!("n{i}"),
                        target: format!("n{j}"),
                        weight: w,
                        relation_type: "r".to_string(),
                        properties: serde_json::json!({}),
                    });
                }
            }
        }
        g
    }

    fn pearson(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len() as f64;
        let ma: f64 = a.iter().sum::<f64>() / n;
        let mb: f64 = b.iter().sum::<f64>() / n;
        let mut num = 0.0f64;
        let mut da = 0.0f64;
        let mut db = 0.0f64;
        for i in 0..a.len() {
            let x = a[i] - ma;
            let y = b[i] - mb;
            num += x * y;
            da += x * x;
            db += y * y;
        }
        if da <= 0.0 || db <= 0.0 {
            return 1.0;
        }
        num / (da.sqrt() * db.sqrt())
    }

    #[test]
    fn test_csr_pagerank_vs_dense_pearson() {
        let g = deterministic_graph(40, 0.15, 0x9E37_79B9_7F4A_7C15);
        let pr_csr: HashMap<String, f64> = {
            let csr = CsrAdj::from_graph(&g.graph);
            let vec = csr.pagerank(g.damping_factor, 100);
            rank_vec_to_map(&vec, &g.node_map)
        };
        let pr_dense = g.pagerank_dense_legacy(100);
        let mut ids: Vec<&String> = pr_csr.keys().collect();
        ids.sort();
        let a: Vec<f64> = ids.iter().map(|k| pr_csr[*k]).collect();
        let b: Vec<f64> = ids.iter().map(|k| pr_dense[*k]).collect();
        let r = pearson(&a, &b);
        assert!(
            r >= 0.9999,
            "CSR PR vs Dense PR pearson = {r}, need >= 0.9999"
        );
    }

    #[test]
    fn test_csr_ppr_vs_dense_pearson() {
        let g = deterministic_graph(40, 0.15, 0x517C_C1CC_8115_3929);
        let mut pers = HashMap::new();
        pers.insert("n3".to_string(), 2.0);
        pers.insert("n17".to_string(), 1.0);
        pers.insert("n29".to_string(), 0.5);
        let ppr_csr: HashMap<String, f64> = {
            let n = g.node_count();
            let mut p = vec![0.0f64; n];
            let total: f64 = pers.values().sum();
            for (id, w) in &pers {
                if let Some(&idx) = g.node_map.get(id) {
                    p[idx.index()] = w / total;
                }
            }
            let csr = CsrAdj::from_graph(&g.graph);
            let vec = csr.pagerank_personalized(g.damping_factor, PPR_MAX_ITER, &p);
            rank_vec_to_map(&vec, &g.node_map)
        };
        let ppr_dense = g.ppr_dense_legacy(&pers, PPR_MAX_ITER);
        let mut ids: Vec<&String> = ppr_csr.keys().collect();
        ids.sort();
        let a: Vec<f64> = ids.iter().map(|k| ppr_csr[*k]).collect();
        let b: Vec<f64> = ids.iter().map(|k| ppr_dense[*k]).collect();
        let r = pearson(&a, &b);
        assert!(
            r >= 0.9999,
            "CSR PPR vs Dense PPR pearson = {r}, need >= 0.9999"
        );
    }

    #[test]
    fn test_degree_matrix_csr_equals_dense() {
        let g = deterministic_graph(25, 0.25, 0xCAFE_BABE);
        let deg_csr = g.degree_matrix();
        let adj = g.adjacency_matrix();
        let n = g.node_count();
        for i in 0..n {
            let sum: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            assert_relative_eq!(deg_csr[(i, i)], sum, epsilon = 1e-12);
            for j in 0..n {
                if i != j {
                    assert_relative_eq!(deg_csr[(i, j)], 0.0);
                }
            }
        }
    }

    #[test]
    fn test_normalized_laplacian_csr_equals_dense() {
        let g = deterministic_graph(20, 0.3, 0xC0FFEE_5EED);
        let lap_csr = g.normalized_laplacian();
        let n = g.node_count();
        let adj = g.adjacency_matrix();
        let mut d_inv_sqrt = DMatrix::zeros(n, n);
        for i in 0..n {
            let d: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            if d > 1e-15 {
                d_inv_sqrt[(i, i)] = 1.0 / d.sqrt();
            }
        }
        let iden = DMatrix::identity(n, n);
        let lap_ref = &iden - &(&d_inv_sqrt * &adj * &d_inv_sqrt);
        for i in 0..n {
            for j in 0..n {
                assert_relative_eq!(lap_csr[(i, j)], lap_ref[(i, j)], epsilon = 1e-12);
            }
        }
    }

    #[test]
    fn test_stats() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let stats = graph.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }
}
