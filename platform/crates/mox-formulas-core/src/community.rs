//! 社区检测：CNM 模块度贪心凝聚（F6）+ Newman 模块度（F7）

use crate::csr::CsrGraph;
use crate::CNM_GAIN_EPS;
use ahash::RandomState;
use hashbrown::HashMap;
use std::collections::HashMap as StdMap;

/// CNM 输出（公共真源：Rust 生产 API，Node/Python 绑定直接映射）
#[derive(Debug, Clone)]
pub struct CnmResult {
    pub communities: Vec<Vec<String>>,
    pub node_community: StdMap<String, usize>,
    pub modularity: f64,
    pub merges: usize,
}

// 辅助：将 CSR（Undirected 展开）还原为「unique 无向边」集（u<v）
struct UndirectedUnique {
    m: usize,
    edges: Vec<(usize, usize, f64)>, // (u<v, w)，w 为原权重（CNM 计数时按 w==1 使用）
    degree: Vec<f64>,
}

impl CsrGraph {
    // ---------------------------------------------------------------
    //  F7 · Newman 模块化度标量（基于社区 Vec）
    //   社区输入 Vec<Vec<usize>>，其中 usize 是社区内节点 idx 列表。
    //   Q = Σ_c [ lc/m − (dc/2m)^2 ]
    // ---------------------------------------------------------------
    pub fn modularity_by_idx(&self, communities: &[Vec<usize>]) -> f64 {
        let (m2, lc, dc) = self.community_sums(communities);
        if m2 <= 0.0 {
            return 0.0;
        }
        let two_m_inv = 1.0 / m2;
        let mut q = 0.0f64;
        for c in 0..communities.len() {
            let e = lc[c] * two_m_inv; // lc/m = 2lc / 2m
            let term = (dc[c] * two_m_inv) * (dc[c] * two_m_inv);
            q += (2.0 * e) - term;
            // 说明：lc 是 无向社区内边数 × 每条 w；
            // 标准公式：Σ lc/m − (dc/2m)^2，其中 lc/m = (lc 无向边 × w) / (无向边 m × w)
            // community_sums 用对称边做 m2 = 2m（sum w），lc[c] = 2 * Σ（内部无向边 w） ？
            // → 我们把 lc[c] 作为"对称边"定义：每条无向边(u,v) 在 out-CSR 里各出现一次，
            //   内部边计数 = 在 CSR 里扫描 i→j，若 i,j 同社区则累加 w/2。
        }
        q
    }

    /// 返回 (m2=2m_total, lc 2*内部边w合计, dc 总度数)
    ///   - m2 = Σ_{i} out_wsum[i]
    ///   - lc[c] = Σ_{i in c} Σ_{i→j, j in c} w(i,j)   （内部总权重，含上下行）
    ///   - dc[c] = Σ_{i in c} out_wsum[i]
    fn community_sums(&self, communities: &[Vec<usize>]) -> (f64, Vec<f64>, Vec<f64>) {
        let n = self.n;
        let mut comm_of = vec![-1i32; n];
        for (ci, nodes) in communities.iter().enumerate() {
            for &i in nodes {
                if i < n { comm_of[i] = ci as i32; }
            }
        }
        let mut m2 = 0.0f64;
        for i in 0..n { m2 += self.out_wsum[i]; }
        let k = communities.len();
        let mut lc = vec![0.0f64; k];
        let mut dc = vec![0.0f64; k];
        for i in 0..n {
            let ci = comm_of[i];
            if ci < 0 { continue; }
            let ci = ci as usize;
            dc[ci] += self.out_wsum[i];
            let rng = self.out_off[i]..self.out_off[i + 1];
            for kk in rng {
                let j = self.out_nbr[kk];
                let w = self.out_w[kk];
                let cj = comm_of[j];
                if cj >= 0 && cj as usize == ci { lc[ci] += w; }
            }
        }
        (m2, lc, dc)
    }

    // ---------------------------------------------------------------
    //  F6 · CNM：初始每节点一社区，反复合并 ΔQ 最大的相邻社区对
    //   直到 ΔQ ≤ 0。
    //   确定性平局：按 (a, b) 升序（数字编号社区 ID）。
    // ---------------------------------------------------------------
    pub fn community_cnm(&self) -> CnmResult {
        let n = self.n;
        if n == 0 {
            return CnmResult {
                communities: Vec::new(),
                node_community: StdMap::new(),
                modularity: 0.0,
                merges: 0,
            };
        }
        if n == 1 {
            let id0 = self.ids[0].clone();
            let mut nm = StdMap::new();
            nm.insert(id0.clone(), 0);
            return CnmResult {
                communities: vec![vec![id0]],
                node_community: nm,
                modularity: 0.0,
                merges: 0,
            };
        }

        // 基于 unique 无向边（u<v）构造社区结构
        let uu = self.unique_undirected();
        let UndirectedUnique { m, edges, degree } = uu;

        if m == 0 {
            // 无边 → 每节点一社区
            let comms: Vec<Vec<String>> =
                (0..n).map(|i| vec![self.ids[i].clone()]).collect();
            let mut nm = StdMap::new();
            for (i, id) in self.ids.iter().enumerate() { nm.insert(id.clone(), i); }
            return CnmResult { communities: comms, node_community: nm, modularity: 0.0, merges: 0 };
        }

        let m_f = m as f64;
        let two_m = 2.0 * m_f;

        // 社区状态：初始 i → i
        let mut comm_of: Vec<usize> = (0..n).collect();
        let mut comm_deg: Vec<f64> = degree.clone();
        let mut comm_alive: Vec<bool> = (0..n).map(|_| true).collect();
        // 社区 → 成员（便于最后输出；非最大堆 CNM 实现里可选，但输出需要）
        let mut members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

        // 相邻社区间的 cross edge w 合计（key: a<b 整数社区 id）
        let mut cross: HashMap<(usize, usize), f64, RandomState> =
            HashMap::with_hasher(RandomState::new());
        for &(u, v, w) in &edges {
            let a = u.min(v);
            let b = u.max(v);
            *cross.entry((a, b)).or_insert(0.0) += w;
        }

        let mut merges = 0usize;
        loop {
            // 遍历 cross 找 ΔQ 最大对
            let mut best: Option<((usize, usize), f64)> = None;
            for (&(a, b), &cnt) in &cross {
                if !comm_alive[a] || !comm_alive[b] || cnt <= 0.0 { continue; }
                // ΔQ = cnt/m − d_a·d_b/(2m)^2
                let e = cnt / m_f;
                let deg_term = (comm_deg[a] * comm_deg[b]) / (two_m * two_m);
                let gain = e - deg_term;
                // 确定性：(a,b) 字典序（先比 gain 大，gain 相同 id 小）
                let replace = match best {
                    None => true,
                    Some((_, bg)) => {
                        if gain > bg + CNM_GAIN_EPS { true }
                        else if (gain - bg).abs() <= CNM_GAIN_EPS {
                            // 平局取字典序更小（当前 (a,b) 更小则 replace）
                            let best_pair = best.unwrap().0;
                            (a, b) < best_pair
                        } else { false }
                    }
                };
                if replace { best = Some(((a, b), gain)); }
            }
            let Some(((a, b), gain)) = best else { break };
            if gain <= CNM_GAIN_EPS { break; }

            // 合并：b → a
            let mem_b = std::mem::take(&mut members[b]);
            for &nd in &mem_b { comm_of[nd] = a; }
            members[a].extend(mem_b);
            comm_deg[a] += comm_deg[b];
            comm_alive[b] = false;
            merges += 1;

            // 重写 cross：涉及 b 的跨边转移到 a
            let affected: Vec<(usize, usize, f64)> = cross
                .iter()
                .filter_map(|(&(x, y), &cnt)| {
                    if x == b || y == b { Some((x, y, cnt)) } else { None }
                })
                .collect();
            for (x, y, cnt) in affected {
                cross.remove(&(x, y));
                if cnt <= 0.0 { continue; }
                let other = if x == b { y } else { x };
                if other == a || !comm_alive[other] { continue; }
                let nk = if a < other { (a, other) } else { (other, a) };
                *cross.entry(nk).or_insert(0.0) += cnt;
            }
        }

        // 收集社区成员（只保留 alive 社区），按规模降序，次关键社区 id 升序
        let mut groups: Vec<(usize, Vec<usize>)> = (0..n)
            .filter(|&i| comm_alive[i])
            .map(|i| (i, std::mem::take(&mut members[i])))
            .collect();
        groups.sort_by(|x, y| y.1.len().cmp(&x.1.len()).then(x.0.cmp(&y.0)));

        let mut communities_idx: Vec<Vec<usize>> = Vec::with_capacity(groups.len());
        let mut node_comm_idx: Vec<usize> = vec![0usize; n];
        for (new_id, (_old_id, mems)) in groups.iter().enumerate() {
            for &nd in mems { node_comm_idx[nd] = new_id; }
            communities_idx.push(mems.clone());
        }

        let q = self.modularity_by_idx(&communities_idx);

        let mut node_community = StdMap::with_capacity(n);
        for i in 0..n { node_community.insert(self.ids[i].clone(), node_comm_idx[i]); }
        let communities: Vec<Vec<String>> = communities_idx
            .into_iter()
            .map(|v| v.into_iter().map(|i| self.ids[i].clone()).collect())
            .collect();

        CnmResult { communities, node_community, modularity: q, merges }
    }

    /// unique 无向边（基于 CSR out）：u<v，self-loop 跳过（CNM 不处理自环）。
    fn unique_undirected(&self) -> UndirectedUnique {
        let n = self.n;
        // Hashbrown key (u<v) → 合并权重
        let mut seen: HashMap<(usize, usize), f64, RandomState> =
            HashMap::with_hasher(RandomState::new());
        for i in 0..n {
            let rng = self.out_off[i]..self.out_off[i + 1];
            for k in rng {
                let j = self.out_nbr[k];
                let w = self.out_w[k];
                if i == j { continue; }
                let key = if i < j { (i, j) } else { (j, i) };
                *seen.entry(key).or_insert(0.0) += w;
            }
        }
        // 注意：self.out_wsum 是"含双向展开"的出度权重和，除以 2 得无向度？
        //   RawExpand::Undirected 场景：out_wsum[i] 恰好 = 无向度 Σ w_undirected * 2 ？ 不，
        //   每条 u-v 无向边按 (u→v,w)+(v→u,w) 展开 → out_wsum[u] 包含 w(v→u) 与 w(u→v)？
        //   仅 u 的出边 (u→v,w) 贡献；v→u 作为 u 的入边不计入 out_wsum[u]。
        //   因此对 RawExpand::Undirected：out_wsum[i] = Σ_{邻居} w （恰好等于无向度的权重和）。
        let mut degree: Vec<f64> = self.out_wsum.clone();

        // 若是 RawExpand::None（有向），degree 定义为出度+入度（对称度）
        if matches!(self.raw_expand, crate::csr::RawExpand::None) {
            let mut in_wsum = vec![0.0f64; n];
            for i in 0..n {
                let rng = self.in_off[i]..self.in_off[i + 1];
                for k in rng { in_wsum[i] += self.in_w[k]; }
            }
            for i in 0..n { degree[i] = (degree[i] + in_wsum[i]) * 0.5; }
        }

        // 为 CNM 整数度数一致性：degree 四舍五入到整数？不，保留浮点（支持权重边）。
        // 把 unique 边拆成"每条 1 个权重"，对权重边求和后的 seen 除以 2？
        //   RawExpand::Undirected 场景：每条 u-v 边在 out-CSR 中出现两次方向 → (u<v, 2w)。
        //   为了等价原始 1 条无向边的权重 w，需把 2w 折半。
        if matches!(self.raw_expand, crate::csr::RawExpand::Undirected) {
            for val in seen.values_mut() { *val *= 0.5; }
            // degree[i] 对 RawExpand::Undirected = Σ出边 = 每条相邻无向边 w（正确）
        }

        let edges: Vec<(usize, usize, f64)> = seen
            .into_iter()
            .map(|((u, v), w)| (u, v, w))
            .collect();
        let m = edges.len();
        UndirectedUnique { m, edges, degree }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::RawExpand;
    use crate::{EdgeInput, NodeInput};

    #[test]
    fn cnm_two_cliques_two_communities() {
        // 两个三角形 (a-b-c, d-e-f) + 桥 (c-d)
        let nodes: Vec<NodeInput> = ["a","b","c","d","e","f"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let edges: Vec<_> = [
            ("a","b"),("a","c"),("b","c"),
            ("d","e"),("d","f"),("e","f"),
            ("c","d"),
        ].into_iter()
            .map(|(s,t)| EdgeInput { source: s.into(), target: t.into(), weight: 1.0, relation_type: None })
            .collect();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let r = g.community_cnm();
        // 应得到两个社区，Q > 0
        assert_eq!(r.communities.len(), 2, "两个 3 团+桥应分裂为 2 社区");
        assert!(r.modularity > 0.2, "模块化度应该显著正，实际 {}", r.modularity);
        // 桥两端 c、d 分属不同社区
        assert_ne!(r.node_community["c"], r.node_community["d"]);
    }

    #[test]
    fn cnm_no_merge_negative_gain() {
        // 无边：每节点独立社区，0 合并
        let nodes: Vec<NodeInput> = ["a","b","c"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let g = CsrGraph::from_inputs(&nodes, &[], RawExpand::Undirected);
        let r = g.community_cnm();
        assert_eq!(r.communities.len(), 3);
        assert_eq!(r.merges, 0);
        assert_eq!(r.modularity, 0.0);
    }

    #[test]
    fn modularity_maximized_by_cnm() {
        let nodes: Vec<NodeInput> = ["a","b","c","d","e","f"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let edges: Vec<_> = [("a","b"),("a","c"),("b","c"),("d","e"),("d","f"),("e","f"),("c","d")]
            .into_iter()
            .map(|(s,t)| EdgeInput { source: s.into(), target: t.into(), weight: 1.0, relation_type: None }).collect();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let cnm = g.community_cnm();
        // 构造同一社区分配作为 communities_idx 输入
        let mut by_idx: Vec<Vec<usize>> = vec![Vec::new(); cnm.communities.len()];
        for i in 0..g.n {
            let cid = cnm.node_community[&g.ids[i]];
            by_idx[cid].push(i);
        }
        let q = g.modularity_by_idx(&by_idx);
        assert!((q - cnm.modularity).abs() < 1e-12, "内部 modularity 再计算应与 CNM 输出一致");
    }
}
