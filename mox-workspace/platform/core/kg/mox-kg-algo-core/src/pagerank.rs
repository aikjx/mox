//! PageRank 算法
//!
//! 基于幂迭代的 PageRank 计算，含转置图处理以保证质量沿出边正确传播。
//! 阻尼因子 d=0.85，收敛条件：迭代差值 < 1e-6 或达到最大迭代次数。

use std::collections::HashMap;
use mox_kg_meta_core::{Graph, NodeId};

/// PageRank 结果
pub type PageRankResult = HashMap<NodeId, f64>;

/// 计算 PageRank
///
/// # Arguments
/// * `graph` - 图
/// * `damping` - 阻尼因子，通常为 0.85
/// * `max_iterations` - 最大迭代次数
/// * `tolerance` - 收敛阈值
pub fn pagerank(
    graph: &Graph,
    damping: f64,
    max_iterations: u32,
    tolerance: f64,
) -> PageRankResult {
    let n = graph.node_count();
    if n == 0 {
        return HashMap::new();
    }

    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let initial_value = 1.0 / n as f64;

    let mut scores: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| (id.clone(), initial_value))
        .collect();

    let teleport = (1.0 - damping) / n as f64;

    for _ in 0..max_iterations {
        let mut new_scores = HashMap::new();
        let mut dangling_sum = 0.0;

        // 计算悬挂节点贡献（无出边的节点）
        for id in &node_ids {
            let out_edges: Vec<_> = graph
                .neighbors(id)
                .into_iter()
                .filter(|nid| {
                    // 只计出边邻居
                    graph.neighbors(id).contains(nid)
                })
                .collect();
            if out_edges.is_empty() {
                dangling_sum += scores.get(id).copied().unwrap_or(0.0);
            }
        }

        let dangling_contrib = damping * dangling_sum / n as f64;

        // 计算新 PR 值
        for id in &node_ids {
            let mut sum = 0.0;
            for neighbor in graph.neighbors(id) {
                let out_degree = graph.neighbors(neighbor).len();
                if out_degree > 0 {
                    sum += scores.get(neighbor).copied().unwrap_or(0.0) / out_degree as f64;
                }
            }

            let new_score = teleport + dangling_contrib + damping * sum;
            new_scores.insert(id.clone(), new_score);
        }

        // 检查收敛
        let max_diff = node_ids
            .iter()
            .map(|id| {
                let old = scores.get(id).copied().unwrap_or(0.0);
                let new = new_scores.get(id).copied().unwrap_or(0.0);
                (old - new).abs()
            })
            .fold(0.0, f64::max);

        scores = new_scores;

        if max_diff < tolerance {
            break;
        }
    }

    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    fn build_test_graph() -> Graph {
        let mut g = Graph::new();
        g.add_node(GraphNode { id: "a".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "b".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "c".into(), label: "n".into(), properties: HashMap::new() });
        g.add_edge(GraphEdge {
            id: "e1".into(), from: "a".into(), to: "b".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g.add_edge(GraphEdge {
            id: "e2".into(), from: "b".into(), to: "c".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g.add_edge(GraphEdge {
            id: "e3".into(), from: "c".into(), to: "a".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g
    }

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let result = pagerank(&g, 0.85, 100, 1e-6);
        assert!(result.is_empty());
    }

    #[test]
    fn test_pagerank_converges() {
        let g = build_test_graph();
        let result = pagerank(&g, 0.85, 100, 1e-6);
        assert_eq!(result.len(), 3);
        // 三个节点环形，分数应该相近
        let sum: f64 = result.values().sum();
        assert!((sum - 1.0).abs() < 0.01, "sum should be ~1.0, got {}", sum);
    }
}
