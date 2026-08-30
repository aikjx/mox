// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use crate::types::PathResult;
use crate::Result;
use petgraph::algo::dijkstra;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

impl KnowledgeGraph {
    /// 最短路径 - Dijkstra算法
    pub fn shortest_path(&self, source: &str, target: &str) -> Result<Option<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let distances = dijkstra(&self.graph, *source_idx, Some(*target_idx), |e| *e.weight());

        if let Some(&dist) = distances.get(target_idx) {
            let mut path = Vec::new();
            let mut current = *target_idx;
            path.push(self.graph[current].id.clone());

            let mut predecessors = HashMap::new();
            for (node, &d) in &distances {
                for edge in self
                    .graph
                    .edges_directed(*node, petgraph::Direction::Incoming)
                {
                    let from = edge.source();
                    if let Some(&from_d) = distances.get(&from) {
                        if (d - from_d - edge.weight()).abs() < 1e-10 {
                            predecessors.insert(*node, from);
                        }
                    }
                }
            }

            while current != *source_idx {
                if let Some(&prev) = predecessors.get(&current) {
                    path.push(self.graph[prev].id.clone());
                    current = prev;
                } else {
                    break;
                }
            }
            path.reverse();

            Ok(Some(PathResult {
                path,
                total_weight: dist,
                length: distances.len(),
            }))
        } else {
            Ok(None)
        }
    }
}
