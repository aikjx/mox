//! hermes-flow-bridge：零侵入插件，把 flow-ai + xuanji-expert 流程图/关系网优化内核
//! 注入 Hermes Agent Ultra。
//!
//! 模块：
//! - `normalize`：Hermes 工具调用 ↔ flow_ai::FlowNode 映射
//! - `recorder`：跨回合累积会话执行流程图
//! - `router`：复用模板最短路径点亮（轻量同步版）
//! - `plugin`：实现 Hermes Plugin trait，注册两个中间件（含算法否决拦截）

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "hermes-flow-bridge";

/// 璇玑系统 Crate 结构化元数据。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    pub uuid: &'static str,
    pub ais_layers: &'static [&'static str],
    pub owner_project: &'static str,
    pub capabilities: &'static [&'static str],
    pub data_tables_read: &'static [&'static str],
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "07d3c9e8-41ad-40b1-5c7d-8e9fa0b1c2d3",
    ais_layers: &["L2-Gateway", "L3-Service", "L7-Tool"],
    owner_project: "proj-auto-dev",
    capabilities: &[],
    data_tables_read: &["hermes_sessions.db"],
    data_tables_write: &[],
};

pub mod bridge;
pub mod hooks;
pub mod mini_hermes;
pub mod normalize;
pub mod plugin;
pub mod recorder;
pub mod router;
pub mod state;

#[cfg(feature = "hermes")]
pub mod integration;

#[cfg(feature = "live")]
pub mod live;

pub use bridge::{optimize_session, spawn_optimizer};
pub use hooks::{on_tool_execution, on_tool_request};
pub use plugin::FlowBridgePlugin;
pub use state::{BridgeState, GateState};
