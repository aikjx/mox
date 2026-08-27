// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! PageRank + PPR · Gauss-Seidel 原地迭代（CSR "拉"模式，使用 in-CSR）
//!
//! 算法要点：
//! - 标准 PR：x_new[j] = (1−α)/N + α·( Σ_{i→j} x[i]·w(i,j)/out_sum[i] + dangling·u[j] )
//!   其中 u[j]=1/N 为均匀分布。Gauss-Seidel：写回 x[j] 立即参与后续同轮计算，
//!   经验上比 Power 迭代少 30~50% 迭代。
//! - PPR：u[j] = personalization[j] / Σp 取代均匀分布；悬挂质量按 p 分配。
//! - 精度护栏（项目记忆）：α=PPR_D=0.85，最多 PPR_MAX_ITER=30 轮；但实际收敛可以提前
//!   （提前停止仅为性能优化，与 max_iter=30 上限同时成立）。
//! - 接口：`pagerank_map` 返回 HashMap<String,f64>，与 Node 层 schema 完全一致。

use crate::csr::CsrGraph;
use crate::{PPR_D, PPR_MAX_ITER, PR_EPS};
use ahash::RandomState;
use hashbrown::HashMap;
use std::collections::HashMap as StdMap;

impl CsrGraph {
    /// 标准 PageRank（CSR Gauss-Seidel，α=PPR_D，上限 PPR_MAX_ITER 轮）
    ///
    /// 返回 Vec<f64>，长度 = self.n；值顺序 = ids 顺序。
    pub fn pagerank(&self) -> (Vec<f64>, usize) {
        self.pagerank_alpha(PPR_D, PPR_MAX_ITER)
    }

    /// 允许调用方自定义 α，但受 PPR_MAX_ITER 上限保护。
    pub fn pagerank_alpha(&self, alpha: f64, max_iter: usize) -> (Vec<f64>, usize) {
        let n = self.n;
        if n == 0 {
            return (Vec::new(), 0);
        }
        let max_iter = max_iter.min(PPR_MAX_ITER);
        let nf = n as f64;
        let teleport = (1.0 - alpha) / nf;
        let uniform_dang = alpha / nf; // dangling mass 平均分配系数（每次使用时 × total dangling）

        let mut x = vec![1.0 / nf; n];

        let mut iter_used = 0;
        for it in 0..max_iter {
            iter_used = it + 1;
            // 1. 计算悬挂质量（out_wsum[i]==0 的 i 贡献 x[i]）
            let mut dangling = 0.0f64;
            for i in 0..n {
                if self.out_wsum[i] < 1e-15 {
                    dangling += x[i];
                }
            }
            let dang_term = dangling * uniform_dang;
            let mut max_diff = 0.0f64;

            // 2. Gauss-Seidel 拉模式：按 j 顺序计算新值，立即写回 x[j]
            //    x[j] = teleport + dang_term + α·Σ_{i→j} x[i]·w(i,j)/out_wsum[i]
            for j in 0..n {
                let rng = self.in_off[j]..self.in_off[j + 1];
                let mut acc = 0.0f64;
                for k in rng {
                    let i = self.in_nbr[k];
                    let w = self.in_w[k];
                    let ws = self.out_wsum[i];
                    if ws < 1e-15 {
                        continue;
                    }
                    acc += x[i] * (w / ws);
                }
                let new = teleport + dang_term + alpha * acc;
                let d = (new - x[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                x[j] = new;
            }

            if max_diff < PR_EPS {
                break;
            }
        }
        // 精度护栏：Gauss-Seidel 数值舍入可能导致 ΣPR 偏离 1（~2e-6 级别）。
        // 归一化到 Σ = 1（L1 renorm）：对 serde/对账/断言均严格。
        let s: f64 = x.iter().sum();
        if s > 1e-300 {
            let inv = 1.0 / s;
            for v in x.iter_mut() { *v *= inv; }
        }
        (x, iter_used)
    }

    /// 个性化 PageRank：seed_map (id → weight) 作为偏好向量。
    /// 返回 (rank Vec, converged_at)。
    pub fn ppr(
        &self,
        seed_map: &StdMap<String, f64>,
    ) -> (Vec<f64>, usize) {
        self.ppr_alpha(PPR_D, PPR_MAX_ITER, seed_map)
    }

    pub fn ppr_alpha(
        &self,
        alpha: f64,
        max_iter: usize,
        seed_map: &StdMap<String, f64>,
    ) -> (Vec<f64>, usize) {
        let n = self.n;
        if n == 0 {
            return (Vec::new(), 0);
        }
        let max_iter = max_iter.min(PPR_MAX_ITER);

        // 构建偏好向量 p
        let mut p = vec![0.0f64; n];
        let mut total = 0.0f64;
        for (id, &w) in seed_map {
            if let Some(&idx) = self.id_to_idx.get(id.as_str()) {
                let w = if w < 0.0 { 0.0 } else { w };
                p[idx] += w;
                total += w;
            }
        }
        if total < 1e-15 {
            // 空偏好 → 退化为均匀
            let u = 1.0 / n as f64;
            for v in p.iter_mut() {
                *v = u;
            }
        } else {
            let inv = 1.0 / total;
            for v in p.iter_mut() {
                *v *= inv;
            }
        }

        let mut x = p.clone();
        let mut iter_used = 0;
        for it in 0..max_iter {
            iter_used = it + 1;
            let mut dangling = 0.0f64;
            for i in 0..n {
                if self.out_wsum[i] < 1e-15 {
                    dangling += x[i];
                }
            }
            let mut max_diff = 0.0f64;
            for j in 0..n {
                let rng = self.in_off[j]..self.in_off[j + 1];
                let mut acc = 0.0f64;
                for k in rng {
                    let i = self.in_nbr[k];
                    let w = self.in_w[k];
                    let ws = self.out_wsum[i];
                    if ws < 1e-15 {
                        continue;
                    }
                    acc += x[i] * (w / ws);
                }
                let pj = p[j];
                let new = (1.0 - alpha) * pj + alpha * (acc + dangling * pj);
                let d = (new - x[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                x[j] = new;
            }
            if max_diff < PR_EPS {
                break;
            }
        }
        // PPR：同归一化护栏
        let s: f64 = x.iter().sum();
        if s > 1e-300 {
            let inv = 1.0 / s;
            for v in x.iter_mut() { *v *= inv; }
        }
        (x, iter_used)
    }

    /// 把 rank Vec 映射为 HashMap<String,f64>（使用 Hashbrown AHashMap 作为底层）。
    pub fn rank_to_map(&self, rank: &[f64]) -> HashMap<String, f64, RandomState> {
        let mut out: HashMap<String, f64, RandomState> =
            HashMap::with_capacity_and_hasher(self.n, RandomState::new());
        for (i, &r) in rank.iter().enumerate().take(self.n) {
            out.insert(self.ids[i].clone(), r);
        }
        out
    }

    /// 公共 PageRank map 输出（to_string HashMap 类型便于 serde 与 std::collections 调用）
    pub fn pagerank_stdmap(&self) -> StdMap<String, f64> {
        let (rank, _used) = self.pagerank();
        let mut m = StdMap::with_capacity(self.n);
        for (i, &r) in rank.iter().enumerate().take(self.n) {
            m.insert(self.ids[i].clone(), r);
        }
        m
    }

    pub fn ppr_stdmap(&self, seed: &StdMap<String, f64>) -> StdMap<String, f64> {
        let (rank, _used) = self.ppr(seed);
        let mut m = StdMap::with_capacity(self.n);
        for (i, &r) in rank.iter().enumerate().take(self.n) {
            m.insert(self.ids[i].clone(), r);
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeInput, NodeInput};

    fn simple_graph() -> (Vec<NodeInput>, Vec<EdgeInput>) {
        let nodes: Vec<NodeInput> = vec!["A", "B", "C", "D"]
            .iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None })
            .collect();
        // A→B, A→C, B→C, C→A, D→C（悬挂 D？不：D→C，C 的出度只到 A）
        let edges: Vec<EdgeInput> = vec![
            ("A", "B", 1.0),
            ("A", "C", 1.0),
            ("B", "C", 1.0),
            ("C", "A", 1.0),
            ("D", "C", 1.0),
        ]
        .into_iter()
        .map(|(s, t, w)| EdgeInput {
            source: s.into(),
            target: t.into(),
            weight: w,
            relation_type: None,
        })
        .collect();
        (nodes, edges)
    }

    #[test]
    fn pagerank_sum_one() {
        let (nodes, edges) = simple_graph();
        let g = CsrGraph::from_inputs(&nodes, &edges, super::super::csr::RawExpand::None);
        let (rank, _used) = g.pagerank();
        let s: f64 = rank.iter().sum();
        // ΣPR ≈ 1
        assert!((s - 1.0).abs() < 1e-6, "Σ PR = {s}");
    }

    #[test]
    fn pagerank_dangling_is_handled() {
        // B 是悬挂（无出边），其质量应回传全图
        let nodes = ["A", "B"]
            .iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None })
            .collect::<Vec<_>>();
        let edges = vec![EdgeInput { source: "A".into(), target: "B".into(), weight: 1.0, relation_type: None }];
        let g = CsrGraph::from_inputs(&nodes, &edges, super::super::csr::RawExpand::None);
        let (rank, _u) = g.pagerank();
        // B 应非零（悬挂回传 → A 仍能得分，两节点之和 1）
        assert!(rank[0] > 0.0 && rank[1] > 0.0);
        assert!((rank[0] + rank[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ppr_peak_on_seed() {
        let (nodes, edges) = simple_graph();
        let g = CsrGraph::from_inputs(&nodes, &edges, super::super::csr::RawExpand::None);
        let mut seed = StdMap::new();
        seed.insert("A".to_string(), 10.0);
        let (rank, _u) = g.ppr(&seed);
        let aid = g.idx_of("A").unwrap();
        let bid = g.idx_of("B").unwrap();
        let cid = g.idx_of("C").unwrap();
        // A 应得最高 PR（偏好），B 和 C 非零，D 最低（无入边但悬挂回传略得）
        assert!(rank[aid] > rank[bid] && rank[aid] > rank[cid]);
    }
}
