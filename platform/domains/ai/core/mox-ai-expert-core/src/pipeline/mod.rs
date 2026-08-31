// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 全维处理管线（骨架 · TODO：后续迭代补全完整实现）
//!
//! normalize → 并行派发专家 → 裁决 → flow-ai 求解 → 治理闸门 → 出码
//!
//! P2 架构解耦 · 阶段 4：
//! 当前为骨架实现，提供 `mox_optimize` 的最小可运行版本。
//! 完整的插件化运行时（Harness）、瀑布扩展点等将在后续迭代中迁移。

use crate::context::GovernContext;
use crate::expert::dispatch;
use crate::experts::all_experts;
use crate::govern::{apply_rules, govern, FlowStatus, GateResult};
use crate::ir::auto_dimension;
use crate::reconcile::reconcile;
use crate::verify::verify;
use crate::context::ExpertContext;
use mox_ai_flow_svc::model::FlowGraph;
use mox_ai_flow_svc::pipeline::optimize;
use serde::{Deserialize, Serialize};

/// 全维治理报告：专家评分 + 裁决冲突 + 优化报告 + 治理闸门
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceReport {
    pub flow_id: String,
    pub flow_name: String,
    /// 各专家健康分 (专家, 分)
    pub expert_scores: Vec<(String, f64)>,
    /// 优化报告（含并行/关键路径/调度/冲突/代码）
    pub optimization: mox_ai_flow_svc::pipeline::OptimizationReport,
    /// ⛨ 璇玑验证报告（最高权限，不可被治理覆盖）
    pub algo: crate::verify::AlgoVerification,
    /// 治理闸门结果
    pub gate: GateResult,
    /// 审计链（使用 mox-audit 的 SHA-256 哈希链）
    pub audit: mox_audit::AuditChain,
    /// 采纳的专家优化建议
    pub adopted_suggestions: Vec<mox_ai_expert_proto::Suggestion>,
}

/// 全维优化入口（骨架实现）
///
/// TODO(P2 阶段 4 后续迭代)：
/// - 接入插件化运行时（Harness）
/// - 接入治理 8 闸门全量门禁
/// - 接入专家否决级风险汇总
/// - 接入裁决冲突升级阻断
pub fn mox_optimize(raw: &FlowGraph, ctx: &GovernContext) -> GovernanceReport {
    // 1. 归一化：维度着色
    let df = auto_dimension(raw);
    let base = &df.base;

    // 2. 并行派发十四位专家
    let ectx = ExpertContext::new(base, ctx);
    let experts = all_experts();
    let opinions = dispatch(&ectx, &experts);

    // 3. 归一化裁决 → ReconciledPlan
    let plan = reconcile(&opinions, base, &base.pools);

    // 4. 交给 flow-ai 引擎做最优求解
    let mut graph = plan.graph.clone();
    apply_rules(&mut graph, &plan);
    let mut opt = optimize(&graph, &mox_ai_flow_svc::pipeline::OptimizeConfig::default());

    // 把专家采纳的算力路由并入优化报告
    if !plan.model_routes.is_empty() {
        opt.model_routing = plan
            .model_routes
            .iter()
            .map(|(node, tier)| mox_ai_flow_svc::schedule::ModelRouting {
                node_id: node.clone(),
                model_tier: *tier,
                reason: "mox-expert 算法/资源专家路由".into(),
            })
            .collect();
    }

    // 5. ⛨ 璇玑验证网关（最高权限，在治理之前）
    let algo = verify(base, &opt);

    // 6. 治理闸门（尊重算法否决）
    let status = if algo.vetoed {
        FlowStatus::Blocked
    } else {
        FlowStatus::Approved
    };
    let gate = govern(
        &plan,
        &opt,
        status,
        &ctx.quota,
        &ctx.principal.subject,
        algo.vetoed,
    );

    // 7. 审计（使用 mox-audit 的 SHA-256 哈希链）
    let mut audit = mox_audit::AuditChain::new();
    let event = mox_audit::AuditEvent::new(
        mox_audit::AuditActor::human(&ctx.principal.subject, &ctx.principal.roles.join(",")),
        mox_audit::AuditAction::FlowApproved,
        mox_audit::AuditResource::flow(&raw.id, &ctx.tenant.id),
        if gate.approved {
            mox_audit::AuditOutcome::Success
        } else {
            mox_audit::AuditOutcome::Failure
        },
        mox_audit::AuditSeverity::Info,
        ctx.tenant.id.clone(),
    );
    let _ = audit.append(&event);

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

    #[test]
    fn mox_optimize_runs_on_empty_graph() {
        let g = FlowGraph::new("test", "test-flow");
        let tenant = Tenant::new("t", "ns");
        let principal = Principal::new("admin").with_roles(vec!["admin".into()]);
        let ctx = GovernContext::new(tenant, principal);
        let rep = mox_optimize(&g, &ctx);
        assert_eq!(rep.flow_id, "test");
        assert_eq!(rep.expert_scores.len(), 14, "应有 14 位专家");
        assert!(!rep.algo.checks.is_empty());
    }

    #[test]
    fn fourteen_experts_all_present() {
        let g = FlowGraph::new("x", "t");
        let tenant = Tenant::new("t", "ns");
        let principal = Principal::new("admin").with_roles(vec!["admin".into()]);
        let ctx = GovernContext::new(tenant, principal);
        let rep = mox_optimize(&g, &ctx);

        let dims: std::collections::HashSet<String> =
            rep.expert_scores.iter().map(|(d, _)| d.clone()).collect();
        for expected in [
            "business",
            "algorithm",
            "permission",
            "resource",
            "security",
            "data",
            "observability",
            "architecture",
            "security_code",
            "code_quality",
            "performance",
            "testing",
            "documentation",
            "maintainability",
        ] {
            assert!(
                dims.contains(expected),
                "治理报告缺少维度 {expected}"
            );
        }
        assert_eq!(dims.len(), 14);
    }

    #[test]
    fn audit_chain_records_event() {
        let g = FlowGraph::new("x", "t");
        let tenant = Tenant::new("t", "ns");
        let principal = Principal::new("admin").with_roles(vec!["admin".into()]);
        let ctx = GovernContext::new(tenant, principal);
        let rep = mox_optimize(&g, &ctx);
        // 审计链应有记录（骨架实现至少有一个 emit 事件）
        assert!(!rep.audit.is_empty());
        // 链完整性校验
        assert!(rep.audit.verify().is_ok());
    }
}
