//! 端到端集成测试：政务场景 + MCP/Skills/Loops/LLM 兼容性

use flow_ai::model::FlowGraph;
use flow_ai::model::{Access, EdgeKind, FlowEdge, FlowNode, NodeKind, ToolKind};
use xuanji_expert::context::{GovernContext, LoopGuard, LoopPolicy, McpTool, Principal, Tenant};
use xuanji_expert::govern::AuditChain;
use xuanji_expert::ir::auto_dimension;
use xuanji_expert::pipeline::xuanji_optimize;
use xuanji_expert::reconcile::reconcile;

fn gov_flow() -> FlowGraph {
    let mut g = FlowGraph::new("gov", "政务数据归集");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("read", "读取公民库", ToolKind::Database, 300)
            .with_access(Access::read("db:citizen_info"))
            .with_access(Access::write("var:citizen")),
    );
    g.add_node(FlowNode::task("guard", "脱敏", ToolKind::Compute, 50).with_tag("desensitize"));
    g.add_node(FlowNode::task(
        "web1",
        "网办系统A填报",
        ToolKind::Browser,
        500,
    ));
    g.add_node(FlowNode::task(
        "web2",
        "网办系统B填报",
        ToolKind::Browser,
        400,
    ));
    g.add_node(
        FlowNode::task("merge", "汇总", ToolKind::Compute, 100)
            .with_access(Access::read("var:citizen")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "read"));
    g.add_edge(FlowEdge::seq("read", "guard"));
    g.add_edge(FlowEdge::seq("guard", "web1"));
    g.add_edge(FlowEdge::seq("guard", "web2"));
    g.add_edge(FlowEdge::seq("web1", "merge"));
    g.add_edge(FlowEdge::seq("web2", "merge"));
    g.add_edge(FlowEdge::seq("merge", "e"));
    g.pools.push(flow_ai::model::ResourcePool {
        name: "browser".into(),
        capacity: 1,
    });
    g
}

#[test]
fn xuanji_optimize_passes_and_audit_clean() {
    let g = gov_flow();
    let tenant = Tenant::new("gov-tenant", "ns-gov")
        .regulated(true)
        .with_pool("browser", 1);
    let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
    let mut ctx = GovernContext::new(tenant, principal);
    // 兼容性注册：MCP 工具、Skill、Loop
    ctx.registry.register_mcp(
        "fs",
        vec![McpTool {
            server: "fs".into(),
            name: "write".into(),
            input_schema: "{}".into(),
            pool: "mcp_fs".into(),
        }],
    );
    ctx.registry
        .register_skill(xuanji_expert::context::SkillRef {
            id: "rpa-citizen".into(),
            keywords: vec!["政务".into(), "公民".into()],
            flow_template: None,
        });
    ctx.registry.register_loop(LoopGuard {
        node: "web1".into(),
        policy: LoopPolicy::Bounded { max_iter: 3 },
    });

    let rep = xuanji_optimize(&g, &ctx);

    // 治理闸门：干净政务流程应批准
    assert!(rep.gate.approved, "gate reason: {}", rep.gate.reason);
    // 审计链完整
    assert!(rep.audit.verify(), "审计链被篡改");
    // 双璇玑十四维全维评分（业务七维 + 开发七维）
    assert!(
        rep.expert_scores.len() >= 14,
        "双璇玑应覆盖十四维，实际 {}",
        rep.expert_scores.len()
    );
    // 优化生效：并行层 > 1 或剪除伪依赖
    assert!(rep.optimization.gains.parallel_layers >= 1);
}

#[test]
fn missing_desensitize_blocked_by_gate() {
    // 读取敏感库但无脱敏 Guard → 权限专家报 Blocking → 闸门拦截
    let mut g = FlowGraph::new("leak", "泄露场景");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("read", "读公民库", ToolKind::Database, 100)
            .with_access(Access::read("db:citizen_info")),
    );
    g.add_node(
        FlowNode::task("send", "外发", ToolKind::Http, 100)
            .with_access(Access::read("db:citizen_info")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "read"));
    g.add_edge(FlowEdge::seq("read", "send"));
    g.add_edge(FlowEdge::seq("send", "e"));

    let tenant = Tenant::new("gov", "ns").regulated(true);
    let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
    let ctx = GovernContext::new(tenant, principal);
    let rep = xuanji_optimize(&g, &ctx);

    // 权限专家应注入脱敏 Guard，使图变为干净（Blocking 在注入后消除）
    // 但若未注入（如缺 edit 权限），则闸门拦截
    if !rep
        .optimization
        .optimized_graph
        .nodes
        .iter()
        .any(|n| n.kind == NodeKind::Guard)
    {
        assert!(!rep.gate.approved);
    } else {
        assert!(rep.gate.approved);
    }
}

#[test]
fn rbac_denied_experts_skipped_not_panicked() {
    let g = gov_flow();
    let tenant = Tenant::new("t", "ns")
        .regulated(true)
        .with_pool("browser", 1);
    // 仅 viewer，无 edit 权限 → 业务/权限/安全/数据专家跳过但不 panic
    let principal = Principal::new("viewer");
    let ctx = GovernContext::new(tenant, principal);
    let rep = xuanji_optimize(&g, &ctx);
    // 跳过专家不 panic，且至少业务七维评分仍存在（双璇玑下总计 >= 7）
    assert!(
        rep.expert_scores.len() >= 7,
        "被跳过专家场景下仍应有评分，实际 {}",
        rep.expert_scores.len()
    );
}

#[test]
fn browser_mutex_injected_from_resource_conflict() {
    let g = gov_flow(); // 两个浏览器节点 + browser 容量 1
    let df = auto_dimension(&g);
    let experts = xuanji_expert::experts::all_experts();
    let tenant = Tenant::new("t", "ns").with_pool("browser", 1);
    let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
    let ctx = GovernContext::new(tenant, principal);
    let ectx = xuanji_expert::context::ExpertContext::new(&df.base, &ctx);
    let opinions = xuanji_expert::expert::dispatch(&ectx, &experts);
    let plan = reconcile(&opinions, &df.base, &g.pools);
    // 资源专家检测到浏览器超额 → 注入 Mutex 硬边
    assert!(
        plan.graph.edges.iter().any(|e| e.kind == EdgeKind::Mutex),
        "浏览器互斥边应被注入"
    );
}

#[test]
fn audit_chain_tamper_proof() {
    let mut c = AuditChain::new();
    c.append("u", "f", "edit", "ok");
    c.append("u", "f", "approve", "ok");
    assert!(c.verify());
    c.events[0].action = "hacked".into();
    assert!(!c.verify());
}
