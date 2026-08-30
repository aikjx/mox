// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

// ============================================================================
// CSR 稀疏邻接：O(N+E) 表示，避免 O(N²) dense 矩阵。(私有，对外零暴露)
// ============================================================================
/// 按出边组织的 CSR 邻接（出边表 i → j₁,j₂…）
#[derive(Debug, Clone)]
pub(crate) struct CsrAdj {
    pub(crate) n: usize,
    /// offsets[i+1] - offsets[i] == i 的出边数
    pub(crate) offsets: Vec<usize>,
    /// targets[offsets[i]..offsets[i+1]]：i 指向的邻居
    pub(crate) targets: Vec<usize>,
    /// weights[*]：与 targets 一一对应
    pub(crate) weights: Vec<f64>,
    /// out_weight[i] = Σ W(i,·)；0 表示 dangling
    pub(crate) out_weight: Vec<f64>,
    /// true ⟺ 所有边权 == 1.0（此时 closeness 可走 BFS，跳过二叉堆）
    pub(crate) all_unit_weight: bool,
}

impl CsrAdj {
    pub(crate) fn from_graph<N>(g: &DiGraph<N, f64>) -> Self {
        let n = g.node_count();
        let m = g.edge_count();

        let mut out_deg = vec![0usize; n];
        let mut out_weight = vec![0.0f64; n];
        let mut all_unit_weight = true;
        let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(m);

        for e in g.edge_references() {
            let i = e.source().index();
            let j = e.target().index();
            let w = *e.weight();
            out_deg[i] += 1;
            out_weight[i] += w;
            if (w - 1.0).abs() > 1e-15 {
                all_unit_weight = false;
            }
            edges.push((i, j, w));
        }

        let mut offsets = vec![0usize; n + 1];
        for i in 0..n {
            offsets[i + 1] = offsets[i] + out_deg[i];
        }
        let mut targets = vec![0usize; m];
        let mut weights = vec![0.0f64; m];
        let mut curs = offsets[0..n].to_vec();
        for (i, j, w) in edges {
            let slot = curs[i];
            curs[i] += 1;
            targets[slot] = j;
            weights[slot] = w;
        }

        Self {
            n,
            offsets,
            targets,
            weights,
            out_weight,
            all_unit_weight,
        }
    }

    /// 标准 PageRank（CSR 推模型）
    pub(crate) fn pagerank(&self, alpha: f64, iterations: usize) -> Vec<f64> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }
        let nf = n as f64;
        let mut rank = vec![1.0 / nf; n];
        let teleport = 1.0 / nf;
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        for _ in 0..iterations {
            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            let dterm = alpha * dangling_mass * teleport;
            let tterm = (1.0 - alpha) * teleport;
            for j in 0..n {
                let new = tterm + alpha * propagated[j] + dterm;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }
            if max_diff < 1e-6 {
                break;
            }
        }
        rank
    }

    /// 个性化 PageRank（CSR）：悬挂质量按 p 分配。
    pub(crate) fn pagerank_personalized(&self, alpha: f64, iterations: usize, p: &[f64]) -> Vec<f64> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }
        let mut rank = p.to_vec();
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        for _ in 0..iterations {
            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            for j in 0..n {
                let pj = p[j];
                let new = alpha * propagated[j] + alpha * dangling_mass * pj + (1.0 - alpha) * pj;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }
            if max_diff < 1e-6 {
                break;
            }
        }
        rank
    }
}

pub(crate) fn rank_vec_to_map(rank: &[f64], node_map: &HashMap<String, NodeIndex>) -> HashMap<String, f64> {
    let mut result = HashMap::with_capacity(rank.len());
    for (id, idx) in node_map {
        result.insert(id.clone(), rank[idx.index()]);
    }
    result
}
