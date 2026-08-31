//! 激活扩散算法
//!
//! 个性化 PageRank 的简化版本，用于意图识别、影响面分析、推荐召回。
//! 阻尼因子 d=0.85，迭代 30 轮收敛。

use std::collections::HashMap;
use mox_kg_meta_core::{Graph, NodeId};

/// 激活扩散结果
pub type SpreadResult = HashMap<NodeId, f64>;

/// 激活扩散算法（Activation Spread）
///
/// 从种子节点出发，沿边传播激活值，每轮衰减 damping 比例。
///
/// # Arguments
/// * `graph` - 图
/// * `seeds` - 种子节点及初始激活值
/// * `damping` - 阻尼因子（保留比例），通常 0.85
/// * `max_iterations` - 最大迭代次数，默认 30
pub fn activation_spread(
    graph: &Graph,
    seeds: &HashMap<NodeId, f64>,
    damping: f64,
    max_iterations: u32,
) -> SpreadResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let n = graph.node_count();

    if n == 0 {
        return HashMap::new();
    }

    // 初始化激活值
    let mut activation: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| {
            let seed_val = seeds.get(id).copied().unwrap_or(0.0);
            (id.clone(), seed_val)
        })
        .collect();

    let teleport: f64 = seeds.values().sum::<f64>() * (1.0 - damping) / n as f64;

    for _ in 0..max_iterations {
        let mut new_activation: HashMap<NodeId, f64> = node_ids
            .iter()
            .map(|id| (id.clone(), teleport + seeds.get(id).copied().unwrap_or(0.0) * (1.0 - damping)))
            .collect();

        // 沿边传播
        for node_id in &node_ids {
            let act = activation.get(node_id).copied().unwrap_or(0.0);
            if act <= 0.0 {
                continue;
            }

            let neighbors = graph.neighbors(node_id);
            if neighbors.is_empty() {
                continue;
            }

            let share = damping * act / neighbors.len() as f64;
            for &neighbor in &neighbors {
                *new_activation.get_mut(neighbor).unwrap() += share;
            }
        }

        activation = new_activation;
    }

    activation
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let seeds = HashMap::new();
        let result = activation_spread(&g, &seeds, 0.85, 30);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_seed() {
        let mut g = Graph::new();
        g.add_node(GraphNode { id: "a".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "b".into(), label: "n".into(), properties: HashMap::new() });
        g.add_edge(GraphEdge {
            id: "e1".into(), from: "a".into(), to: "b".into(),
            label: "l".into(), properties: HashMap::new(), directed: false,
        });

        let mut seeds = HashMap::new();
        seeds.insert("a".into(), 1.0);

        let result = activation_spread(&g, &seeds, 0.85, 30);
        assert!(result.get("a").copied().unwrap_or(0.0) > 0.0);
        assert!(result.get("b").copied().unwrap_or(0.0) > 0.0);
    }
}
