// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use crate::types::GraphStats;
use petgraph::graph::NodeIndex;
use std::collections::HashSet;

impl KnowledgeGraph {
    /// 图统计信息
    pub fn stats(&self) -> GraphStats {
        let n = self.node_count();
        let m = self.edge_count();

        let density = if n > 1 {
            m as f64 / (n as f64 * (n as f64 - 1.0))
        } else {
            0.0
        };

        let average_degree = if n > 0 {
            2.0 * m as f64 / n as f64
        } else {
            0.0
        };

        // 简单聚类系数计算
        let mut clustering_sum = 0.0;
        for idx in self.node_map.values() {
            let neighbors: Vec<NodeIndex> = self
                .graph
                .neighbors(*idx)
                .chain(
                    self.graph
                        .neighbors_directed(*idx, petgraph::Direction::Incoming),
                )
                .collect();
            let unique_neighbors: HashSet<_> = neighbors.iter().collect();
            let k = unique_neighbors.len();

            if k >= 2 {
                let mut triangles = 0;
                let neighbor_vec: Vec<_> = unique_neighbors.into_iter().collect();
                for (i, &&n1) in neighbor_vec.iter().enumerate() {
                    for &&n2 in neighbor_vec.iter().skip(i + 1) {
                        if self.graph.find_edge(n1, n2).is_some()
                            || self.graph.find_edge(n2, n1).is_some()
                        {
                            triangles += 1;
                        }
                    }
                }
                clustering_sum += (2 * triangles) as f64 / (k * (k - 1)) as f64;
            }
        }

        let clustering_coefficient = if n > 0 {
            clustering_sum / n as f64
        } else {
            0.0
        };

        GraphStats {
            node_count: n,
            edge_count: m,
            density,
            average_degree,
            strongly_connected_components: 1,
            diameter: None,
            clustering_coefficient,
        }
    }
}
