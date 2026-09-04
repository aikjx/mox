// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 安全专家：注入、PII 外发、提示词越狱、沙箱隔离
//!
//! 核心职责：
//! - 外部调用（HTTP/Shell）需沙箱隔离
//! - LLM 节点输出需内容校验 Guard（防提示词注入/越狱）
//! - 强合规租户：PII 外发必须脱敏（与权限专家互补，但安mox 模块化系统架构维度独立告警）
//!
//! 敏感度判定统一使用 `crate::sensitivity::is_sensitive_leak` SSOT。

use crate::context::Capability;
use crate::expert::Expert;
use crate::sensitivity::is_sensitive_leak;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion, Severity};
use mox_ai_flow_svc::model::{NodeKind, ToolKind};

pub struct SecurityExpert;

impl Expert for SecurityExpert {
    fn id(&self) -> ExpertId {
        "security".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Security
    }

    fn analyze(&self, ctx: &crate::context::ExpertContext) -> ExpertOpinion {
        if !ctx.can(Capability::EditFlow) {
            return ExpertOpinion::skipped("security", Dimension::Security, "无 edit-flow 权限");
        }
        let mut o = ExpertOpinion::empty("security", Dimension::Security);
        let g = ctx.flow;

        for n in &g.nodes {
            // 外部调用（HTTP/Shell）需要隔离执行，避免宿主越权
            if matches!(n.tool, Some(ToolKind::Http) | Some(ToolKind::Shell)) {
                o.constraints.push(mox_ai_expert_proto::Constraint::MustIsolate(
                    n.id.clone(),
                ));
                o.push_risk(
                    Severity::Warning,
                    vec![n.id.clone()],
                    format!("节点「{}」执行外部命令/请求，建议在沙箱中隔离运行", n.name),
                    Some("落沙箱 / 容器隔离".into()),
                );
            }
            // LLM 节点：提示词注入风险，需输出校验 Guard
            if matches!(n.tool, Some(ToolKind::Llm)) {
                let followed_by_guard = g.edges.iter().any(|e| {
                    e.from == n.id
                        && g.node(&e.to)
                            .map(|s| s.kind == NodeKind::Guard)
                            .unwrap_or(false)
                });
                if !followed_by_guard {
                    o.push_risk(
                        Severity::Info,
                        vec![n.id.clone()],
                        format!(
                            "LLM 节点「{}」输出未做内容校验，存在提示词注入/越狱外溢风险",
                            n.name
                        ),
                        Some("下游增加输出校验 Guard".into()),
                    );
                }
            }
        }

        // 强合规租户：PII 外发必须有脱敏（与权限专家互补，但安mox 模块化系统架构维度独立告警）
        if ctx.tenant.regulated {
            // 使用单一权威判定 is_sensitive_leak：已脱敏资源（如 var:citizen_safe）不再误判为泄露
            let pii_out = g.nodes.iter().any(|n| {
                matches!(n.tool, Some(ToolKind::Http))
                    && n.accesses.iter().any(|a| is_sensitive_leak(&a.resource))
            });
            if pii_out {
                o.push_risk(
                    Severity::Blocking,
                    vec![],
                    "检测到 PII/公民数据经 HTTP 外发，缺少脱敏即属数据泄露风险",
                    Some("强制 desensitize Guard 前置".into()),
                );
            }
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ExpertContext, GovernContext, Principal, Tenant};
    use mox_ai_flow_svc::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

    fn make_gctx(tenant: Tenant) -> GovernContext {
        let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
        GovernContext::new(tenant, principal)
    }

    #[test]
    fn http_node_triggers_isolate() {
        let mut g = FlowGraph::new("http-test", "HTTP 测试");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("req", "外部请求", ToolKind::Http, 50));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "req"));
        g.add_edge(FlowEdge::seq("req", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = SecurityExpert.analyze(&ectx);

        assert!(o
            .constraints
            .iter()
            .any(|c| matches!(c, mox_ai_expert_proto::Constraint::MustIsolate(_))));
    }

    #[test]
    fn llm_without_guard_warns() {
        let mut g = FlowGraph::new("llm-test", "LLM 测试");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("llm", "大模型", ToolKind::Llm, 80));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "llm"));
        g.add_edge(FlowEdge::seq("llm", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = SecurityExpert.analyze(&ectx);

        assert!(o.risks.iter().any(|r| r.message.contains("提示词注入")));
    }

    #[test]
    fn llm_with_guard_no_warn() {
        let mut g = FlowGraph::new("llm-guard", "LLM 有 Guard");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("llm", "大模型", ToolKind::Llm, 80));
        g.add_node(FlowNode::new("guard", "输出校验", NodeKind::Guard));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "llm"));
        g.add_edge(FlowEdge::seq("llm", "guard"));
        g.add_edge(FlowEdge::seq("guard", "e"));

        let tenant = Tenant::new("t", "ns");
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = SecurityExpert.analyze(&ectx);

        assert!(!o.risks.iter().any(|r| r.message.contains("提示词注入")));
    }

    #[test]
    fn regulated_tenant_pii_outbound_blocking() {
        let mut g = FlowGraph::new("pii-out", "PII 外发");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("http", "外发数据", ToolKind::Http, 30)
                .with_access(Access::write("var:citizen_phone")),
        );
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "http"));
        g.add_edge(FlowEdge::seq("http", "e"));

        let tenant = Tenant::new("gov", "ns-gov").regulated(true);
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = SecurityExpert.analyze(&ectx);

        assert!(o.risks.iter().any(|r| r.severity == Severity::Blocking));
    }

    #[test]
    fn desensitized_pii_not_flagged() {
        // 已脱敏资源不应触发 PII 外发告警（SSOT 验证）
        let mut g = FlowGraph::new("safe-out", "已脱敏外发");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("http", "外发脱敏数据", ToolKind::Http, 30)
                .with_access(Access::write("var:citizen_phone_safe")),
        );
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "http"));
        g.add_edge(FlowEdge::seq("http", "e"));

        let tenant = Tenant::new("gov", "ns-gov").regulated(true);
        let gctx = make_gctx(tenant);
        let ectx = ExpertContext::new(&g, &gctx);
        let o = SecurityExpert.analyze(&ectx);

        assert!(
            !o.risks.iter().any(|r| r.severity == Severity::Blocking),
            "已脱敏数据外发不应触发 Blocking 风险"
        );
    }
}
