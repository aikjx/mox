// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::graph::KnowledgeGraph;
use crate::types::NodeRecommendation;
use std::collections::HashMap;

impl KnowledgeGraph {
    /// 智能推荐 - 基于激活传播和中心性
    pub fn recommend(&self, context_nodes: &[String], limit: usize) -> Vec<NodeRecommendation> {
        let mut scores = HashMap::new();
        let pagerank = self.pagerank(20);
        let centrality = self.degree_centrality();

        // 初始分数：PageRank + 中心性
        for id in self.node_map.keys() {
            if !context_nodes.contains(id) {
                let pr = pagerank.get(id).copied().unwrap_or(0.0);
                let dc = centrality.get(id).copied().unwrap_or(0.0);
                scores.insert(id.clone(), pr * 0.5 + dc * 0.3);
            }
        }

        // 基于上下文节点的关联度加分
        let score_ids: Vec<String> = scores.keys().cloned().collect();
        for context in context_nodes {
            for id in &score_ids {
                if let Ok(relevance) = self.total_relevance(context, id) {
                    if let Some(score) = scores.get_mut(id) {
                        *score += relevance * 0.2;
                    }
                }
            }
        }

        // 排序并生成推荐
        let mut sorted: Vec<_> = scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        sorted
            .into_iter()
            .take(limit)
            .map(|(node_id, score)| {
                let mut reasons = Vec::new();
                if let Some(node) = self.get_node(&node_id) {
                    reasons.push(format!("类型: {}", node.node_type));
                }
                reasons.push(format!("相关度得分: {:.4}", score));

                NodeRecommendation {
                    node_id,
                    score,
                    reasons,
                }
            })
            .collect()
    }
}
