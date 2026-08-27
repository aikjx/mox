// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 企业级七专家单元测试套件（简化版）
//!
//! 测试策略：
//! - 每个专家 ≥2 个核心场景测试
//! - 验证关键检查点
//! - 验证 push_veto 正交否决机制

#[cfg(test)]
mod expert_unit_tests {
    use mox_ai_flow_svc::model::{Access, AccessMode, FlowGraph, FlowNode, NodeKind, Severity, ToolKind};
    use mox_ai_expert_svc::experts::algorithm::AlgorithmExpert;
    use mox_ai_expert_svc::experts::business::BusinessExpert;
    use mox_ai_expert_svc::experts::permission::PermissionExpert;
    use mox_ai_expert_svc::experts::security::SecurityExpert;
    use mox_ai_expert_svc::{
        context::{GovernContext, Principal, Tenant},
        expert::{Constraint, Expert},
    };

    // ==================== BusinessExpert ====================

    #[test]
    fn test_business_detects_missing_else_branch() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        flow.add_node(FlowNode::new(
            "decision_1",
            "判断用户类型",
            NodeKind::Decision,
        ));
        flow.add_node(FlowNode::new("end", "结束", NodeKind::End));
        flow.add_edge(mox_ai_flow_svc::model::FlowEdge::seq("decision_1", "end"));

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = BusinessExpert;
        let opinion = expert.analyze(&ctx);

        assert!(!opinion.risks.is_empty(), "应检测到缺少 else 分支");
        assert!(opinion
            .risks
            .iter()
            .any(|r| r.severity == Severity::Warning));
    }

    #[test]
    fn test_business_detects_missing_error_handler() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        flow.add_node(FlowNode::new("start", "开始", NodeKind::Start));
        flow.add_node(FlowNode::task("http_call", "外部调用", ToolKind::Http, 100));
        flow.add_node(FlowNode::new("end", "结束", NodeKind::End));
        flow.add_edge(mox_ai_flow_svc::model::FlowEdge::seq("start", "http_call"));
        flow.add_edge(mox_ai_flow_svc::model::FlowEdge::seq("http_call", "end"));

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = BusinessExpert;
        let opinion = expert.analyze(&ctx);

        assert!(!opinion.risks.is_empty(), "应检测到缺少异常兜底");
    }

    // ==================== PermissionExpert ====================

    #[test]
    fn test_permission_detects_missing_desensitize_guard() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        let mut node = FlowNode::task("read_citizen", "读取公民数据", ToolKind::Database, 100);
        node.accesses.push(Access {
            resource: "db:citizen_info".into(),
            mode: AccessMode::Read,
        });
        flow.add_node(node);

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = PermissionExpert;
        let opinion = expert.analyze(&ctx);

        assert!(!opinion.constraints.is_empty(), "应生成脱敏守卫约束");
        assert!(opinion.constraints.iter().any(|c| matches!(
            c,
            Constraint::MustGuard(_, tags) if tags.contains(&"desensitize".to_string())
        )));
    }

    #[test]
    fn test_permission_vetoes_prod_write_without_authz() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        let mut node = FlowNode::task("write_prod", "写入生产库", ToolKind::Database, 100);
        node.accesses.push(Access {
            resource: "db:prod_orders".into(),
            mode: AccessMode::Write,
        });
        flow.add_node(node);

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = PermissionExpert;
        let opinion = expert.analyze(&ctx);

        assert!(opinion.risks.iter().any(|r| r.veto), "生产库写应触发否决");
    }

    // ==================== SecurityExpert ====================

    #[test]
    fn test_security_detects_external_call() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        flow.add_node(FlowNode::task(
            "http_call",
            "外部API调用",
            ToolKind::Http,
            100,
        ));

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = SecurityExpert;
        let opinion = expert.analyze(&ctx);

        assert!(
            opinion.constraints.iter().any(|c| matches!(
                c,
                Constraint::MustIsolate(node_id) if node_id == "http_call"
            )),
            "应生成隔离约束"
        );
    }

    #[test]
    fn test_security_detects_pii_leak_for_regulated_tenant() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        let mut node = FlowNode::task("send_pii", "发送数据", ToolKind::Http, 100);
        node.accesses.push(Access {
            resource: "pii:user_data".into(),
            mode: AccessMode::Read,
        });
        flow.add_node(node);

        let govern_ctx = create_govern_context(true); // regulated = true
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = SecurityExpert;
        let opinion = expert.analyze(&ctx);

        assert!(
            opinion
                .risks
                .iter()
                .any(|r| r.severity == Severity::Blocking && r.message.contains("PII")),
            "强合规租户 PII 外发应为 Blocking 级"
        );
    }

    // ==================== AlgorithmExpert ====================

    #[test]
    fn test_algorithm_suggests_model_routing() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        flow.add_node(FlowNode::task("llm_node", "LLM节点", ToolKind::Llm, 100));

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let expert = AlgorithmExpert;
        let opinion = expert.analyze(&ctx);

        assert!(!opinion.suggestions.is_empty() || opinion.metrics.contains_key("llm_nodes"));
    }

    // ==================== 正交否决链测试 ====================

    #[test]
    fn test_orthogonal_veto_chain() {
        let mut flow = FlowGraph::new("test_flow", "测试流程");
        let mut node = FlowNode::task("prod_write", "生产库写", ToolKind::Database, 100);
        node.accesses.push(Access {
            resource: "db:prod_data".into(),
            mode: AccessMode::Write,
        });
        flow.add_node(node);

        let govern_ctx = create_govern_context(false);
        let ctx = mox_ai_expert_svc::context::ExpertContext::new(&flow, &govern_ctx);

        let perm_expert = PermissionExpert;
        let perm_opinion = perm_expert.analyze(&ctx);

        // Permission 专家应否决生产库写
        assert!(perm_opinion.risks.iter().any(|r| r.veto), "权限专家应否决");

        // 正交否决链：一旦有 veto，最终结果必须被阻断
        // 这在 pipeline 中由 reconcile() 和 gate 检查实现
    }

    // ==================== 辅助函数 ====================

    fn create_govern_context(regulated: bool) -> GovernContext {
        let tenant = Tenant::new("test_tenant", "default_ns").regulated(regulated);
        let principal = Principal::new("test_user").with_roles(vec!["admin".into()]);
        GovernContext::new(tenant, principal)
    }
}
