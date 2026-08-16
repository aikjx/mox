//! 缺口 P1.1 —— Multi-expert 交互测试（Permission + Security 双专家同时触发 veto）
//!
//! 目的：验证当同一份流程图同时违反「权限专家的生产/敏感数据越权写」与
//! 「安全专家的强合规租户 PII 外发」时，两位专家**独立**给出各自最强反对意见，
//! 且联盟编排层（alliance_optimize）把 Permission 的 `push_veto` 并入算法验证否决，
//! 整体闸门 BLOCK。
//!
//! 正交性子用例：
//! 仅权限越权写 → Permission 触发 veto，整体被否决；
//! 仅安全 PII 外发（regulated 租户）→ Security 给出 Blocking 级风险，但**不**触发硬 veto
//! （说明当前 Security 的阻断级风险走的是「记录」语义，只有 push_veto 才是硬否决入口——
//! 这正是多专家正交机制的设计意图：任何专家想强制否决，只需 push_veto）。

use expert_alliance::context::{GovernContext, Principal, Tenant};
use expert_alliance::expert::dispatch;
use expert_alliance::experts::all_experts;
use expert_alliance::govern::FlowStatus;
use expert_alliance::pipeline::alliance_optimize;
use flow_ai::model::{Access, AccessMode, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

/// 有 edit-flow 权限的主体（专家才会真正分析，否则直接 skip）
fn editor_ctx(regulated: bool) -> GovernContext {
    let t = Tenant::new("acme", "fin").regulated(regulated);
    let p = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
    GovernContext::new(t, p)
}

/// 同时触发两套反对意见的图：
///  - `wdb`：Database 工具 + Write `db:prod`（无 authz 标签）→ Permission.push_veto
///  - `http_pii`：Http 工具 + 访问含 "pii" 的资源，regulated 租户 → Security Blocking 风险
fn dual_violation_graph() -> FlowGraph {
    let mut g = FlowGraph::new("dual", "越权写生产库 + PII外发");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    // 权限专家否决级：越权写生产库（无 authz）
    g.add_node(
        FlowNode::task("wdb", "改写生产数据", ToolKind::Database, 100)
            .with_access(Access::write("db:prod")),
    );
    // 安全专家阻断级：regulated 租户下经 HTTP 外发 PII
    g.add_node(
        FlowNode::task("http_pii", "外发客户PII", ToolKind::Http, 100)
            .with_access(Access::read("pii:customer_profile")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "wdb"));
    g.add_edge(FlowEdge::seq("wdb", "http_pii"));
    g.add_edge(FlowEdge::seq("http_pii", "e"));
    g
}

/// 仅权限越权写（无 PII 外发）
fn permission_only_graph() -> FlowGraph {
    let mut g = FlowGraph::new("perm", "越权写生产库");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("wdb", "改写生产数据", ToolKind::Database, 100)
            .with_access(Access::write("db:prod")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "wdb"));
    g.add_edge(FlowEdge::seq("wdb", "e"));
    g
}

/// 仅安全 PII 外发（无生产库越权写），regulated 租户
fn security_only_graph() -> FlowGraph {
    let mut g = FlowGraph::new("sec", "PII外发");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("http_pii", "外发客户PII", ToolKind::Http, 100)
            .with_access(Access::read("pii:customer_profile")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "http_pii"));
    g.add_edge(FlowEdge::seq("http_pii", "e"));
    g
}

#[test]
fn dual_violation_both_experts_object_and_flow_blocked() {
    let g = dual_violation_graph();
    let ctx = editor_ctx(true);

    // 1) 两位专家独立分析：各自给出最强反对意见
    let ectx = expert_alliance::context::ExpertContext::new(&g, &ctx);
    let opinions = dispatch(&ectx, &all_experts());

    let perm = opinions.iter().find(|o| o.expert == "permission").expect("应有 permission 专家");
    let sec = opinions.iter().find(|o| o.expert == "security").expect("应有 security 专家");

    // Permission 必须触发否决级风险（veto=true），针对越权写生产库
    assert!(
        perm.risks.iter().any(|r| r.veto && r.severity == flow_ai::model::Severity::Blocking),
        "Permission 专家应对越权写生产库触发 push_veto，实际 risks={:?}",
        perm.risks
    );
    // Security 必须给出 Blocking 级风险（regulated 租户 PII 外发）
    assert!(
        sec.risks.iter().any(|r| r.severity == flow_ai::model::Severity::Blocking),
        "Security 专家应对 regulated 租户 PII 外发给出 Blocking 风险，实际 risks={:?}",
        sec.risks
    );

    // 2) 联盟编排层：Permission 的 veto 并入算法否决 → 整体 BLOCK
    let rep = alliance_optimize(&g, &ctx);
    assert!(
        rep.algo.vetoed,
        "双违规图必须被算法验证否决，checks={:?}",
        rep.algo.checks
    );
    assert!(rep.gate.algorithm_veto, "治理闸门应记录 algorithm_veto");
    assert_eq!(rep.gate.status, FlowStatus::Blocked, "状态应为 Blocked");
    assert!(!rep.gate.approved, "不应通过治理闸门");
    // 审计链已记录一次 blocked 事件
    assert_eq!(rep.audit.events.len(), 1);
    assert_eq!(rep.audit.events[0].decision, "blocked");
}

#[test]
fn permission_only_veto_blocks_even_without_security() {
    let g = permission_only_graph();
    let ctx = editor_ctx(true);

    let ectx = expert_alliance::context::ExpertContext::new(&g, &ctx);
    let opinions = dispatch(&ectx, &all_experts());
    let perm = opinions.iter().find(|o| o.expert == "permission").unwrap();
    let sec = opinions.iter().find(|o| o.expert == "security").unwrap();

    assert!(perm.risks.iter().any(|r| r.veto), "Permission 应触发 veto");
    // 没有 PII 外发 → Security 不应产生 Blocking 风险
    assert!(
        !sec.risks.iter().any(|r| r.severity == flow_ai::model::Severity::Blocking),
        "无 PII 外发时 Security 不应给 Blocking"
    );

    let rep = alliance_optimize(&g, &ctx);
    assert!(rep.algo.vetoed, "仅权限越权写也应被否决");
    assert_eq!(rep.gate.status, FlowStatus::Blocked);
}

#[test]
fn security_only_blocking_is_recorded_but_not_hard_veto() {
    let g = security_only_graph();
    let ctx = editor_ctx(true);

    let ectx = expert_alliance::context::ExpertContext::new(&g, &ctx);
    let opinions = dispatch(&ectx, &all_experts());
    let perm = opinions.iter().find(|o| o.expert == "permission").unwrap();
    let sec = opinions.iter().find(|o| o.expert == "security").unwrap();

    // Security 给出 Blocking 级风险
    assert!(
        sec.risks.iter().any(|r| r.severity == flow_ai::model::Severity::Blocking),
        "Security 应对 PII 外发给 Blocking 风险"
    );
    // 但 Security 用的是 push_risk（veto=false），不构成硬否决入口
    assert!(
        !sec.risks.iter().any(|r| r.veto),
        "Security 阻断级风险不应是 veto（只有 push_veto 才是硬否决入口）"
    );
    // 没有生产/敏感库越权写 → Permission 不触发 veto
    assert!(!perm.risks.iter().any(|r| r.veto), "无越权写时 Permission 不应 veto");

    let rep = alliance_optimize(&g, &ctx);
    // 当前语义：仅 Security 阻断级风险（非 veto）不会触发算法硬否决
    assert!(
        !rep.algo.vetoed,
        "仅 Security 阻断级风险（非 push_veto）当前不触发硬否决——记录但需人工审批/补 Guard"
    );
    // 但 Security 的 Blocking 风险必须被记录在专家意见里，供治理/审计可见
    let sec_scores = rep.expert_scores.iter().find(|(e, _)| e == "security");
    assert!(sec_scores.is_some(), "Security 专家评分应被记录");
}

#[test]
fn both_experts_skipped_without_edit_flow_capability() {
    // 无 edit-flow 权限的主体 → 两专家都 skip，不应产生任何 veto/blocking
    let t = Tenant::new("acme", "fin").regulated(true);
    let p = Principal::new("viewer"); // 默认角色 viewer，无 editor
    let ctx = GovernContext::new(t, p);
    let g = dual_violation_graph();

    let ectx = expert_alliance::context::ExpertContext::new(&g, &ctx);
    let opinions = dispatch(&ectx, &all_experts());
    let perm = opinions.iter().find(|o| o.expert == "permission").unwrap();
    let sec = opinions.iter().find(|o| o.expert == "security").unwrap();
    assert!(perm.skipped, "无 edit-flow 权限时 Permission 应 skip");
    assert!(sec.skipped, "无 edit-flow 权限时 Security 应 skip");
    assert!(!perm.risks.iter().any(|r| r.veto));
    assert!(!sec.risks.iter().any(|r| r.severity == flow_ai::model::Severity::Blocking));
}

/// 收益性检查：AccessMode::Write 在双违规图中确实存在，确保构造正确
#[test]
fn graph_constructs_expected_access_modes() {
    let g = dual_violation_graph();
    let wdb = g.node("wdb").expect("wdb 节点存在");
    assert!(wdb
        .accesses
        .iter()
        .any(|a| a.mode == AccessMode::Write && a.resource == "db:prod"));
    let http = g.node("http_pii").expect("http_pii 节点存在");
    assert!(http
        .accesses
        .iter()
        .any(|a| a.resource.contains("pii")));
}
