//! 全维处理流水线：normalize → 并行派发专家 → 裁决 → flow-ai 求解 → 治理闸门 → 出码

use crate::context::{ExpertContext, GovernContext};
use crate::expert::dispatch;
use crate::govern::{apply_rules, govern, AuditChain, FlowStatus, GateResult};
use crate::ir::auto_dimension;
use crate::reconcile::reconcile;
use crate::verify::{verify, AlgoVerification};
use flow_ai::model::FlowGraph;
use flow_ai::pipeline::optimize;
use serde::{Deserialize, Serialize};

/// 全维治理报告：专家评分 + 裁决冲突 + 优化报告 + 治理闸门
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceReport {
    pub flow_id: String,
    pub flow_name: String,
    /// 各专家健康分 (专家, 分)
    pub expert_scores: Vec<(String, f64)>,
    /// 优化报告（含并行/关键路径/调度/冲突/代码）
    pub optimization: flow_ai::pipeline::OptimizationReport,
    /// ⛨ 璇玑验证报告（最高权限，不可被治理覆盖）
    pub algo: AlgoVerification,
    /// 治理闸门结果
    pub gate: GateResult,
    /// 审计链（已追加 emit 事件）
    pub audit: AuditChain,
}

/// 全维优化入口
pub fn alliance_optimize(raw: &FlowGraph, ctx: &GovernContext) -> GovernanceReport {
    // 1. 归一化：维度着色
    let df = auto_dimension(raw);
    let base = &df.base;

    // 2. 并行派发七位专家
    let experts = crate::experts::all_experts();
    let ectx = ExpertContext::new(base, ctx);
    let opinions = dispatch(&ectx, &experts);

    // 3. 归一化裁决 → ReconciledPlan
    let plan = reconcile(&opinions, base, &base.pools);

    // 4. 交给 flow-ai 引擎做最优求解
    let mut graph = plan.graph.clone();
    apply_rules(&mut graph, &plan);
    let mut opt = optimize(&graph, &flow_ai::pipeline::OptimizeConfig::default());

    // 把专家采纳的算力路由并入优化报告
    if !plan.model_routes.is_empty() {
        opt.model_routing = plan
            .model_routes
            .iter()
            .map(|(node, tier)| flow_ai::schedule::ModelRouting {
                node_id: node.clone(),
                model_tier: *tier,
                reason: "expert-alliance 算法/资源专家路由".into(),
            })
            .collect();
    }

    // 5. ⛨ 璇玑验证网关（最高权限，在治理之前）
    // 5.5 汇总专家「否决级」风险（Risk.veto=true）→ 并入算法验证否决。
    //     这是正交机制：未来任何专家判定「不可自动修复、必须人工审批」的风险，
    //     只需 push_veto，即可自动触发否决，无需在编排层单独补丁。
    let expert_veto = opinions.iter().any(|o| o.risks.iter().any(|r| r.veto));
    let mut algo = verify(base, &opt);
    if expert_veto {
        algo.vetoed = true;
        algo.summary = format!(
            "{}; 专家否决级风险(生产/敏感数据越权写等)未通过安全审批",
            algo.summary
        );
    }

    // 6. 治理闸门（尊重算法否决）
    let status = if algo.vetoed { FlowStatus::Blocked } else { FlowStatus::Approved };
    let gate = govern(&plan, &opt, status, &ctx.quota, &ctx.principal.subject, algo.vetoed);

    // 6. 审计
    let mut audit = AuditChain::new();
    audit.append(
        &ctx.principal.subject,
        &raw.id,
        "alliance_optimize",
        if gate.approved { "approved" } else { "blocked" },
    );

    GovernanceReport {
        flow_id: raw.id.clone(),
        flow_name: raw.name.clone(),
        expert_scores: plan.scores.clone(),
        optimization: opt,
        algo,
        gate,
        audit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Principal, Tenant};
    use flow_ai::model::{FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

    fn gov_flow() -> FlowGraph {
        let mut g = FlowGraph::new("gov", "政务数据归集");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("read", "读取公民库", ToolKind::Database, 300)
            .with_access(flow_ai::model::Access::read("db:citizen_info"))
            .with_access(flow_ai::model::Access::write("var:citizen")));
        g.add_node(FlowNode::task("guard", "脱敏", ToolKind::Compute, 50)
            .with_tag("desensitize"));
        g.add_node(FlowNode::task("web1", "网办系统A填报", ToolKind::Browser, 500));
        g.add_node(FlowNode::task("web2", "网办系统B填报", ToolKind::Browser, 400));
        g.add_node(FlowNode::task("merge", "汇总", ToolKind::Compute, 100)
            .with_access(flow_ai::model::Access::read("var:citizen")));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "read"));
        g.add_edge(FlowEdge::seq("read", "guard"));
        g.add_edge(FlowEdge::seq("guard", "web1"));
        g.add_edge(FlowEdge::seq("guard", "web2"));
        g.add_edge(FlowEdge::seq("web1", "merge"));
        g.add_edge(FlowEdge::seq("web2", "merge"));
        g.add_edge(FlowEdge::seq("merge", "e"));
        g
    }

    #[test]
    fn alliance_end_to_end_runs() {
        let g = gov_flow();
        let tenant = Tenant::new("gov-tenant", "ns-gov").regulated(true).with_pool("browser", 1);
        let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
        let ctx = GovernContext::new(tenant, principal);
        let rep = alliance_optimize(&g, &ctx);
        dbp(&rep);
    }

    fn dbp(_r: &GovernanceReport) {}
}
