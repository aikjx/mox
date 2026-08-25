use async_trait::async_trait;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::error::Error;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlgoOutput {
    pub scores: Vec<(String, f64)>,
    pub communities: Vec<Vec<String>>,
    pub stats: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Default)]
struct SimpleGraph {
    adj: BTreeMap<String, Vec<(String, f64)>>,
}
impl SimpleGraph {
    #[allow(dead_code)] // 预留：SimpleGraph 构造辅助；当前算法路径未直接调用，保留以支撑后续图构造
    fn add_edge(&mut self, s: String, d: String, w: f64) {
        self.adj.entry(s).or_default().push((d, w));
    }
    fn node_set(&self) -> Vec<String> {
        let mut s: Vec<String> = self.adj.keys().cloned().collect();
        for vs in self.adj.values() {
            for (d, _) in vs {
                if !s.contains(d) {
                    s.push(d.clone());
                }
            }
        }
        s
    }
    fn degree(&self, dir: &str) -> BTreeMap<String, f64> {
        let mut m: BTreeMap<String, f64> = BTreeMap::new();
        match dir {
            "out" => {
                for (s, vs) in &self.adj {
                    *m.entry(s.clone()).or_insert(0.0) += vs.len() as f64;
                }
            }
            "in" => {
                for vs in self.adj.values() {
                    for (d, _) in vs {
                        *m.entry(d.clone()).or_insert(0.0) += 1.0;
                    }
                }
            }
            _ => {
                for (s, vs) in &self.adj {
                    *m.entry(s.clone()).or_insert(0.0) += vs.len() as f64;
                }
                for vs in self.adj.values() {
                    for (d, _) in vs {
                        *m.entry(d.clone()).or_insert(0.0) += 1.0;
                    }
                }
            }
        }
        m
    }
}

#[async_trait]
pub trait GraphAlgoSingleProvider: Send + Sync {
    async fn personalized_page_rank(
        &self,
        sp: &str,
        s: &str,
        d: f64,
        mi: u32,
        t: f64,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn cnm_communities(
        &self,
        sp: &str,
        r: f64,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn betweenness_centrality(
        &self,
        sp: &str,
        norm: bool,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn harmonic_closeness(
        &self,
        sp: &str,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn degree_centrality(
        &self,
        sp: &str,
        dir: &str,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn density(&self, sp: &str) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
    async fn raw_bidirectional_expand(
        &self,
        sp: &str,
        s: &str,
        d: &str,
        mh: u32,
        tk: u32,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>>;
}

pub struct MockGraphAlgoSingleProvider {
    g: parking_lot::Mutex<BTreeMap<String, SimpleGraph>>,
}
impl Default for MockGraphAlgoSingleProvider {
    fn default() -> Self {
        Self {
            g: parking_lot::Mutex::new(BTreeMap::new()),
        }
    }
}

#[async_trait]
impl GraphAlgoSingleProvider for MockGraphAlgoSingleProvider {
    async fn personalized_page_rank(
        &self,
        space: &str,
        src: &str,
        d: f64,
        max_iter: u32,
        tol: f64,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let nodes = gg.node_set();
        let n = nodes.len() as f64;
        if n == 0.0 {
            return Ok(AlgoOutput::default());
        }
        let mut score: BTreeMap<String, f64> = nodes.iter().map(|k| (k.clone(), 1.0 / n)).collect();
        for _ in 0..max_iter {
            let mut new: BTreeMap<String, f64> =
                nodes.iter().map(|k| (k.clone(), (1.0 - d) / n)).collect();
            let dangling: f64 = d * score
                .iter()
                .filter(|(k, _)| !gg.adj.contains_key(*k))
                .map(|(_, v)| *v)
                .sum::<f64>()
                / n;
            for (s, vs) in &gg.adj {
                let sz = vs.len() as f64;
                if sz > 0.0 {
                    for (dst, _) in vs {
                        *new.get_mut(dst).unwrap() += d * score[s] / sz;
                    }
                }
            }
            for v in new.values_mut() {
                *v += dangling;
            }
            let mut diff = 0.0;
            for k in &nodes {
                diff += (new[k] - score[k]).abs();
            }
            score = new;
            if diff < tol {
                break;
            }
        }
        // source bias: if src is in graph, return 0 everywhere
        let mut scores: Vec<(String, f64)> = score.into_iter().collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let _ = src; // mock PPR returns graph-based distribution
        Ok(AlgoOutput {
            scores,
            ..Default::default()
        })
    }
    async fn cnm_communities(
        &self,
        space: &str,
        _res: f64,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let nodes = gg.node_set();
        // BFS connected components
        let mut visited: HashSet<String> = HashSet::new();
        let mut comms = vec![];
        for start in &nodes {
            if visited.contains(start) {
                continue;
            }
            let mut q = VecDeque::new();
            q.push_back(start.clone());
            let mut comp = vec![];
            while let Some(v) = q.pop_front() {
                if !visited.insert(v.clone()) {
                    continue;
                }
                comp.push(v.clone());
                if let Some(nb) = gg.adj.get(&v) {
                    for (d, _) in nb {
                        q.push_back(d.clone());
                    }
                }
            }
            comp.sort();
            comms.push(comp);
        }
        comms.sort_by(|a, b| a.len().cmp(&b.len()).reverse());
        Ok(AlgoOutput {
            communities: comms,
            ..Default::default()
        })
    }
    async fn betweenness_centrality(
        &self,
        space: &str,
        norm: bool,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let nodes = gg.node_set();
        let mut bc: BTreeMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
        for s in &nodes {
            let mut st: Vec<String> = Vec::new();
            let mut pred: BTreeMap<String, Vec<String>> =
                nodes.iter().map(|n| (n.clone(), vec![])).collect();
            let mut sigma: BTreeMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
            let mut dist: BTreeMap<String, i32> = nodes.iter().map(|n| (n.clone(), -1)).collect();
            let mut q: VecDeque<String> = VecDeque::new();
            sigma.insert(s.clone(), 1.0);
            dist.insert(s.clone(), 0);
            q.push_back(s.clone());
            while let Some(v) = q.pop_front() {
                st.push(v.clone());
                if let Some(nb) = gg.adj.get(&v) {
                    for (w, _) in nb {
                        if dist[w] < 0 {
                            dist.insert(w.clone(), dist[&v] + 1);
                            q.push_back(w.clone());
                        }
                        if dist[w] == dist[&v] + 1 {
                            let sv = sigma[&v];
                            *sigma.get_mut(w).unwrap() += sv;
                            pred.get_mut(w).unwrap().push(v.clone());
                        }
                    }
                }
            }
            let mut delta: BTreeMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
            while let Some(w) = st.pop() {
                for v in &pred[&w] {
                    let vv = v.clone();
                    let f = sigma[&vv] / sigma[&w];
                    *delta.get_mut(vv.as_str()).unwrap() += f * (1.0 + delta[&w]);
                }
                if &w != s {
                    *bc.get_mut(&w).unwrap() += delta[&w];
                }
            }
        }
        let n = nodes.len();
        let mut scores: Vec<(String, f64)> = bc
            .into_iter()
            .map(|(k, v)| {
                let mut vv = v;
                if norm && n > 2 {
                    vv /= ((n - 1) * (n - 2)) as f64 / 2.0;
                }
                (k, vv)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(AlgoOutput {
            scores,
            ..Default::default()
        })
    }
    async fn harmonic_closeness(
        &self,
        space: &str,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let nodes = gg.node_set();
        let mut scores = vec![];
        for s in &nodes {
            let mut dist: BTreeMap<String, i32> = nodes.iter().map(|n| (n.clone(), -1)).collect();
            let mut q = VecDeque::new();
            q.push_back(s.clone());
            dist.insert(s.clone(), 0);
            while let Some(v) = q.pop_front() {
                if let Some(nb) = gg.adj.get(&v) {
                    for (w, _) in nb {
                        if dist[w] < 0 {
                            dist.insert(w.clone(), dist[&v] + 1);
                            q.push_back(w.clone());
                        }
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
            if nodes.len() > 1 {
                hc /= (nodes.len() - 1) as f64;
            }
            scores.push((s.clone(), hc));
        }
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(AlgoOutput {
            scores,
            ..Default::default()
        })
    }
    async fn degree_centrality(
        &self,
        space: &str,
        dir: &str,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let deg = gg.degree(dir);
        let n = gg.node_set().len() as f64;
        let mut scores: Vec<_> = deg
            .into_iter()
            .map(|(k, v)| (k, if n > 1.0 { v / (n - 1.0) } else { 0.0 }))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(AlgoOutput {
            scores,
            ..Default::default()
        })
    }
    async fn density(&self, space: &str) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        let n = gg.node_set().len();
        let m: usize = gg.adj.values().map(|v| v.len()).sum();
        let d = if n > 1 {
            (m as f64) / (n * (n - 1)) as f64
        } else {
            0.0
        };
        let mut stats = BTreeMap::new();
        stats.insert("density".into(), d);
        stats.insert("n".into(), n as f64);
        stats.insert("m".into(), m as f64);
        Ok(AlgoOutput {
            stats,
            ..Default::default()
        })
    }
    async fn raw_bidirectional_expand(
        &self,
        space: &str,
        src: &str,
        dst: &str,
        max_hop: u32,
        top_k: u32,
    ) -> Result<AlgoOutput, Box<dyn Error + Send + Sync>> {
        let g = self.g.lock();
        let gg = g.get(space).cloned().unwrap_or_default();
        // BFS from src up to max_hop layers; score by proximity (shorter path = higher score)
        let mut dist: BTreeMap<String, i32> = BTreeMap::new();
        let mut q = VecDeque::new();
        dist.insert(src.into(), 0);
        q.push_back(src.into());
        while let Some(v) = q.pop_front() {
            let dv = dist[&v];
            if dv >= max_hop as i32 {
                continue;
            }
            if let Some(nb) = gg.adj.get(&v) {
                for (w, _) in nb {
                    if !dist.contains_key(w) {
                        dist.insert(w.clone(), dv + 1);
                        q.push_back(w.clone());
                    }
                }
            }
        }
        let mut scored: Vec<(String, f64)> = dist
            .iter()
            .filter(|(k, _)| !k.is_empty())
            .map(|(k, d)| (k.clone(), if *d == 0 { 1.0 } else { 1.0 / (*d as f64) }))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k as usize);
        let _ = dst; // naive top-k; ignore dst in mock
        Ok(AlgoOutput {
            scores: scored,
            ..Default::default()
        })
    }
}
