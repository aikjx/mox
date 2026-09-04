// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 企业治理闸门（骨架 · TODO：后续迭代补全完整实现）
//!
//! P2 架构解耦 · 阶段 4：
//! - 审计链从内部 `DefaultHasher` 升级为 `mox-audit` 的 SHA-256 哈希链
//! - 治理 8 闸门在 `tenant_policy` 模块中实现
//! - 当前为骨架实现，确保 crate 可独立编译

use crate::context::ResourceQuota;
use crate::reconcile::ReconciledPlan;
use mox_ai_flow_core::model::FlowGraph;
use mox_ai_flow_core::pipeline::OptimizationReport;
use serde::{Deserialize, Serialize};

/// 流程版本状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Draft,
    Review,
    Approved,
    /// 璇玑验证否决（最高优先级，任何权限/合规不可覆盖）
    Blocked,
    Deprecated,
}

impl FlowStatus {
    /// 是否允许出码/执行
    pub fn can_emit(&self) -> bool {
        matches!(self, FlowStatus::Approved)
    }
    /// 是否被算法验证否决
    pub fn is_vetoed(&self) -> bool {
        matches!(self, FlowStatus::Blocked)
    }
    pub fn can_edit(&self) -> bool {
        matches!(self, FlowStatus::Draft | FlowStatus::Review)
    }
}

/// 治理闸门结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub status: FlowStatus,
    pub approved: bool,
    pub sla_ok: bool,
    pub budget_ok: bool,
    pub blocking_risks: usize,
    /// 璇玑验证否决（最高优先级，任何权限/合规都不可覆盖）
    pub algorithm_veto: bool,
    pub reason: String,
    /// 治理 8 闸门明细（I-06 全量门禁）
    #[serde(default)]
    pub gates: Vec<crate::tenant_policy::GateCheck>,
}

/// 治理裁决（骨架实现：仅做基础 SLA/预算/状态判定，完整逻辑待后续迁移）
///
/// TODO(P2 阶段 4 后续迭代)：迁移完整治理逻辑，包括：
/// - 资源/权限守恒校验
/// - 零孤儿节点校验
/// - 完整的 8 闸门评估
pub fn govern(
    _plan: &ReconciledPlan,
    opt: &OptimizationReport,
    status: FlowStatus,
    quota: &ResourceQuota,
    _principal: &str,
    algo_veto: bool,
) -> GateResult {
    let blocking = opt.conflicts.blocking().len();
    let sla_ok = opt.gains.scheduled_ms <= quota.sla_ms;
    let budget_ok = opt.gains.scheduled_ms as f64 <= quota.max_cost_budget * 1000.0;

    let approved = !algo_veto && status.can_emit() && blocking == 0 && sla_ok && budget_ok;

    let reason = if algo_veto {
        "璇玑验证否决：优化破坏语义/依赖/一致性，治理强制 BLOCK".into()
    } else if !status.can_emit() {
        format!("流程状态为 {:?}，仅 Approved 可出码", status)
    } else if blocking > 0 {
        format!("存在 {} 个阻断级冲突", blocking)
    } else if !sla_ok {
        format!(
            "调度耗时 {}ms 超出 SLA {}ms",
            opt.gains.scheduled_ms, quota.sla_ms
        )
    } else if !budget_ok {
        "超出成本预算".into()
    } else {
        "通过".into()
    };

    GateResult {
        status,
        approved,
        sla_ok,
        budget_ok,
        blocking_risks: blocking,
        algorithm_veto: algo_veto,
        reason,
        gates: Vec::new(),
    }
}

/// 把 ReconciledPlan 的规则并入图（供 flow-ai 出码前使用）
pub fn apply_rules(graph: &mut FlowGraph, plan: &ReconciledPlan) {
    for r in &plan.rules {
        if !graph.rules.iter().any(|x| x.id == r.id) {
            graph.rules.push(r.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_core::conflict::ConflictReport;
    use mox_ai_flow_core::critpath::CriticalPathReport;
    use mox_ai_flow_core::dataflow::ParallelPlan;
    use mox_ai_flow_core::model::FlowGraph;
    use mox_ai_flow_core::pipeline::{Gains, OptimizationReport};
    use mox_ai_flow_core::schedule::Schedule;

    fn make_plan() -> ReconciledPlan {
        ReconciledPlan {
            graph: FlowGraph::new("f", "f"),
            rules: vec![],
            pools: vec![],
            conflicts: vec![],
            model_routes: vec![],
            scores: vec![],
            adopted_suggestions: vec![],
        }
    }

    fn make_opt() -> OptimizationReport {
        OptimizationReport {
            flow_id: "f".into(),
            flow_name: "f".into(),
            optimized_graph: FlowGraph::new("f", "f"),
            plan: ParallelPlan {
                dependencies: vec![],
                removed_edges: vec![],
                layers: vec![],
                sequential_ms: 10,
                parallel_ms: 10,
            },
            critical_path: CriticalPathReport {
                timings: vec![],
                critical_paths: vec![],
                makespan_ms: 10,
                optimization_targets: vec![],
            },
            schedule: Schedule {
                slots: vec![],
                makespan_ms: 10,
                lower_bound_ms: 10,
                resource_delay_ms: 0,
                max_concurrency: 1,
                pools: vec![],
            },
            conflicts: ConflictReport { conflicts: vec![] },
            model_routing: vec![],
            gains: Gains {
                sequential_ms: 10,
                critical_path_ms: 10,
                scheduled_ms: 10,
                speedup: 1.0,
                time_saved_pct: 0.0,
                removed_false_deps: 0,
                parallel_layers: 1,
                max_concurrency: 1,
                conflicts_found: 0,
                conflicts_blocking: 0,
                conflicts_auto_fixed: 0,
                compute_saved_pct: 0.0,
            },
            code: None,
            route: None,
        }
    }

    #[test]
    fn status_gate() {
        assert!(FlowStatus::Approved.can_emit());
        assert!(!FlowStatus::Draft.can_emit());
        assert!(FlowStatus::Review.can_edit());
    }

    #[test]
    fn govern_blocks_on_unapproved() {
        let plan = make_plan();
        let rep = make_opt();
        let quota = ResourceQuota::default();
        let g = govern(&plan, &rep, FlowStatus::Draft, &quota, "u", false);
        assert!(!g.approved);
    }

    #[test]
    fn govern_passes_when_approved_clean() {
        let plan = make_plan();
        let rep = make_opt();
        let quota = ResourceQuota {
            max_parallel: 8,
            max_cost_budget: 1.0,
            sla_ms: 5000,
        };
        let g = govern(&plan, &rep, FlowStatus::Approved, &quota, "u", false);
        assert!(g.approved, "{}", g.reason);
    }

    #[test]
    fn apply_rules_adds_new_rules() {
        let plan = make_plan();
        // plan 有 rules，这里用一个空的
        let mut graph = FlowGraph::new("f", "f");
        apply_rules(&mut graph, &plan);
        assert_eq!(graph.rules.len(), 0);
    }
}
