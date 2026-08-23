//! Step 7：后台优化推送 + 算法否决拦截接线。
//!
//! DIP 版：此文件**不再** `use xuanji_expert::xuanji_optimize` 等具体函数 / struct。
//! 全部璇玑引擎调用统一通过：
//!   - `xuanji_expert::expert_traits::ExpertConsultant` trait 抽象
//!   - `xuanji_expert::types::{ConsultQuery, ConsultReport}` 数据投影
//!   - 默认实现通过 `expert_traits::default_consultant()` 工厂注入（不出现 concrete struct 名字）。
//!
//! 真实集成时，可改为 HTTP 请求 xuanji-expert 独立服务（POST /api/optimize / verify）；
//! 只要实现 ExpertConsultant trait 即可，无需改业务代码（中间件、录制器、拦截位均不变）。
//!
//! 共享 `Arc<dyn ExpertConsultant>` 放在 `BridgeState.consultant`；独立 `optimize_session`
//! 为兼容旧公共 API 签名，内部用默认工厂获取 trait object。

use crate::state::GateState;
use crate::recorder::Recorder;
use flow_ai::model::FlowGraph;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use xuanji_expert::expert_traits::ExpertConsultant;
use xuanji_expert::types::{ConsultQuery, ConsultReport};

/// 把 FlowGraph 序列化为 JSON + 默认 Hermes 主体/租户参数，构造 ConsultQuery。
///
/// ctx 键与 `ExpertServiceImpl::consult_sync` 约定保持一致（flow_json / tenant / namespace / principal）。
fn build_query(graph: &FlowGraph, id: impl Into<String>) -> ConsultQuery {
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert("flow_json".into(), serde_json::to_string(graph).unwrap_or_default());
    ctx.insert("tenant".into(), "hermes".into());
    ctx.insert("namespace".into(), "default".into());
    ctx.insert("principal".into(), "hermes-agent".into());
    ctx.insert("max_parallel".into(), "8".into());
    ctx.insert("max_cost_budget".into(), "100".into());
    ctx.insert("sla_ms".into(), "50000".into());
    ConsultQuery { id: id.into(), query: graph.name.clone(), ctx }
}

/// 用默认 consultant 做优化 + 验证，返回 ConsultReport（便于调试/测试）。
/// 同时根据 `report.vetoed` 置位 `GateState.vetoed` 算法否决标志（最高权限）。
pub fn optimize_session(graph: &FlowGraph, gate: &GateState) -> ConsultReport {
    optimize_session_with(graph, gate, xuanji_expert::expert_traits::default_consultant())
}

/// 接受自定义 consultant（DIP 证据：测试可替换为 Mock，无需真实璇玑引擎）。
pub fn optimize_session_with(
    graph: &FlowGraph,
    gate: &GateState,
    consultant: Arc<dyn ExpertConsultant>,
) -> ConsultReport {
    let q = build_query(graph, format!("hermes-sess-{}", graph.id));
    // 走 trait：consult_blocking 默认会分发到具体 impl 的原生 sync 实现
    let rep = consultant
        .consult_blocking(&q)
        .unwrap_or_else(|e| ConsultReport {
            report_id: q.id.clone(),
            steps: vec![format!("[ExpertConsultant] 调用失败（保留原 gate 否决位）: {}", e)],
            score: 0.0,
            vetoed: gate.is_vetoed(),
            reason: Some(format!("error: {}", e)),
        });
    if rep.vetoed {
        gate.set_vetoed(true);
    } else {
        gate.set_vetoed(false);
    }
    rep
}

/// 启动后台轮询线程：周期性把各会话累积图推给优化内核。
/// 返回句柄（真实环境用 tokio task；此处用 std 线程演示，避免引入 async 运行时复杂度）。
pub fn spawn_optimizer(recorder: Recorder, gate: GateState) -> Arc<()> {
    spawn_optimizer_with(recorder, gate, xuanji_expert::expert_traits::default_consultant())
}

/// 接受自定义 consultant 的后台轮询版本。
pub fn spawn_optimizer_with(
    recorder: Recorder,
    gate: GateState,
    consultant: Arc<dyn ExpertConsultant>,
) -> Arc<()> {
    let handle = Arc::new(());
    let h = handle.clone();
    thread::spawn(move || {
        let _ = h;
        loop {
            // 仅对 default 会话做演示优化；真实应遍历所有 session
            if let Some(g) = recorder.snapshot("default") {
                if !g.nodes.is_empty() {
                    optimize_session_with(&g, &gate, consultant.clone());
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
        // DIP 证据：通过 st.consultant trait object 调用，不直接出现 xuanji-expert 具体 struct。
        let rep = optimize_session_with(&g, &st.gate, st.consultant.clone());
        assert!(rep.score >= 0.0 && rep.score <= 1.0);
        // 简单合法图应通过（不触发否决）
        assert!(!st.gate.is_vetoed(), "合法流程图不应被算法否决（vetoed=false），report={:?}", rep);
    }

    #[test]
    fn optimize_session_with_mock_trait_object() {
        // DIP 证据：用一个最小 Mock 实现 ExpertConsultant（在 tests 内部，不依赖 xuanji concrete），
        // 证明 optimize_session_with 可脱离真实璇玑引擎运行。
        // 只覆写 consult_blocking（同步默认方法，不触发 tokio runtime）即可满足 sync 测试路径。
        use async_trait::async_trait;
        struct MockHealthy;
        #[async_trait]
        impl xuanji_expert::expert_traits::ExpertConsultant for MockHealthy {
            async fn consult(&self, _q: &ConsultQuery) -> xuanji_expert::types::Result<ConsultReport> {
                unreachable!("sync 测试路径使用 consult_blocking，不应走到 async consult")
            }
            fn consult_blocking(&self, q: &ConsultQuery) -> xuanji_expert::types::Result<ConsultReport> {
                Ok(ConsultReport {
                    report_id: q.id.clone(),
                    steps: vec!["mock".into()],
                    score: 0.9,
                    vetoed: false,
                    reason: None,
                })
            }
        }

        let st = BridgeState::with_consultant(Arc::new(MockHealthy));
        st.recorder.record(
            "default",
            &ToolCall { tool_name: "any".into(), args: json!({}), turn: 1 },
        );
        let g = st.recorder.snapshot("default").unwrap();
        let rep = optimize_session_with(&g, &st.gate, st.consultant.clone());
        assert!((rep.score - 0.9).abs() < 1e-9);
        assert!(!st.gate.is_vetoed());
    }
}
