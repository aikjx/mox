// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! hermes-flow-bridge：零侵入插件，把 flow-ai + mox-expert 流程图/关系网优化内核
//! 注入 Hermes Agent Ultra。
//!
//! 模块：
//! - `normalize`：Hermes 工具调用 ↔ mox_ai_flow_svc::FlowNode 映射
//! - `recorder`：跨回合累积会话执行流程图
//! - `router`：复用模板最短路径点亮（轻量同步版）
//! - `plugin`：实现 Hermes Plugin trait，注册两个中间件（含算法否决拦截）

pub const CRATE_ID: &str = "9bfaf43b-385a-5a44-9fb2-65b4003ee80d";
pub const ENGINE_NAME: &str = "mox::hermes_flow_bridge";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
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
