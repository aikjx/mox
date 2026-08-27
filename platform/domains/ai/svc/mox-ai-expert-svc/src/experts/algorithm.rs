// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 算法专家：关键路径瓶颈、复杂度、缓存与算力路由提议

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion};
use crate::ir::Dimension;
use mox_ai_flow_svc::model::{NodeKind, ToolKind};

pub struct AlgorithmExpert;

impl Expert for AlgorithmExpert {
    fn id(&self) -> String {
        "algorithm".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Algorithm
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut o = ExpertOpinion::empty("algorithm", Dimension::Algorithm);
        let g = ctx.flow;

        // 1) 标记高耗时 LLM / 计算节点为关键瓶颈并建议缓存
        let mut llm_count = 0u32;
        for n in &g.nodes {
            if matches!(n.tool, Some(ToolKind::Llm)) {
                llm_count += 1;
                if n.duration_ms >= 1000 {
                    o.suggestions.push(crate::expert::Suggestion::Cache);
                    o.push_risk(
                        mox_ai_flow_svc::model::Severity::Info,
                        vec![n.id.clone()],
                        format!(
                            "节点「{}」为重型 LLM 推理({}ms)，建议结果缓存以降低算力",
                            n.name, n.duration_ms
                        ),
                        Some("对幂等输入启用缓存键".into()),
                    );
                }
            }
        }

        // 2) 建议把简单问答降级到轻量模型（算力智能分配）
        for n in &g.nodes {
            if matches!(n.tool, Some(ToolKind::Llm)) && n.duration_ms < 400 {
                o.constraints.push(crate::expert::Constraint::RouteModel(
                    n.id.clone(),
                    mox_ai_flow_svc::schedule::ModelTier::Light,
                ));
                o.suggestions.push(crate::expert::Suggestion::Offload(
                    mox_ai_flow_svc::schedule::ModelTier::Light,
                ));
            }
        }

        // 3) 复杂度提示：循环体过大
        let loop_cnt = g
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::LoopStart)
            .count();
        o.metrics.insert("llm_nodes".into(), llm_count as f64);
        o.metrics.insert("loops".into(), loop_cnt as f64);
        if loop_cnt > 0 {
            o.push_risk(
                mox_ai_flow_svc::model::Severity::Info,
                vec![],
                format!(
                    "检测到 {} 个循环结构，请确保有终止条件且单轮 O(1) 副作用",
                    loop_cnt
                ),
                None,
            );
        }
        o
    }
}
