// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! CSR 稀疏图：输入 → 构建 → 算法复用
//!
//! 设计要点：
//! - `from_inputs` 一次扫描完成：RAW 展开 + 去重 + 权重累加（同边多次出现自动求和为加权）
//! - 提供 out / in 双 CSR（in CSR 用于 PageRank 的"拉"形式，便于 Gauss-Seidel 原地更新）
//! - RawExpand::Undirected：每条 RAW 边 (u,v,w) 展开为 (u→v,w) 和 (v→u,w)，自环不展开
//! - RawExpand::None：有向图直接使用
//! - 节点 id <-> index 映射使用 ahash/Hashbrown 以获得比 std HashMap 2~3× 构造速度

use crate::{EdgeInput, NodeInput};
use ahash::RandomState;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawExpand {
    /// 有向图：保留原方向（默认 false，仅 degree directed=true 场景使用）
    None,
    /// 无向语义：每条边双向展开（介数/紧密/社区 等使用）
    Undirected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub src_idx: usize,
    pub dst_idx: usize,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct CsrGraph {
    pub n: usize,
    pub ids: Vec<String>,
    pub id_to_idx: HashMap<String, usize, RandomState>,
    /// out CSR：offsets/targets/weights 出邻接
    pub out_off: Vec<usize>,
    pub out_nbr: Vec<usize>,
    pub out_w: Vec<f64>,
    /// in CSR：offsets/targets/weights 入邻接（PR Gauss-Seidel "拉"模式使用）
    pub in_off: Vec<usize>,
    pub in_nbr: Vec<usize>,
    pub in_w: Vec<f64>,
    /// 出度权重和 Σ W_out(i)（dangling 判定使用）
    pub out_wsum: Vec<f64>,
    /// 所有边权 == 1.0（则 Harmonic / 最短路 走 BFS 快路径）
    pub all_unit: bool,
    /// 构造参数（保留给 directed 判断）
    pub raw_expand: RawExpand,
}

impl CsrGraph {
    pub fn from_inputs(nodes: &[NodeInput], edges: &[EdgeInput], expand: RawExpand) -> Self {
        // ---- Pass 1: 建 id → idx（Hashbrown AHashMap 更快构造）
        let hasher = RandomState::new();
        let mut id_to_idx: HashMap<String, usize, RandomState> =
            HashMap::with_capacity_and_hasher(nodes.len(), hasher.clone());
        let mut ids: Vec<String> = Vec::with_capacity(nodes.len());
        for nd in nodes {
            let idx = ids.len();
            id_to_idx.insert(nd.id.clone(), idx);
            ids.push(nd.id.clone());
        }
        let n = ids.len();
        if n == 0 {
            return Self::empty(expand);
        }

        // ---- Pass 2: 边扫描（RAW 展开 + 权重累积）
        // 使用 edge_key -> weight，避免同 (src,dst) 多次展开重复
        let mut edge_weights: HashMap<(usize, usize), f64, RandomState> =
            HashMap::with_hasher(hasher.clone());
        let mut all_unit = true;
        let mut insert_directed = |s: usize, t: usize, w: f64, all_unit: &mut bool| {
            if (w - 1.0).abs() > 1e-15 {
                *all_unit = false;
            }
            *edge_weights.entry((s, t)).or_insert(0.0) += w;
        };

        for e in edges {
            let Some(&s) = id_to_idx.get(&e.source) else {
                continue;
            };
            let Some(&t) = id_to_idx.get(&e.target) else {
                continue;
            };
            let w = if e.weight == 0.0 { 1.0 } else { e.weight };
            match expand {
                RawExpand::None => insert_directed(s, t, w, &mut all_unit),
                RawExpand::Undirected => {
                    if s == t {
                        insert_directed(s, t, w, &mut all_unit);
                    } else {
                        insert_directed(s, t, w, &mut all_unit);
                        insert_directed(t, s, w, &mut all_unit);
                    }
                }
            }
        }

        // ---- Pass 3: 计数出/入度，构造 CSR
        let m_total = edge_weights.len();
        let mut out_deg = vec![0usize; n];
        let mut in_deg = vec![0usize; n];
        let mut out_wsum = vec![0.0f64; n];

        for (&(s, t), &w) in &edge_weights {
            out_deg[s] += 1;
            in_deg[t] += 1;
            out_wsum[s] += w;
        }

        let mut out_off = vec![0usize; n + 1];
        let mut in_off = vec![0usize; n + 1];
        for i in 0..n {
            out_off[i + 1] = out_off[i] + out_deg[i];
            in_off[i + 1] = in_off[i] + in_deg[i];
        }

        let mut out_nbr = vec![0usize; m_total];
        let mut out_w = vec![0.0f64; m_total];
        let mut in_nbr = vec![0usize; m_total];
        let mut in_w = vec![0.0f64; m_total];

        let mut curs_out = out_off[0..n].to_vec();
        let mut curs_in = in_off[0..n].to_vec();

        // 为了稳定性：按 (s, t) 字典序填（与 Node 层对账不敏感，仅保证内部确定性）
        let mut entries: Vec<((usize, usize), f64)> = edge_weights.into_iter().collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for ((s, t), w) in entries {
            let p1 = curs_out[s];
            out_nbr[p1] = t;
            out_w[p1] = w;
            curs_out[s] = p1 + 1;

            let p2 = curs_in[t];
            in_nbr[p2] = s;
            in_w[p2] = w;
            curs_in[t] = p2 + 1;
        }

        Self {
            n,
            ids,
            id_to_idx,
            out_off,
            out_nbr,
            out_w,
            in_off,
            in_nbr,
            in_w,
            out_wsum,
            all_unit,
            raw_expand: expand,
        }
    }

    fn empty(raw_expand: RawExpand) -> Self {
        Self {
            n: 0,
            ids: Vec::new(),
            id_to_idx: HashMap::with_hasher(RandomState::new()),
            out_off: vec![0],
            out_nbr: Vec::new(),
            out_w: Vec::new(),
            in_off: vec![0],
            in_nbr: Vec::new(),
            in_w: Vec::new(),
            out_wsum: Vec::new(),
            all_unit: true,
            raw_expand,
        }
    }

    /// 图的边数（m_total）
    #[inline]
    pub fn edge_count(&self) -> usize {
        self.out_nbr.len()
    }

    /// 取节点索引的 id（O(1) 直接索引 ids）
    #[inline]
    pub fn id_of(&self, idx: usize) -> &str {
        &self.ids[idx]
    }

    /// id → idx，查不到 None
    #[inline]
    pub fn idx_of(&self, id: &str) -> Option<usize> {
        self.id_to_idx.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(ids: &[&str]) -> Vec<NodeInput> {
        ids.iter()
            .map(|s| NodeInput {
                id: (*s).to_string(),
                label: None,
                properties: None,
            })
            .collect()
    }

    #[test]
    fn triangle_undirected_expands_correctly() {
        let nodes = nodes(&["a", "b", "c"]);
        let edges = vec![
            EdgeInput { source: "a".into(), target: "b".into(), weight: 1.0, relation_type: None },
            EdgeInput { source: "b".into(), target: "c".into(), weight: 1.0, relation_type: None },
            EdgeInput { source: "c".into(), target: "a".into(), weight: 1.0, relation_type: None },
        ];
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        assert_eq!(g.n, 3);
        // 无向展开 → 3 条 raw × 2 方向 = 6 条有向边
        assert_eq!(g.edge_count(), 6);
        assert!(g.all_unit);
        for i in 0..3 {
            assert_eq!(g.out_wsum[i], 2.0);
        }
    }

    #[test]
    fn directed_does_not_expand() {
        let nodes = nodes(&["a", "b"]);
        let edges = vec![EdgeInput { source: "a".into(), target: "b".into(), weight: 1.0, relation_type: None }];
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::None);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.out_wsum[g.idx_of("a").unwrap()], 1.0);
        assert_eq!(g.out_wsum[g.idx_of("b").unwrap()], 0.0);
    }
}
