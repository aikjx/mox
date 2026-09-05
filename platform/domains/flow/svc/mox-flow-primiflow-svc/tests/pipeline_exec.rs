// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 端到端验证：需求 → κτ 涌现 → **真实执行算子** → 注荷 → 落盘。
//! 证明 `run_pipeline` 不再模拟成功，而是真跑每条子任务并把质量回灌引擎。

use mox_ai_flow_sdk::model::ToolKind;
use mox_ai_flow_sdk::primitive::{DeliveryPolicy, KnowledgeBase, PrimiEngine};
use mox_flow_primiflow_svc::assoc::AssocGraph;
use mox_flow_primiflow_svc::runner::{enterprise_specs, run_all, run_pipeline, Spec};

fn fresh_engine() -> PrimiEngine {
    PrimiEngine::new(
        1.0,
        KnowledgeBase::new(),
        mox_ai_flow_sdk::primitive::ResourceBudget::default(),
    )
}

#[test]
fn pipeline_executes_real_operators_and_charges_q() {
    let mut engine = fresh_engine();
    let mut master = AssocGraph::new();

    let spec = Spec::new("r1", "电商月度经营分析", DeliveryPolicy::Balanced)
        .sub("fetch", "抓取销售数据", ToolKind::Http, 300)
        .sub("clean", "清洗对账", ToolKind::Compute, 200)
        .sub("report", "生成图表报告", ToolKind::Llm, 400);
    let req = spec.requirement();
    let out = std::env::temp_dir().join("primiflow_test_exec");

    let rep = run_pipeline(&mut engine, &req, spec.policy, &mut master, &out).unwrap();

    assert!(!rep.execution.is_empty(), "应记录真实执行");
    assert_eq!(rep.execution.len(), 3, "应有 3 条算子记录");
    assert!(rep.execution.iter().all(|r| r.ok), "所有算子应真实执行成功");
    // 执行质量应来自真实执行（exec_q≈0.95 回灌），使 Q 真实上升——而非硬编码 0.9
    assert!(
        rep.q_after > rep.q_before,
        "注荷后 Q 应上升（实得 q_after={:.2}）",
        rep.q_after
    );
    assert!(rep.all_ok(), "全部分步验证应通过");

    // 真实执行记录应落盘为审计产物
    assert!(out.join("exec_r1.json").exists(), "exec_r1.json 应落盘");
}

#[test]
fn run_all_persists_exec_records_per_requirement() {
    let mut engine = fresh_engine();
    let specs = enterprise_specs();
    let out = std::env::temp_dir().join("primiflow_test_exec_all");

    let reps = run_all(&mut engine, &specs, &out).unwrap();
    assert_eq!(reps.len(), specs.len());

    for r in &reps {
        assert!(!r.execution.is_empty(), "每个需求都应真实执行算子");
        assert!(
            r.execution.iter().all(|e| e.ok),
            "{} 的算子应全部执行成功",
            r.requirement
        );
    }

    // 每个需求的执行记录都落盘
    for s in &specs {
        assert!(
            out.join(format!("exec_{}.json", s.id)).exists(),
            "exec_{}.json 应落盘",
            s.id
        );
    }
}
