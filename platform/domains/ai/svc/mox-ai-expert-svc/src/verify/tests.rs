// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use super::*;
use mox_ai_flow_svc::model::{Access, FlowEdge, FlowNode, ToolKind};

fn base_flow() -> FlowGraph {
    let mut g = FlowGraph::new("t", "测试");
    g.add_node(FlowNode::task("a", "写x", ToolKind::File, 100).with_access(Access::write("var:x")));
    g.add_node(
        FlowNode::task("b", "读x", ToolKind::Compute, 100).with_access(Access::read("var:x")),
    );
    g.add_node(
        FlowNode::task("c", "读y", ToolKind::Compute, 100).with_access(Access::read("var:y")),
    );
    g.add_edge(FlowEdge::seq("a", "b")); // 真数据依赖 a→b (x)
    g.add_edge(FlowEdge::seq("a", "c")); // 真数据依赖 a→c (x)
    g
}

#[test]
fn normal_optimization_passes_verification() {
    let g = base_flow();
    let opt = mox_ai_flow_svc::optimize(&g, &mox_ai_flow_svc::OptimizeConfig::default());
    let v = verify(&g, &opt);
    // 阻断级检查（拓扑/数据依赖/冲突）必须全部通过，不得否决
    assert!(!v.vetoed, "正常优化不应被否决: {:?}", v.checks);
    // 各阻断级 check 必须 passed
    for c in &v.checks {
        if c.blocking {
            assert!(c.passed, "阻断级检查失败: {:?}", c);
        }
    }
}

#[test]
fn veto_when_data_dependency_broken() {
    // 构造一个「坏优化」：删掉真依赖边 a→b，且不保留任何可达路径
    let g = base_flow();
    let mut opt = mox_ai_flow_svc::optimize(&g, &mox_ai_flow_svc::OptimizeConfig::default());
    // 人为破坏：移除 b 节点，制造语义丢失
    opt.optimized_graph.nodes.retain(|n| n.id != "b");
    // 同时让 removed_edges 不含这条（模拟优化器误删真依赖）
    opt.plan.removed_edges.push(("a".into(), "b".into()));
    let v = verify(&g, &opt);
    // 拓扑守恒应失败（节点缺失）
    assert!(v.vetoed, "节点缺失必须被否决: {:?}", v.checks);
    assert!(!v.check("topology").unwrap().passed);
}

#[test]
fn veto_when_blocking_conflict_remains() {
    use mox_ai_flow_svc::conflict::ConflictKind;
    use mox_ai_flow_svc::model::Severity;
    let g = base_flow();
    let mut opt = mox_ai_flow_svc::optimize(&g, &mox_ai_flow_svc::OptimizeConfig::default());
    // 注入一个阻塞冲突且未修复
    opt.conflicts
        .conflicts
        .push(mox_ai_flow_svc::conflict::Conflict::new(
            ConflictKind::DbTransaction,
            Severity::Blocking,
            vec!["x".into()],
            Some("browser".into()),
            "测试阻塞冲突",
            None,
        ));
    let v = verify(&g, &opt);
    assert!(v.vetoed);
    assert!(!v.check("conflict").unwrap().passed);
}

#[test]
fn code_roundtrip_passes_for_generated_code() {
    let g = base_flow();
    let cfg = mox_ai_flow_svc::OptimizeConfig {
        emit_code: true,
        ..Default::default()
    };
    let opt = mox_ai_flow_svc::optimize(&g, &cfg);
    let v = verify(&g, &opt);
    // 不应因代码往返失败而否决（最多告警）
    assert!(!v.vetoed, "代码往返不应阻断: {:?}", v.checks);
}
