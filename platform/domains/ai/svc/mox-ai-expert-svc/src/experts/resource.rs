//! 资源专家：把租户配额翻译为资源池上限，识别单实例冲突

use crate::context::ExpertContext;
use crate::expert::{Constraint, Expert, ExpertOpinion};
use crate::ir::Dimension;
use mox_ai_flow_svc::model::ToolKind;

pub struct ResourceExpert;

impl Expert for ResourceExpert {
    fn id(&self) -> String {
        "resource".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Resource
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut o = ExpertOpinion::empty("resource", Dimension::Resource);
        let g = ctx.flow;

        // 1) 租户配额 → 资源池上限约束
        for (pool, cap) in &ctx.tenant.pool_caps {
            o.constraints
                .push(Constraint::ResourceCap(pool.clone(), *cap));
        }
        o.constraints.push(Constraint::ResourceCap(
            "cpu".into(),
            ctx.quota.max_parallel,
        ));

        // 2) 浏览器/单实例资源：多实例并发必冲突
        for tool in [ToolKind::Browser] {
            let users: Vec<&String> = g
                .nodes
                .iter()
                .filter(|n| n.tool == Some(tool))
                .map(|n| &n.id)
                .collect();
            if users.len() > 1 {
                let cap = ctx.tenant.pool_caps.get("browser").copied().unwrap_or(1);
                if cap < users.len() as u32 {
                    o.push_risk(
                        mox_ai_flow_svc::model::Severity::Blocking,
                        users.iter().map(|s| (*s).clone()).collect(),
                        format!(
                            "{} 个浏览器任务被并发调度，但实例容量仅 {}，将互相抢占页面",
                            users.len(),
                            cap
                        ),
                        Some("自动插入串行互斥边".into()),
                    );
                    // 建议两两串行（由裁决器物化为 Mutex 边）
                    for w in users.windows(2) {
                        o.constraints
                            .push(Constraint::MustSerialize(crate::expert::NodeEdge {
                                from: w[0].clone(),
                                to: w[1].clone(),
                            }));
                    }
                }
            }
        }

        // 3) 超出 SLA 预算预警
        let total: u64 = g.nodes.iter().map(|n| n.duration_ms).sum();
        o.metrics.insert("sum_duration_ms".into(), total as f64);
        if total > ctx.quota.sla_ms {
            o.push_risk(
                mox_ai_flow_svc::model::Severity::Warning,
                vec![],
                format!(
                    "串行耗时 {}ms 超出 SLA {}ms，需并行优化",
                    total, ctx.quota.sla_ms
                ),
                Some("依赖 flow-ai 自动并行化压缩关键路径".into()),
            );
        }
        o
    }
}
