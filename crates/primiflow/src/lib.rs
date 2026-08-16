//! PrimiFlow 全域原语智能平台 · 关联图谱驱动的需求→代码/数据骨架生成层
//!
//! 设计哲学：以 [`assoc::AssocGraph`] 作为唯一事实源（对应 `primiflow/SPEC.md` 的六维溯源
//! `trace_links`），所有功能、代码、数据都从图谱派生，保证需求↔功能↔业务↔算法↔任务↔代码↔数据
//! 全链路可溯源、一一对应。
//!
//! 底层复用 `flow-ai` 的 κ‑τ 拓扑原语引擎（守恒公理 C²=κ²+τ²）。

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
pub use flow_ai::primitive;

/// 企业级端到端运行入口与场景规格（示例与集成测试共用）
pub use runner::{enterprise_specs, run_all, run_pipeline, PipelineReport, Spec, Step};
