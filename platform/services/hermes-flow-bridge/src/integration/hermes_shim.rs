//! Step 9 真实 Hermes 适配层（feature = "hermes" 时编译）。
//!
//! 用法（用户侧联调）：
//!   1) 把本 crate 作为依赖加入 Hermes workspace，或把本 crate 的 src 复制到 Hermes 的插件目录
//!   2) 在 Hermes 侧 `cargo build --features hermes`，并把 `hermes_mirror` 替换为真实类型：
//!      - `use hermes_agent::plugins::*` 取代本文件的 `hermes_mirror`
//!      - `ToolResult` 取 `hermes_agent::prelude::ToolResult`（或你 checkout 中实际定义路径）
//!   3) 在 Hermes 插件入口构造 `FlowBridgePlugin`，调用 `.register(&mut ctx)`
//!
//! 业务逻辑（累积/路由/否决拦截）全部来自 `hooks.rs`，此处只做「真实类型 ↔ 纯钩子」转换，
//! 因此**逻辑零漂移、零重复**。

#![cfg(feature = "hermes")]

use crate::hooks::{on_tool_execution, on_tool_request};
use crate::plugin::BridgeState;
use hermes_agent::plugins::*; // 真实 Hermes 插件 API（镜像见 plugin.rs 的 hermes_mirror）
use serde_json::Value;
use std::sync::Arc;

/// 真实 Hermes 插件实现。把 bridge 的核心钩子接到 Hermes 生命周期。
pub struct HermesFlowBridge {
    state: Arc<BridgeState>,
    session: String,
}

impl HermesFlowBridge {
    pub fn new(state: Arc<BridgeState>) -> Self {
        Self {
            state,
            session: "default".into(),
        }
    }
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = session.into();
        self
    }
}

impl Plugin for HermesFlowBridge {
    fn register(&self, ctx: &mut PluginContext) {
        let st = self.state.clone();
        let sess = self.session.clone();

        ctx.on_tool_request(Arc::new(
            move |c: &ToolRequestMiddlewareContext| match on_tool_request(
                &st,
                &sess,
                &c.tool_name,
                &c.args,
                c.turn,
            ) {
                Some(d) => Some(ToolRequestMiddlewareUpdate {
                    args: d.args,
                    source: d.source,
                    reason: d.reason,
                }),
                None => None,
            },
        ));

        let st2 = self.state.clone();
        ctx.on_tool_execution(Arc::new(
            move |_c: &ToolExecutionMiddlewareContext,
                  run: &mut dyn FnMut(Option<Value>) -> ToolResult| {
                let decision = on_tool_execution(&st2);
                if decision.blocked {
                    // ⛨ 璇玑验证否决
                    return ToolResult::error(&decision.reason.unwrap_or_default());
                }
                run(None)
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::FlowTemplate;

    #[test]
    fn hermes_plugin_registers_middlewares() {
        let st = Arc::new(BridgeState::new());
        st.router.register(FlowTemplate {
            id: "x".into(),
            tool_seq: vec!["a".into()],
        });
        let p = HermesFlowBridge::new(st);
        let mut ctx = PluginContext::new();
        p.register(&mut ctx);
        assert_eq!(ctx.tool_request.len(), 1);
        assert_eq!(ctx.tool_exec.len(), 1);
    }
}
