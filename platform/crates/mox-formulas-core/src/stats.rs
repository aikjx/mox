//! 统计指标：K-Core（Batagelj-Zaversnik O(m) 桶排序）、三角计数（forward）、
//! 平均聚集系数、全局聚集系数、度同配系数（Pearson）

use crate::csr::CsrGraph;
use std::collections::HashMap;

impl CsrGraph {
    // -----------------------------------------------------------
    // F9 · K-Core 分解（Batagelj-Zaversnik 2003，O(m)）
    //   返回每个节点的 core number。
    //   说明：使用无权度（邻居计数），支持加权的 out_degree。
    // -----------------------------------------------------------
    pub fn k_core(&self) -> HashMap<String, usize> {
        let n = self.n;
        if n == 0 { return HashMap::new(); }
        // 使用邻接列表（双向无向语义下，out-neighbors 是对称邻居）
        let deg: Vec<usize> = (0..n).map(|i| {
            let d = self.out_off[i + 1] - self.out_off[i];
            // 对 RawExpand::Undirected：deg 为 2 倍（每条无向边双向）— 正确做法是除以 2
            // 对 RawExpand::None：保持原样（有向按出度）
            if matches!(self.raw_expand, crate::csr::RawExpand::Undirected) { d / 2 } else { d }
        }).collect();

        let mut md = 0usize;
        for &d in &deg { if d > md { md = d; } }
        // 若 RawExpand::None，用对称度 min(out+in) 更合理？保持 out_deg 简单版本。
        let mut core = deg.clone();
        // BinSort
        let mut bin: Vec<usize> = vec![0; md + 2];
        for &c in &core { bin[c] += 1; }
        let mut start = 0usize;
        for d in 0..=md {
            let tmp = bin[d];
            bin[d] = start;
            start += tmp;
        }
        // pos[i] = 在 bin 数组里的索引；vert[pos] = i
        let mut pos = vec![0usize; n];
        let mut vert = vec![0usize; n];
        for (i, &ci) in core.iter().enumerate().take(n) {
            pos[i] = bin[ci];
            vert[pos[i]] = i;
            bin[ci] += 1;
        }
        // 还原 bin 起始
        for d in (1..=md).rev() { bin[d] = bin[d - 1]; }
        bin[0] = 0;

        for i in 0..n {
            let v = vert[i];
            let cv = core[v];
            // 对 v 的每个邻居 u
            let rng = self.out_off[v]..self.out_off[v + 1];
            for k in rng {
                let u = self.out_nbr[k];
                if core[u] > cv {
                    // 将 u 从 bin[core[u]] 头部与首元素交换位置
                    let du = core[u];
                    let pu = pos[u];
                    let w0 = bin[du];
                    if u != vert[w0] {
                        let w = vert[w0];
                        vert[pu] = w;
                        vert[w0] = u;
                        pos[w] = pu;
                        pos[u] = w0;
                    }
                    bin[du] += 1;
                    core[u] -= 1;
                }
            }
        }

        let mut out = HashMap::with_capacity(n);
        for i in 0..n { out.insert(self.ids[i].clone(), core[i]); }
        out
    }

    // -----------------------------------------------------------
    // F11 · 三角计数 + 平均/全局聚集系数
    //   forward 算法（u<v<w）：对每个节点 v，按度排序后枚举邻居对 (u,w) 存在性检查
    // -----------------------------------------------------------
    pub fn triangle_count_and_clustering(&self) -> (u64, f64, f64) {
        let n = self.n;
        // 使用"度"out-degree（RawExpand::Undirected 下为双向）
        let deg: Vec<usize> = (0..n).map(|i| self.out_off[i + 1] - self.out_off[i]).collect();
        // 按度升序 rank
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| deg[a].cmp(&deg[b]).then(a.cmp(&b)));
        let mut rank = vec![0usize; n];
        for (i, &v) in order.iter().enumerate() { rank[v] = i; }

        // 对每个节点 v：只考虑 rank[u] > rank[v] 的 out 邻居（higher）
        let mut higher: Vec<Vec<usize>> = vec![Vec::new(); n];
        for v in 0..n {
            let rv = rank[v];
            let rng = self.out_off[v]..self.out_off[v + 1];
            for k in rng {
                let u = self.out_nbr[k];
                if rank[u] > rv { higher[v].push(u); }
            }
        }

        // 三角形计数（每条 u-w 都 higher[v] 且 higher[u] 存在 w）
        let mut tri: u64 = 0;
        // 节点三角计数
        let mut node_tri = vec![0u64; n];

        // 存在性集合：higher[u] 放入临时 hash
        use std::collections::HashSet;
        for v in 0..n {
            // higher[v] 中的每个 u1,u2，若 u1 < u2 且 higher[u1] 中含 u2 → v,u1,u2 三角形
            let neighbors = &higher[v];
            // 构建 higher 邻居的 set 用于快速查询 u in higher[x]
            // → 对每个 u ∈ higher[v]，枚举 w ∈ higher[u] ∩ higher[v] 直接累加即可
            let setv: HashSet<usize> = neighbors.iter().copied().collect();
            for &u in neighbors {
                for &w in &higher[u] {
                    if setv.contains(&w) {
                        tri += 1;
                        node_tri[v] += 1;
                        node_tri[u] += 1;
                        node_tri[w] += 1;
                    }
                }
            }
        }

        // 平均局部聚集系数：对每个 v，
        //   actual_tri(v)  /  (deg(v) choose 2)   （deg ≥ 2 计入；RawExpand::Undirected 下 deg/2）
        let mut avg_local = 0.0f64;
        let mut counted = 0usize;
        for v in 0..n {
            let d_raw = deg[v];
            let d = if matches!(self.raw_expand, crate::csr::RawExpand::Undirected) {
                d_raw / 2
            } else {
                d_raw
            };
            if d >= 2 {
                let possible = (d * (d - 1)) as f64;
                // node_tri[v] 在 RawExpand::Undirected 下，三角形每条边会各出现一次方向组合？
                // forward 算法中，每个无向三角 {a,b,c} 恰好当 v=rank 最小时被计 1 次（因 higher 仅 rank 大）。
                // → 但我们在 v's (u,w) 组合里对每对 higher[v] 枚举，每个三角形 {a,b,c} 在各自最小节点的循环里
                //   被 tri 加 1；node_tri[v] 则对三个顶点各加 1（无论谁是最小值）？
                //   让我们仔细看：假设 rank[a]<rank[b]<rank[c]。仅当 v=a 时枚举 u=b,c，higher[a] 有 b、c；
                //   u=b，higher[b] 应有 c（因 rank[c]>rank[b]）且 setv[c]=true → tri += 1；
                //   node_tri[a] += 1，node_tri[b] += 1，node_tri[c] += 1 各 1 次。完美。
                let local = (2 * node_tri[v]) as f64 / possible; // node_tri[v] 是 v 参加的"有序三角 1 次"
                // 说明：上面 node_tri[v] 在三角 {a,b,c} 中 v 每个各 +1 → node_tri[v] 恰好是 v 参与的三角数
                //   possible = d choose 2，局部聚集系数 = 2·t / (d(d−1))
                avg_local += local;
                counted += 1;
            }
        }
        if counted > 0 { avg_local /= counted as f64; }

        // 全局聚集系数（传递性）= 3·triangles / (closed path length 2)
        //   path length 2（连通三元组） = Σ_v deg(v) choose 2
        let mut triples = 0u64;
        for v in 0..n {
            let d_raw = deg[v];
            let d = if matches!(self.raw_expand, crate::csr::RawExpand::Undirected) {
                d_raw / 2
            } else {
                d_raw
            };
            if d >= 2 { triples += (d * (d - 1) / 2) as u64; }
        }
        let global = if triples > 0 { (3 * tri) as f64 / (2 * triples) as f64 } else { 0.0 };
        (tri, avg_local, global)
    }

    // -----------------------------------------------------------
    // F12 · 度同配系数（Pearson r 对边 (j_i, j_k) 做积和）
    //   对无向：每条无向边 (j,k) 算两份 (j,k) 与 (k,j)？标准公式用 j·k 的和。
    //   Newman 公式：r = (Σ_e j_e·k_e − [Σ_e (j_e+k_e)/2]^2) /
    //                    (Σ_e (j_e^2+k_e^2)/2 − [Σ_e (j_e+k_e)/2]^2)
    //   我们实现简化：对每条 unique 无向边的两端度 (x,y)，
    //   r = Cov(x,y) / (σx · σy)。
    // -----------------------------------------------------------
    pub fn assortativity_degree(&self) -> f64 {
        let n = self.n;
        let deg_raw: Vec<usize> = (0..n).map(|i| self.out_off[i + 1] - self.out_off[i]).collect();
        let deg: Vec<f64> = deg_raw.iter().map(|&d| {
            if matches!(self.raw_expand, crate::csr::RawExpand::Undirected) { (d as f64) * 0.5 } else { d as f64 }
        }).collect();

        // 收集 unique 无向边的 (x,y)
        let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(self.edge_count() / 2 + 1);
        use std::collections::HashSet;
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        for i in 0..n {
            let rng = self.out_off[i]..self.out_off[i + 1];
            for k in rng {
                let j = self.out_nbr[k];
                if i == j { continue; }
                let key = if i < j { (i, j) } else { (j, i) };
                if !seen.insert(key) { continue; }
                pairs.push((deg[i], deg[j]));
            }
        }
        let m = pairs.len();
        if m == 0 { return 0.0; }
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_x2 = 0.0f64;
        let mut sum_y2 = 0.0f64;
        for &(x, y) in &pairs {
            sum_x += x; sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x; sum_y2 += y * y;
        }
        let inv = 1.0 / (m as f64);
        let mean_x = sum_x * inv;
        let mean_y = sum_y * inv;
        let cov = sum_xy * inv - mean_x * mean_y;
        let var_x = sum_x2 * inv - mean_x * mean_x;
        let var_y = sum_y2 * inv - mean_y * mean_y;
        if var_x <= 0.0 || var_y <= 0.0 { return 0.0; }
        cov / (var_x.sqrt() * var_y.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::RawExpand;
    use crate::{EdgeInput, NodeInput};

    fn triangle_graph() -> (Vec<NodeInput>, Vec<EdgeInput>) {
        let nodes = ["a","b","c"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let edges = [("a","b"),("b","c"),("c","a")].into_iter()
            .map(|(s,t)| EdgeInput { source: s.into(), target: t.into(), weight: 1.0, relation_type: None }).collect();
        (nodes, edges)
    }

    #[test]
    fn k_core_triangle_is_2() {
        let (nodes, edges) = triangle_graph();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let k = g.k_core();
        for id in ["a","b","c"] { assert_eq!(k[id], 2, "{id} core 应为 2"); }
    }

    #[test]
    fn triangle_counts_one() {
        let (nodes, edges) = triangle_graph();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let (tri, local, global) = g.triangle_count_and_clustering();
        assert_eq!(tri, 1, "三角形应为 1");
        // 完全图 K3 的局部聚集系数 = 1，全局聚集系数 = 1
        assert!((local - 1.0).abs() < 1e-12, "avg local = {local}");
        assert!((global - 1.0).abs() < 1e-12, "global = {global}");
    }

    #[test]
    fn assortativity_regular_graph_na() {
        // 正则图所有节点度相同 → 协方差 0 → r=0（正确，无同配异配之分）
        let (nodes, edges) = triangle_graph();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let r = g.assortativity_degree();
        assert_eq!(r, 0.0);
    }

    #[test]
    fn assortativity_starmix_nonzero() {
        // 星型：中心度 4、叶子度 1 → 所有边是 (1,4) 组合 → 异配（负）
        let nodes = ["c","a","b","d","e"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let edges = ["a","b","d","e"].into_iter()
            .map(|t| EdgeInput { source: "c".into(), target: t.into(), weight: 1.0, relation_type: None })
            .collect();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let r = g.assortativity_degree();
        assert!(r.is_finite(), "r 应有限");
        // 星型：每条 unique 无向边的两端度是 (1,4) → cov = E[XY] - E[X]E[Y]
        // E[X] = E[Y] = (Σ deg_j + deg_k) / (2M) = (4*4 + 1*4)/8 = (20)/8 = 2.5
        // E[XY] = 4*1 * 4 条边 / 4 = 4.0 → cov = 4.0 - 2.5*2.5 = 4 - 6.25 = -2.25
        // var_x = (4·16 + 4·1)/8 − 6.25 = (68)/8 − 6.25 = 8.5 − 6.25 = 2.25
        // σx = 1.5 → r = -2.25 / (1.5·1.5) = -1.0 （最大异配）
        assert!((r + 1.0).abs() < 1e-9, "star 应为完全异配 r=-1，实际 {r}");
    }
}
