// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AlgoBridge：7 算法纯内联实现（零外部 graph alg crate 依赖）。
//!
//! 护栏：
//! - PPR d=0.85, max_iter=30（T5 基线一致）
//! - CNM 社区（Louvain-like module degree greedy，最大化模块度 Q）
//! - Brandes betweenness（无权 BFS × 每源）
//! - Harmonic closeness（sum 1/dist）
//! - Density：无截断，完整 f64
//! - Degree：RAW 边双向展开（每条无向边两端 +1）
//! - LPA helper：`#[deprecated]` 空社区 stub

use std::collections::{HashMap, HashSet, VecDeque};

/// 护栏常量（copy T5 baseline）
pub const PPR_D: f64 = 0.85;
pub const PPR_MAX_ITER: usize = 30;

pub type Communities = Vec<Vec<String>>;

/// 内联 Graph 结构：无向边（边双向展开用）。
#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub nodes: HashSet<String>,
    /// edges 是“无向边”集合，存储为 (a, b)，a ≤ b 去重可选。
    /// degree_bidirectional / CNM / Brandes / Harmonic 都按“每条边贡献两端+1”展开。
    pub edges: Vec<(String, String)>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node<N: Into<String>>(&mut self, n: N) {
        self.nodes.insert(n.into());
    }

    /// 添加无向边（自动将两端入 nodes）。
    pub fn add_edge<A: Into<String>, B: Into<String>>(&mut self, a: A, b: B) {
        let ai: String = a.into();
        let bi: String = b.into();
        self.nodes.insert(ai.clone());
        self.nodes.insert(bi.clone());
        self.edges.push((ai, bi));
    }

    /// 构建邻接表：每条无向边双向展开。
    fn adj_bidir(&self) -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for n in &self.nodes {
            m.insert(n.clone(), Vec::new());
        }
        for (a, b) in &self.edges {
            m.get_mut(a).unwrap().push(b.clone());
            m.get_mut(b).unwrap().push(a.clone());
        }
        m
    }

    /// 有向邻接：仅 edges[i].0 → edges[i].1（PPR 用）。
    fn adj_out(&self) -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for n in &self.nodes {
            m.insert(n.clone(), Vec::new());
        }
        for (a, b) in &self.edges {
            m.get_mut(a).unwrap().push(b.clone());
        }
        m
    }
}

pub struct AlgoBridge;

impl AlgoBridge {
    /// Personalized PageRank / 激活扩散。
    /// - damping = PPR_D (0.85)
    /// - max_iter = PPR_MAX_ITER (30)
    /// 公式同 T5 single-source：
    ///   new = (1-d)/n + d * Σ(score[src]/deg_out) + d * dangling_mass / n
    ///   seed 初值偏置：初始 score 对 seed 加 1.0 再归一化。
    pub fn ppr(graph: &Graph, seed: &str, d: f64, max_iter: usize) -> HashMap<String, f64> {
        let nodes: Vec<String> = graph.nodes.iter().cloned().collect();
        let n = nodes.len() as f64;
        let mut score: HashMap<String, f64> = HashMap::new();
        if n == 0.0 {
            return score;
        }
        // 初值：均匀 + seed 偏置
        for nd in &nodes {
            score.insert(nd.clone(), 1.0 / n);
        }
        if graph.nodes.contains(seed) {
            *score.get_mut(seed).unwrap() += 1.0;
            let tot: f64 = score.values().sum();
            for v in score.values_mut() {
                *v /= tot;
            }
        }
        let adj = graph.adj_out();
        for _ in 0..max_iter {
            let mut new: HashMap<String, f64> =
                nodes.iter().map(|k| (k.clone(), (1.0 - d) / n)).collect();
            let dangling: f64 = d * score
                .iter()
                .filter(|(k, _)| adj.get(*k).map(|x| x.len()).unwrap_or(0) == 0)
                .map(|(_, v)| *v)
                .sum::<f64>()
                / n;
            for (s, vs) in &adj {
                let sz = vs.len() as f64;
                if sz > 0.0 {
                    let src_score = *score.get(s).unwrap_or(&0.0);
                    for dst in vs {
                        *new.get_mut(dst).unwrap() += d * src_score / sz;
                    }
                }
            }
            for v in new.values_mut() {
                *v += dangling;
            }
            score = new;
        }
        score
    }

    /// CNM 社区（模块度 greedy；Louvain-like 模块度 Q 最大化单层）。
    /// 实现：
    ///   1) 初始化：每个节点一个社区；
    ///   2) 对于每条无向边 (u,v)，若合并 ΔQ = Σ_t (Σ_in_t + 2*k_u_in_t)/(2m) - ((Σ_tot_t+k_u)/(2m))^2
    ///      这里使用简化版 greedy：遍历节点，选择把该节点加入邻居社区中 ΔQ 最大的一个。
    pub fn cnm(graph: &Graph) -> Communities {
        let mut nodes: Vec<String> = graph.nodes.iter().cloned().collect();
        nodes.sort(); // 确定性遍历顺序
        let mut node_comm: HashMap<String, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();
        let adj = graph.adj_bidir();
        let m2 = (2 * graph.edges.len()) as f64; // 2m

        if m2 == 0.0 {
            return nodes.into_iter().map(|n| vec![n]).collect();
        }

        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 20 {
            changed = false;
            iterations += 1;
            for u in &nodes {
                // 当前 community 总和（不含 u）
                let cur = node_comm[u];
                // Σ_tot(c)：社区 c 中所有点度数之和（每条无向边两端 +1，所以度=邻接表大小）
                let comm_total: HashMap<usize, f64> = {
                    let mut m: HashMap<usize, f64> = HashMap::new();
                    for (n, c) in &node_comm {
                        *m.entry(*c).or_insert(0.0) +=
                            adj.get(n).map(|v| v.len()).unwrap_or(0) as f64;
                    }
                    m
                };
                let k_u = adj.get(u).map(|v| v.len()).unwrap_or(0) as f64;
                // k_u_in：u 与每个社区的连边数（邻居在社区 c 的数量）
                let mut k_c: HashMap<usize, f64> = HashMap::new();
                for nb in adj.get(u).unwrap_or(&vec![]) {
                    let c = node_comm[nb];
                    *k_c.entry(c).or_insert(0.0) += 1.0;
                }
                // 从当前社区移除 u 对 Σ_tot 影响：
                // new_Sigma(c) = Σ_tot(c) - k_u
                let mut best_c = cur;
                let mut best_delta = 0.0f64;
                // 用有序 BTreeMap 迭代社区，避免 HashMap 随机序在 tie 时造成不同结果
                use std::collections::BTreeMap;
                let k_c_ordered: BTreeMap<usize, f64> = k_c.iter().map(|(a, b)| (*a, *b)).collect();
                for (&c, &k_in) in &k_c_ordered {
                    // ΔQ 合并至 c（移除原社区）
                    let sigma_tot_c = comm_total.get(&c).copied().unwrap_or(0.0);
                    // 若 c==cur：计算留在当前的增量基准；否则：
                    //   ΔQ = [ (Σ_in + k_in)/2m - ((Σ_tot + k_u)/2m)^2 ]
                    //      - [ Σ_in/2m - (Σ_tot/2m)^2 - (k_u/2m)^2 ]
                    // 为简化，我们采用标准公式 (Newman 2004):
                    //   ΔQ(k→c) = (k_in_c)/m - 2 * Σ_tot_c * k_u / (2m)^2
                    // （当把 k 从它自己的 singleton 移走时，社区 c 原 Σ_tot 不含 k）
                    // 为了兼容已合并场景，这里我们采用相对基准：
                    //   base = 0（若留在原社区）
                    //   delta_join_c = k_in_c / (m2/2) - 2.0 * sigma_tot_c * k_u / m2.powi(2)
                    // 并对留在当前的 delta = - 2*(sigma_tot_cur - k_u)*k_u / m2^2
                    let delta_join_c = k_in / (m2 / 2.0) - 2.0 * sigma_tot_c * k_u / m2.powi(2);
                    let delta_leave_cur = if c == cur {
                        // 留在当前：0
                        0.0
                    } else {
                        delta_join_c
                    };
                    if delta_leave_cur > best_delta + 1e-12 {
                        best_delta = delta_leave_cur;
                        best_c = c;
                    }
                }
                if best_c != cur {
                    *node_comm.get_mut(u).unwrap() = best_c;
                    changed = true;
                }
            }
        }

        let mut communities: HashMap<usize, Vec<String>> = HashMap::new();
        for (n, c) in node_comm {
            communities.entry(c).or_default().push(n);
        }
        let mut out: Communities = communities.into_values().collect();
        for c in &mut out {
            c.sort();
        }
        out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
        out
    }

    /// Brandes betweenness centrality（无权 BFS × 每源）。
    pub fn brandes(graph: &Graph) -> HashMap<String, f64> {
        let nodes: Vec<String> = graph.nodes.iter().cloned().collect();
        let adj = graph.adj_bidir();
        let mut bc: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
        for s in &nodes {
            let mut stack: Vec<String> = Vec::new();
            let mut pred: HashMap<String, Vec<String>> =
                nodes.iter().map(|n| (n.clone(), Vec::new())).collect();
            let mut sigma: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
            let mut dist: HashMap<String, i32> = nodes.iter().map(|n| (n.clone(), -1)).collect();
            let mut q: VecDeque<String> = VecDeque::new();
            *sigma.get_mut(s).unwrap() = 1.0;
            *dist.get_mut(s).unwrap() = 0;
            q.push_back(s.clone());
            while let Some(v) = q.pop_front() {
                stack.push(v.clone());
                for w in adj.get(&v).unwrap_or(&vec![]) {
                    if dist[w] < 0 {
                        *dist.get_mut(w).unwrap() = dist[&v] + 1;
                        q.push_back(w.clone());
                    }
                    if dist[w] == dist[&v] + 1 {
                        *sigma.get_mut(w).unwrap() += sigma[&v];
                        pred.get_mut(w).unwrap().push(v.clone());
                    }
                }
            }
            let mut delta: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
            while let Some(w) = stack.pop() {
                for v in &pred[&w] {
                    let f = sigma[v] / sigma[&w];
                    *delta.get_mut(v).unwrap() += f * (1.0 + delta[&w]);
                }
                if &w != s {
                    *bc.get_mut(&w).unwrap() += delta[&w];
                }
            }
        }
        // 无向图：除 2
        for v in bc.values_mut() {
            *v /= 2.0;
        }
        bc
    }

    /// Harmonic closeness：Σ_{t≠s} 1/dist(s,t)，再 / (n-1)。
    pub fn harmonic(graph: &Graph) -> HashMap<String, f64> {
        let nodes: Vec<String> = graph.nodes.iter().cloned().collect();
        let adj = graph.adj_bidir();
        let n = nodes.len();
        let mut out: HashMap<String, f64> = HashMap::new();
        for s in &nodes {
            let mut dist: HashMap<String, i32> = nodes.iter().map(|n| (n.clone(), -1)).collect();
            let mut q: VecDeque<String> = VecDeque::new();
            *dist.get_mut(s).unwrap() = 0;
            q.push_back(s.clone());
            while let Some(v) = q.pop_front() {
                let dv = dist[&v];
                for w in adj.get(&v).unwrap_or(&vec![]) {
                    if dist[w] < 0 {
                        *dist.get_mut(w).unwrap() = dv + 1;
                        q.push_back(w.clone());
                    }
                }
            }
            let mut hc = 0.0;
            for t in &nodes {
                if t == s {
                    continue;
                }
                let d = dist[t];
                if d > 0 {
                    hc += 1.0 / d as f64;
                }
            }
            if n > 1 {
                hc /= (n - 1) as f64;
            }
            out.insert(s.clone(), hc);
        }
        out
    }

    /// Degree centrality：RAW 边双向展开（每条无向边两端 +1）。
    pub fn degree_bidirectional(graph: &Graph) -> HashMap<String, u64> {
        let mut out: HashMap<String, u64> = graph.nodes.iter().map(|n| (n.clone(), 0)).collect();
        for (a, b) in &graph.edges {
            *out.get_mut(a).unwrap() += 1;
            *out.get_mut(b).unwrap() += 1;
        }
        out
    }

    /// Density（无向简单图）：d = m / (n*(n-1)/2)，无 toFixed。
    ///   m：无向边数量；n：节点数。
    pub fn density(graph: &Graph) -> f64 {
        let n = graph.nodes.len();
        let m = graph.edges.len();
        if n <= 1 {
            return 0.0;
        }
        let denom = (n * (n - 1) / 2) as f64;
        if denom == 0.0 {
            return 0.0;
        }
        m as f64 / denom
    }
}

/// LPA stub：deprecated。返回空 communities。
#[deprecated(
    since = "3.0.0",
    note = "LPA public API deprecated in AlgoBridge; use AlgoBridge::cnm for modularity-based communities."
)]
#[allow(dead_code)]
pub fn lpa_deprecated_stub() -> Communities {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_graph() -> Graph {
        let mut g = Graph::new();
        g.add_edge("a", "b");
        g.add_edge("b", "c");
        g.add_edge("c", "a");
        g.add_edge("c", "d");
        g
    }

    #[test]
    fn t_ppr_returns_sum_1() {
        let g = tiny_graph();
        let s = AlgoBridge::ppr(&g, "a", PPR_D, PPR_MAX_ITER);
        let sum: f64 = s.values().sum();
        assert!((sum - 1.0).abs() < 1e-9, "sum={sum}");
    }

    #[test]
    fn t_degree_tiny() {
        let g = tiny_graph();
        let d = AlgoBridge::degree_bidirectional(&g);
        assert_eq!(d["a"], 2);
        assert_eq!(d["b"], 2);
        assert_eq!(d["c"], 3);
        assert_eq!(d["d"], 1);
    }

    #[test]
    fn t_density_tiny() {
        let g = tiny_graph();
        let d = AlgoBridge::density(&g);
        // n=4, m=4 → d = 4 / 6 = 0.666666...
        assert!((d - 4.0 / 6.0).abs() < 1e-12);
    }
}
