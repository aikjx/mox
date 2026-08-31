//! 中心性算法
//!
//! - 介数中心性 (Brandes 2001)
//! - 紧密中心性 (Harmonic Closeness)

use std::collections::{HashMap, VecDeque};
use mox_kg_meta_core::{Graph, NodeId};

/// 中心性结果
pub type CentralityResult = HashMap<NodeId, f64>;

/// 介数中心性（Brandes 算法，简化版 BFS 无权图）
///
/// 计算每个节点作为最短路径中介的次数比例。
pub fn betweenness_centrality(graph: &Graph, normalized: bool) -> CentralityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let mut betweenness: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| (id.clone(), 0.0))
        .collect();

    for s in &node_ids {
        let mut stack: Vec<NodeId> = Vec::new();
        let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut sigma: HashMap<NodeId, f64> = HashMap::new();
        let mut dist: HashMap<NodeId, i32> = HashMap::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();

        for v in &node_ids {
            predecessors.insert(v.clone(), Vec::new());
            sigma.insert(v.clone(), 0.0);
            dist.insert(v.clone(), -1);
        }
        sigma.insert(s.clone(), 1.0);
        dist.insert(s.clone(), 0);
        queue.push_back(s.clone());

        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            for w in graph.neighbors(&v) {
                let w = w.clone();
                // 首次发现
                if *dist.get(&w).unwrap() == -1 {
                    dist.insert(w.clone(), *dist.get(&v).unwrap() + 1);
                    queue.push_back(w.clone());
                }
                // 最短路径经过 v
                if *dist.get(&w).unwrap() == *dist.get(&v).unwrap() + 1 {
                    let sv = *sigma.get(&v).unwrap();
                    *sigma.get_mut(&w).unwrap() += sv;
                    predecessors.get_mut(&w).unwrap().push(v.clone());
                }
            }
        }

        let mut delta: HashMap<NodeId, f64> = node_ids
            .iter()
            .map(|id| (id.clone(), 0.0))
            .collect();

        while let Some(w) = stack.pop() {
            for v in predecessors.get(&w).unwrap() {
                let ratio = sigma.get(v).unwrap() / sigma.get(&w).unwrap();
                let dw = *delta.get(&w).unwrap();
                *delta.get_mut(v).unwrap() += ratio * (1.0 + dw);
            }
            if &w != s {
                *betweenness.get_mut(&w).unwrap() += *delta.get(&w).unwrap();
            }
        }
    }

    // 有向图除以 2，标准化
    let n = graph.node_count() as f64;
    if normalized && n > 2.0 {
        let factor = 1.0 / ((n - 1.0) * (n - 2.0));
        betweenness.iter_mut().for_each(|(_, v)| *v *= factor);
    }

    betweenness
}

/// 紧密中心性（Harmonic Closeness）
///
/// 使用调和平均，解决不可达节点的问题。
pub fn harmonic_closeness(graph: &Graph) -> CentralityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let mut result = HashMap::new();
    let n = graph.node_count() as f64;

    for s in &node_ids {
        let mut dist: HashMap<NodeId, f64> = HashMap::new();
        let mut visited: HashMap<NodeId, bool> = HashMap::new();
        let mut queue: VecDeque<(NodeId, i32)> = VecDeque::new();

        for v in &node_ids {
            dist.insert(v.clone(), f64::INFINITY);
            visited.insert(v.clone(), false);
        }
        dist.insert(s.clone(), 0.0);
        visited.insert(s.clone(), true);
        queue.push_back((s.clone(), 0));

        while let Some((v, d)) = queue.pop_front() {
            for w in graph.neighbors(&v) {
                let w = w.clone();
                if !*visited.get(&w).unwrap() {
                    visited.insert(w.clone(), true);
                    dist.insert(w.clone(), (d + 1) as f64);
                    queue.push_back((w.clone(), d + 1));
                }
            }
        }

        let harmonic_sum: f64 = dist
            .values()
            .filter(|&&d| d > 0.0 && d.is_finite())
            .map(|&d| 1.0 / d)
            .sum();

        let closeness = if n > 1.0 { harmonic_sum / (n - 1.0) } else { 0.0 };
        result.insert(s.clone(), closeness);
    }

    result
}
