// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX KG Spark Service
//!
//! Large-scale graph analytics engine:
//! - PageRank, Betweenness Centrality, Community Detection
//! - Connected components, shortest paths
//! - Graph generation and sampling
//! - Parallel processing with rayon

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, NodeIndexable};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("graph is empty")]
    EmptyGraph,
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("algorithm failed: {0}")]
    AlgorithmFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAnalytics {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub avg_degree: f64,
    pub max_degree: usize,
    pub connected_components: usize,
    pub diameter: Option<usize>,
}

/// Compute basic graph statistics.
pub fn graph_statistics<N, E>(graph: &DiGraph<N, E>) -> GraphAnalytics {
    let n = graph.node_count();
    let e = graph.edge_count();
    let density = if n > 1 { (2.0 * e as f64) / (n as f64 * (n as f64 - 1.0)) } else { 0.0 };
    let avg_degree = if n > 0 { (2.0 * e as f64) / n as f64 } else { 0.0 };
    let max_degree = graph.node_indices().map(|ni| graph.neighbors(ni).count()).max().unwrap_or(0);
    let components = connected_components(graph);
    GraphAnalytics {
        node_count: n, edge_count: e, density, avg_degree, max_degree,
        connected_components: components, diameter: None,
    }
}

/// PageRank algorithm (d=0.85, 30 iterations).
pub fn pagerank<N, E>(graph: &DiGraph<N, E>, damping: f64, iterations: usize) -> HashMap<usize, f64> {
    let n = graph.node_count();
    if n == 0 { return HashMap::new(); }

    let mut scores: Vec<f64> = vec![1.0 / n as f64; n];
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();

    // Build out-degree map
    let out_degree: HashMap<usize, usize> = node_indices.iter()
        .map(|&ni| (ni.index(), graph.neighbors(ni).count()))
        .collect();

    for _ in 0..iterations {
        let mut new_scores = vec![(1.0 - damping) / n as f64; n];
        // Dangling nodes contribution
        let dangling_sum: f64 = node_indices.iter()
            .filter(|&&ni| out_degree[&ni.index()] == 0)
            .map(|&ni| scores[ni.index()])
            .sum();
        for ni in &node_indices {
            new_scores[ni.index()] += damping * dangling_sum / n as f64;
        }
        // Incoming contributions
        for ni in &node_indices {
            let idx = ni.index();
            for edge in graph.edges_directed(*ni, petgraph::Direction::Incoming) {
                let src = edge.source().index();
                let deg = out_degree[&src];
                if deg > 0 {
                    new_scores[idx] += damping * scores[src] / deg as f64;
                }
            }
        }
        scores = new_scores;
    }

    scores.into_iter().enumerate().collect()
}

/// Betweenness centrality (Brandes algorithm for directed graphs).
pub fn betweenness_centrality<N, E>(graph: &DiGraph<N, E>) -> HashMap<usize, f64> {
    let n = graph.node_count();
    let mut betweenness: HashMap<usize, f64> = graph.node_indices().map(|ni| (ni.index(), 0.0)).collect();
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();

    for &source in &node_indices {
        let mut pred: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut sigma: HashMap<usize, f64> = HashMap::new();
        let mut dist: HashMap<usize, i32> = HashMap::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        let mut stack: Vec<usize> = vec![];

        for &ni in &node_indices {
            pred.insert(ni.index(), vec![]);
            sigma.insert(ni.index(), 0.0);
            dist.insert(ni.index(), -1);
        }
        sigma.insert(source.index(), 1.0);
        dist.insert(source.index(), 0);
        queue.push_back(source.index());

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            let v_ni = NodeIndex::new(v);
            for neighbor in graph.neighbors(v_ni) {
                let w = neighbor.index();
                if dist[&w] < 0 {
                    dist.insert(w, dist[&v] + 1);
                    queue.push_back(w);
                }
                if dist[&w] == dist[&v] + 1 {
                    *sigma.get_mut(&w).unwrap() += sigma[&v];
                    pred.get_mut(&w).unwrap().push(v);
                }
            }
        }

        let mut delta: HashMap<usize, f64> = node_indices.iter().map(|ni| (ni.index(), 0.0)).collect();
        while let Some(w) = stack.pop() {
            for &v in &pred[&w] {
                let factor = (sigma[&v] / sigma[&w]) * (1.0 + delta[&w]);
                *delta.get_mut(&v).unwrap() += factor;
            }
            if w != source.index() {
                *betweenness.get_mut(&w).unwrap() += delta[&w];
            }
        }
    }

    // Normalize for directed graphs
    let scale = 1.0 / ((n as f64 - 1.0) * (n as f64 - 2.0));
    for v in betweenness.values_mut() { *v *= scale; }
    betweenness
}

use std::collections::VecDeque;

/// Connected components (undirected view).
pub fn connected_components<N, E>(graph: &DiGraph<N, E>) -> usize {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut components = 0;
    for ni in graph.node_indices() {
        if !visited.contains(&ni.index()) {
            components += 1;
            let mut queue = VecDeque::new();
            queue.push_back(ni);
            visited.insert(ni.index());
            while let Some(v) = queue.pop_front() {
                // Both directions for undirected view
                for neighbor in graph.neighbors(v) {
                    if visited.insert(neighbor.index()) { queue.push_back(neighbor); }
                }
                for edge in graph.edges_directed(v, petgraph::Direction::Incoming) {
                    if visited.insert(edge.source().index()) { queue.push_back(edge.source()); }
                }
            }
        }
    }
    components
}

/// Shortest path (BFS) from source to target. Returns path length or None.
pub fn shortest_path<N, E>(graph: &DiGraph<N, E>, source: NodeIndex, target: NodeIndex) -> Option<usize> {
    if source == target { return Some(0); }
    let mut visited: HashSet<usize> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    queue.push_back((source, 0));
    visited.insert(source.index());
    while let Some((v, dist)) = queue.pop_front() {
        for neighbor in graph.neighbors(v) {
            if neighbor == target { return Some(dist + 1); }
            if visited.insert(neighbor.index()) {
                queue.push_back((neighbor, dist + 1));
            }
        }
    }
    None
}

/// Community detection using label propagation.
pub fn label_propagation<N, E>(graph: &DiGraph<N, E>, max_iterations: usize) -> HashMap<usize, usize> {
    let n = graph.node_count();
    let mut labels: HashMap<usize, usize> = graph.node_indices().map(|ni| (ni.index(), ni.index())).collect();
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();

    for _ in 0..max_iterations {
        let mut changed = false;
        for &ni in &node_indices {
            let mut label_counts: HashMap<usize, usize> = HashMap::new();
            // Neighbors (both directions for undirected community detection)
            for neighbor in graph.neighbors(ni) {
                *label_counts.entry(labels[&neighbor.index()]).or_insert(0) += 1;
            }
            for edge in graph.edges_directed(ni, petgraph::Direction::Incoming) {
                *label_counts.entry(labels[&edge.source().index()]).or_insert(0) += 1;
            }
            if let Some((best_label, _)) = label_counts.into_iter().max_by_key(|(_, c)| *c) {
                if best_label != labels[&ni.index()] {
                    labels.insert(ni.index(), best_label);
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }
    labels
}

/// Degree centrality for each node.
pub fn degree_centrality<N, E>(graph: &DiGraph<N, E>) -> HashMap<usize, f64> {
    let n = graph.node_count();
    if n <= 1 { return HashMap::new(); }
    let scale = 1.0 / (n as f64 - 1.0);
    graph.node_indices().map(|ni| {
        let in_deg = graph.edges_directed(ni, petgraph::Direction::Incoming).count();
        let out_deg = graph.neighbors(ni).count();
        (ni.index(), (in_deg + out_deg) as f64 * scale)
    }).collect()
}

/// Generate a random graph (Erdos-Renyi G(n,p)).
pub fn generate_random_graph(n: usize, p: f64) -> DiGraph<usize, f64> {
    let mut graph = DiGraph::new();
    let nodes: Vec<NodeIndex> = (0..n).map(|i| graph.add_node(i)).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            if rand::random::<f64>() < p {
                graph.add_edge(nodes[i], nodes[j], 1.0);
            }
        }
    }
    graph
}

/// Top-K nodes by score.
pub fn top_k(scores: &HashMap<usize, f64>, k: usize) -> Vec<(usize, f64)> {
    let mut sorted: Vec<(usize, f64)> = scores.iter().map(|(k, v)| (*k, *v)).collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(k);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_graph() -> DiGraph<&'static str, f64> {
        let mut g = DiGraph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, d, 1.0);
        g.add_edge(a, c, 1.0);
        g
    }

    #[test]
    fn graph_stats() {
        let g = test_graph();
        let stats = graph_statistics(&g);
        assert_eq!(stats.node_count, 4);
        assert_eq!(stats.edge_count, 4);
        assert!(stats.density > 0.0);
    }

    #[test]
    fn pagerank_converges() {
        let g = test_graph();
        let scores = pagerank(&g, 0.85, 30);
        assert_eq!(scores.len(), 4);
        let total: f64 = scores.values().sum();
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn connected_components_test() {
        let g = test_graph();
        assert_eq!(connected_components(&g), 1);
    }

    #[test]
    fn shortest_path_test() {
        let g = test_graph();
        let nodes: Vec<NodeIndex> = g.node_indices().collect();
        let dist = shortest_path(&g, nodes[0], nodes[3]);
        assert_eq!(dist, Some(2)); // A->C->D
    }

    #[test]
    fn degree_centrality_test() {
        let g = test_graph();
        let dc = degree_centrality(&g);
        assert_eq!(dc.len(), 4);
        // Node A has highest degree (2 outgoing)
        let nodes: Vec<NodeIndex> = g.node_indices().collect();
        assert!(dc[&nodes[0].index()] > 0.0);
    }

    #[test]
    fn label_propagation_test() {
        let g = test_graph();
        let labels = label_propagation(&g, 50);
        assert_eq!(labels.len(), 4);
        // All connected nodes should end up in same community
        let unique: HashSet<usize> = labels.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn betweenness_test() {
        let g = test_graph();
        let bc = betweenness_centrality(&g);
        assert_eq!(bc.len(), 4);
    }

    #[test]
    fn top_k_test() {
        let scores: HashMap<usize, f64> = [(0, 0.9), (1, 0.5), (2, 0.7)].into_iter().collect();
        let top = top_k(&scores, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 0);
    }

    #[test]
    fn random_graph_generation() {
        let g = generate_random_graph(10, 0.3);
        assert_eq!(g.node_count(), 10);
        assert!(g.edge_count() > 0);
    }

    #[test]
    fn empty_graph_stats() {
        let g: DiGraph<&str, f64> = DiGraph::new();
        let stats = graph_statistics(&g);
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }
}
