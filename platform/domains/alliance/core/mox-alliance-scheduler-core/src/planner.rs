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
use std::collections::HashMap;
use uuid::Uuid;

use mox_alliance_scheduler_proto::{MatchScoreBreakdown, MatchedExpert, PlanGenerationRequest};

/// 动态模式（Dynamic）的规划期路由决策
///
/// 协议层语义：「根据中间结果动态决定下一步」。
/// 当前实现为**规划期动态选型**：依据任务意图与专家结构特征，在既有拓扑中选定最适配的一种。
///
/// 诚实边界：这是确定性启发式规则，非学习得到；执行器侧「按中间结果实时改写拓扑」
/// 尚未实现，故本决策在计划生成时一次性完成并留痕。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRoutingDecision {
    /// 选定的协作拓扑
    pub mode: AllianceMode,
    /// 决策理由（写入首节点描述与日志，便于审计）
    pub reason: String,
}

/// 简单计划生成器
pub struct SimplePlanGenerator {
    /// 专家 ID → 模块配置 ID 的可选映射（用于为节点填充 module_id）
    expert_to_module: Option<HashMap<String, String>>,
}

impl SimplePlanGenerator {
    pub fn new() -> Self {
        Self { expert_to_module: None }
    }

    /// 注入专家→模块配置映射（expert_id → module_id），节点将携带模块配置 ID
    pub fn with_expert_to_module(mut self, map: HashMap<String, String>) -> Self {
        self.expert_to_module = Some(map);
        self
    }

    /// 生成协作计划
    pub fn generate(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> AllianceResult<CollaborationPlan> {
        let mode = request.preferred_mode.unwrap_or(AllianceMode::Parallel);

        // 0 匹配兜底：入选专家为空时不静默产出 0 节点「假成功」计划，
        // 退化为单个通用专家单跑（单跑场景天然融合策略为 BestOf，见 scheduler 自动选型）。
        let effective: Vec<MatchedExpert> = if matched_experts.is_empty() {
            vec![Self::generic_fallback_expert()]
        } else {
            matched_experts.to_vec()
        };
        // 影子绑定：后续节点生成逻辑继续引用 matched_experts，此时指向兜底后的有效列表。
        let matched_experts: &[MatchedExpert] = &effective;
        // 匹配分 → 尾部融合真权重（expert_id -> score），替换原先恒空 HashMap 的等权行为。
        let expert_weights: HashMap<String, f64> = effective
            .iter()
            .map(|me| (me.expert.expert_id.clone(), me.score))
            .collect();

        let nodes = match mode {
            AllianceMode::Parallel => self.generate_parallel_plan(request, matched_experts),
            AllianceMode::Sequential => self.generate_sequential_plan(request, matched_experts),
            AllianceMode::Voting => self.generate_voting_plan(request, matched_experts), // 投票模式：同题多解
            AllianceMode::Hierarchical => self.generate_hierarchical_plan(request, matched_experts),
            AllianceMode::Debate => self.generate_debate_plan(request, matched_experts),
            AllianceMode::Iterative => self.generate_iterative_plan(request, matched_experts),
            AllianceMode::Dynamic => self.generate_dynamic_plan(request, matched_experts),
        };

        let plan = CollaborationPlan {
            task_id: request.task_id,
            mode,
            fusion_strategy: request.fusion_strategy,
            nodes,
            expert_weights,
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
                    &request.task_description,
                )
            })
            .collect()
    }

    /// 投票计划：所有专家对同一问题独立作答，结果投票融合
    fn generate_voting_plan(
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
                    &format!("{} (投票)", me.expert.name),
                    &request.task_description,
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
                &request.task_description,
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
                    &request.task_description,
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
            &request.task_description,
        ));

        // 反方（第二个专家）
        nodes.push(self.make_node(
            request.task_id,
            "node-con",
            &matched_experts[1].expert,
            vec![],
            "反方观点",
            &request.task_description,
        ));

        // 裁判（第三个专家，如果有的话）
        if matched_experts.len() >= 3 {
            nodes.push(self.make_node(
                request.task_id,
                "node-judge",
                &matched_experts[2].expert,
                vec!["node-pro".to_string(), "node-con".to_string()],
                "裁判裁决",
                &request.task_description,
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
                    &request.task_description,
                ));

                prev_node = Some(node_id);
            }
        }

        nodes
    }

    // ── 动态模式（Dynamic）──────────────────────────────────────────────
    // 动态模式不新增第七套拓扑，而是在既有六种拓扑中按特征选型，避免「多一种模式
    // 多一套不可测分支」。以下阈值为显式常量，便于单测锁定与后续调优。

    /// 匹配分极差 ≥ 此值：强弱分明 → 分层（高分者领衔）
    const DYNAMIC_SPREAD_HIERARCHICAL: f64 = 0.25;
    /// 匹配分极差 < 此值 且专家数 ≥ 3：实力接近 → 投票裁决
    const DYNAMIC_SPREAD_VOTING: f64 = 0.10;
    /// 领域去重数 ≥ 此值：跨域协作 → 分层协同
    const DYNAMIC_DOMAIN_DIVERSITY_MIN: usize = 3;

    /// 动态模式：规划期选定拓扑并生成对应计划，决策理由写入首节点描述
    fn generate_dynamic_plan(
        &self,
        request: &PlanGenerationRequest,
        matched_experts: &[MatchedExpert],
    ) -> Vec<Node> {
        let decision = self.decide_dynamic_mode(request, matched_experts);
        tracing::info!(
            task_id = %request.task_id,
            selected = ?decision.mode,
            reason = %decision.reason,
            "dynamic routing: collaboration topology selected"
        );

        let mut nodes = match decision.mode {
            AllianceMode::Sequential => self.generate_sequential_plan(request, matched_experts),
            AllianceMode::Hierarchical => self.generate_hierarchical_plan(request, matched_experts),
            AllianceMode::Debate => self.generate_debate_plan(request, matched_experts),
            AllianceMode::Voting => self.generate_voting_plan(request, matched_experts),
            AllianceMode::Iterative => self.generate_iterative_plan(request, matched_experts),
            _ => self.generate_parallel_plan(request, matched_experts),
        };

        // 决策留痕：前端 DAG 与审计日志可见「为何选这个拓扑」
        if let Some(first) = nodes.first_mut() {
            let cur = first.description.clone().unwrap_or_default();
            first.description = Some(format!(
                "[动态路由→{}] {}。{}",
                Self::dynamic_mode_label(decision.mode),
                decision.reason,
                cur
            ));
        }
        nodes
    }

    /// 动态模式决策规则（确定性，优先级自上而下短路）
    fn decide_dynamic_mode(
        &self,
        request: &PlanGenerationRequest,
        experts: &[MatchedExpert],
    ) -> DynamicRoutingDecision {
        let desc = request.task_description.to_lowercase();

        // 1) 任务文本中的显式意图优先（用户说了算）
        if ["迭代", "优化", "反复", "改进", "refine", "iterate"]
            .iter()
            .any(|k| desc.contains(k))
        {
            return DynamicRoutingDecision {
                mode: AllianceMode::Iterative,
                reason: "任务描述含迭代/优化意图".to_string(),
            };
        }
        if ["评审", "审核", "复核", "review"].iter().any(|k| desc.contains(k)) {
            return DynamicRoutingDecision {
                mode: AllianceMode::Sequential,
                reason: "任务描述含评审意图，先产出后复核".to_string(),
            };
        }

        // 2) 单专家无需并行/辩论
        if experts.len() <= 1 {
            return DynamicRoutingDecision {
                mode: AllianceMode::Sequential,
                reason: format!("仅 {} 位专家，采用串行单链", experts.len()),
            };
        }

        // 3) 领域广度：跨域并行易失焦，分层协同更稳
        let mut domains: Vec<&str> = experts
            .iter()
            .filter_map(|me| me.expert.domains.first().map(|s| s.as_str()))
            .collect();
        domains.sort_unstable();
        domains.dedup();
        if domains.len() >= Self::DYNAMIC_DOMAIN_DIVERSITY_MIN {
            return DynamicRoutingDecision {
                mode: AllianceMode::Hierarchical,
                reason: format!("覆盖 {} 个领域，采用分层协同", domains.len()),
            };
        }

        // 4) 能力分布：极差大 → 分层领衔；极差小且人数多 → 投票；其余 → 辩论仲裁
        let scores: Vec<f64> = experts.iter().map(|me| me.score).collect();
        let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = scores.iter().copied().fold(f64::INFINITY, f64::min);
        let spread = max - min;

        if spread >= Self::DYNAMIC_SPREAD_HIERARCHICAL {
            return DynamicRoutingDecision {
                mode: AllianceMode::Hierarchical,
                reason: format!("专家匹配分极差 {:.2}，强弱分明，采用分层", spread),
            };
        }
        if spread < Self::DYNAMIC_SPREAD_VOTING && experts.len() >= 3 {
            return DynamicRoutingDecision {
                mode: AllianceMode::Voting,
                reason: format!(
                    "{} 位专家匹配分接近（极差 {:.2}），采用投票裁决",
                    experts.len(),
                    spread
                ),
            };
        }
        DynamicRoutingDecision {
            mode: AllianceMode::Debate,
            reason: format!("{} 位专家存在分歧（极差 {:.2}），采用辩论仲裁", experts.len(), spread),
        }
    }

    /// 动态决策的中文标签（用于留痕文案）
    fn dynamic_mode_label(mode: AllianceMode) -> &'static str {
        match mode {
            AllianceMode::Sequential => "串行",
            AllianceMode::Parallel => "并行",
            AllianceMode::Hierarchical => "分层",
            AllianceMode::Debate => "辩论",
            AllianceMode::Voting => "投票",
            AllianceMode::Iterative => "迭代",
            AllianceMode::Dynamic => "动态",
        }
    }

    /// 0 匹配兜底专家：无法路由到任何领域时退化为单个通用专家单跑。
    ///
    /// 这是「不静默出空计划」与「不破坏既有成功用例」之间的取舍：
    /// 既有单测用空 matcher 提交任务并期望成功，故选择兜底而非硬报错。
    fn generic_fallback_expert() -> MatchedExpert {
        let mut expert = Expert::new_system(
            "通用专家".to_string(),
            "无法自动路由领域时的兜底通用专家，单跑处理通用任务。".to_string(),
        );
        expert.expert_id = "generic-fallback".to_string();
        expert.domains = vec!["general".to_string()];
        MatchedExpert {
            expert,
            score: 1.0,
            match_reason: "0 匹配兜底：退化到单个通用专家".to_string(),
            score_breakdown: MatchScoreBreakdown {
                domain_match: 1.0,
                capability_match: 0.5,
                health_score: 1.0,
                priority_score: 0.5,
                performance_score: 1.0,
            },
        }
    }

    /// 创建一个节点
    ///
    /// `description` 为节点角色说明（如「金融分析专家 (并行)」），
    /// `task_description` 为任务级描述；后者非空时拼接进节点描述，
    /// 使真实 LLM 专家在执行时能拿到完整任务内容（否则模型无法分析）。
    fn make_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        expert: &Expert,
        dependencies: Vec<String>,
        description: &str,
        task_description: &str,
    ) -> Node {
        let full_description = if task_description.trim().is_empty() {
            description.to_string()
        } else {
            format!("{}。任务描述：{}", description, task_description.trim())
        };
        Node {
            node_id: node_id.to_string(),
            task_id,
            expert_id: expert.expert_id.clone(),
            module_id: self.expert_to_module.as_ref().and_then(|m| m.get(&expert.expert_id).cloned()),
            name: expert.name.clone(),
            description: Some(full_description),
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

    fn make_matched_expert_with_score(
        id: &str,
        name: &str,
        domains: Vec<&str>,
        score: f64,
    ) -> MatchedExpert {
        let mut me = make_matched_expert(id, name, domains);
        me.score = score;
        me
    }

    fn dynamic_request(task_description: &str) -> PlanGenerationRequest {
        PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: task_description.to_string(),
            preferred_mode: Some(AllianceMode::Dynamic),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        }
    }

    fn first_node_desc(plan: &CollaborationPlan) -> String {
        plan.nodes
            .first()
            .and_then(|n| n.description.clone())
            .unwrap_or_default()
    }

    #[test]
    fn test_dynamic_single_expert_uses_sequential() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![make_matched_expert("e1", "E1", vec!["code"])];
        let plan = gen.generate(&dynamic_request("分析代码"), &experts).unwrap();
        // 契约保持：计划 mode 仍为 Dynamic，只是内部拓扑按特征选定
        assert_eq!(plan.mode, AllianceMode::Dynamic);
        assert_eq!(plan.nodes.len(), 1);
        assert!(first_node_desc(&plan).contains("动态路由→串行"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_cross_domain_uses_hierarchical() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "E1", vec!["code"]),
            make_matched_expert("e2", "E2", vec!["security"]),
            make_matched_expert("e3", "E3", vec!["data"]),
        ];
        let plan = gen.generate(&dynamic_request("设计一个系统"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→分层"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_skewed_scores_uses_hierarchical() {
        // 同领域但匹配分极差 0.40（≥0.25）→ 分层领衔
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert_with_score("e1", "E1", vec!["code"], 0.9),
            make_matched_expert_with_score("e2", "E2", vec!["code"], 0.5),
        ];
        let plan = gen.generate(&dynamic_request("实现一个功能"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→分层"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_close_scores_uses_voting() {
        // 3 位专家分数极差 0.02（<0.10）→ 投票裁决
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert_with_score("e1", "E1", vec!["code"], 0.80),
            make_matched_expert_with_score("e2", "E2", vec!["code"], 0.81),
            make_matched_expert_with_score("e3", "E3", vec!["code"], 0.82),
        ];
        let plan = gen.generate(&dynamic_request("评估方案优劣"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→投票"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_default_uses_debate() {
        // 2 位专家、极差 0.15（介于两阈值之间）→ 辩论仲裁
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert_with_score("e1", "E1", vec!["code"], 0.90),
            make_matched_expert_with_score("e2", "E2", vec!["code"], 0.75),
        ];
        let plan = gen.generate(&dynamic_request("权衡两种架构"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→辩论"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_iterative_by_keyword() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "E1", vec!["code"]),
            make_matched_expert("e2", "E2", vec!["code"]),
        ];
        let plan = gen.generate(&dynamic_request("持续迭代优化该方案"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→迭代"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_review_keyword_uses_sequential() {
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "E1", vec!["code"]),
            make_matched_expert("e2", "E2", vec!["security"]),
        ];
        let plan = gen.generate(&dynamic_request("完成后需要评审"), &experts).unwrap();
        assert!(first_node_desc(&plan).contains("动态路由→串行"));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_dynamic_decision_reason_is_auditable() {
        // 决策理由必须进入节点描述，保证前端 DAG 与审计可见选型依据
        let gen = SimplePlanGenerator::new();
        let experts = vec![
            make_matched_expert("e1", "E1", vec!["code"]),
            make_matched_expert("e2", "E2", vec!["security"]),
            make_matched_expert("e3", "E3", vec!["data"]),
        ];
        let plan = gen.generate(&dynamic_request("设计一个系统"), &experts).unwrap();
        let desc = first_node_desc(&plan);
        assert!(desc.contains("[动态路由→"), "缺少动态路由标记: {}", desc);
        assert!(desc.contains("覆盖 3 个领域"), "缺少决策理由: {}", desc);
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
    fn test_node_description_embeds_task_description() {
        // 真实 LLM 专家需要拿到任务内容：节点 description 必须携带任务描述
        let gen = SimplePlanGenerator::new();
        let experts = vec![make_matched_expert("e1", "Expert 1", vec!["code"])];
        let task_desc = "分析某代码仓库的安全风险";
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: task_desc.to_string(),
            preferred_mode: Some(AllianceMode::Parallel),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &experts).unwrap();
        let desc = plan.nodes[0].description.clone().unwrap_or_default();
        assert!(
            desc.contains(task_desc),
            "节点描述应包含任务描述，实际: {}",
            desc
        );
        assert!(desc.contains("任务描述"));
    }

    #[test]
    fn test_module_id_mapping_filled() {
        // 注入 expert->module 映射后，节点应携带对应 module_id（消除硬编码 None）
        let mut map = HashMap::new();
        map.insert("e1".to_string(), "mod-e1".to_string());
        map.insert("e2".to_string(), "mod-e2".to_string());
        let gen = SimplePlanGenerator::new().with_expert_to_module(map);
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
        let mut found = std::collections::HashMap::new();
        for node in &plan.nodes {
            found.insert(node.expert_id.clone(), node.module_id.clone());
        }
        assert_eq!(found.get("e1").and_then(|m| m.as_deref()), Some("mod-e1"));
        assert_eq!(found.get("e2").and_then(|m| m.as_deref()), Some("mod-e2"));
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

    #[test]
    fn test_expert_weights_carry_match_scores() {
        // 优化1：尾部融合权重应来自匹配分，而非恒等权 1.0
        let gen = SimplePlanGenerator::new();
        let mut e1 = make_matched_expert("e1", "Expert 1", vec!["code"]);
        e1.score = 0.95;
        let mut e2 = make_matched_expert("e2", "Expert 2", vec!["code"]);
        e2.score = 0.55;
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: "test".to_string(),
            preferred_mode: Some(AllianceMode::Parallel),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &[e1, e2]).unwrap();
        assert_eq!(plan.expert_weights.get("e1"), Some(&0.95));
        assert_eq!(plan.expert_weights.get("e2"), Some(&0.55));
        assert_ne!(
            plan.expert_weights.get("e1"),
            plan.expert_weights.get("e2"),
            "权重应反映不同匹配分，而非全 1.0"
        );
    }

    #[test]
    fn test_empty_matches_falls_back_to_generic_expert() {
        // 优化2：0 匹配不应静默产出 0 节点空计划，应退化为单个通用专家
        let gen = SimplePlanGenerator::new();
        let request = PlanGenerationRequest {
            task_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_description: "test".to_string(),
            preferred_mode: Some(AllianceMode::Parallel),
            preferred_experts: vec![],
            constraints: serde_json::json!({}),
            fusion_strategy: FusionStrategy::Weighted,
        };

        let plan = gen.generate(&request, &[]).unwrap();
        assert_eq!(
            plan.nodes.len(),
            1,
            "空匹配应兜底为 1 个通用专家，而非 0 节点假成功"
        );
        assert_eq!(plan.nodes[0].expert_id, "generic-fallback");
        assert_eq!(plan.expert_weights.get("generic-fallback"), Some(&1.0));
    }
}
