// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::f64::consts::E;

impl KnowledgeGraph {
    /// 激活传播 - AI神经网络风格传播
    pub fn propagate_activation(
        &mut self,
        start_nodes: &[String],
        iterations: usize,
    ) -> HashMap<String, f64> {
        // 重置激活值
        for idx in self.node_map.values() {
            self.graph[*idx].activation = 0.0;
        }

        // 设置初始激活
        for node_id in start_nodes {
            if let Some(&idx) = self.node_map.get(node_id) {
                self.graph[idx].activation = 1.0;
            }
        }

        let n = self.node_count();
        let indices: Vec<NodeIndex> = self.node_map.values().copied().collect();

        for _ in 0..iterations {
            let mut new_activations = vec![0.0; n];

            for (i, &idx) in indices.iter().enumerate() {
                let mut incoming = 0.0;
                for edge in self
                    .graph
                    .edges_directed(idx, petgraph::Direction::Incoming)
                {
                    let weight = *edge.weight();
                    incoming += self.graph[edge.source()].activation * weight;
                }

                // Sigmoid激活函数
                let current = self.graph[idx].activation;
                new_activations[i] = 1.0 / (1.0 + E.powf(-incoming)) * 0.3 + current * 0.7;
            }

            for (i, &idx) in indices.iter().enumerate() {
                self.graph[idx].activation = new_activations[i];
            }
        }

        // 记录历史
        let mut activations = HashMap::new();
        for (id, idx) in &self.node_map {
            activations.insert(id.clone(), self.graph[*idx].activation);
        }
        self.activation_history.push(activations.clone());
        activations
    }
}
