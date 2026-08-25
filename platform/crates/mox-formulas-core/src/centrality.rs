//! 中心性：度 / Brandes 介数（rayon 并行）/ Harmonic 紧密 / 特征向量中心性（幂迭代 CSR）

use crate::csr::CsrGraph;
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;

impl CsrGraph {
    // =========================================================
    //  F2 · 度中心性（RAW 边 incident/(N−1)）
    // =========================================================
    pub fn degree_centrality(&self) -> HashMap<String, f64> {
        let n = self.n;
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f64 };
        let mut out = HashMap::with_capacity(n);
        for i in 0..n {
            let out_d = (self.out_off[i + 1] - self.out_off[i]) as f64;
            let in_d = (self.in_off[i + 1] - self.in_off[i]) as f64;
            out.insert(self.ids[i].clone(), (out_d + in_d) / denom);
        }
        out
    }

    // =========================================================
    //  F3 · Brandes 介数中心性（并行，O(N·E)）
    // =========================================================
    pub fn betweenness_centrality(&self) -> HashMap<String, f64> {
        let n = self.n;
        if n < 3 {
            let mut out = HashMap::with_capacity(n);
            for id in &self.ids { out.insert(id.clone(), 0.0); }
            return out;
        }
        let contributions: Vec<Vec<f64>> = (0..n)
            .into_par_iter()
            .map(|s| self.brandes_one(s))
            .collect();
        let mut cb = vec![0.0f64; n];
        for local in &contributions {
            for i in 0..n { cb[i] += local[i]; }
        }
        let norm = ((n - 1) * (n - 2)) as f64;
        let mut out = HashMap::with_capacity(n);
        for i in 0..n { out.insert(self.ids[i].clone(), cb[i] / norm); }
        out
    }

    fn brandes_one(&self, s: usize) -> Vec<f64> {
        let n = self.n;
        let mut dist = vec![-1i32; n];
        let mut sigma = vec![0.0f64; n];
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::with_capacity(n);
        let mut q = VecDeque::with_capacity(n);
        dist[s] = 0; sigma[s] = 1.0; q.push_back(s);
        while let Some(v) = q.pop_front() {
            order.push(v);
            let rng = self.out_off[v]..self.out_off[v + 1];
            for k in rng {
                let w = self.out_nbr[k];
                if dist[w] < 0 { dist[w] = dist[v] + 1; q.push_back(w); }
                if dist[w] == dist[v] + 1 { sigma[w] += sigma[v]; preds[w].push(v); }
            }
        }
        let mut delta = vec![0.0f64; n];
        for &w in order.iter().rev() {
            let sw = sigma[w];
            if sw > 0.0 {
                let inv = 1.0 / sw;
                for &v in &preds[w] { delta[v] += sigma[v] * inv * (1.0 + delta[w]); }
            }
        }
        let mut cb = vec![0.0f64; n];
        for (i, &d) in delta.iter().enumerate().take(n) {
            if i != s { cb[i] = d; }
        }
        cb
    }

    // =========================================================
    //  F5 · Harmonic 紧密中心性（并行）
    // =========================================================
    pub fn closeness_harmonic(&self) -> HashMap<String, f64> {
        let n = self.n;
        if n == 0 { return HashMap::new(); }
        let values: Vec<f64> = (0..n)
            .into_par_iter()
            .map(|s| if self.all_unit { self.harmonic_bfs(s) } else { self.harmonic_dijkstra(s) })
            .collect();
        let mut out = HashMap::with_capacity(n);
        for i in 0..n { out.insert(self.ids[i].clone(), values[i]); }
        out
    }

    fn harmonic_bfs(&self, s: usize) -> f64 {
        let n = self.n;
        let mut dist = vec![-1i32; n];
        let mut q = VecDeque::with_capacity(n);
        dist[s] = 0; q.push_back(s);
        while let Some(u) = q.pop_front() {
            let rng = self.out_off[u]..self.out_off[u + 1];
            for k in rng {
                let v = self.out_nbr[k];
                if dist[v] < 0 { dist[v] = dist[u] + 1; q.push_back(v); }
            }
        }
        let mut acc = 0.0f64;
        for d in dist.iter().take(n) { if *d > 0 { acc += 1.0 / (*d as f64); } }
        if n > 1 { acc / (n as f64 - 1.0) } else { 0.0 }
    }

    fn harmonic_dijkstra(&self, s: usize) -> f64 {
        let n = self.n;
        let mut dist = vec![f64::INFINITY; n];
        dist[s] = 0.0;
        #[derive(PartialEq, PartialOrd)]
        struct OrdF64(f64);
        impl Eq for OrdF64 {}
        #[allow(clippy::derive_ord_xor_partial_ord)]
        impl std::cmp::Ord for OrdF64 {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                other.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
        let mut heap = std::collections::BinaryHeap::with_capacity(n);
        heap.push((OrdF64(0.0), s));
        while let Some((OrdF64(d), u)) = heap.pop() {
            if d > dist[u] + 1e-15 { continue; }
            let rng = self.out_off[u]..self.out_off[u + 1];
            for k in rng {
                let v = self.out_nbr[k];
                let w = self.out_w[k];
                let nd = d + w;
                if nd < dist[v] { dist[v] = nd; heap.push((OrdF64(nd), v)); }
            }
        }
        let mut acc = 0.0f64;
        for &d in dist.iter().take(n) {
            if d.is_finite() && d > 1e-15 { acc += 1.0 / d; }
        }
        if n > 1 { acc / (n as f64 - 1.0) } else { 0.0 }
    }

    // =========================================================
    //  F10 · 特征向量中心性（幂迭代）
    // =========================================================
    pub fn eigenvector_centrality(&self, max_iter: usize, eps: f64) -> HashMap<String, f64> {
        let n = self.n;
        if n == 0 { return HashMap::new(); }
        let mut x = vec![1.0 / (n as f64).sqrt(); n];
        let mut y = vec![0.0f64; n];
        for _ in 0..max_iter {
            for v in y.iter_mut().take(n) { *v = 0.0; }
            for i in 0..n {
                let rng = self.out_off[i]..self.out_off[i + 1];
                let xi = x[i];
                for k in rng {
                    let j = self.out_nbr[k];
                    let w = self.out_w[k];
                    y[j] += w * xi;
                }
            }
            let mut norm2 = 0.0f64;
            for &v in &y { norm2 += v * v; }
            let nrm = norm2.sqrt();
            if nrm < 1e-18 { break; }
            let inv = 1.0 / nrm;
            let mut max_diff = 0.0f64;
            for i in 0..n {
                let nv = y[i] * inv;
                let d = (nv - x[i]).abs();
                if d > max_diff { max_diff = d; }
                x[i] = nv;
            }
            if max_diff < eps { break; }
        }
        let mut out = HashMap::with_capacity(n);
        for i in 0..n { out.insert(self.ids[i].clone(), x[i]); }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::RawExpand;
    use crate::{EdgeInput, NodeInput};

    fn star() -> (Vec<NodeInput>, Vec<EdgeInput>) {
        let nodes = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None })
            .collect();
        let edges = vec!["a", "b", "d", "e"]
            .into_iter()
            .map(|t| EdgeInput { source: "c".into(), target: t.into(), weight: 1.0, relation_type: None })
            .collect();
        (nodes, edges)
    }

    #[test]
    fn degree_directed_matches_tr42() {
        let (nodes, edges) = star();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::None);
        let d = g.degree_centrality();
        assert!((d["c"] - 1.0).abs() < 1e-15, "c incident=4, N-1=4 → 1.0");
        assert!((d["a"] - 0.25).abs() < 1e-15, "a incident=1, N-1=4 → 0.25");
    }

    #[test]
    fn brandes_chain() {
        let nodes = ["a","b","c","d"].iter()
            .map(|s| NodeInput { id: (*s).to_string(), label: None, properties: None }).collect();
        let edges = vec![("a","b"),("b","c"),("c","d")].into_iter()
            .map(|(s,t)| EdgeInput { source: s.into(), target: t.into(), weight: 1.0, relation_type: None }).collect();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let cb = g.betweenness_centrality();
        assert_eq!(cb["a"], 0.0);
        assert_eq!(cb["d"], 0.0);
        assert!(cb["b"] > 0.0 && cb["c"] > 0.0);
    }

    #[test]
    fn harmonic_star_center_highest() {
        let (nodes, edges) = star();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let h = g.closeness_harmonic();
        for id in ["a","b","d","e"] { assert!(h["c"] >= h[id]); }
    }

    #[test]
    fn eigenvector_sos_eq_one() {
        let (nodes, edges) = star();
        let g = CsrGraph::from_inputs(&nodes, &edges, RawExpand::Undirected);
        let ev = g.eigenvector_centrality(500, 1e-12);
        let s2: f64 = ev.values().map(|x| x*x).sum();
        assert!((s2 - 1.0).abs() < 1e-9);
    }
}
