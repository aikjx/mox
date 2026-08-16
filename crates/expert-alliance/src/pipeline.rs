//! 全维处理流水线：normalize → 并行派发专家 → 裁决 → flow-ai 求解 → 治理闸门 → 出码

use crate::context::{ExpertContext, GovernContext};
use crate::govern::{apply_rules, govern, AuditChain, FlowStatus, GateResult};
use crate::harness::{
    expert_plugins, run_experts, HarnessCtx, HarnessProfile, ModelAdapterConfig, WaterfallEvent,
    WaterfallState,
};
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
    /// 采纳的专家优化建议（经裁决器确认、未与硬约束冲突的 Suggestion）。
    /// P1：此前专家产出的建议停留在 ExpertOpinion，流水线从不消费；现显式采纳并对外暴露。
    pub adopted_suggestions: Vec<crate::expert::Suggestion>,
}

/// 全维优化入口
///
/// 采用插件化运行时（参考 DeepSeek Harness "Everything is a Plugin"）：
/// 专家被装载为 [`ExpertPlugin`]，由 [`HarnessCtx`] 的瀑布扩展点驱动，治理钩子可在
/// `pre_gate` / `post_gate` 注入策略与审计。无插件运行时时行为等价于旧版硬编码派发。
pub fn alliance_optimize(raw: &FlowGraph, ctx: &GovernContext) -> GovernanceReport {
    // 0. 构建插件化运行时：装载七位专家 + 治理/审计钩子
    let profile = HarnessProfile {
        name: "default-alliance".into(),
        plugins: vec![
            "algorithm".into(), "business".into(), "data".into(),
            "observability".into(), "permission".into(), "resource".into(), "security".into(),
        ],
        audit_enabled: true,
        model: ModelAdapterConfig::default(),
    };
    let harness = HarnessCtx::new(profile);
    // 装载专家插件（保持与 all_experts() 一致的能力集）
    for p in expert_plugins(crate::experts::all_experts()) {
        harness.load_plugin(p);
    }
    // 装载治理钩子：闸门前后做审计切面（reversible effect 示例）
    harness.hook(
        WaterfallEvent::PostGate,
        std::sync::Arc::new(|_ev, hctx, state, next| {
            if let Some(gate) = &state.gate {
                hctx.emit(if gate.approved { "gate/approved" } else { "gate/blocked" });
            }
            next(state)
        }),
    );

    // 1. 归一化：维度着色
    let df = auto_dimension(raw);
    let base = &df.base;

    // 2. 并行派发七位专家（经插件化运行时 + 瀑布扩展点）
    let ectx = ExpertContext::new(base, ctx);
    let experts = crate::experts::all_experts();
    let opinions = run_experts(&harness, &ectx, &experts);

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
    let expert_veto = opinions.iter().any(|o| o.risks.iter().any(|r| r.veto));
    // 5.6 裁决冲突升级：同优先级维度约束冲突无法仲裁 → 升级 Blocking 否决
    let escalated_conflict = plan.conflicts.iter().any(|c| c.escalated);
    let mut algo = verify(base, &opt);
    if expert_veto {
        algo.vetoed = true;
        algo.summary = format!(
            "{}; 专家否决级风险(生产/敏感数据越权写等)未通过安全审批",
            algo.summary
        );
    }
    if escalated_conflict {
        algo.vetoed = true;
        let detail: Vec<String> = plan
            .conflicts
            .iter()
            .filter(|c| c.escalated)
            .map(|c| c.resolution.clone())
            .collect();
        algo.summary = format!(
            "{}; 裁决冲突升级阻断: {}",
            algo.summary,
            detail.join(" | ")
        );
    }

    // 6. 治理闸门（尊重算法否决）
    let status = if algo.vetoed { FlowStatus::Blocked } else { FlowStatus::Approved };
    let mut gate = govern(&plan, &opt, status, &ctx.quota, &ctx.principal.subject, algo.vetoed);

    // 6.5 闸门瀑布扩展点：PreGate 钩子可追加前置校验，PostGate 钩子做后置审计
    let mut wf_state = WaterfallState {
        opinions: opinions.clone(),
        gate: Some(gate.clone()),
        bag: std::collections::HashMap::new(),
    };
    if let Err(e) = harness.run_waterfall(WaterfallEvent::PreGate, &mut wf_state) {
        tracing::warn!(target: "harness", "PreGate 瀑布执行失败: {}", e);
    }
    // 若 PreGate 钩子重写了闸门结果，采用之
    if let Some(g) = wf_state.gate.clone() {
        gate = g;
    }
    if let Err(e) = harness.run_waterfall(WaterfallEvent::PostGate, &mut wf_state) {
        tracing::warn!(target: "harness", "PostGate 瀑布执行失败: {}", e);
    }

    // 7. 审计（内部链）
    let mut audit = AuditChain::new();
    audit.append(
        &ctx.principal.subject,
        &raw.id,
        "alliance_optimize",
        if gate.approved { "approved" } else { "blocked" },
    );

    // 8. 运行时收尾：unload 插件 + unwind 可逆副作用
    harness.shutdown();

    GovernanceReport {
        flow_id: raw.id.clone(),
        flow_name: raw.name.clone(),
        expert_scores: plan.scores.clone(),
        optimization: opt,
        algo,
        gate,
        audit,
        adopted_suggestions: plan.adopted_suggestions.clone(),
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
