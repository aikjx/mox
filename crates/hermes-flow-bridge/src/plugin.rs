//! Step 5 / Step 9：实现 Hermes `Plugin` trait，注册两个中间件。
//!
//! 设计铁律：两个中间件都是**同步**的，不在闭包内跑 `alliance_optimize`（async + 重计算）。
//! 重计算在 `BridgeState` 持有的后台任务里跑（见 bridge.rs），中间件只做：
//!   1) ToolRequestMiddleware：累积流程图 + 轻量复用路由标注（调用 hooks::on_tool_request）
//!   2) ToolExecutionMiddleware：读 `algo.vetoed` 共享状态，强制拦截（调用 hooks::on_tool_execution）
//!
//! 本文件在「独立编译」时用 `hermes_mirror` 子模块（与 Hermes `hermes-agent::plugins` 同构）
//! 保证 crate 可独立 build + 单测。**真实集成到 Hermes 时**：
//!   - 把 `use hermes_mirror::*` 换成 `use hermes_agent::plugins::*`
//!   - 业务逻辑（累积/路由/拦截）已在 hooks.rs 写成框架无关函数，本文件只做类型转换，零逻辑重复
//!   - 或直接使用 `integration/hermes_shim.rs`（feature="hermes"）的真实适配

use crate::hooks::{on_tool_execution, on_tool_request, ExecutionDecision};
use crate::state::BridgeState;
use serde_json::Value;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Hermes 中间件镜像（真实集成时删除本模块，改用 hermes_agent::plugins）
// ---------------------------------------------------------------------------
#[allow(dead_code)]
mod hermes_mirror {
    use serde_json::Value;
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    pub struct ToolRequestMiddlewareContext {
        pub tool_name: String,
        pub args: Value,
        pub original_args: Value,
        pub turn: u32,
    }
    #[derive(Debug, Clone)]
    pub struct ToolRequestMiddlewareUpdate {
        pub args: Value,
        pub source: Option<String>,
        pub reason: Option<String>,
    }
    impl ToolRequestMiddlewareUpdate {
        pub fn new(args: Value) -> Self {
            Self { args, source: None, reason: None }
        }
    }
    #[derive(Debug, Clone)]
    pub struct ToolExecutionMiddlewareContext {
        pub tool_name: String,
        pub tool_call_id: String,
        pub args: Value,
        pub original_args: Value,
        pub turn: u32,
    }
    /// 镜像：真实 Hermes 的 ToolResult 由 hermes-intelligence 定义；这里仅做最小投影。
    /// 真实集成时替换为 `use hermes_agent::prelude::ToolResult` 或对应路径。
    pub struct ToolResult {
        pub ok: bool,
        pub content: String,
    }
    impl ToolResult {
        pub fn error(msg: &str) -> Self {
            Self { ok: false, content: msg.into() }
        }
        pub fn ok(content: String) -> Self {
            Self { ok: true, content }
        }
    }
    pub type ToolRequestMiddleware =
        Arc<dyn Fn(&ToolRequestMiddlewareContext) -> Option<ToolRequestMiddlewareUpdate> + Send + Sync>;
    pub type ToolExecutionMiddleware = Arc<
        dyn Fn(&ToolExecutionMiddlewareContext, &mut dyn FnMut(Option<Value>) -> ToolResult) -> ToolResult
            + Send
            + Sync,
    >;
    pub struct PluginContext {
        pub tool_request: Vec<ToolRequestMiddleware>,
        pub tool_exec: Vec<ToolExecutionMiddleware>,
    }
    impl PluginContext {
        pub fn new() -> Self {
            Self { tool_request: Vec::new(), tool_exec: Vec::new() }
        }
        pub fn on_tool_request(&mut self, mw: ToolRequestMiddleware) {
            self.tool_request.push(mw);
        }
        pub fn on_tool_execution(&mut self, mw: ToolExecutionMiddleware) {
            self.tool_exec.push(mw);
        }
    }
    pub trait Plugin: Send + Sync {
        fn register(&self, ctx: &mut PluginContext);
    }
}

use hermes_mirror::*;

/// FlowBridge 插件主体（独立编译版，用 hermes_mirror）。
pub struct FlowBridgePlugin {
    state: Arc<BridgeState>,
    session: String,
}

impl FlowBridgePlugin {
    pub fn new(state: Arc<BridgeState>) -> Self {
        Self { state, session: "default".into() }
    }
    /// 指定会话 id（真实环境由 Hermes 注入）。
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = session.into();
        self
    }
}

impl Plugin for FlowBridgePlugin {
    fn register(&self, ctx: &mut PluginContext) {
        let st = self.state.clone();
        let sess = self.session.clone();

        // --- ToolRequestMiddleware：累积流程图 + 轻量复用路由 ---
        ctx.on_tool_request(Arc::new(move |c: &ToolRequestMiddlewareContext| {
            match on_tool_request(&st, &sess, &c.tool_name, &c.args, c.turn) {
                Some(d) => Some(ToolRequestMiddlewareUpdate {
                    args: d.args,
                    source: d.source,
                    reason: d.reason,
                }),
                None => None,
            }
        }));

        // --- ToolExecutionMiddleware：算法否决拦截（最高权限）---
        let st2 = self.state.clone();
        ctx.on_tool_execution(Arc::new(move |_c: &ToolExecutionMiddlewareContext,
                                            run: &mut dyn FnMut(Option<Value>) -> ToolResult| {
            let decision: ExecutionDecision = on_tool_execution(&st2);
            if decision.blocked {
                return ToolResult::error(&decision.reason.unwrap_or_default());
            }
            run(None)
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_registers_two_middlewares() {
        let st = BridgeState::new();
        let plugin = FlowBridgePlugin::new(st.clone());
        let mut ctx = PluginContext::new();
        plugin.register(&mut ctx);
        assert_eq!(ctx.tool_request.len(), 1);
        assert_eq!(ctx.tool_exec.len(), 1);
    }

    #[test]
    fn execution_middleware_blocks_when_vetoed() {
        let st = BridgeState::new();
        st.set_vetoed(true);
        let plugin = FlowBridgePlugin::new(st.clone());
        let mut ctx = PluginContext::new();
        plugin.register(&mut ctx);
        let mw = &ctx.tool_exec[0];
        let c = ToolExecutionMiddlewareContext {
            tool_name: "db.write".into(),
            tool_call_id: "x".into(),
            args: Value::Null,
            original_args: Value::Null,
            turn: 1,
        };
        let res = mw(&c, &mut |_| ToolResult { ok: true, content: "ok".into() });
        assert!(!res.ok, "vetoed 时必须拦截，不应执行工具");
    }

    #[test]
    fn request_middleware_returns_route_source() {
        let st = BridgeState::new();
        st.router.register(crate::router::FlowTemplate {
            id: "gov".into(),
            tool_seq: vec!["a".into(), "b".into()],
        });
        st.recorder.record(
            "default",
            &crate::normalize::ToolCall { tool_name: "a".into(), args: Value::Null, turn: 1 },
        );
        let plugin = FlowBridgePlugin::new(st.clone());
        let mut ctx = PluginContext::new();
        plugin.register(&mut ctx);
        let mw = &ctx.tool_request[0];
        let c = ToolRequestMiddlewareContext {
            tool_name: "b".into(),
            args: Value::Null,
            original_args: Value::Null,
            turn: 1,
        };
        let upd = mw(&c);
        assert!(upd.is_some());
        assert!(upd.unwrap().source.unwrap().contains("gov"));
    }
}
