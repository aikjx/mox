// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 璇玑全维融合端到端集成测试（I-05 / I-06 / I-07 闭环验收）
//!
//! 不依赖服务器进程，纯库级串联：
//!   1. 构造业务流程图（受监管租户的真实场景）
//!   2. 走全维治理流水线 `mox_optimize` —— 触发治理 8 闸门（I-06）
//!   3. 双验收联动（I-05）：任务 Done ∧ 算法未否决 ∧ 闸门放行
//!   4. 全维融合落盘 `publish_unified` —— 固化产物溯源（I-07 provenance）
//!
//! 这是企业级「优化 → 治理 → 发布溯源」链路的最高级别硬验收证据。

use mox_ai_flow_svc::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind};
use mox_platform_orchestrator_svc::market::{publish_unified, OperatorPackage};
use mox_ai_expert_svc::context::{GovernContext, Principal, Tenant};
use mox_ai_expert_svc::pipeline::mox_optimize;

/// 构造一张合规流程图：读取脱敏后的敏感数据 → 聚合 → 回写结果库（带 desensitize/rollback Guard）。
fn build_compliant_graph() -> FlowGraph {
    let mut graph = FlowGraph::new("fg-e2e-001", "E2E 合规融合流程");

    let mut read = FlowNode::new("read", "读取敏感数据(脱敏)", NodeKind::Task);
    read.accesses.push(Access::read("db.citizen_profile"));
    read.tags.push("desensitize".into());

    let agg = FlowNode::new("agg", "聚合分析", NodeKind::Task);

    let mut write = FlowNode::new("write", "回写结果库", NodeKind::Task);
    write.accesses.push(Access::write("db.result_agg"));
    write.tags.push("rollback".into());
    write.tags.push("backup".into());

    graph.add_node(read);
    graph.add_node(agg);
    graph.add_node(write);
    graph.add_edge(FlowEdge::seq("read", "agg"));
    graph.add_edge(FlowEdge::seq("agg", "write"));
    graph
}

/// 受监管租户（regulated → require_dr / force_desensitize_guard 严格度提升）。
fn build_regulated_ctx() -> GovernContext {
    let tenant = Tenant::new("acme-reg", "acme").regulated(true);
    let principal = Principal::new("alice@acme").with_roles(vec!["editor".into()]);
    GovernContext::new(tenant, principal)
}

#[test]
fn e2e_governance_eight_gates_pipeline_publish_provenance() {
    let graph = build_compliant_graph();
    let ctx = build_regulated_ctx();

    // 1. 全维治理流水线：归一化 → 派发专家 → 裁决 → 求解 → 治理 8 闸门 → 出码
    let report = mox_optimize(&graph, &ctx);

    // 2. I-06 治理 8 闸门全量门禁已生效
    assert_eq!(report.gate.gates.len(), 8, "治理 8 闸门必须全量门禁");
    let failed: Vec<_> = report.gate.gates.iter().filter(|g| !g.passed).collect();
    assert!(
        failed.is_empty(),
        "合规流程应 8 闸全过，未过: {:?}",
        failed.iter().map(|g| g.id.code()).collect::<Vec<_>>()
    );

    // 3. I-05 双验收联动：算法未否决 + 闸门放行
    assert!(!report.algo.vetoed, "合规流程算法不应否决");
    assert!(report.gate.approved, "8 闸全过应批准出码");
    let dual_ok = !report.algo.vetoed && report.gate.approved;
    assert!(dual_ok, "I-05 双验收应达成");

    // 4. I-07 全维融合落盘：把治理报告固化为产物溯源
    let pkg: OperatorPackage = publish_unified(
        "公民数据聚合算子".into(),
        "端到端验证产物".into(),
        "聚合脱敏后公民数据并回写结果库".into(),
        report.optimization.optimized_graph.nodes.clone(),
        report.optimization.optimized_graph.edges.clone(),
        vec!["fusion".into(), "compliance".into()],
        Some(&report),
        Some("task-e2e-001".into()),
    )
    .expect("publish_unified 落盘应成功");

    // 5. 溯源证据齐备且自洽
    let prov = pkg.provenance.expect("产物必须携带溯源 provenance");
    assert!(prov.algo_verified, "溯源应标记算法验证通过");
    assert!(prov.gates_passed, "溯源应标记 8 闸全过");
    assert!(prov.critical_path_before >= 1, "优化前关键路径必须有效");
    assert!(prov.critical_path_after >= 1, "优化后关键路径必须有效");
    assert!(prov.speedup > 0.0, "加速比必须为正");
    assert!(
        prov.expert_score > 0.0 && prov.expert_score <= 100.0,
        "专家健康分应在 (0,100]"
    );
    assert_eq!(
        pkg.source_flow_id,
        Some(report.flow_id.clone()),
        "产物应回指来源 flow_id"
    );
    assert_eq!(
        pkg.source_task_id,
        Some("task-e2e-001".into()),
        "产物应回指双璇玑任务 ID"
    );
    assert!(pkg.dual_acceptance, "产物应标记双验收达成");
}

#[test]
fn e2e_regulated_tenant_blocks_raw_sensitive_write() {
    // 反例：受监管租户直接写敏感库、无脱敏 Guard → G3/G6 应拦截，产物溯源标记未通过
    let mut graph = FlowGraph::new("fg-bad", "违规流程");
    let mut n = FlowNode::new("n1", "敏感写库", NodeKind::Task);
    n.accesses.push(Access::write("db:secret"));
    graph.add_node(n);

    let ctx = build_regulated_ctx();
    let report = mox_optimize(&graph, &ctx);

    assert!(!report.gate.approved, "强合规租户原始敏感写必须被闸门拦截");
    assert!(
        report.gate.gates.iter().any(|g| !g.passed),
        "应存在未通过闸门"
    );

    let pkg = publish_unified(
        "违规算子".into(),
        "反例".into(),
        "不应上架".into(),
        report.optimization.optimized_graph.nodes.clone(),
        report.optimization.optimized_graph.edges.clone(),
        vec![],
        Some(&report),
        Some("task-bad-001".into()), // source_task_id
    )
    .expect("即便被拦截也应能落盘（溯源标记失败）");

    let prov = pkg.provenance.expect("产物仍应携带溯源");
    assert!(
        !prov.gates_passed,
        "被拦截流程的溯源应标记 gates_passed=false"
    );
    assert!(!pkg.dual_acceptance, "双验收未达成则不应标记通过");
}
