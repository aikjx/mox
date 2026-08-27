// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 端到端会话级集成测试：模拟 Hermes 多轮工具调用，走完整 bridge 链路。
//!
//! 覆盖：录制→复用路由→后台优化(mox_optimize)→算法网关；以及否决拦截接线。
//! 这是「真实 Hermes 会话」的投影，不依赖 Hermes 源码编译（hook 只认最小投影）。

use mox_flow_bridge_svc::bridge::optimize_session;
use mox_flow_bridge_svc::hooks::{on_tool_execution, on_tool_request};
use mox_flow_bridge_svc::router::FlowTemplate;
use mox_flow_bridge_svc::state::BridgeState;
use serde_json::json;

/// 模拟 Hermes 一个会话里跨多轮的工具调用序列（含脱敏 PII 政务流程）。
fn simulate_session(st: &BridgeState, session: &str) {
    // 回合 1：读库 + 脱敏
    on_tool_request(
        st,
        session,
        "db.read",
        &json!({"query":"select * from citizen_info"}),
        1,
    );
    on_tool_request(
        st,
        session,
        "guard.desensitize",
        &json!({"var":"citizen"}),
        1,
    );
    // 回合 2：浏览器填报 + 汇总
    on_tool_request(st, session, "web1.submit", &json!({}), 2);
    on_tool_request(st, session, "merge.report", &json!({}), 2);
}

#[test]
fn session_recording_accumulates_graph_nodes() {
    let st = BridgeState::new();
    simulate_session(&st, "s1");
    let g = st.recorder.snapshot("s1").expect("session graph exists");
    // 4 次工具调用 → 4 个节点（id 为 <tool>#<turn>）
    assert_eq!(g.nodes.len(), 4, "应累积 4 个节点");
    let ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"db.read#1"), "db.read#1 应在图中");
    assert!(ids.contains(&"guard.desensitize#1"), "脱敏节点应在图中");
}

#[test]
fn unknown_session_has_no_graph() {
    let st = BridgeState::new();
    assert!(st.recorder.snapshot("never").is_none());
}

#[test]
fn router_short_circuits_repeated_sequence() {
    let st = BridgeState::new();
    // 注册一张「政务 PII 归集」复用模板
    st.router.register(FlowTemplate {
        id: "gov-pii".into(),
        tool_seq: vec![
            "db.read".into(),
            "guard.desensitize".into(),
            "web1.submit".into(),
            "merge.report".into(),
        ],
    });

    // 逐条注入，前 3 条仅累积（模板是 4 长序列，未凑齐时不命中）
    assert!(on_tool_request(&st, "s2", "db.read", &json!({}), 1).is_none());
    assert!(on_tool_request(&st, "s2", "guard.desensitize", &json!({}), 1).is_none());
    assert!(on_tool_request(&st, "s2", "web1.submit", &json!({}), 2).is_none());
    // 第 4 条凑齐序列 → 命中模板，给来源注解（Hermes 上游据此跳过完整 ReAct）
    let d = on_tool_request(&st, "s2", "merge.report", &json!({}), 2);
    let d = d.expect("应命中复用模板");
    assert_eq!(d.source.as_deref(), Some("flow-template:gov-pii"));
    assert!(d.reason.unwrap().contains("跳过完整 ReAct"));
}

#[test]
fn optimize_runs_real_engine_and_passes_verification() {
    let st = BridgeState::new();
    simulate_session(&st, "s3");
    let g = st.recorder.snapshot("s3").expect("graph");
    // 调真实 mox-expert 引擎（非桩）：优化 + 七专家 + 算法验证网关
    optimize_session(&g, &st.gate);
    // 合法政务图应通过的：算法验证不否决
    assert!(!st.gate.is_vetoed(), "合法流程图不应被算法否决");
}

#[test]
fn execution_middleware_blocks_when_algorithm_vetoes() {
    let st = BridgeState::new();
    // 模拟算法验证网关否决（verify 在发现语义破坏时会置位）
    st.set_vetoed(true);
    let decision = on_tool_execution(&st);
    assert!(decision.blocked, "vetoed 时必须拦截工具执行");
    assert!(decision.reason.unwrap().contains("algorithm_veto"));
    // 解除否决后放行
    st.set_vetoed(false);
    assert!(!on_tool_execution(&st).blocked);
}

#[test]
fn full_chain_session_then_optimize_then_route_free() {
    let st = BridgeState::new();
    // 1) Hermes 真实调用逐轮录制
    on_tool_request(
        &st,
        "s4",
        "db.read",
        &json!({"query":"select * from citizen_info"}),
        1,
    );
    on_tool_request(&st, "s4", "guard.desensitize", &json!({"var":"citizen"}), 1);
    on_tool_request(&st, "s4", "web1.submit", &json!({}), 2);
    on_tool_request(&st, "s4", "merge.report", &json!({}), 2);

    // 2) 后台把会话图推给璇玑引擎做最优求解 + 算法验证网关
    let g = st.recorder.snapshot("s4").unwrap();
    optimize_session(&g, &st.gate);
    assert!(!st.gate.is_vetoed());

    // 3) 后续同类会话命中复用模板（最短路径点亮），Hermes 跳过完整 ReAct
    st.router.register(FlowTemplate {
        id: "gov-pii".into(),
        tool_seq: vec![
            "db.read".into(),
            "guard.desensitize".into(),
            "web1.submit".into(),
            "merge.report".into(),
        ],
    });
    on_tool_request(&st, "s5", "db.read", &json!({}), 1);
    on_tool_request(&st, "s5", "guard.desensitize", &json!({}), 1);
    on_tool_request(&st, "s5", "web1.submit", &json!({}), 2);
    let d = on_tool_request(&st, "s5", "merge.report", &json!({}), 2);
    assert!(d.is_some(), "同类会话应命中复用模板走快路径");
}
