// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! PrimiFlow 全域原语智能平台 · 关联图谱驱动的需求→代码/数据骨架生成层
//!
//! 设计哲学：以 [`assoc::AssocGraph`] 作为唯一事实源（对应 `primiflow/SPEC.md` 的六维溯源
//! `trace_links`），所有功能、代码、数据都从图谱派生，保证需求↔功能↔业务↔算法↔任务↔代码↔数据
//! 全链路可溯源、一一对应。
//!
//! 底层复用 `flow-ai` 的 κ‑τ 拓扑原语引擎（守恒公理 C²=κ²+τ²）。

pub const CRATE_ID: &str = "8c8d2382-6f9f-5218-894e-a07a43aa9554";
pub const ENGINE_NAME: &str = "mox::primiflow_core";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
};

pub mod assoc;
pub mod executor;
pub mod generate;
pub mod parse;
pub mod persistence;
pub mod runner;
#[cfg(feature = "server")]
pub mod server;

// 由关联图谱生成的落地代码：先运行 `cargo run --example gen` 生成 src/gen/* 后取消下行注释。
pub mod gen;

/// 复用 flow-ai 的 κ‑τ 拓扑原语引擎作为底层调度内核
pub use mox_ai_flow_sdk::primitive;

/// 企业级端到端运行入口与场景规格（示例与集成测试共用）
pub use runner::{enterprise_specs, run_all, run_pipeline, PipelineReport, Spec, Step};
