// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 知识图谱图算法（mox 模块化系统架构）：邻接表 / 邻居 / Dijkstra 最短路径 / 中心性 / PageRank / 社区发现 / 激活传播
//!
//! 由内存边表（DashMap<String, Value>）构建无向带权邻接表，再执行各类图算法，
//! 供 `/api/graph/*` 系列接口使用，替代原先的空壳 stub。

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

/// 从 (src, target, weight) 迭代器构建无向带权邻接表
pub fn adjacency_from_edges(
    edges: impl Iterator<Item = (String, String, f64)>,
) -> HashMap<String, Vec<(String, f64)>> {
    let mut adj: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for (src, tgt, w) in edges {
        if src.is_empty() || tgt.is_empty() {
            continue;
        }
        adj.entry(src.clone()).or_default().push((tgt.clone(), w));
        adj.entry(tgt).or_default().push((src, w));
    }
    adj
}

/// 从 AppState 图边表构建邻接表
pub fn adjacency_from_state(state: &super::AppState) -> HashMap<String, Vec<(String, f64)>> {
    adjacency_from_edges(state.graph_edges.iter().filter_map(|e| {
        let v = e.value();
        let src = v.get("source")?.as_str()?.to_string();
        let tgt = v.get("target")?.as_str()?.to_string();
        let w = v.get("weight").and_then(|x| x.as_f64()).unwrap_or(1.0);
        Some((src, tgt, w))
    }))
}

/// 节点邻居，按权重降序
pub fn neighbors(adj: &HashMap<String, Vec<(String, f64)>>, id: &str) -> Vec<(String, f64)> {
    let mut ns = adj.get(id).cloned().unwrap_or_default();
    ns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    ns
}

/// Dijkstra 最短路径，返回 (路径节点序列, 总权重)；不可达时返回空路径
pub fn shortest_path(
    adj: &HashMap<String, Vec<(String, f64)>>,
    start: &str,
    end: &str,
) -> (Vec<String>, f64) {
    if start == end {
        return (vec![start.to_string()], 0.0);
    }
    if !adj.contains_key(start) || !adj.contains_key(end) {
        return (Vec::new(), 0.0);
    }
    // 小顶堆：自定义 Ord 使 dist 小的优先弹出
    #[derive(PartialEq)]
    struct Item(f64, String);
    impl Eq for Item {}
    impl PartialOrd for Item {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            other.0.partial_cmp(&self.0)
        }
    }
    impl Ord for Item {
        fn cmp(&self, other: &Self) -> Ordering {
            other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
        }
    }

    let mut dist: HashMap<String, f64> = HashMap::new();
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(start.to_string(), 0.0);
    heap.push(Item(0.0, start.to_string()));

    while let Some(Item(d, u)) = heap.pop() {
        if d > dist.get(&u).copied().unwrap_or(f64::INFINITY) {
            continue;
        }
        if u == end {
            break;
        }
        if let Some(nbs) = adj.get(&u) {
            for (v, w) in nbs {
                let nd = d + w;
                if nd < dist.get(v).copied().unwrap_or(f64::INFINITY) {
                    dist.insert(v.clone(), nd);
                    prev.insert(v.clone(), u.clone());
                    heap.push(Item(nd, v.clone()));
                }
            }
        }
    }

    if !dist.contains_key(end) {
        return (Vec::new(), 0.0);
    }
    let mut path = vec![end.to_string()];
    let mut cur = end.to_string();
    while let Some(p) = prev.get(&cur) {
        if p == &start {
            break;
        }
        cur = p.clone();
        path.push(cur.clone());
    }
    path.push(start.to_string());
    path.reverse();
    (path, dist.get(end).copied().unwrap_or(0.0))
}

/// 度中心性：返回 node -> (degree, normalized)
pub fn degree_centrality(adj: &HashMap<String, Vec<(String, f64)>>) -> HashMap<String, (usize, f64)> {
    let n = adj.len();
    let norm = |deg: usize| -> f64 {
        if n <= 1 {
            0.0
        } else {
            deg as f64 / (n - 1) as f64
        }
    };
    adj.iter()
        .map(|(id, nbs)| {
            // 邻接表可能含重复自环，去重后计数
            let mut seen = HashSet::new();
            for (nb, _) in nbs {
                seen.insert(nb.clone());
            }
            let deg = seen.len();
            (id.clone(), (deg, norm(deg)))
        })
        .collect()
}

/// 中介中心性（Brandes BFS 无权版）
pub fn betweenness(adj: &HashMap<String, Vec<(String, f64)>>) -> HashMap<String, f64> {
    let mut cb: HashMap<String, f64> = HashMap::new();
    let nodes: Vec<String> = adj.keys().cloned().collect();
    for s in &nodes {
        // BFS
        let mut stack: Vec<String> = Vec::new();
        let mut pred: HashMap<String, Vec<String>> = HashMap::new();
        let mut sigma: HashMap<String, f64> = HashMap::new();
        let mut dist: HashMap<String, i32> = HashMap::new();
        for n in &nodes {
            dist.insert(n.clone(), -1);
            sigma.insert(n.clone(), 0.0);
        }
        dist.insert(s.clone(), 0);
        sigma.insert(s.clone(), 1.0);
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(s.clone());
        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            if let Some(nbs) = adj.get(&v) {
                for (w, _) in nbs {
                    if dist.get(w).copied().unwrap_or(-1) < 0 {
                        dist.insert(w.clone(), dist.get(&v).copied().unwrap_or(0) + 1);
                        queue.push_back(w.clone());
                    }
                    if dist.get(w).copied().unwrap_or(-1) == dist.get(&v).copied().unwrap_or(0) + 1 {
                        let sv = sigma.get(&v).copied().unwrap_or(0.0);
                        *sigma.entry(w.clone()).or_insert(0.0) += sv;
                        pred.entry(w.clone()).or_default().push(v.clone());
                    }
                }
            }
        }
        let mut delta: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
        while let Some(w) = stack.pop() {
            if let Some(prs) = pred.get(&w) {
                for v in prs {
                    let sw = sigma.get(&w).copied().unwrap_or(0.0);
                    let sv = sigma.get(v).copied().unwrap_or(0.0);
                    if sw > 0.0 {
                        let coeff = (sv / sw) * (1.0 + delta.get(&w).copied().unwrap_or(0.0));
                        *delta.entry(v.clone()).or_insert(0.0) += coeff;
                    }
                }
            }
            if w != *s {
                let dv = delta.get(&w).copied().unwrap_or(0.0);
                *cb.entry(w.clone()).or_insert(0.0) += dv;
            }
        }
    }
    cb
}

/// PageRank（迭代式，默认 0.85 阻尼，100 轮）
pub fn pagerank(adj: &HashMap<String, Vec<(String, f64)>>, damping: f64, iterations: usize) -> HashMap<String, f64> {
    let n = adj.len();
    let mut pr: HashMap<String, f64> = HashMap::new();
    let mut out: HashMap<String, f64> = HashMap::new();
    for (id, nbs) in adj {
        pr.insert(id.clone(), 1.0 / n as f64);
        out.insert(id.clone(), nbs.len() as f64);
    }
    let base = (1.0 - damping) / n as f64;
    for _ in 0..iterations {
        let mut next: HashMap<String, f64> = HashMap::new();
        for (id, nbs) in adj {
            let mut acc = 0.0;
            for (nb, _) in nbs {
                let pr_v = pr.get(nb).copied().unwrap_or(0.0);
                let out_v = out.get(nb).copied().unwrap_or(0.0);
                if out_v > 0.0 {
                    acc += pr_v / out_v;
                }
            }
            next.insert(id.clone(), base + damping * acc);
        }
        pr = next;
    }
    pr
}

/// 社区发现：标签传播（Label Propagation，20 轮）
pub fn label_propagation(adj: &HashMap<String, Vec<(String, f64)>>) -> Vec<(String, Vec<String>)> {
    let nodes: Vec<String> = adj.keys().cloned().collect();
    let mut labels: HashMap<String, String> = nodes.iter().map(|n| (n.clone(), n.clone())).collect();
    for _ in 0..20 {
        let mut changed = false;
        for n in &nodes {
            let mut counts: HashMap<String, usize> = HashMap::new();
            if let Some(nbs) = adj.get(n) {
                for (nb, _) in nbs {
                    if let Some(l) = labels.get(nb) {
                        *counts.entry(l.clone()).or_insert(0) += 1;
                    }
                }
            }
            if counts.is_empty() {
                continue;
            }
            let best = counts
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
                .map(|(l, _)| l);
            if let Some(b) = best {
                if labels.get(n) != Some(&b) {
                    labels.insert(n.clone(), b);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for (n, l) in labels {
        groups.entry(l).or_default().push(n);
    }
    let mut out: Vec<(String, Vec<String>)> = groups.into_iter().collect();
    out.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    out
}

/// 激活传播：从种子节点沿边扩散（迭代衰减）
pub fn activation_spread(
    adj: &HashMap<String, Vec<(String, f64)>>,
    seeds: &[String],
    iterations: usize,
) -> HashMap<String, f64> {
    let mut energy: HashMap<String, f64> = HashMap::new();
    for s in seeds {
        energy.insert(s.clone(), 1.0);
    }
    for _ in 0..iterations {
        let mut next: HashMap<String, f64> = energy.clone();
        for (node, en) in &energy {
            if *en <= 0.0 {
                continue;
            }
            if let Some(nbs) = adj.get(node) {
                for (nb, w) in nbs {
                    let delta = en * w * 0.4;
                    if delta > 1e-6 {
                        let e = next.entry(nb.clone()).or_insert(0.0);
                        *e += delta;
                    }
                }
            }
        }
        // 归一化防发散
        let maxv = next.values().cloned().fold(0.0_f64, f64::max);
        if maxv > 0.0 {
            for v in next.values_mut() {
                *v /= maxv;
            }
        }
        energy = next;
    }
    energy
}

/// 简易推荐：以上下文节点集为中心，按最近可达跳数给分
pub fn recommend(
    adj: &HashMap<String, Vec<(String, f64)>>,
    context: &[String],
    limit: usize,
) -> Vec<(String, f64)> {
    let mut score: HashMap<String, f64> = HashMap::new();
    for c in context {
        if let Some(nbs) = adj.get(c) {
            for (nb, w) in nbs {
                if context.contains(nb) {
                    continue;
                }
                let e = score.entry(nb.clone()).or_insert(0.0);
                *e += 1.0 / (1.0 + w);
            }
        }
    }
    let mut ranked: Vec<(String, f64)> = score.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    ranked.truncate(limit);
    ranked
}

/// 从图边表提取 (src, target, weight)，供外部使用
pub fn extract_edges(state: &super::AppState) -> Vec<(String, String, f64)> {
    state
        .graph_edges
        .iter()
        .filter_map(|e| {
            let v = e.value();
            let src = v.get("source")?.as_str()?.to_string();
            let tgt = v.get("target")?.as_str()?.to_string();
            let w = v.get("weight").and_then(|x| x.as_f64()).unwrap_or(1.0);
            Some((src, tgt, w))
        })
        .collect()
}
