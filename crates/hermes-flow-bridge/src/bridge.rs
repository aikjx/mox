//! Step 7：后台优化推送 + 算法否决拦截接线。
//!
//! 设计：ToolRequestMiddleware 同步累积流程图后，由本模块在**异步后台**把会话流程图快照
//! 推给 `expert-alliance::alliance_optimize` 做最优求解 + 七专家裁决 + 算法验证网关；
//! 结果写入 `GateState`（veto 共享标志）供 `ToolExecutionMiddleware` 同步读。
//!
//! 真实集成时，可改为 HTTP 请求 `expert-alliance` 独立服务（`POST /api/optimize` /
//! `POST /api/verify`）；此处直接复用已验证的库函数，保证 crate 独立可编译、可单测。

use crate::state::GateState;
use crate::recorder::Recorder;
use expert_alliance::{alliance_optimize, context::GovernContext};
use flow_ai::model::FlowGraph;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 用默认租户/主体构造治理上下文（真实环境从 Hermes 会话身份注入）。
fn default_ctx() -> GovernContext {
    GovernContext::new(
        expert_alliance::context::Tenant::new("hermes", "default"),
        expert_alliance::context::Principal::new("hermes-agent"),
    )
}

/// 对单张流程图做优化 + 验证，返回是否触发算法否决。
pub fn optimize_session(graph: &FlowGraph, gate: &GateState) {
    let ctx = default_ctx();
    let rep = alliance_optimize(graph, &ctx);
    // ⛨ 最高权限：算法验证否决 → 置位否决标志，强制拦截后续工具执行
    if rep.algo.vetoed {
        gate.set_vetoed(true);
        // 审计链已记录 algorithm_veto（见 expert-alliance govern/audit）
    } else {
        gate.set_vetoed(false);
    }
}

/// 启动后台轮询线程：周期性把各会话累积图推给优化内核。
/// 返回句柄（真实环境用 tokio task；此处用 std 线程演示，避免引入 async 运行时复杂度）。
pub fn spawn_optimizer(recorder: Recorder, gate: GateState) -> Arc<()> {
    let handle = Arc::new(());
    let h = handle.clone();
    thread::spawn(move || {
        let _ = h;
        loop {
            // 仅对 default 会话做演示优化；真实应遍历所有 session
            if let Some(g) = recorder.snapshot("default") {
                if !g.nodes.is_empty() {
                    optimize_session(&g, &gate);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::ToolCall;
    use crate::state::BridgeState;
    use serde_json::json;

    #[test]
    fn optimize_marks_non_veto_for_simple_graph() {
        let st = BridgeState::new();
        // 构造一张简单政务图：db.read → guard → web1
        st.recorder.record(
            "default",
            &ToolCall { tool_name: "db.read".into(), args: json!({"query":"select * from citizen_info"}), turn: 1 },
        );
        st.recorder.record(
            "default",
            &ToolCall { tool_name: "guard.desensitize".into(), args: json!({"var":"citizen"}), turn: 1 },
        );
        let g = st.recorder.snapshot("default").unwrap();
        optimize_session(&g, &st.gate);
        // 简单合法图应通过的（不触发否决）
        assert!(!st.gate.is_vetoed(), "合法流程图不应被算法否决");
    }
}
