//! 数据专家：血缘、幂等、schema 一致性、保序

use crate::context::ExpertContext;
use crate::expert::{Constraint, Expert, ExpertOpinion};
use crate::ir::Dimension;
use flow_ai::model::{AccessMode, ToolKind};

pub struct DataExpert;

impl Expert for DataExpert {
    fn id(&self) -> String {
        "data".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Data
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if !ctx.can(crate::context::Capability::EditFlow) {
            return ExpertOpinion::skipped("data", Dimension::Data, "无 edit-flow 权限");
        }
        let mut o = ExpertOpinion::empty("data", Dimension::Data);
        let g = ctx.flow;

        // 1) 非幂等写操作：提示失败重放风险，强制保序
        for n in &g.nodes {
            let writes = n.accesses.iter().any(|a| a.mode == AccessMode::Write);
            let non_idem = matches!(
                n.tool,
                Some(ToolKind::Browser)
                    | Some(ToolKind::Http)
                    | Some(ToolKind::Shell)
                    | Some(ToolKind::Database)
            ) && !n.idempotent;
            if writes && non_idem {
                o.push_risk(
                    flow_ai::model::Severity::Warning,
                    vec![n.id.clone()],
                    format!("节点「{}」为非幂等写操作，重放可能产生脏数据", n.name),
                    Some("标记重试安全边界 / 加幂等键".into()),
                );
                // 保序：强制其前驱在其之前完成（已由 flow-ai 数据流处理，这里补一条软约束）
                if let Some(pred) = g.edges.iter().find(|e| e.to == n.id) {
                    o.constraints
                        .push(Constraint::MustOrder(crate::expert::NodeEdge {
                            from: pred.from.clone(),
                            to: n.id.clone(),
                        }));
                }
            }
        }

        // 2) 血缘孤立：写出的变量从未被读（可能遗漏下游）
        for n in &g.nodes {
            for w in n.accesses.iter().filter(|a| a.mode == AccessMode::Write) {
                let consumed = g.nodes.iter().any(|m| {
                    m.id != n.id
                        && m.accesses
                            .iter()
                            .any(|a| a.mode == AccessMode::Read && a.resource == w.resource)
                });
                if !consumed && !w.resource.starts_with("file:") && !w.resource.starts_with("db:") {
                    o.push_risk(
                        flow_ai::model::Severity::Info,
                        vec![n.id.clone()],
                        format!("产出 {} 未被任何节点消费，数据血缘断裂", w.resource),
                        Some("确认是否遗漏下游汇总节点".into()),
                    );
                }
            }
        }

        // 3) 强合规：数据库写入需事务（保一致性）
        for n in &g.nodes {
            if matches!(n.tool, Some(ToolKind::Database))
                && n.accesses.iter().any(|a| a.mode == AccessMode::Write)
                && !n.transactional
            {
                o.push_risk(
                    flow_ai::model::Severity::Warning,
                    vec![n.id.clone()],
                    format!(
                        "节点「{}」执行库写但未开启事务，部分失败将破坏一致性",
                        n.name
                    ),
                    Some("开启事务边界".into()),
                );
            }
        }
        o
    }
}
