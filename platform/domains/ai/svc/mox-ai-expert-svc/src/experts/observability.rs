// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 可观测专家：埋点、追踪、告警覆盖

use crate::context::ExpertContext;
use crate::expert::{Constraint, Expert, ExpertOpinion};
use crate::ir::Dimension;
use mox_ai_flow_svc::model::NodeKind;

pub struct ObservabilityExpert;

impl Expert for ObservabilityExpert {
    fn id(&self) -> String {
        "observability".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Observability
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut o = ExpertOpinion::empty("observability", Dimension::Observability);
        let g = ctx.flow;

        // 关键路径节点（高耗时）必须埋点
        let critical_threshold = 300u64;
        for n in &g.nodes {
            if n.duration_ms >= critical_threshold && n.kind.is_executable() {
                o.constraints.push(Constraint::MustAudit(n.id.clone()));
            }
        }

        // 外部依赖（DB/HTTP/Browser）必须有追踪 span，否则告警盲区
        for n in &g.nodes {
            let external = matches!(
                n.tool,
                Some(mox_ai_flow_svc::model::ToolKind::Database)
                    | Some(mox_ai_flow_svc::model::ToolKind::Http)
                    | Some(mox_ai_flow_svc::model::ToolKind::Browser)
            );
            if external && !n.tags.iter().any(|t| t == "traced") {
                o.push_risk(
                    mox_ai_flow_svc::model::Severity::Info,
                    vec![n.id.clone()],
                    format!("外部依赖节点「{}」缺少追踪埋点，故障难以定位", n.name),
                    Some("插入 tracing span".into()),
                );
            }
        }

        // 异常边必须有告警收口
        let has_handler = g
            .nodes
            .iter()
            .any(|n| n.tags.iter().any(|t| t == "error_handler"));
        let has_exception_edge = g
            .edges
            .iter()
            .any(|e| e.kind == mox_ai_flow_svc::model::EdgeKind::Exception);
        if has_exception_edge && !has_handler {
            o.push_risk(
                mox_ai_flow_svc::model::Severity::Warning,
                vec![],
                "存在异常分支但无告警收口节点，错误将被吞没",
                Some("补 error_handler 并接告警".into()),
            );
        }

        // Start/End 必须有 span 起点/终点
        let has_start = g.nodes.iter().any(|n| n.kind == NodeKind::Start);
        let has_end = g.nodes.iter().any(|n| n.kind == NodeKind::End);
        if !has_start || !has_end {
            o.push_risk(
                mox_ai_flow_svc::model::Severity::Info,
                vec![],
                "缺少 Start/End 锚点，无法形成完整 trace",
                None,
            );
        }
        o
    }
}
