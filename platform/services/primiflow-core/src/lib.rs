//! PrimiFlow 全域原语智能平台 · 关联图谱驱动的需求→代码/数据骨架生成层
//!
//! 设计哲学：以 [`assoc::AssocGraph`] 作为唯一事实源（对应 `primiflow/SPEC.md` 的六维溯源
//! `trace_links`），所有功能、代码、数据都从图谱派生，保证需求↔功能↔业务↔算法↔任务↔代码↔数据
//! 全链路可溯源、一一对应。
//!
//! 底层复用 `flow-ai` 的 κ‑τ 拓扑原语引擎（守恒公理 C²=κ²+τ²）。

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "primiflow-core";

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
    uuid: "6d39c5e4-a703-46b7-b2d3-e4f5a6b7c8d9",
    ais_layers: &["L4-Core", "L5-Infra", "L1-Ingress"],
    owner_project: "proj-xuanji-core",
    capabilities: &[
        "六维溯源关联图谱 (AssocGraph) SSOT",
        "需求→功能→业务→算法→任务→代码→数据 全链路生成",
        "Pipeline 执行器与企业级规格 run_all",
        "SQL DDL 骨架生成",
        "示例/企业场景规约",
    ],
    data_tables_read: &["assoc_graph.bin", "specs/*.json"],
    data_tables_write: &["src/gen/*"],
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
pub use flow_ai::primitive;

/// 企业级端到端运行入口与场景规格（示例与集成测试共用）
pub use runner::{enterprise_specs, run_all, run_pipeline, PipelineReport, Spec, Step};
