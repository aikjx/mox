// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! bridge-demo —— 模拟「真实 Hermes 多轮会话」走完整 bridge 闭环并出报告。
//!
//! 闭环：Hermes 工具调用 → bridge 录制 FlowGraph → 后台 ExpertConsultant trait 调用（璇玑引擎）
//!       → 算法验证否决位 → 否决时 ToolExecutionMiddleware 强制拦截。
//!
//! DIP 版：不再直接引用 mox_ai_expert_svc::pipeline / context / GovernanceReport 等 concrete 结构。
//! 所有引擎调用统一通过 `ExpertConsultant` trait 抽象，展示 `ConsultReport`（投影类型）。
//!
//! 运行：cargo run -p hermes-flow-bridge --bin bridge-demo

use mox_flow_bridge_svc::bridge::{optimize_session, optimize_session_with};
use mox_flow_bridge_svc::hooks::{on_tool_execution, on_tool_request};
use mox_flow_bridge_svc::router::FlowTemplate;
use mox_flow_bridge_svc::state::BridgeState;
use serde_json::json;
use std::collections::HashMap;
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport};

fn main() {
    println!("=== hermes-flow-bridge 闭环演示（零侵入插件注入 · DIP 版）===\n");

    let st = BridgeState::new();

    // 注册一张「政务 PII 归集」复用模板（来自 mox-expert 关系网最短路径挖掘）
    st.router.register(FlowTemplate {
        id: "gov-pii".into(),
        tool_seq: vec![
            "db.read".into(),
            "guard.desensitize".into(),
            "web1.submit".into(),
            "merge.report".into(),
        ],
    });

    // ---- 阶段 1：Hermes 真实多轮工具调用（同步中间件投影）----
    println!("[1] Hermes 会话 s_demo 多轮工具调用录制：");
    on_tool_request(
        &st,
        "s_demo",
        "db.read",
        &json!({"query":"select * from citizen_info"}),
        1,
    );
    on_tool_request(
        &st,
        "s_demo",
        "guard.desensitize",
        &json!({"var":"citizen"}),
        1,
    );
    on_tool_request(&st, "s_demo", "web1.submit", &json!({}), 2);
    let d = on_tool_request(&st, "s_demo", "merge.report", &json!({}), 2);
    let g = st.recorder.snapshot("s_demo").expect("会话图存在");
    println!("    录制节点数 = {}", g.nodes.len());
    if let Some(d) = d {
        println!(
            "    复用路由命中：{} —— {}",
            d.source.unwrap_or_default(),
            d.reason.unwrap_or_default()
        );
    }

    // ---- 阶段 2：后台把会话图推给璇玑引擎（通过 ExpertConsultant trait）----
    println!("\n[2] 后台咨询（通过 ExpertConsultant trait，不出现 concrete struct）：");
    optimize_session(&g, &st.gate);
    println!("    算法否决 = {}", st.gate.is_vetoed());

    // ---- 阶段 3：通过 trait 调 ExpertConsultant.consult_blocking 获取投影报告 ConsultReport ----
    println!("\n[3] 投影 ConsultReport（DIP 归一化类型）：");
    let consultant = st.consultant.clone();
    let mut ctx = HashMap::new();
    ctx.insert(
        "flow_json".into(),
        serde_json::to_string(&g).unwrap_or_default(),
    );
    ctx.insert("tenant".into(), "hermes".into());
    ctx.insert("namespace".into(), "default".into());
    ctx.insert("principal".into(), "hermes-agent".into());
    ctx.insert("max_parallel".into(), "8".into());
    ctx.insert("max_cost_budget".into(), "100".into());
    ctx.insert("sla_ms".into(), "50000".into());
    let q = ConsultQuery {
        id: "bridge-demo".into(),
        query: String::new(),
        ctx,
    };
    let rep: ConsultReport = consultant
        .consult_blocking(&q)
        .expect("ExpertConsultant.consult_blocking 不应失败");
    println!("    report_id   = {}", rep.report_id);
    println!("    综合健康分  = {:.2}", rep.score);
    println!(
        "    治理闸门状态 = {}",
        if rep.vetoed {
            "⛨ Blocked / 否决"
        } else {
            "✅ Approved"
        }
    );
    if let Some(r) = &rep.reason {
        println!("    原因        : {}", r);
    }
    println!("    执行步骤（归一化文本）：");
    for (i, step) in rep.steps.iter().enumerate() {
        println!("      [{}/{}] {}", i + 1, rep.steps.len(), step);
    }
    // 直接调 optimize_session_with（演示传入自定义 consultant 能力）
    let rep2 = optimize_session_with(&g, &st.gate, consultant.clone());
    assert_eq!(
        rep2.vetoed,
        st.gate.is_vetoed(),
        "veto 位必须与 gate 保持一致"
    );

    // ---- 阶段 4：算法否决拦截接线（演示否决位如何阻断工具执行）----
    println!("\n[4] ToolExecutionMiddleware 拦截接线：");
    let before = on_tool_execution(&st);
    println!("    当前否决位下执行工具 → blocked = {}", before.blocked);
    st.set_vetoed(true);
    let after = on_tool_execution(&st);
    println!(
        "    强制置位否决后执行工具 → blocked = {} ({})",
        after.blocked,
        after.reason.unwrap_or_default()
    );

    // ---- 阶段 5：LLM 调用次数对比（用户原方案核心收益：调用减半）----
    println!("\n[5] LLM 调用次数对比（与 linear ReAct baseline）：");
    let plan = mox_flow_bridge_svc::mini_hermes::gov_pii_plan();
    // baseline：每步一次 LLM
    let b_tracer = mox_flow_bridge_svc::mini_hermes::LlmTracer::new();
    mox_flow_bridge_svc::mini_hermes::run_baseline(&plan, &b_tracer);
    let baseline_calls = b_tracer.count();
    // bridge：复用模板整段回放 → 0 次 LLM
    let b_st = BridgeState::new();
    mox_flow_bridge_svc::mini_hermes::register_gov_template(&b_st);
    let br_tracer = mox_flow_bridge_svc::mini_hermes::LlmTracer::new();
    let br_out = mox_flow_bridge_svc::mini_hermes::run_bridge(&b_st, &plan, &br_tracer);
    let bridge_calls = br_tracer.count();
    let saved = if baseline_calls > 0 {
        (1.0 - bridge_calls as f64 / baseline_calls as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "    baseline (linear ReAct) : {} 次 LLM 调用",
        baseline_calls
    );
    println!("    bridge   (复用回放)    : {} 次 LLM 调用", bridge_calls);
    println!("    削减比例              : {:.1}%", saved);
    println!(
        "    回放动作样本          : {:?}",
        &br_out[..3.min(br_out.len())]
    );

    println!("\n=== 闭环验证完成：合法图通过算法网关且复用路由命中；否决位可强制拦截；LLM 调用显著削减 ===");
}
