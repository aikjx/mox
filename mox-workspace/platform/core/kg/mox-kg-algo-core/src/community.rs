//! 社区检测算法
//!
//! CNM (Clauset-Newman-Moore) 模块度贪心凝聚算法

use std::collections::{HashMap, HashSet};
use mox_kg_meta_core::{Graph, NodeId};

/// 社区检测结果：社区 ID -> 节点 ID 列表
pub type CommunityResult = HashMap<usize, Vec<NodeId>>;

/// CNM 社区检测（模块度贪心）
///
/// 自底向上的层次聚类，每次合并使模块度增益最大的两个社区。
pub fn cnm_community(graph: &Graph) -> CommunityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let n = graph.node_count();
    let m = graph.edge_count() as f64;

    if n == 0 || m == 0.0 {
        let mut result = HashMap::new();
        for (i, id) in node_ids.iter().enumerate() {
            result.insert(i, vec![id.clone()]);
        }
        return result;
    }

    // 初始化：每个节点一个社区
    let mut node_community: HashMap<NodeId, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    let mut communities: HashMap<usize, HashSet<NodeId>> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (i, HashSet::from([id.clone()])))
        .collect();

    // 计算每个社区的度
    let mut community_degree: HashMap<usize, f64> = HashMap::new();
    for (cid, nodes) in &communities {
        let deg: f64 = nodes.iter().map(|id| graph.neighbors(id).len() as f64).sum();
        community_degree.insert(*cid, deg);
    }

    // 初始模块度
    let mut max_modularity = -1.0;
    let mut best_communities = communities.clone();

    // 贪心合并
    let mut num_communities = n;

    while num_communities > 1 {
        let mut best_gain = f64::NEG_INFINITY;
        let mut best_pair: Option<(usize, usize)> = None;

        // 遍历所有可能的社区对（简化：找相邻社区对）
        let community_list: Vec<usize> = communities.keys().copied().collect();

        for i in 0..community_list.len() {
            for j in (i + 1)..community_list.len() {
                let ci = community_list[i];
                let cj = community_list[j];

                // 检查是否相邻（有边连接）
                let mut has_connection = false;
                'outer: for ni in communities.get(&ci).unwrap() {
                    for nj in communities.get(&cj).unwrap() {
                        // 简化判断：互为邻居
                        let neighbors_i = graph.neighbors(ni);
                        if neighbors_i.iter().any(|&id| id == nj) {
                            has_connection = true;
                            break 'outer;
                        }
                    }
                }

                if !has_connection {
                    continue;
                }

                // 计算模块度增益 ΔQ
                let ki = community_degree.get(&ci).copied().unwrap_or(0.0);
                let kj = community_degree.get(&cj).copied().unwrap_or(0.0);

                // 计算两社区之间的边数
                let mut e_ij = 0.0;
                for ni in communities.get(&ci).unwrap() {
                    for nj in communities.get(&cj).unwrap() {
                        let neighbors_i = graph.neighbors(ni);
                        if neighbors_i.iter().any(|&id| id == nj) {
                            e_ij += 1.0;
                        }
                    }
                }

                let delta_q = (e_ij / m) - (ki * kj) / (2.0 * m * m);

                if delta_q > best_gain {
                    best_gain = delta_q;
                    best_pair = Some((ci, cj));
                }
            }
        }

        if best_pair.is_none() || best_gain <= 0.0 {
            break;
        }

        let (ci, cj) = best_pair.unwrap();

        // 合并社区
        let cj_nodes = communities.remove(&cj).unwrap();
        let ci_nodes = communities.get_mut(&ci).unwrap();
        for node in &cj_nodes {
            node_community.insert(node.clone(), ci);
            ci_nodes.insert(node.clone());
        }

        // 更新度
        let cj_deg = community_degree.remove(&cj).unwrap_or(0.0);
        *community_degree.get_mut(&ci).unwrap() += cj_deg;

        num_communities -= 1;

        // 计算当前模块度（简化）
        let mut modularity = 0.0;
        for (_, nodes) in &communities {
            let mut internal_edges = 0.0;
            let mut total_degree = 0.0;
            for node in nodes {
                total_degree += graph.neighbors(node).len() as f64;
                for neighbor in graph.neighbors(node) {
                    if nodes.contains(neighbor) {
                        internal_edges += 1.0;
                    }
                }
            }
            internal_edges /= 2.0; // 每条边计了两次
            modularity += internal_edges / m - (total_degree / (2.0 * m)).powi(2);
        }

        if modularity > max_modularity {
            max_modularity = modularity;
            best_communities = communities.clone();
        }
    }

    // 转换输出格式
    let mut result = HashMap::new();
    for (i, (_, nodes)) in best_communities.into_iter().enumerate() {
        result.insert(i, nodes.into_iter().collect());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let result = cnm_community(&g);
        assert!(result.is_empty());
    }
}
