// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 安全专家：注入、PII 外发、提示词越狱、沙箱隔离

use crate::context::ExpertContext;
use crate::expert::{Constraint, Expert, ExpertOpinion};
use crate::ir::Dimension;
use crate::sensitivity::is_sensitive_leak;
use mox_ai_flow_svc::model::{NodeKind, ToolKind};

pub struct SecurityExpert;

impl Expert for SecurityExpert {
    fn id(&self) -> String {
        "security".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Security
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if !ctx.can(crate::context::Capability::EditFlow) {
            return ExpertOpinion::skipped("security", Dimension::Security, "无 edit-flow 权限");
        }
        let mut o = ExpertOpinion::empty("security", Dimension::Security);
        let g = ctx.flow;

        for n in &g.nodes {
            // 外部调用（HTTP/Shell）需要隔离执行，避免宿主越权
            if matches!(n.tool, Some(ToolKind::Http) | Some(ToolKind::Shell)) {
                o.constraints.push(Constraint::MustIsolate(n.id.clone()));
                o.push_risk(
                    mox_ai_flow_svc::model::Severity::Warning,
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
                        mox_ai_flow_svc::model::Severity::Info,
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

        // 强合规租户：PII 外发必须有脱敏（与权限专家互补，但安全维度独立告警）
        if ctx.tenant.regulated {
            // 使用单一权威判定 is_sensitive_leak：已脱敏资源（如 var:citizen_safe）不再误判为泄露
            let pii_out = g.nodes.iter().any(|n| {
                matches!(n.tool, Some(ToolKind::Http))
                    && n.accesses.iter().any(|a| is_sensitive_leak(&a.resource))
            });
            if pii_out {
                o.push_risk(
                    mox_ai_flow_svc::model::Severity::Blocking,
                    vec![],
                    "检测到 PII/公民数据经 HTTP 外发，缺少脱敏即属数据泄露风险",
                    Some("强制 desensitize Guard 前置".into()),
                );
            }
        }
        o
    }
}
