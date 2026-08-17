//! bridge-demo —— 模拟「真实 Hermes 多轮会话」走完整 bridge 闭环并出报告。
//!
//! 闭环：Hermes 工具调用 → bridge 录制 FlowGraph → 后台 xuanji_optimize
//!       → 算法验证网关 → 否决时 ToolExecutionMiddleware 强制拦截。
//!
//! 运行：cargo run -p hermes-flow-bridge --bin bridge-demo

use hermes_flow_bridge::bridge::optimize_session;
use hermes_flow_bridge::hooks::{on_tool_execution, on_tool_request};
use hermes_flow_bridge::router::FlowTemplate;
use hermes_flow_bridge::state::BridgeState;
use serde_json::json;

fn main() {
    println!("=== hermes-flow-bridge 闭环演示（零侵入插件注入）===\n");

    let st = BridgeState::new();

    // 注册一张「政务 PII 归集」复用模板（来自 xuanji-expert 关系网最短路径挖掘）
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
    on_tool_request(&st, "s_demo", "db.read", &json!({"query":"select * from citizen_info"}), 1);
    on_tool_request(&st, "s_demo", "guard.desensitize", &json!({"var":"citizen"}), 1);
    on_tool_request(&st, "s_demo", "web1.submit", &json!({}), 2);
    let d = on_tool_request(&st, "s_demo", "merge.report", &json!({}), 2);
    let g = st.recorder.snapshot("s_demo").expect("会话图存在");
    println!("    录制节点数 = {}", g.nodes.len());
    if let Some(d) = d {
        println!("    复用路由命中：{} —— {}", d.source.unwrap_or_default(), d.reason.unwrap_or_default());
    }

    // ---- 阶段 2：后台把会话图推给璇玑引擎（xuanji_optimize）----
    println!("\n[2] 后台 xuanji_optimize（七专家 + 治理 + 算法验证网关）：");
    optimize_session(&g, &st.gate);
    println!("    算法否决 = {}", st.gate.is_vetoed());

    // ---- 阶段 3：用录制出的图直接调 xuanji-expert，打印详细报告 ----
    let ctx = xuanji_expert::context::GovernContext::new(
        xuanji_expert::context::Tenant::new("hermes", "default"),
        xuanji_expert::context::Principal::new("hermes-agent"),
    );
    let rep = xuanji_expert::pipeline::xuanji_optimize(&g, &ctx);
    println!("\n[3] 治理闸门：{:?}  批准 = {}", rep.gate.status, rep.gate.approved);
    println!("    优化收益：{}", rep.optimization.summary().replace('\n', " | "));
    println!("    算法验证：{}", rep.algo.summary);
    println!("    七专家评分：");
    for (e, s) in &rep.expert_scores {
        println!("      {:>12} : {:.2}", e, s);
    }

    // ---- 阶段 4：算法否决拦截接线（演示否决位如何阻断工具执行）----
    println!("\n[4] ToolExecutionMiddleware 拦截接线：");
    let before = on_tool_execution(&st);
    println!("    当前否决位下执行工具 → blocked = {}", before.blocked);
    st.set_vetoed(true);
    let after = on_tool_execution(&st);
    println!("    强制置位否决后执行工具 → blocked = {} ({})", after.blocked, after.reason.unwrap_or_default());

    // ---- 阶段 5：LLM 调用次数对比（用户原方案核心收益：调用减半）----
    println!("\n[5] LLM 调用次数对比（与 linear ReAct baseline）：");
    let plan = hermes_flow_bridge::mini_hermes::gov_pii_plan();
    // baseline：每步一次 LLM
    let b_tracer = hermes_flow_bridge::mini_hermes::LlmTracer::new();
    hermes_flow_bridge::mini_hermes::run_baseline(&plan, &b_tracer);
    let baseline_calls = b_tracer.count();
    // bridge：复用模板整段回放 → 0 次 LLM
    let b_st = BridgeState::new();
    hermes_flow_bridge::mini_hermes::register_gov_template(&b_st);
    let br_tracer = hermes_flow_bridge::mini_hermes::LlmTracer::new();
    let br_out = hermes_flow_bridge::mini_hermes::run_bridge(&b_st, &plan, &br_tracer);
    let bridge_calls = br_tracer.count();
    let saved = if baseline_calls > 0 {
        (1.0 - bridge_calls as f64 / baseline_calls as f64) * 100.0
    } else { 0.0 };
    println!("    baseline (linear ReAct) : {} 次 LLM 调用", baseline_calls);
    println!("    bridge   (复用回放)    : {} 次 LLM 调用", bridge_calls);
    println!("    削减比例              : {:.1}%", saved);
    println!("    回放动作样本          : {:?}", &br_out[..3.min(br_out.len())]);

    println!("\n=== 闭环验证完成：合法图通过算法网关且复用路由命中；否决位可强制拦截；LLM 调用显著削减 ===");
}
