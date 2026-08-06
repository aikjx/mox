//! 迭代测试：企业级业务流程 + 算法控制流（并行/互斥/异常/循环/子流程）
//!
//! 目的：用复杂真实用例压 `programming_pipeline`，验证 G-A~E 护栏与
//! ①-⑩ 在边界场景下的正确性。每轮失败→修复→复测，结果记入工作区文档。

use expert_alliance::context::{GovernContext, LoopGuard, LoopPolicy, Principal, ResourceQuota, Tenant};
use expert_alliance::programming::{programming_pipeline, Checkpoint};
use flow_ai::model::{Access, EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

/// 放宽配额，避免 SLA/成本预算误杀正常示例
fn base_ctx(roles: Vec<&str>, regulated: bool) -> GovernContext {
    let mut t = Tenant::new("acme", "fin").with_pool("browser", 4);
    t.regulated = regulated;
    let principal = Principal::new("ops").with_roles(roles.into_iter().map(String::from).collect());
    let mut ctx = GovernContext::new(t, principal);
    ctx.quota = ResourceQuota {
        max_parallel: 8,
        max_cost_budget: 100.0,
        sla_ms: 50_000,
    };
    ctx
}

/// B1：企业报销并行审批 + 互斥支付 + 异常归档
fn enterprise_approval_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b1", "企业报销审批");
    g.add_node(FlowNode::new("s", "发起", NodeKind::Start));
    g.add_node(FlowNode::new("fork", "并行审批", NodeKind::ParallelFork));
    g.add_node(
        FlowNode::task("fin", "财务审核", ToolKind::File, 200)
            .with_access(Access::read("file:receipt"))
            .with_access(Access::write("var:fin_ok")),
    );
    g.add_node(
        FlowNode::task("mgr", "主管审核", ToolKind::Compute, 150)
            .with_access(Access::read("var:emp"))
            .with_access(Access::write("var:mgr_ok")),
    );
    g.add_node(
        FlowNode::task("cmp", "合规审核", ToolKind::Compute, 300)
            .with_tag("security")
            .with_access(Access::read("var:emp"))
            .with_access(Access::write("var:cmp_ok")),
    );
    g.add_node(FlowNode::new("join", "汇审", NodeKind::ParallelJoin));
    g.add_node(FlowNode::new("guard", "合规校验", NodeKind::Guard));
    g.add_node(
        FlowNode::task("pay", "资金支付", ToolKind::File, 250)
            .with_access(Access::write("var:pay")),
    );
    g.add_node(FlowNode::new("ok", "完成", NodeKind::End));
    // 异常归档处理器（Guard 语义，满足 verify 对异常边目标必须 Handler/Guard 的约束）
    g.add_node(FlowNode::new("rej", "拒绝归档", NodeKind::Guard));

    g.add_edge(FlowEdge::seq("s", "fork"));
    g.add_edge(FlowEdge::seq("fork", "fin"));
    g.add_edge(FlowEdge::seq("fork", "mgr"));
    g.add_edge(FlowEdge::seq("fork", "cmp"));
    g.add_edge(FlowEdge::seq("fin", "join"));
    g.add_edge(FlowEdge::seq("mgr", "join"));
    g.add_edge(FlowEdge::seq("cmp", "join"));
    g.add_edge(FlowEdge::seq("join", "guard"));
    g.add_edge(FlowEdge::seq("guard", "pay"));
    g.add_edge(FlowEdge::seq("pay", "ok"));
    g.add_edge(FlowEdge::mutex("mgr", "pay"));
    g.add_edge(FlowEdge::exception("cmp", "rej"));
    g
}

/// B2：危险图——无界循环 + 越权写生产库（应被否决）
fn dangerous_unbounded_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b2", "危险自循环");
    g.add_node(FlowNode::new("s", "发起", NodeKind::Start));
    g.add_node(FlowNode::new("ls", "循环入口", NodeKind::LoopStart));
    g.add_node(
        FlowNode::task("scr", "抓取并改写", ToolKind::Database, 100)
            .with_access(Access::write("db:prod")),
    );
    g.add_node(FlowNode::new("le", "循环出口", NodeKind::LoopEnd));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "ls"));
    g.add_edge(FlowEdge::seq("ls", "scr"));
    g.add_edge(FlowEdge::seq("scr", "le"));
    g.add_edge(FlowEdge::seq("le", "ls"));
    g.add_edge(FlowEdge::seq("le", "e"));
    g
}

/// A1：有界循环批处理 + 决策分支 + 子流程复用
fn bounded_loop_graph() -> FlowGraph {
    let mut g = FlowGraph::new("a1", "批量重试处理");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("pull", "拉取批次", ToolKind::Database, 200)
            .with_access(Access::read("db:batch")),
    );
    g.add_node(FlowNode::new("ls", "循环入口", NodeKind::LoopStart));
    g.add_node(FlowNode::new("proc", "处理单条", NodeKind::SubFlow));
    g.add_node(
        FlowNode::task("dec", "成功?", ToolKind::Compute, 50)
            .with_access(Access::read("var:item"))
            .with_access(Access::write("var:result")),
    );
    g.add_node(FlowNode::new("le", "循环出口", NodeKind::LoopEnd));
    g.add_node(
        FlowNode::task("sum", "汇总", ToolKind::Compute, 120)
            .with_access(Access::write("var:summary")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));

    g.add_edge(FlowEdge::seq("s", "pull"));
    g.add_edge(FlowEdge::seq("pull", "ls"));
    g.add_edge(FlowEdge::seq("ls", "proc"));
    g.add_edge(FlowEdge::seq("proc", "dec"));
    g.add_edge(FlowEdge::cond("dec", "le", "success == true"));
    g.add_edge(FlowEdge::cond("dec", "ls", "success == false"));
    g.add_edge(FlowEdge::seq("le", "sum"));
    g.add_edge(FlowEdge::seq("sum", "e"));
    g
}

#[test]
fn b1_enterprise_parallel_emit_ok() {
    let g = enterprise_approval_graph();
    let ctx = base_ctx(vec!["editor", "approver"], true);
    let rep = programming_pipeline(
        "并行审批：财务/主管/合规三路并行，合规校验后支付，合规失败转拒绝",
        vec![
            "三路审核必须并行".into(),
            "支付与主管审核互斥（共用资金账户）".into(),
            "合规失败必须走异常归档".into(),
        ],
        true,
        &g,
        &ctx,
    );
    assert!(
        rep.safe_to_emit,
        "B1 应出码，got {:?} gate={:?} vetoed={:?}",
        rep.checkpoint,
        rep.governance.as_ref().map(|x| &x.gate.reason),
        rep.governance.as_ref().map(|x| x.algo.vetoed)
    );
    assert_eq!(rep.checkpoint, Checkpoint::Governed);
    let gov = rep.governance.unwrap();
    let opt = &gov.optimization.optimized_graph;
    let has_mutex = opt.edges.iter().any(|e| e.kind == EdgeKind::Mutex);
    let has_exc = opt.edges.iter().any(|e| e.kind == EdgeKind::Exception);
    assert!(has_mutex, "互斥 Mutex 边不应被优化剪除");
    assert!(has_exc, "异常 Exception 边不应被优化剪除");
}

#[test]
fn b2_dangerous_unbounded_should_be_blocked() {
    let g = dangerous_unbounded_graph();
    let mut ctx = base_ctx(vec!["editor", "approver"], true);
    ctx.registry.register_loop(LoopGuard {
        node: "ls".into(),
        policy: LoopPolicy::Unbounded,
    });
    let rep = programming_pipeline(
        "无界自循环抓取并改写生产库",
        vec!["循环执行".into()],
        true,
        &g,
        &ctx,
    );
    // 期望：循环护栏在治理前拦截（checkpoint=Modeled），禁止出码
    assert!(
        !rep.safe_to_emit,
        "危险无界循环不应出码，checkpoint={:?} governance={:?}",
        rep.checkpoint,
        rep.governance.is_some()
    );
    assert_eq!(rep.checkpoint, Checkpoint::Modeled);
    let blocked = rep.governance.map(|g| g.algo.vetoed).unwrap_or(true);
    assert!(blocked, "无界循环必须被循环护栏否决");
}

#[test]
fn a1_bounded_loop_emit_ok() {
    let g = bounded_loop_graph();
    let mut ctx = base_ctx(vec!["editor", "approver"], false);
    ctx.registry.register_loop(LoopGuard {
        node: "ls".into(),
        policy: LoopPolicy::Bounded { max_iter: 100 },
    });
    let rep = programming_pipeline(
        "有界循环批量重试：拉批次→逐条处理→成功出循环否则重试→汇总",
        vec![
            "循环必须有界(max_iter=100)".into(),
            "处理单条复用子流程模板".into(),
            "决策分支控制重试".into(),
        ],
        true,
        &g,
        &ctx,
    );
    assert!(
        rep.safe_to_emit,
        "A1 有界循环应出码，got {:?} gate={:?}",
        rep.checkpoint,
        rep.governance.as_ref().map(|x| &x.gate.reason)
    );
    assert_eq!(rep.checkpoint, Checkpoint::Governed);
    let gov = rep.governance.unwrap();
    let opt = &gov.optimization.optimized_graph;
    let has_loop = opt
        .nodes
        .iter()
        .any(|n| n.kind == NodeKind::LoopStart || n.kind == NodeKind::LoopEnd);
    assert!(has_loop, "循环节点不应被优化抹除");
}

#[test]
fn g_a_blocks_vague_enterprise_intent() {
    let g = enterprise_approval_graph();
    let ctx = base_ctx(vec!["editor"], true);
    let rep = programming_pipeline(
        "尽快把审批流程搞好，越自动越好",
        vec!["尽量快".into(), "差不多就行".into()],
        false,
        &g,
        &ctx,
    );
    assert!(!rep.safe_to_emit);
    assert_eq!(rep.checkpoint, Checkpoint::Normalized);
}

#[test]
fn debug_b1_veto_detail() {
    let g = enterprise_approval_graph();
    let ctx = base_ctx(vec!["editor", "approver"], true);
    let rep = programming_pipeline(
        "并行审批",
        vec!["并行".into(), "互斥".into(), "异常".into()],
        true,
        &g,
        &ctx,
    );
    if let Some(gov) = &rep.governance {
        eprintln!("[B1] vetoed={} reason={}", gov.algo.vetoed, gov.algo.summary);
        for c in &gov.algo.checks {
            eprintln!("[B1] check {} passed={} blocking={}", c.name, c.passed, c.blocking);
        }
        eprintln!("[B1] gate.approved={} reason={:?}", gov.gate.approved, gov.gate.reason);
        eprintln!("[B1] conflicts.len={}", gov.optimization.conflicts.conflicts.len());
        for c in &gov.optimization.conflicts.conflicts {
            eprintln!("[B1] conflict kind={:?} sev={:?} nodes={:?} msg={}", c.kind, c.severity, c.nodes, c.message);
        }
    } else {
        eprintln!("[B1] no governance, checkpoint={:?}", rep.checkpoint);
    }
}

/// B3：有界循环（合法）但越权写生产库（无 authz 标签）→ 应被生产写保护否决
fn prod_write_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b3", "越权写生产库");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("ls", "循环入口", NodeKind::LoopStart));
    g.add_node(
        FlowNode::task("w", "改写生产数据", ToolKind::Database, 100)
            .with_access(Access::write("db:prod")), // 越权写生产库，无 authz
    );
    g.add_node(FlowNode::new("le", "循环出口", NodeKind::LoopEnd));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "ls"));
    g.add_edge(FlowEdge::seq("ls", "w"));
    g.add_edge(FlowEdge::seq("w", "le"));
    g.add_edge(FlowEdge::seq("le", "ls"));
    g.add_edge(FlowEdge::seq("le", "e"));
    g
}

#[test]
fn b3_protected_write_should_be_blocked() {
    let g = prod_write_graph();
    let mut ctx = base_ctx(vec!["editor", "approver"], true);
    // 循环本身合法（有界），但越权写生产库无 authz
    ctx.registry.register_loop(LoopGuard {
        node: "ls".into(),
        policy: LoopPolicy::Bounded { max_iter: 10 },
    });
    let rep = programming_pipeline(
        "有界循环改写生产库数据",
        vec!["循环有界(max_iter=10)".into(), "写 db:prod".into()],
        true,
        &g,
        &ctx,
    );
    assert!(
        !rep.safe_to_emit,
        "越权写生产库不应出码，checkpoint={:?} vetoed={:?}",
        rep.checkpoint,
        rep.governance.as_ref().map(|x| x.algo.vetoed)
    );
    // 经 permission 专家 push_veto 正交触发 algo.vetoed（checkpoint=Optimized）
    assert_eq!(rep.checkpoint, Checkpoint::Optimized);
    let gov = rep.governance.expect("应有治理报告");
    assert!(gov.algo.vetoed, "越权写生产库必须被专家否决级风险否决");
}

/// B4：异常 → 普通 End 归档（迭代 4-① 放宽后应为合法业务语义，可出码）
/// 此前 verify 悬空异常边约束过严，强制要求 Exception 目标为 Guard/Handler；
/// 放宽后允许 Exception → End（异常归档终止为常见业务需求）。
fn exception_to_end_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b4", "异常归档到普通终点");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("chk", "合规审核", NodeKind::Task));
    g.add_node(FlowNode::task("pay", "支付", ToolKind::File, 80));
    g.add_node(FlowNode::new("ok", "完成", NodeKind::End));
    g.add_node(FlowNode::new("rej", "拒绝归档", NodeKind::End)); // 普通 End，异常归档
    g.add_edge(FlowEdge::seq("s", "chk"));
    g.add_edge(FlowEdge::seq("chk", "pay"));
    g.add_edge(FlowEdge::seq("pay", "ok"));
    g.add_edge(FlowEdge::exception("chk", "rej")); // 异常 → 普通 End
    g
}

#[test]
fn b4_exception_to_end_should_emit_ok() {
    let g = exception_to_end_graph();
    let ctx = base_ctx(vec!["editor", "approver"], false);
    let rep = programming_pipeline(
        "合规审核异常时归档拒绝",
        vec!["异常归档到普通终点".into()],
        true,
        &g,
        &ctx,
    );
    assert!(
        rep.safe_to_emit,
        "异常→普通End归档应合法出码，checkpoint={:?} vetoed={:?}",
        rep.checkpoint,
        rep.governance.as_ref().map(|x| x.algo.vetoed)
    );
    assert_eq!(rep.checkpoint, Checkpoint::Governed);
}

/// B5：子流程(SubFlow)节点级双向映射验证（迭代 5-①）
/// 图含 SubFlow 节点，正向生成应在 tasks.py 中为其生成 `def <py_ident>(ctx:` 函数；
/// 节点级校验器应判定双向映射一致，允许出码。
fn subflow_graph() -> FlowGraph {
    let mut g = FlowGraph::new("b5", "子流程出码");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::task("t1", "采集", ToolKind::File, 50));
    g.add_node(FlowNode::new("sub", "批量子流程", NodeKind::SubFlow));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "t1"));
    g.add_edge(FlowEdge::seq("t1", "sub"));
    g.add_edge(FlowEdge::seq("sub", "e"));
    g
}

#[test]
fn b5_subflow_node_level_roundtrip_ok() {
    let g = subflow_graph();
    let ctx = base_ctx(vec!["editor", "approver"], false);
    let rep = programming_pipeline(
        "含子流程的业务流",
        vec!["子流程复用".into()],
        true,
        &g,
        &ctx,
    );
    assert!(
        rep.safe_to_emit,
        "子流程图应合法出码，checkpoint={:?} vetoed={:?}",
        rep.checkpoint,
        rep.governance.as_ref().map(|x| x.algo.vetoed)
    );
    assert_eq!(rep.checkpoint, Checkpoint::Governed);
    // 节点级双向映射校验须通过（SubFlow 节点映射到生成函数）
    assert_eq!(rep.roundtrip_ok, Some(true));
}

/// B6：有界循环 max_iter 落实到生成代码（迭代 6）
/// 图登记 LoopGuard(Bounded{max_iter=100})，生成的 scheduler.py 须含 `range(100)` 迭代上限护栏。
#[test]
fn b6_bounded_loop_max_iter_in_code() {
    let g = bounded_loop_graph();
    let mut ctx = base_ctx(vec!["editor", "approver"], false);
    ctx.registry.register_loop(LoopGuard {
        node: "ls".into(),
        policy: LoopPolicy::Bounded { max_iter: 100 },
    });
    let rep = programming_pipeline(
        "有界循环批量重试：拉批次→逐条处理→成功出循环否则重试→汇总",
        vec!["循环最多 100 次".into()],
        true,
        &g,
        &ctx,
    );
    assert!(rep.safe_to_emit, "有界循环应出码，checkpoint={:?}", rep.checkpoint);
    let code = rep.code.expect("应有生成代码");
    let sched = code.file("generated/scheduler.py").expect("应有 scheduler.py");
    assert!(
        sched.content.contains("range(100)"),
        "生成的调度层须含 range(100) 迭代上限护栏，实际:\n{}",
        sched.content
    );
}
