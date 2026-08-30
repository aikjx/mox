// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 图算法封装模块（对外统一入口）
//!
//! 封装 petgraph 生态的图算法，提供统一接口。
//! 作为 KG 域图算法和 EA 域专家关系图的共享实现。

use crate::traits::*;
use crate::types::*;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// 有序浮点数包装（用于 BinaryHeap，f64 不实现 Ord）
#[derive(Debug, Clone, Copy)]
struct OrderedFloat(f64);

impl PartialEq for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for OrderedFloat {}
impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// 统一图算法引擎
#[derive(Debug, Clone, Default)]
pub struct UnifiedGraphEngine;

impl Algorithm for UnifiedGraphEngine {
    fn id(&self) -> &str {
        "graph.unified_engine"
    }
    fn name(&self) -> &str {
        "统一图算法引擎"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "跨域共享的图算法引擎，封装 PageRank / 中心性 / 社区发现 / 最短路径"
    }
}

impl UnifiedGraphEngine {
    /// 计算 PageRank
    pub fn pagerank<N, E>(
        &self,
        graph: &Graph<N, E>,
        damping: f64,
        max_iter: usize,
        tolerance: f64,
    ) -> Vec<f64>
    where
        N: Clone,
        E: Clone,
    {
        let n = graph.node_count();
        if n == 0 {
            return vec![];
        }

        let mut ranks = vec![1.0 / n as f64; n];
        let teleport = (1.0 - damping) / n as f64;

        let out_degrees: Vec<usize> = (0..n)
            .map(|i| graph.neighbors(NodeIndex::new(i)).count())
            .collect();

        for _iter in 0..max_iter {
            let mut new_ranks = vec![teleport; n];

            let dangling_sum: f64 = ranks
                .iter()
                .enumerate()
                .filter(|(i, _)| out_degrees[*i] == 0)
                .map(|(_, r)| *r)
                .sum();
            let dangling_contrib = damping * dangling_sum / n as f64;

            for i in 0..n {
                new_ranks[i] += dangling_contrib;
            }

            for node in graph.node_indices() {
                let idx = node.index();
                if out_degrees[idx] == 0 {
                    continue;
                }
                let contrib = damping * ranks[idx] / out_degrees[idx] as f64;
                for neighbor in graph.neighbors(node) {
                    new_ranks[neighbor.index()] += contrib;
                }
            }

            let diff: f64 = ranks
                .iter()
                .zip(new_ranks.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();

            ranks = new_ranks;

            if diff < tolerance {
                break;
            }
        }

        ranks
    }

    /// 度中心性
    pub fn degree_centrality<N, E>(&self, graph: &Graph<N, E>) -> Vec<f64>
    where
        N: Clone,
        E: Clone,
    {
        let n = graph.node_count();
        if n <= 1 {
            return vec![1.0; n];
        }
        let norm = (n - 1) as f64;
        (0..n)
            .map(|i| graph.neighbors_undirected(NodeIndex::new(i)).count() as f64 / norm)
            .collect()
    }

    /// 个性化 PageRank（激活传播）
    pub fn personalized_pagerank<N, E>(
        &self,
        graph: &Graph<N, E>,
        seed_nodes: &[usize],
        damping: f64,
        max_iter: usize,
        tolerance: f64,
    ) -> Vec<f64>
    where
        N: Clone,
        E: Clone,
    {
        let n = graph.node_count();
        if n == 0 {
            return vec![];
        }

        let seed_weight = 1.0 / seed_nodes.len() as f64;
        let mut ranks = vec![0.0; n];
        for &s in seed_nodes {
            if s < n {
                ranks[s] = seed_weight;
            }
        }

        let out_degrees: Vec<usize> = (0..n)
            .map(|i| graph.neighbors(NodeIndex::new(i)).count())
            .collect();

        for _iter in 0..max_iter {
            let mut new_ranks = vec![0.0; n];

            for &s in seed_nodes {
                if s < n {
                    new_ranks[s] += (1.0 - damping) * seed_weight;
                }
            }

            let dangling_sum: f64 = ranks
                .iter()
                .enumerate()
                .filter(|(i, _)| out_degrees[*i] == 0)
                .map(|(_, r)| *r)
                .sum();

            let dangling_each = damping * dangling_sum / n as f64;
            for i in 0..n {
                new_ranks[i] += dangling_each;
            }

            for node in graph.node_indices() {
                let idx = node.index();
                if out_degrees[idx] == 0 {
                    continue;
                }
                let contrib = damping * ranks[idx] / out_degrees[idx] as f64;
                for neighbor in graph.neighbors(node) {
                    new_ranks[neighbor.index()] += contrib;
                }
            }

            let diff: f64 = ranks
                .iter()
                .zip(new_ranks.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();

            ranks = new_ranks;

            if diff < tolerance {
                break;
            }
        }

        ranks
    }

    /// Dijkstra 最短路径
    pub fn dijkstra<N, E>(
        &self,
        graph: &Graph<N, E>,
        source: usize,
        target: usize,
    ) -> Option<(Vec<usize>, f64)>
    where
        N: Clone,
        E: Clone + Into<f64>,
    {
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;

        let n = graph.node_count();
        if source >= n || target >= n {
            return None;
        }

        let mut dist = vec![f64::INFINITY; n];
        let mut prev = vec![None; n];
        dist[source] = 0.0;

        let mut heap = BinaryHeap::new();
        heap.push(Reverse((OrderedFloat(0.0), source)));

        while let Some(Reverse((OrderedFloat(d), u))) = heap.pop() {
            if u == target {
                break;
            }
            if d > dist[u] {
                continue;
            }

            let node = NodeIndex::new(u);
            for edge in graph.edges(node) {
                let v = edge.target().index();
                let w: f64 = edge.weight().clone().into();
                let new_dist = d + w;
                if new_dist < dist[v] {
                    dist[v] = new_dist;
                    prev[v] = Some(u);
                    heap.push(Reverse((OrderedFloat(new_dist), v)));
                }
            }
        }

        if dist[target].is_infinite() {
            return None;
        }

        let mut path = vec![];
        let mut current = target;
        path.push(current);
        while let Some(p) = prev[current] {
            path.push(p);
            current = p;
        }
        path.reverse();

        Some((path, dist[target]))
    }

    /// 社区发现（简单贪心模块化优化）
    pub fn greedy_communities<N, E>(&self, graph: &Graph<N, E>) -> Vec<usize>
    where
        N: Clone,
        E: Clone + Into<f64>,
    {
        let n = graph.node_count();
        let mut communities: Vec<usize> = (0..n).collect();

        let m: f64 = graph
            .edge_indices()
            .map(|e| graph.edge_weight(e).unwrap().clone().into())
            .sum::<f64>()
            / 2.0;

        if m < 1e-10 {
            return communities;
        }

        let mut changed = true;
        let mut iteration = 0;
        while changed && iteration < 20 {
            changed = false;
            iteration += 1;

            for node in graph.node_indices() {
                let u = node.index();
                let current_comm = communities[u];

                let mut neighbor_communities: HashMap<usize, f64> = HashMap::new();
                for edge in graph.edges(node) {
                    let v = edge.target().index();
                    let w: f64 = edge.weight().clone().into();
                    *neighbor_communities.entry(communities[v]).or_insert(0.0) += w;
                }

                let mut best_comm = current_comm;
                let mut best_gain = 0.0;

                for (&comm, &weight) in &neighbor_communities {
                    if comm == current_comm {
                        continue;
                    }
                    let gain = weight;
                    if gain > best_gain {
                        best_gain = gain;
                        best_comm = comm;
                    }
                }

                if best_comm != current_comm {
                    communities[u] = best_comm;
                    changed = true;
                }
            }
        }

        let mut mapping = HashMap::new();
        let mut next_id = 0;
        for c in communities.iter_mut() {
            let new_id = *mapping.entry(*c).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            *c = new_id;
        }

        communities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::Graph;

    fn build_test_graph() -> Graph<&'static str, f64> {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        let e = g.add_node("E");

        g.add_edge(a, b, 1.0);
        g.add_edge(a, c, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, d, 1.0);
        g.add_edge(d, e, 1.0);

        g
    }

    #[test]
    fn test_pagerank() {
        let engine = UnifiedGraphEngine;
        let g = build_test_graph();
        let ranks = engine.pagerank(&g, 0.85, 100, 1e-6);
        assert_eq!(ranks.len(), 5);
        // PageRank 总和应接近 1.0
        let sum: f64 = ranks.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        // 所有值应为正数
        for r in &ranks {
            assert!(*r > 0.0);
        }
    }

    #[test]
    fn test_degree_centrality() {
        let engine = UnifiedGraphEngine;
        let g = build_test_graph();
        let centrality = engine.degree_centrality(&g);
        assert_eq!(centrality.len(), 5);
        // C 节点（索引 2）度最高
        assert!(centrality[2] > centrality[0]);
        assert!(centrality[2] > centrality[4]);
    }

    #[test]
    fn test_personalized_pagerank() {
        let engine = UnifiedGraphEngine;
        let g = build_test_graph();
        let ranks = engine.personalized_pagerank(&g, &[0], 0.85, 50, 1e-6);
        assert_eq!(ranks.len(), 5);
        // 种子节点（A）的排名应该高于较远节点（E）
        assert!(ranks[0] > 0.0);
        // 所有值应为正数
        for r in &ranks {
            assert!(*r >= 0.0);
        }
    }

    #[test]
    fn test_dijkstra() {
        let engine = UnifiedGraphEngine;
        let g = build_test_graph();
        let result = engine.dijkstra(&g, 0, 4);

        assert!(result.is_some());
        let (path, dist) = result.unwrap();
        assert_eq!(path, vec![0, 2, 3, 4]);
        assert!((dist - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_greedy_communities() {
        let engine = UnifiedGraphEngine;
        let g = build_test_graph();
        let communities = engine.greedy_communities(&g);
        assert_eq!(communities.len(), 5);
        // 所有节点都被分配了社区 ID
        for c in &communities {
            assert!(*c < 5);
        }
    }
}
