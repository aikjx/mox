// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use crate::types::CentralityMetrics;
use petgraph::algo::dijkstra;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

impl KnowledgeGraph {
    /// 度中心性
    ///
    /// 修复 R-D4：此前除以 2(n-1)（把无向图当双向计算），与 Node 层 F2 语义不一致。
    /// 统一为 C_D(v) = deg(v)/(N-1)，deg = 入度+出度（无向度语义，与 Node 层一致）。
    pub fn degree_centrality(&self) -> HashMap<String, f64> {
        let n = self.node_count() as f64;
        let mut result = HashMap::new();

        for (id, idx) in &self.node_map {
            let in_degree = self
                .graph
                .edges_directed(*idx, petgraph::Direction::Incoming)
                .count() as f64;
            let out_degree = self
                .graph
                .edges_directed(*idx, petgraph::Direction::Outgoing)
                .count() as f64;
            if n > 1.0 {
                result.insert(id.clone(), (in_degree + out_degree) / (n - 1.0));
            } else {
                result.insert(id.clone(), 0.0);
            }
        }
        result
    }

    /// 介数中心性（Brandes 2001，有向图版）
    ///
    /// 修复 R-D1：此前 centrality_metrics() 中该指标为空占位符（HashMap::new()）。
    /// C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st，BFS 最短路计数 + 反向依赖累积，
    /// 归一化除以 (N-1)(N-2)（有向）。
    pub fn betweenness_centrality(&self) -> HashMap<String, f64> {
        let n = self.node_count();
        let mut cb = vec![0.0f64; n];
        if n < 3 {
            let mut result = HashMap::new();
            for id in self.node_map.keys() {
                result.insert(id.clone(), 0.0);
            }
            return result;
        }

        // 邻接表（有向）
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for edge in self.graph.edge_references() {
            adj[edge.source().index()].push(edge.target().index());
        }

        for s in 0..n {
            // BFS 最短路计数
            let mut dist = vec![-1i64; n];
            let mut sigma = vec![0.0f64; n];
            let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
            let mut order: Vec<usize> = Vec::with_capacity(n);
            let mut queue = std::collections::VecDeque::new();

            dist[s] = 0;
            sigma[s] = 1.0;
            queue.push_back(s);

            while let Some(v) = queue.pop_front() {
                order.push(v);
                for &w in &adj[v] {
                    if dist[w] < 0 {
                        dist[w] = dist[v] + 1;
                        queue.push_back(w);
                    }
                    if dist[w] == dist[v] + 1 {
                        sigma[w] += sigma[v];
                        preds[w].push(v);
                    }
                }
            }

            // 反向累积依赖（δ）
            let mut delta = vec![0.0f64; n];
            for &w in order.iter().rev() {
                for &v in &preds[w] {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
                if w != s {
                    cb[w] += delta[w];
                }
            }
        }

        // 归一化：(N-1)(N-2)
        let norm = ((n - 1) * (n - 2)) as f64;
        let mut result = HashMap::new();
        for (id, idx) in &self.node_map {
            result.insert(id.clone(), cb[idx.index()] / norm);
        }
        result
    }

    /// 紧密中心性（harmonic 版本，对不可达节点稳健）
    ///
    /// 修复 R-D5：harmonic：C_C(v) = (Σ_{u≠v} 1/d(v,u))/(N-1)，不可达贡献 0。
    /// 性能：CSR 判定 all_unit_weight 时改用 BFS（O(N+E)·N，跳过 dijkstra 二叉堆的 log N）；
    /// 否则退化为 Dijkstra（保证有向加权图正确）。
    pub fn closeness_centrality(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        let n = self.node_count();
        if n == 0 {
            return result;
        }

        let csr = CsrAdj::from_graph(&self.graph);
        if csr.all_unit_weight {
            // BFS 版（无权图最短路 = hop 数，距离 = 层数，作为 f64）
            let mut dist = vec![-1i32; n];
            let mut queue = std::collections::VecDeque::with_capacity(n);
            for (id, idx) in &self.node_map {
                let s = idx.index();
                for x in dist.iter_mut() {
                    *x = -1;
                }
                dist[s] = 0;
                queue.clear();
                queue.push_back(s);
                while let Some(u) = queue.pop_front() {
                    let rng = csr.offsets[u]..csr.offsets[u + 1];
                    for k in rng {
                        let v = csr.targets[k];
                        if dist[v] < 0 {
                            dist[v] = dist[u] + 1;
                            queue.push_back(v);
                        }
                    }
                }
                let mut harmonic = 0.0f64;
                for (_u, &d_u) in dist.iter().enumerate().take(n) {
                    if d_u > 0 {
                        harmonic += 1.0 / d_u as f64;
                    }
                }
                let value = if n > 1 {
                    harmonic / (n as f64 - 1.0)
                } else {
                    0.0
                };
                result.insert(id.clone(), value);
            }
        } else {
            for (id, idx) in &self.node_map {
                let distances = dijkstra(&self.graph, *idx, None, |e| *e.weight());
                let mut harmonic = 0.0f64;
                for (other, &d) in &distances {
                    if *other != *idx && d > 0.0 {
                        harmonic += 1.0 / d;
                    }
                }
                let value = if n > 1 {
                    harmonic / (n as f64 - 1.0)
                } else {
                    0.0
                };
                result.insert(id.clone(), value);
            }
        }
        result
    }

    /// 综合中心性指标（一次构造 CSR 复用给 PageRank/Closeness）
    pub fn centrality_metrics(&self) -> CentralityMetrics {
        let n = self.node_count();
        let alpha = self.damping_factor;
        let pr: HashMap<String, f64>;
        let closeness: HashMap<String, f64>;

        if std::env::var("GRAPH_LEGACY_DENSE").is_ok() {
            pr = self.pagerank_dense_legacy(20);
            closeness = self.closeness_centrality(); // Dijkstra fallback 不受 env 影响
        } else if n == 0 {
            pr = HashMap::new();
            closeness = HashMap::new();
        } else {
            // 只做 1 次 CSR 构造
            let csr = CsrAdj::from_graph(&self.graph);
            let pr_vec = csr.pagerank(alpha, 20);
            pr = crate::csr::rank_vec_to_map(&pr_vec, &self.node_map);
            closeness = self.closeness_centrality_with_csr(&csr);
        }

        CentralityMetrics {
            degree_centrality: self.degree_centrality(),
            betweenness_centrality: self.betweenness_centrality(),
            pagerank: pr,
            closeness_centrality: closeness,
        }
    }

    /// closeness_centrality 复用调用方已构造好的 CSR（避免重复扫描边）。
    pub(crate) fn closeness_centrality_with_csr(&self, csr: &CsrAdj) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        let n = self.node_count();
        if n == 0 {
            return result;
        }
        if csr.all_unit_weight {
            let mut dist = vec![-1i32; n];
            let mut queue = std::collections::VecDeque::with_capacity(n);
            for (id, idx) in &self.node_map {
                let s = idx.index();
                for x in dist.iter_mut() {
                    *x = -1;
                }
                dist[s] = 0;
                queue.clear();
                queue.push_back(s);
                while let Some(u) = queue.pop_front() {
                    let rng = csr.offsets[u]..csr.offsets[u + 1];
                    for k in rng {
                        let v = csr.targets[k];
                        if dist[v] < 0 {
                            dist[v] = dist[u] + 1;
                            queue.push_back(v);
                        }
                    }
                }
                let mut harmonic = 0.0f64;
                for (_u, &d_u) in dist.iter().enumerate().take(n) {
                    if d_u > 0 {
                        harmonic += 1.0 / d_u as f64;
                    }
                }
                let value = if n > 1 {
                    harmonic / (n as f64 - 1.0)
                } else {
                    0.0
                };
                result.insert(id.clone(), value);
            }
        } else {
            for (id, idx) in &self.node_map {
                let distances = dijkstra(&self.graph, *idx, None, |e| *e.weight());
                let mut harmonic = 0.0f64;
                for (other, &d) in &distances {
                    if *other != *idx && d > 0.0 {
                        harmonic += 1.0 / d;
                    }
                }
                let value = if n > 1 {
                    harmonic / (n as f64 - 1.0)
                } else {
                    0.0
                };
                result.insert(id.clone(), value);
            }
        }
        result
    }
}
