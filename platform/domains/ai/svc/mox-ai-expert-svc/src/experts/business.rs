// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 业务专家：领域规则校验、分支完整性、失败兜底

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion};
use crate::ir::Dimension;
use mox_ai_flow_svc::model::NodeKind;

pub struct BusinessExpert;

impl Expert for BusinessExpert {
    fn id(&self) -> String {
        "business".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Business
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if !ctx.can(crate::context::Capability::EditFlow) {
            return ExpertOpinion::skipped("business", Dimension::Business, "无 edit-flow 权限");
        }
        let mut o = ExpertOpinion::empty("business", Dimension::Business);
        let g = ctx.flow;

        // 1) 决策节点必须有 else / 默认分支
        for n in &g.nodes {
            if n.kind == NodeKind::Decision {
                let outs: Vec<&mox_ai_flow_svc::model::FlowEdge> =
                    g.edges.iter().filter(|e| e.from == n.id).collect();
                if outs.len() < 2 {
                    o.push_risk(
                        mox_ai_flow_svc::model::Severity::Warning,
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
                mox_ai_flow_svc::model::Severity::Warning,
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
                    mox_ai_flow_svc::model::Severity::Warning,
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
                o.suggestions.push(crate::expert::Suggestion::Merge);
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
