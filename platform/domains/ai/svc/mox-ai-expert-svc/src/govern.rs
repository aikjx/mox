//! 企业治理（Phase 1 最小实现）：审计日志 + 版本状态机 + SLA 闸门

use crate::context::ResourceQuota;
use crate::reconcile::ReconciledPlan;
use chrono::{DateTime, Utc};
use mox_ai_flow_svc::model::FlowGraph;
use serde::{Deserialize, Serialize};

/// 流程版本状态机
/// 【大白话】一条流程从生到死的"档位"：草稿(Draft)→评审(Review)→批准(Approved)才能出码；
/// 一旦被算法验证(璇玑)否决就进 Blocked，谁也改不了；弃用就是 Deprecated。
/// 只有 Approved 这个档位允许真正生成代码/执行——这是出码的总开关。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Draft,
    Review,
    Approved,
    /// ⛨ 璇玑验证否决（最高优先级，任何权限/合规不可覆盖）
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

/// 审计事件（追加写，不可篡改）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub ts: DateTime<Utc>,
    pub subject: String,
    pub flow_id: String,
    pub action: String,
    pub decision: String,
    pub prev_hash: String,
    pub hash: String,
}

/// 不可篡改审计链（哈希链）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditChain {
    pub events: Vec<AuditEvent>,
}

impl AuditChain {
    pub fn new() -> Self {
        Self::default()
    }
    fn hash(prev: &str, ev: &AuditEvent) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        prev.hash(&mut h);
        ev.subject.hash(&mut h);
        ev.flow_id.hash(&mut h);
        ev.action.hash(&mut h);
        ev.decision.hash(&mut h);
        format!("{:016x}", h.finish())
    }
    pub fn append(
        &mut self,
        subject: &str,
        flow_id: &str,
        action: &str,
        decision: &str,
    ) -> AuditEvent {
        let prev = self
            .events
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "GENESIS".into());
        let prev_hash = prev.clone();
        let mut ev = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            ts: Utc::now(),
            subject: subject.into(),
            flow_id: flow_id.into(),
            action: action.into(),
            decision: decision.into(),
            prev_hash,
            hash: String::new(),
        };
        ev.hash = Self::hash(&prev, &ev);
        self.events.push(ev.clone());
        ev
    }
    /// 返回链上最新一个事件的哈希（作为下一个事件的 prev_hash 基准）。
    /// 空链返回 None（调用方应以 "GENESIS" 作为起点）。
    pub fn latest_hash(&self) -> Option<String> {
        self.events.last().map(|e| e.hash.clone())
    }

    /// 校验链完整性（防篡改）
    pub fn verify(&self) -> bool {
        let mut prev = "GENESIS".to_string();
        for e in &self.events {
            if e.prev_hash != prev {
                return false;
            }
            let mut re = e.clone();
            re.hash.clear();
            let h = Self::hash(&prev, e);
            if h != e.hash {
                return false;
            }
            prev = e.hash.clone();
            let _ = re;
        }
        true
    }
}

/// 治理闸门结果
/// 【大白话】这次"能不能放行"的判决书：approved=true 才准出码；
/// 下面每一栏(sla_ok/budget_ok/algorithm_veto 等)都是判它通过或驳回的理由。
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
    /// 治理 8 闸门明细（I-06 全量门禁）：含 G1~G8 的逐闸结果
    #[serde(default)]
    pub gates: Vec<crate::tenant_policy::GateCheck>,
}

/// 治理裁决
pub fn govern(
    _plan: &ReconciledPlan,
    opt: &mox_ai_flow_svc::pipeline::OptimizationReport,
    status: FlowStatus,
    quota: &ResourceQuota,
    _principal: &str,
    algo_veto: bool,
) -> GateResult {
    let blocking = opt.conflicts.blocking().len();
    let sla_ok = opt.gains.scheduled_ms <= quota.sla_ms;
    let budget_ok = opt.gains.scheduled_ms as f64 <= quota.max_cost_budget * 1000.0;
    // 最高优先级：算法验证否决不可被任何权限/合规覆盖
    // 【大白话】放行的硬条件(全部满足才准出码)：
    //   1) 算法验证(璇玑)没否决——这是最高优先级，权限/合规都压不过它；
    //   2) 流程状态是 Approved(已批准)；
    //   3) 没有阻断级冲突；
    //   4) 跑起来不超时(sla_ok)、不超钱(budget_ok)。
    let approved = !algo_veto && status.can_emit() && blocking == 0 && sla_ok && budget_ok;

    let reason = if algo_veto {
        "⛨ 璇玑验证否决：优化破坏语义/依赖/一致性，治理强制 BLOCK".into()
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
    use mox_ai_flow_svc::conflict::ConflictReport;
    use mox_ai_flow_svc::critpath::CriticalPathReport;
    use mox_ai_flow_svc::dataflow::ParallelPlan;
    use mox_ai_flow_svc::model::FlowGraph;
    use mox_ai_flow_svc::pipeline::{Gains, OptimizationReport};
    use mox_ai_flow_svc::schedule::Schedule;

    #[test]
    fn audit_chain_tamper_detected() {
        let mut c = AuditChain::new();
        c.append("u", "f1", "edit", "ok");
        c.append("u", "f1", "approve", "ok");
        assert!(c.verify());
        // 篡改中间事件
        c.events[0].action = "hacked".into();
        assert!(!c.verify());
    }

    #[test]
    fn status_gate() {
        assert!(FlowStatus::Approved.can_emit());
        assert!(!FlowStatus::Draft.can_emit());
        assert!(FlowStatus::Review.can_edit());
    }

    #[test]
    fn govern_blocks_on_unapproved() {
        let plan = ReconciledPlan {
            graph: FlowGraph::new("f", "f"),
            rules: vec![],
            pools: vec![],
            conflicts: vec![],
            model_routes: vec![],
            scores: vec![],
            adopted_suggestions: vec![],
        };
        let rep = OptimizationReport {
            flow_id: "f".into(),
            flow_name: "f".into(),
            optimized_graph: plan.graph.clone(),
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
        };
        let quota = ResourceQuota::default();
        let g = govern(&plan, &rep, FlowStatus::Draft, &quota, "u", false);
        assert!(!g.approved);
    }

    #[test]
    fn govern_passes_when_approved_clean() {
        let plan = ReconciledPlan {
            graph: FlowGraph::new("f", "f"),
            rules: vec![],
            pools: vec![],
            conflicts: vec![],
            model_routes: vec![],
            scores: vec![],
            adopted_suggestions: vec![],
        };
        let rep = OptimizationReport {
            flow_id: "f".into(),
            flow_name: "f".into(),
            optimized_graph: plan.graph.clone(),
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
        };
        let quota = ResourceQuota {
            max_parallel: 8,
            max_cost_budget: 1.0,
            sla_ms: 5000,
        };
        let g = govern(&plan, &rep, FlowStatus::Approved, &quota, "u", false);
        assert!(g.approved, "{}", g.reason);
    }
}
