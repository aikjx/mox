//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 溯源链路: R1 → F4 → B1 → A1 → T3 → C2
//! 数据设计: S3(Topology)
//! 说明: κ/τ 预算 + ℛ̂ 裁剪（封装 mox_ai_flow_svc κ‑τ 引擎 validate/regularize）。
//! 规格: primiflow/SPEC.md（§1 滑块映射 / §5 调度算法 / §10 DoD）

use mox_ai_flow_svc::model::FlowGraph;
use mox_ai_flow_svc::primitive::{
    regularize as kt_regularize, validate as kt_validate, CandidateTopology, PrimitiveState,
    ResourceBudget, ValidationReport,
};

/// ℛ̂ 输出
#[derive(Debug, Clone)]
pub struct RegularizeOutput {
    /// 合规（或经裁剪后合规）的拓扑
    pub graph: FlowGraph,
    /// 调整后的 κ‑τ 状态
    pub state: PrimitiveState,
    /// 守恒残差 Δ = C² − (κ² + τ²)
    pub delta: f64,
    /// 是否经过了正则化裁剪
    pub regularized: bool,
    /// 实际算力代价（全部可执行节点耗时之和，ms）
    pub cost_ms: u64,
    /// 预算（ms）
    pub budget_ms: u64,
}

/// κ/τ 调度器：滑块映射 + ℛ̂ 正则化
#[derive(Debug, Default)]
pub struct Scheduler;

impl Scheduler {
    pub fn new() -> Self {
        Self
    }
    /// 滑块 s∈[0,1]（稳定优先 ↔ 探索优先）→ 原语状态（SPEC §1）
    ///
    /// ```text
    /// θ = s · π/2 ;  κ = cos θ ;  τ = sin θ ;  C = C_base · (1 + budget_factor)
    /// ```
    /// s=0 → 纯稳定（κ=1,τ=0）；s=1 → 纯探索（κ=0,τ=1）。C≠1 让资源上界独立可调。
    pub fn from_slider(&self, s: f64, c_base: f64, budget_factor: f64) -> PrimitiveState {
        let s = s.clamp(0.0, 1.0);
        let theta = s * std::f64::consts::FRAC_PI_2;
        let kappa = theta.cos();
        let tau = theta.sin();
        let c = (c_base * (1.0 + budget_factor.max(0.0))).max(1e-6);
        PrimitiveState {
            c,
            kappa: kappa * c,
            tau: tau * c,
            q: 0.0,
        }
    }

    /// ℛ̂ 正则化：对给定拓扑 + 预算做守恒/因果/资源三道闸门裁剪，保证 Δ≥0。
    ///
    /// 返回合规拓扑、调整后状态、残差 Δ、是否裁剪。
    pub fn regularize(
        &self,
        graph: FlowGraph,
        state: PrimitiveState,
        budget: ResourceBudget,
    ) -> RegularizeOutput {
        let topo = CandidateTopology {
            graph,
            reused_subtasks: Vec::new(),
            explored_subtasks: Vec::new(),
            fanout: 1,
        };
        let report: ValidationReport = kt_validate(&topo, &state, &budget);
        let (rt, rs) = kt_regularize(&report, &topo, &state, &budget);
        let delta = rs.c * rs.c - (rs.kappa * rs.kappa + rs.tau * rs.tau);
        let cost_ms = total_ms(&rt.graph);
        RegularizeOutput {
            graph: rt.graph,
            state: rs,
            delta,
            regularized: report.has(mox_ai_flow_svc::primitive::ViolationKind::CausalCycle)
                || report.has(mox_ai_flow_svc::primitive::ViolationKind::ResourceQuota),
            cost_ms,
            budget_ms: budget.total_ms,
        }
    }
}

fn total_ms(g: &FlowGraph) -> u64 {
    g.nodes
        .iter()
        .filter_map(|n| n.tool.map(|_| n.duration_ms))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_svc::model::{FlowEdge, FlowNode, ToolKind};

    #[test]
    fn slider_extremes_map_correctly() {
        let sc = Scheduler::new();
        let stable = sc.from_slider(0.0, 10.0, 0.0);
        assert!((stable.kappa - 10.0).abs() < 1e-9, "s=0 应 κ=C");
        assert!(stable.tau.abs() < 1e-9, "s=0 应 τ=0");
        let explore = sc.from_slider(1.0, 10.0, 0.0);
        assert!(explore.kappa.abs() < 1e-9, "s=1 应 κ=0");
        assert!((explore.tau - 10.0).abs() < 1e-9, "s=1 应 τ=C");
    }

    #[test]
    fn budget_factor_scales_c() {
        let sc = Scheduler::new();
        let s1 = sc.from_slider(0.5, 10.0, 0.0);
        let s2 = sc.from_slider(0.5, 10.0, 1.0);
        assert!(
            (s2.c - 2.0 * s1.c).abs() < 1e-9,
            "budget_factor=1 应使 C 翻倍"
        );
    }

    #[test]
    fn regularize_passes_reasonable_budget() {
        let sc = Scheduler::new();
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::task("a", "A", ToolKind::Http, 300));
        g.add_node(FlowNode::task("b", "B", ToolKind::Compute, 200));
        g.add_edge(FlowEdge::seq("a", "b"));
        let state = sc.from_slider(0.3, 10.0, 0.0);
        let budget = ResourceBudget {
            total_ms: 10_000,
            per_pool: Default::default(),
        };
        let out = sc.regularize(g, state, budget);
        assert!(out.delta.abs() < 1e-6, "守恒残差应≈0");
        assert!(!out.regularized);
    }

    #[test]
    fn regularize_fixes_overrun() {
        let sc = Scheduler::new();
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::task("a", "A", ToolKind::Http, 300));
        g.add_node(FlowNode::task("b", "B", ToolKind::Compute, 200));
        g.add_edge(FlowEdge::seq("a", "b"));
        let state = sc.from_slider(0.9, 10.0, 0.0); // 探索优先，节点多
        let budget = ResourceBudget {
            total_ms: 100,
            per_pool: Default::default(),
        };
        let out = sc.regularize(g, state, budget);
        assert!(out.regularized, "超预算应触发正则化");
        assert!(out.cost_ms <= out.budget_ms.max(1), "裁剪后代价应≤预算");
        assert!(out.delta.abs() < 1e-6);
    }
}
