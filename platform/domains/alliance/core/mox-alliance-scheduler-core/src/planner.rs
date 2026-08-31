// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 协作计划生成器
//!
//! 根据任务描述和匹配的专家，生成协作执行计划（DAG）。
//!
//! Phase 1 实现简单的计划生成：
//! - 并行模式：所有专家并行执行
//! - 串行模式：专家按匹配分数排序后串行执行
//! - 分层模式：按领域分组，组内并行，组间串行
//!
//! 后续可以接入 AI 生成更复杂的计划。

use mox_alliance_common_proto::{
    AllianceError, AllianceErrorCode, AllianceMode, AllianceResult, CollaborationPlan, Expert,
    Node, NodeStatus,
};
use uuid::Uuid;

use mox_alliance_scheduler_proto::{MatchedExpert, PlanGenerationRequest};

/// 简单计划生成器
pub struct SimplePlanGenerator;

impl SimplePlanGenerator {
    pub fn new() -> Self {
        Self
    }

    /// 生成协作计划
    pub fn generate(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> AllianceResult<CollaborationPlan> {
        let mode = request.preferred_mode.unwrap_or(AllianceMode::Parallel);

        let nodes = match mode {
            AllianceMode::Parallel => self.generate_parallel_plan(request, matched_experts),
            AllianceMode::Sequential => self.generate_sequential_plan(request, matched_experts),
            AllianceMode::Voting => self.generate_parallel_plan(request, matched_experts), // 投票也是并行
            AllianceMode::Hierarchical => self.generate_hierarchical_plan(request, matched_experts),
            AllianceMode::Debate => self.generate_debate_plan(request, matched_experts),
            AllianceMode::Iterative => self.generate_iterative_plan(request, matched_experts),
        };

        let plan = CollaborationPlan {
            task_id: request.task_id,
            mode,
            fusion_strategy: request.fusion_strategy,
            nodes,
            version: 1,
            created_at: chrono::Utc::now(),
        };

        // 验证计划有效性
        plan.validate().map_err(|e| {
            AllianceError::new(AllianceErrorCode::PlanGenerationFailed, e)
        })?;

        Ok(plan)
    }

    /// 并行计划：所有专家无依赖，同时执行
    fn generate_parallel_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        matched_experts
            .iter()
            .enumerate()
            .map(|(i, me)| {
                self.make_node(
                    request.task_id,
                    &format!("node-{}", i + 1),
                    &me.expert,
                    vec![], // 无依赖
                    &format!("{} (并行)", me.expert.name),
                )
            })
            .collect()
    }

    /// 串行计划：专家按分数排序，依次执行
    fn generate_sequential_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        let mut nodes = Vec::new();
        for (i, me) in matched_experts.iter().enumerate() {
            let deps = if i == 0 {
                vec![]
            } else {
                vec![format!("node-{}", i)]
            };
            nodes.push(self.make_node(
                request.task_id,
                &format!("node-{}", i + 1),
                &me.expert,
                deps,
                &format!("{} (第{}步)", me.expert.name, i + 1),
            ));
        }
        nodes
    }

    /// 分层计划：按领域分组，组内并行，组间串行
    fn generate_hierarchical_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        // 按领域分组
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<String, Vec<&MatchedExpert>> = BTreeMap::new();
        for me in matched_experts {
            let domain = me.expert.domains.first().cloned().unwrap_or_else(|| "other".to_string());
            groups.entry(domain).or_default().push(me);
        }

        let mut nodes = Vec::new();
        let mut prev_layer_last: Vec<String> = Vec::new();
        let mut node_counter = 0;

        for (_, group) in groups {
            let mut current_layer: Vec<String> = Vec::new();

            for me in group {
                node_counter += 1;
                let node_id = format!("node-{}", node_counter);

                // 依赖上一层的所有节点
                let deps = prev_layer_last.clone();

                nodes.push(self.make_node(
                    request.task_id,
                    &node_id,
                    &me.expert,
                    deps,
                    &me.expert.name,
                ));
                current_layer.push(node_id);
            }

            prev_layer_last = current_layer;
        }

        nodes
    }

    /// 辩论计划：正方 + 反方 + 裁判
    fn generate_debate_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        if matched_experts.len() < 2 {
            // 不够 2 个专家就退化为并行
            return self.generate_parallel_plan(request, matched_experts);
        }

        let mut nodes = Vec::new();

        // 正方（第一个专家）
        nodes.push(self.make_node(
            request.task_id,
            "node-pro",
            &matched_experts[0].expert,
            vec![],
            "正方观点",
        ));

        // 反方（第二个专家）
        nodes.push(self.make_node(
            request.task_id,
            "node-con",
            &matched_experts[1].expert,
            vec![],
            "反方观点",
        ));

        // 裁判（第三个专家，如果有的话）
        if matched_experts.len() >= 3 {
            nodes.push(self.make_node(
                request.task_id,
                "node-judge",
                &matched_experts[2].expert,
                vec!["node-pro".to_string(), "node-con".to_string()],
                "裁判裁决",
            ));
        }

        nodes
    }

    /// 迭代计划：多轮迭代，逐步精炼
    fn generate_iterative_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        let iterations = 3; // 默认 3 轮迭代
        let mut nodes = Vec::new();
        let mut prev_node: Option<String> = None;

        for i in 0..iterations {
            for (j, me) in matched_experts.iter().take(2).enumerate() {
                let node_id = format!("node-iter{}-{}", i + 1, j + 1);
                let mut deps = Vec::new();

                if i > 0 {
                    // 依赖上一轮的最后一个节点
                    deps.push(format!("node-iter{}-{}", i, matched_experts.len().min(2)));
                } else if let Some(ref prev) = prev_node {
                    deps.push(prev.clone());
                }

                nodes.push(self.make_node(
                    request.task_id,
                    &node_id,
                    &me.expert,
                    deps,
                    &format!("第{}轮 - {}", i + 1, me.expert.name),
                ));

                prev_node = Some(node_id);
            }
        }

        nodes
    }

    /// 创建一个节点
    fn make_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        expert: &Expert,
        dependencies: Vec<String>,
        description: &str,
    ) -> Node {
        Node {
            node_id: node_id.to_string(),
            task_id,
            expert_id: expert.expert_id.clone(),
            name: expert.name.clone(),
            description: Some(description.to_string()),
            status: NodeStatus::Pending,
            retry_count: 0,
            dependencies,
            input_refs: vec![],
            output_ref: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
        }
    }
}

impl Default for SimplePlanGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{Expert, FusionStrategy};

    fn make_matched_expert(id: &str, name: &str, domains: Vec<&str>) -> MatchedExpert {
        let expert = Expert {
            expert_id: id.to_string(),
            tenant_id: "system".to_string(),
            name: name.to_string(),
            version: "1.0".to_string(),
            description: name.to_string(),
            domains: domains.into_iter().map(|s| s.to_string()).collect(),
            capabilities: vec![],
            tools: vec![],
            status: mox_alliance_common_proto::ExpertStatus::Active,
            health: mox_alliance_common_proto::ExpertHealth::default(),
            priority: 5,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        MatchedExpert {
            expert,
            score: 0.8,
            match_reason: "test".to_string(),
            score_breakdown: mox_alliance_scheduler_proto::MatchScoreBreakdown {
                domain_match: 0.8,
                capability_match: 0.7,
                health_score: 1.0,
                priority_score: 0.5,
                performance_score: 0.9,
            },
        }
    }

    #[test]
    fn test_parallel_plan() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "Expert 1", vec!["code"]),
            make_matched_expert("e2", "Expert 2", vec!["security"]),
        ];
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: "test".to_string(),
            preferred_mode: Some(AllianceMode::Parallel),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &experts).unwrap();
        assert_eq!(plan.nodes.len(), 2);
        assert!(plan.nodes[0].dependencies.is_empty());
        assert!(plan.nodes[1].dependencies.is_empty());
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_sequential_plan() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "Expert 1", vec!["code"]),
            make_matched_expert("e2", "Expert 2", vec!["security"]),
            make_matched_expert("e3", "Expert 3", vec!["data"]),
        ];
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: "test".to_string(),
            preferred_mode: Some(AllianceMode::Sequential),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &experts).unwrap();
        assert_eq!(plan.nodes.len(), 3);
        assert!(plan.nodes[0].dependencies.is_empty());
        assert_eq!(plan.nodes[1].dependencies, vec!["node-1"]);
        assert_eq!(plan.nodes[2].dependencies, vec!["node-2"]);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_debate_plan() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "Pro Expert", vec!["analysis"]),
            make_matched_expert("e2", "Con Expert", vec!["analysis"]),
            make_matched_expert("e3", "Judge Expert", vec!["analysis"]),
        ];
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: "test".to_string(),
            preferred_mode: Some(AllianceMode::Debate),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &experts).unwrap();
        assert_eq!(plan.nodes.len(), 3);
        assert_eq!(plan.nodes[2].dependencies.len(), 2); // 裁判依赖正反方
        assert!(plan.validate().is_ok());
    }
}
