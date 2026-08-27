// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Quick profile — measure optimize + verify + mox_optimize for a 500-deep-chain once.
use std::time::Instant;

use mox_ai_flow_svc::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use mox_ai_flow_svc::{optimize, OptimizeConfig};
use mox_ai_expert_svc::context::{GovernContext, Principal, Tenant};
use mox_ai_expert_svc::verify::verify;

fn build_deep(n: u32) -> FlowGraph {
    let mut g = FlowGraph::new(format!("d{}", n), "deep");
    g.add_node(FlowNode::new("s", "start", NodeKind::Start));
    g.add_node(FlowNode::new("e", "end", NodeKind::End));
    let mut prev = "s".to_string();
    for i in 0..n {
        let id = format!("t{}", i);
        let mut node = FlowNode::task(&id, format!("t{}", i), ToolKind::Compute, 10)
            .with_access(Access::write(format!("var:x{}", i)));
        if i > 0 {
            node = node.with_access(Access::read(format!("var:x{}", i - 1)));
        }
        g.add_node(node);
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(prev.as_str(), "e"));
    g
}

fn cfg() -> OptimizeConfig {
    OptimizeConfig {
        emit_code: false,
        auto_repair: true,
        ..Default::default()
    }
}

fn main() {
    let g = build_deep(500);
    // warmup
    let _ = optimize(&g, &cfg());
    // --- optimize only ---
    let mut t = Instant::now();
    let rep = optimize(&g, &cfg());
    println!("optimize once: {} ms", t.elapsed().as_millis());
    // --- verify only ---
    t = Instant::now();
    let v = verify(&g, &rep);
    println!(
        "verify once: {} ms, vetoed={}",
        t.elapsed().as_millis(),
        v.vetoed
    );
    // --- mox_optimize ---
    let tenant = Tenant::new("acme", "fin");
    let prin = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
    let mut ctx = GovernContext::new(tenant, prin);
    ctx.quota.sla_ms = 1_000_000;
    ctx.quota.max_cost_budget = 1_000_000.0;
    // warmup mox
    let _ = mox_ai_expert_svc::pipeline::mox_optimize(&FlowGraph::new("mini", "m"), &ctx);
    t = Instant::now();
    let r = mox_ai_expert_svc::pipeline::mox_optimize(&g, &ctx);
    println!(
        "mox_optimize once: {} ms, vetoed={}, gate.approved={}",
        t.elapsed().as_millis(),
        r.algo.vetoed,
        r.gate.approved
    );
}
