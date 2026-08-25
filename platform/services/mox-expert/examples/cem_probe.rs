//! Quick sanity: run 1 RED CEM + 1 GREEN CEM and print durations.
use std::time::Instant;

use flow_ai::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use flow_ai::OptimizeConfig;
use mox_expert::verify::{
    cem_deep_chain_with_defaults, CemConfig, ConstraintSpec, ObjectiveSpec,
};

fn build_chain(n: u32) -> FlowGraph {
    let mut g = FlowGraph::new("deep", "chain");
    g.add_node(FlowNode::new("s", "s", NodeKind::Start));
    g.add_node(FlowNode::new("e", "e", NodeKind::End));
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
    g.add_edge(FlowEdge::seq(&prev, "e"));
    g
}

fn main() {
    let g = build_chain(500);
    let cfg = OptimizeConfig {
        emit_code: false,
        auto_repair: true,
        ..Default::default()
    };
    let cs = vec![
        ConstraintSpec {
            id: "scheduled_ms".into(),
            direction: "le".into(),
            threshold: 6000,
        },
        ConstraintSpec {
            id: "conflicts_blocking".into(),
            direction: "eq".into(),
            threshold: 0,
        },
    ];
    let os = vec![
        ObjectiveSpec {
            id: "sched".into(),
            weight_e4: 5500,
            minimize: true,
        },
        ObjectiveSpec {
            id: "speedup".into(),
            weight_e4: 2000,
            minimize: false,
        },
        ObjectiveSpec {
            id: "conflict".into(),
            weight_e4: 1500,
            minimize: true,
        },
        ObjectiveSpec {
            id: "algo".into(),
            weight_e4: 1000,
            minimize: true,
        },
    ];
    let base = CemConfig {
        population: 12,
        max_rounds: 20,
        sigma_stop: 0.06,
        no_improve_stop: 3,
        memo: false,
        obj_prune: false,
        parallel: false,
        verify_cache: false,
        ..Default::default()
    };

    let t = Instant::now();
    let r = cem_deep_chain_with_defaults(&g, &cs, &os, &cfg, base.clone(), None);
    println!(
        "RED 1 run: {} ms, rounds={} stop={:?} pareto={} σ̄={:.4}",
        t.elapsed().as_millis(),
        r.rounds,
        r.stop_reason,
        r.pareto_size,
        r.sigma_final
    );

    let mut green = base.clone();
    green.memo = true;
    green.obj_prune = true;
    green.parallel = true;
    green.verify_cache = true;
    let t = Instant::now();
    let r = cem_deep_chain_with_defaults(&g, &cs, &os, &cfg, green, None);
    println!(
        "GREEN 1 run: {} ms, rounds={} stop={:?} pareto={} σ̄={:.4} memo(hit/miss)={}/{}",
        t.elapsed().as_millis(),
        r.rounds,
        r.stop_reason,
        r.pareto_size,
        r.sigma_final,
        r.memo_hits,
        r.memo_misses
    );
}
