// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 框架无关的核心钩子（单一事实源）。
//!
//! 设计：本模块**不依赖任何 Hermes 类型**，只认 `(tool_name, args_json, turn)` 这种最小投影。
//! `plugin.rs`（含 hermes_mirror）与 `integration/hermes_shim.rs`（真实 Hermes 适配）都调用这里，
//! 保证「业务逻辑只写一份」，Hermes 接入零重复、零逻辑漂移。

use crate::recorder::Recorder;
use crate::state::BridgeState;
use serde_json::Value;

/// 工具请求钩子结果：重写参数 + 来源注解 + 原因。
#[derive(Debug, Clone)]
pub struct RequestDecision {
    pub args: Value,
    pub source: Option<String>,
    pub reason: Option<String>,
}

/// ToolRequest 阶段：累积流程图 + 轻量复用路由。
/// 同步、无 I/O，可直接在 Hermes `ToolRequestMiddleware` 闭包内调用。
pub fn on_tool_request(
    state: &BridgeState,
    session: &str,
    tool_name: &str,
    args: &Value,
    turn: u32,
) -> Option<RequestDecision> {
    // 1) 累积到会话流程图
    let call = crate::normalize::ToolCall {
        tool_name: tool_name.to_string(),
        args: args.clone(),
        turn,
    };
    state.recorder.record(session, &call);

    // 2) 轻量复用路由：查本地模板索引（最短路径点亮的同步投影）
    let recent: Vec<String> = state
        .recorder
        .snapshot(session)
        .map(|g| g.nodes.iter().map(|n| n.name.clone()).collect())
        .unwrap_or_default();
    if let Some(tpl) = state.router.match_template(&recent) {
        return Some(RequestDecision {
            args: args.clone(),
            source: Some(format!("flow-template:{tpl}")),
            reason: Some("命中流程图复用模板，跳过完整 ReAct".into()),
        });
    }
    None
}

/// 工具执行钩子结果：是否拦截 + 拦截原因。
#[derive(Debug, Clone)]
pub struct ExecutionDecision {
    pub blocked: bool,
    pub reason: Option<String>,
}

/// ToolExecution 阶段：读 `algo.vetoed` 共享状态，最高权限拦截。
/// 同步、只读，可直接在 Hermes `ToolExecutionMiddleware` 闭包内调用。
pub fn on_tool_execution(state: &BridgeState) -> ExecutionDecision {
    if state.is_vetoed() {
        ExecutionDecision {
            blocked: true,
            reason: Some(
                "璇玑验证否决：优化破坏语义依赖/一致性，流程已拦截（algorithm_veto）".into(),
            ),
        }
    } else {
        ExecutionDecision {
            blocked: false,
            reason: None,
        }
    }
}

/// 给后台优化任务用：把会话图转成可用于 optimize 的 FlowGraph。
pub fn session_graph(recorder: &Recorder, session: &str) -> Option<mox_ai_flow_svc::model::FlowGraph> {
    recorder.snapshot(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_hook_records_and_routes() {
        let st = BridgeState::new();
        st.router.register(crate::router::FlowTemplate {
            id: "gov".into(),
            tool_seq: vec!["db.read".into(), "guard".into()],
        });
        // 第一条：仅累积
        assert!(on_tool_request(&st, "s", "db.read", &json!({}), 1).is_none());
        // 第二条：同序列命中模板
        let d = on_tool_request(&st, "s", "guard", &json!({}), 1);
        assert!(d.is_some());
        assert_eq!(d.unwrap().source.as_deref(), Some("flow-template:gov"));
        // 图已累积
        assert!(session_graph(&st.recorder, "s").unwrap().nodes.len() >= 2);
    }

    #[test]
    fn execution_hook_blocks_on_veto() {
        let st = BridgeState::new();
        st.gate.set_vetoed(true);
        assert!(on_tool_execution(&st).blocked);
        st.gate.set_vetoed(false);
        assert!(!on_tool_execution(&st).blocked);
    }
}
