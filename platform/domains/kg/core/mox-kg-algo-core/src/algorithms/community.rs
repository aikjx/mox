// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use crate::types::Community;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

impl KnowledgeGraph {
    /// 社区发现：模块度贪心凝聚（CNM / Clauset-Newman-Moore 简化版）
    ///
    /// 修复 R-D3：此前用标签传播（LPA）存在两类缺陷：
    ///   1. 平局时 HashMap 迭代顺序随机 → 结果不可复现；
    ///   2. 标签吞并：双团+桥图坍缩为 1 社区（与 Node 层 D6/D9 同源缺陷）。
    ///
    /// 细节：
    /// - CNM：初始每节点一社区，反复合并 ΔQ 最大的相邻社区对，直到无正增益。
    ///   ΔQ(A,B) = e_cross(A,B)/m − d_A·d_B/(2m²)
    /// - 确定性：平局取 (社区A, 社区B) 字典序最小的对。
    /// - iterations 参数保留以兼容旧 API（仅作迭代上限保护，实际由增益收敛决定）。
    pub fn detect_communities(&self, iterations: usize) -> Vec<Community> {
        let n = self.node_count();
        if n == 0 {
            return Vec::new();
        }

        // 无向边集（合并方向、去重、跳过自环）
        let mut edge_set: HashSet<(usize, usize)> = HashSet::new();
        for edge in self.graph.edge_references() {
            let s = edge.source().index();
            let t = edge.target().index();
            if s != t {
                edge_set.insert((s.min(t), s.max(t)));
            }
        }
        let m = edge_set.len();
        if m == 0 {
            // 无边：每个节点自成社区
            let mut communities = Vec::new();
            for (i, id) in self.node_map.keys().enumerate() {
                communities.push(Community {
                    id: i,
                    nodes: vec![id.clone()],
                    density: 0.0,
                    label: format!("社区 {}", i),
                });
            }
            return communities;
        }

        // 度数（无向语义：每条 RAW 边对两端各贡献 1）
        let mut degree = vec![0usize; n];
        for &(s, t) in &edge_set {
            degree[s] += 1;
            degree[t] += 1;
        }

        // 社区状态
        let mut comm_of: Vec<usize> = (0..n).collect(); // 节点 → 社区 id（初始自身）
        let mut comm_members: Vec<Option<Vec<usize>>> = (0..n).map(|i| Some(vec![i])).collect();
        let mut comm_degree = degree.clone();
        let mut comm_alive: Vec<bool> = (0..n).map(|_| true).collect();

        // 社区间跨边计数：key (a<b)
        let mut cross: HashMap<(usize, usize), usize> = HashMap::new();
        for &(s, t) in &edge_set {
            // 初始每节点一社区，s≠t 必跨社区
            *cross.entry((s.min(t), s.max(t))).or_insert(0) += 1;
        }

        // 贪心合并循环（上限保护：n 次合并足够收敛）
        let max_merges = if iterations == 0 {
            n
        } else {
            iterations.min(n * n)
        };
        let mut merges = 0;
        loop {
            if merges >= max_merges {
                break;
            }
            // 找 ΔQ 最大的相邻社区对（确定性：平局取字典序最小）
            let mut candidates: Vec<((usize, usize), f64)> = Vec::new();
            for (&(a, b), &cnt) in &cross {
                if cnt == 0 || !comm_alive[a] || !comm_alive[b] {
                    continue;
                }
                // CNM 标准 ΔQ = e_ab − (Σ_a·Σ_b)/(2m)^2，其中 e_ab = cnt/m，Σ_x = d_x/(2m)。
                // 修正：原实现再乘 2 会把「边项」重复放大一倍（伪增益），导致过度合并、社区坍缩。
                let e_ab_over_m = cnt as f64 / m as f64;
                let degree_term =
                    (comm_degree[a] as f64 * comm_degree[b] as f64) / (2.0 * m as f64 * m as f64);
                let gain = e_ab_over_m - degree_term;
                candidates.push(((a, b), gain));
            }
            if candidates.is_empty() {
                break;
            }
            candidates.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap().then(x.0.cmp(&y.0)));
            let ((a, b), gain) = candidates[0];
            if gain <= 1e-12 {
                break; // 无正增益 → 收敛
            }

            // 合并 b 入 a（保小 id）
            let members_b = comm_members[b].clone().unwrap_or_default();
            for &node in &members_b {
                comm_of[node] = a;
                if let Some(members) = &mut comm_members[a] {
                    members.push(node);
                }
            }
            comm_degree[a] += comm_degree[b];
            comm_members[b] = None;
            comm_alive[b] = false;
            merges += 1;

            // 更新跨边：b 的所有跨边转入 a
            let keys: Vec<(usize, usize)> = cross.keys().copied().collect();
            for key in keys {
                let (x, y) = key;
                if x != b && y != b {
                    continue;
                }
                let cnt = cross.remove(&key).unwrap_or(0);
                if cnt == 0 {
                    continue;
                }
                let other = if x == b { y } else { x };
                if other == a || !comm_alive[other] {
                    continue; // a-b 间跨边随合并消失
                }
                let nk = (a.min(other), a.max(other));
                *cross.entry(nk).or_insert(0) += cnt;
            }
        }

        // 聚合输出：按规模降序
        let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
        for (i, alive) in comm_alive.iter().enumerate().take(n) {
            if !alive {
                continue;
            }
            if let Some(Some(members)) = comm_members.get(i).map(|m| m.as_ref()) {
                let ids: Vec<String> = members
                    .iter()
                    .map(|&node| self.graph[NodeIndex::new(node)].id.clone())
                    .collect();
                groups.push((i, ids));
            }
        }
        groups.sort_by(|x, y| y.1.len().cmp(&x.1.len()).then(x.0.cmp(&y.0)));

        let mut communities = Vec::new();
        for (i, (_, nodes)) in groups.into_iter().enumerate() {
            // 社区密度：内部边 / 最大可能边
            let density = if nodes.len() > 1 {
                let mut internal_edges = 0;
                for (j, n1) in nodes.iter().enumerate() {
                    for n2 in nodes.iter().skip(j + 1) {
                        if let (Some(idx1), Some(idx2)) = (
                            self.node_map.get(n1.as_str()),
                            self.node_map.get(n2.as_str()),
                        ) {
                            if self.graph.find_edge(*idx1, *idx2).is_some()
                                || self.graph.find_edge(*idx2, *idx1).is_some()
                            {
                                internal_edges += 1;
                            }
                        }
                    }
                }
                let max_edges = nodes.len() * (nodes.len() - 1) / 2;
                internal_edges as f64 / max_edges as f64
            } else {
                0.0
            };

            communities.push(Community {
                id: i,
                nodes,
                density,
                label: format!("社区 {}", i),
            });
        }

        communities
    }
}
