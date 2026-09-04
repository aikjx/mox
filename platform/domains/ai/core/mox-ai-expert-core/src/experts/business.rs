// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 业务专家：领域规则校验、分支完整性、失败兜底
//!
//! 核心职责：
//! - 决策节点必须有 else / 默认分支
//! - 流程必须有失败兜底（异常边到达终点或处理器）
//! - 悬垂节点检测（无入边也无出边的孤立节点）
//! - 强合规租户：敏感业务需走审批（建议补审批分支）

use crate::context::Capability;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion, Severity, Suggestion};
use mox_ai_flow_core::model::NodeKind;

pub struct BusinessExpert;

impl Expert for BusinessExpert {
    fn id(&self) -> ExpertId {
        "business".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Business
    }

    fn analyze(&self, ctx: &crate::context::ExpertContext) -> ExpertOpinion {
        if !ctx.can(Capability::EditFlow) {
            return ExpertOpinion::skipped("business", Dimension::Business, "无 edit-flow 权限");
        }
        let mut o = ExpertOpinion::empty("business", Dimension::Business);
        let g = ctx.flow;

        // 1) 决策节点必须有 else / 默认分支
        for n in &g.nodes {
            if n.kind == NodeKind::Decision {
                let outs: Vec<&mox_ai_flow_core::model::FlowEdge> =
                    g.edges.iter().filter(|e| e.from == n.id).collect();
                if outs.len() < 2 {
                    o.push_risk(
                        Severity::Warning,
                        vec![n.id.clone()],
                        format!(
                            "决策节点「{}」缺少 else 分支，业务流程存在未覆盖路径",
                            n.name
                        ),
                        Some("补充默认分支或兜底处理".into()),
                    );
                }
            }
        }

        // 2) 流程必须有失败兜底（异常边到达终点或处理器）
        let has_error_handler = g
            .nodes
            .iter()
            .any(|n| n.tags.iter().any(|t| t == "error_handler"));
        if !has_error_handler {
            o.push_risk(
                Severity::Warning,
                vec![],
                "流程缺少统一异常兜底节点，外部调用失败将静默中断业务",
                Some("为外部调用节点补齐异常边到统一处理器".into()),
            );
        }

        // 3) 悬垂节点
        for n in &g.nodes {
            let is_end = n.kind == NodeKind::End;
            let has_out = g.edges.iter().any(|e| e.from == n.id);
            let has_in = g.edges.iter().any(|e| e.to == n.id);
            if !is_end && n.kind != NodeKind::Start && !has_out && !has_in {
                o.push_risk(
                    Severity::Warning,
                    vec![n.id.clone()],
                    format!("节点「{}」既无入边也无出边，业务链路断裂", n.name),
                    None,
                );
            }
        }

        // 4) 强合规租户：敏感业务需走审批（建议补审批分支）
        if ctx.tenant.regulated {
            let has_approval = g
                .nodes
                .iter()
                .any(|n| n.tags.iter().any(|t| t == "approval"));
            if !has_approval {
                o.suggestions.push(Suggestion::Merge);
            }
        }

        o.metrics.insert(
            "decision_nodes".into(),
            g.nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Decision)
                .count() as f64,
        );
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ExpertContext, GovernContext, Principal, Tenant};
    use mox_ai_flow_core::model::{FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

    fn make_gctx(tenant: Tenant) -> GovernContext {
        let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
        GovernContext::new(tenant, principal)
    }

    #[test]
    fn decision_without_else_warns() {
        let mut g = FlowGraph::new("decision-test", "决策测试");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("d", "判断", NodeKind::Decision));
        g.add_node(FlowNode::new("a", "分支A", NodeKind::Task));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "d"));
        g.add_edge(FlowEdge::seq("d", "a"));
        g.add_edge(FlowEdge::seq("a", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(o.risks.iter().any(|r| r.message.contains("else 分支")));
    }

    #[test]
    fn decision_with_two_branches_ok() {
        let mut g = FlowGraph::new("decision-ok", "决策正常");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("d", "判断", NodeKind::Decision));
        g.add_node(FlowNode::new("a", "分支A", NodeKind::Task));
        g.add_node(FlowNode::new("b", "分支B", NodeKind::Task));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "d"));
        g.add_edge(FlowEdge::seq("d", "a"));
        g.add_edge(FlowEdge::seq("d", "b"));
        g.add_edge(FlowEdge::seq("a", "e"));
        g.add_edge(FlowEdge::seq("b", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(!o.risks.iter().any(|r| r.message.contains("else 分支")));
    }

    #[test]
    fn missing_error_handler_warns() {
        let mut g = FlowGraph::new("no-err", "无异常兜底");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("api", "外部调用", ToolKind::Http, 50));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "api"));
        g.add_edge(FlowEdge::seq("api", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(o.risks.iter().any(|r| r.message.contains("异常兜底")));
    }

    #[test]
    fn error_handler_present_no_warn() {
        let mut g = FlowGraph::new("has-err", "有异常兜底");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("api", "外部调用", ToolKind::Http, 50));
        let mut err_node = FlowNode::task("err", "异常处理", ToolKind::Compute, 10);
        err_node.tags.push("error_handler".into());
        g.add_node(err_node);
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "api"));
        g.add_edge(FlowEdge::seq("api", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(!o.risks.iter().any(|r| r.message.contains("异常兜底")));
    }

    #[test]
    fn dangling_node_detected() {
        let mut g = FlowGraph::new("dangling", "悬垂节点");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_node(FlowNode::task("orphan", "孤立节点", ToolKind::Compute, 10));
        g.add_edge(FlowEdge::seq("s", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(o.risks.iter().any(|r| r.message.contains("悬垂") || r.message.contains("既无入边")));
    }

    #[test]
    fn regulated_tenant_without_approval_suggests_merge() {
        let mut g = FlowGraph::new("reg-no-appr", "合规租户无审批");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "e"));

        let tenant = Tenant::new("gov", "ns-gov").regulated(true);
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = BusinessExpert.analyze(&ectx);

        assert!(o.suggestions.iter().any(|s| matches!(s, Suggestion::Merge)));
    }
}
