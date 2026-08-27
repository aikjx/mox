// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 缺口 P1.2 —— auto_repair 幂等性测试（多次修复同一冲突）
//!
//! 目的：验证 `mox_ai_flow_svc::conflict::auto_repair` 对同一冲突反复应用是**幂等**的：
//!  - 首次修复落实修正（插入 Guard / 加互斥边），`applied > 0`；
//!  - 再次对「已修复图」运行 detect+auto_repair，`applied == 0` 且图结构不变；
//!  - 循环 10 次后累计额外修正为 0，图保持稳定（Guard 不重复插入、互斥边不重复叠加）。
//!
//! 覆盖两类自动修法：
//!  - `Remedy::InsertGuard`（合规脱敏 Guard 缺失）
//!  - `Remedy::Serialize`（浏览器/资源并发互斥）

use mox_ai_flow_svc::conflict::{auto_repair, detect, ConflictKind, Remedy};
use mox_ai_flow_svc::model::{
    Access, EdgeKind, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity, ToolKind,
};

/// 缺失 desensitize Guard 的合规冲突图（触发 InsertGuard 修法）
fn missing_guard_graph() -> FlowGraph {
    let mut g = FlowGraph::new("c", "公民数据查询");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("q", "查询公民信息", ToolKind::Database, 50)
            .with_access(Access::read("db:citizen_info")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "q"));
    g.add_edge(FlowEdge::seq("q", "e"));
    g.rules.push(ExpertRule {
        id: "GOV-001".into(),
        description: "公民敏感数据必须先脱敏".into(),
        severity: Severity::Blocking,
        resource_prefixes: vec!["db:citizen_".into()],
        tool_kinds: vec![],
        required_guard_tags: vec!["desensitize".into()],
    });
    g
}

/// 三个浏览器节点并发抢占（触发 Serialize 互斥修法）
fn browser_contention_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b", "浏览器并发");
    g.add_node(FlowNode::task("b1", "抓取A", ToolKind::Browser, 100));
    g.add_node(FlowNode::task("b2", "抓取B", ToolKind::Browser, 100));
    g.add_node(FlowNode::task("b3", "抓取C", ToolKind::Browser, 100));
    g
}

fn is_mutex(e: &FlowEdge) -> bool {
    matches!(e.kind, EdgeKind::Mutex)
}

#[test]
fn insert_guard_is_idempotent_across_repeats() {
    let g0 = missing_guard_graph();
    let rep0 = detect(&g0, &[]);
    assert!(rep0.has_blocking(), "缺失脱敏 Guard 应产生阻塞冲突");
    assert_eq!(rep0.count_of(ConflictKind::Compliance), 1);

    // 首次修复：应落实 1 次 InsertGuard
    let (g, n0) = auto_repair(&g0, &rep0);
    assert!(n0 >= 1, "首次修复应至少应用 1 处修正");
    let guard_count = g
        .nodes
        .iter()
        .filter(|x| x.kind == NodeKind::Guard && x.tags.iter().any(|t| t == "desensitize"))
        .count();
    assert_eq!(guard_count, 1, "应恰好插入 1 个 desensitize Guard");
    // 修复后 Guard 前置连线到 q
    assert!(
        g.edges.iter().any(|e| e.from == "__guard_desensitize_q"
            && e.to == "q"
            && e.kind == EdgeKind::Sequence),
        "Guard 应前置连线到 q"
    );

    // 修复后重新检测：阻塞冲突应已清除
    let rep1 = detect(&g, &[]);
    assert!(!rep1.has_blocking(), "修复后不应再残留阻塞冲突");

    // 幂等：对已修复图再次修复，applied 必须为 0，图结构不变
    let (g2, n1) = auto_repair(&g, &rep1);
    assert_eq!(n1, 0, "第二次修复不应再改动");
    assert_eq!(g.nodes.len(), g2.nodes.len(), "图节点数应保持一致");
    assert_eq!(g.edges.len(), g2.edges.len(), "图边数应保持一致");

    // 连续 10 次：除首次外全部幂等（累计额外修正 == 0）
    let mut cur = g.clone();
    let mut extra = 0usize;
    for _ in 0..10 {
        let rep = detect(&cur, &[]);
        assert!(!rep.has_blocking(), "反复检测不应再出现阻塞冲突");
        let (next, applied) = auto_repair(&cur, &rep);
        extra += applied;
        cur = next;
    }
    assert_eq!(
        extra, 0,
        "多次修复同一冲突应完全幂等（除首次外额外修正为 0）"
    );
    let final_guard = cur
        .nodes
        .iter()
        .filter(|x| x.kind == NodeKind::Guard && x.tags.iter().any(|t| t == "desensitize"))
        .count();
    assert_eq!(final_guard, 1, "Guard 不应被重复插入");
}

#[test]
fn serialize_mutex_is_idempotent_across_repeats() {
    let g0 = browser_contention_graph();
    let groups = vec![vec!["b1".into(), "b2".into(), "b3".into()]];
    let rep0 = detect(&g0, &groups);
    assert!(rep0.has_blocking(), "浏览器并发应产生阻塞冲突");
    assert!(rep0.count_of(ConflictKind::BrowserContention) >= 1);

    // 首次修复：落实互斥硬边
    let (g, n0) = auto_repair(&g0, &rep0);
    assert!(n0 >= 1, "首次修复应落实至少 1 条互斥边");
    let mutex0 = g.edges.iter().filter(|e| is_mutex(e)).count();
    assert!(mutex0 > 0, "应存在互斥硬边");
    assert_eq!(
        mutex0, 1,
        "三节点并发组应被序列化为 1 条互斥硬边（而非两两全连）"
    );

    // 幂等：对已修复图再次修复，互斥边不应被重复叠加
    let rep1 = detect(&g, &groups);
    let (g2, n1) = auto_repair(&g, &rep1);
    assert_eq!(n1, 0, "二次修复不应再添加互斥边（幂等）");
    assert_eq!(
        g2.edges.iter().filter(|e| is_mutex(e)).count(),
        mutex0,
        "互斥边数量应保持一致"
    );

    // 连续 10 次：互斥边数量恒定，额外修正为 0
    let mut cur = g.clone();
    let mut extra = 0usize;
    for _ in 0..10 {
        let rep = detect(&cur, &groups);
        let (next, applied) = auto_repair(&cur, &rep);
        extra += applied;
        cur = next;
    }
    assert_eq!(extra, 0, "多次序列化修复应完全幂等");
    assert_eq!(cur.edges.iter().filter(|e| is_mutex(e)).count(), mutex0);
}

#[test]
fn repair_remedy_shape_is_serializable_for_guard() {
    // 确认冲突携带的 remedy 是 InsertGuard（供外部/前端消费）
    let g = missing_guard_graph();
    let rep = detect(&g, &[]);
    let c = rep
        .conflicts
        .iter()
        .find(|c| c.kind == ConflictKind::Compliance)
        .unwrap();
    match &c.remedy {
        Some(Remedy::InsertGuard { before, tag, .. }) => {
            assert_eq!(before, "q");
            assert_eq!(tag, "desensitize");
        }
        other => panic!("合规冲突应给出 InsertGuard 修法，实际 {:?}", other),
    }
}
