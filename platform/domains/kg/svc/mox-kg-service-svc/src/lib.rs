// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # Mox R3 Graph Service
//!
//! Zero third-party成品 graph DB embedded; Parser 100% Mox self-dev.
//!
//! License: MIT OR Apache-2.0
//!
//! 包含：
//! - GraphServer：nGQL / openCypher 执行入口，使用内嵌 StorageEngine 访问存储
//! - NgqlParser：支持 60 条标准 nGQL 语句解析
//! - CypherParser：支持 20 条 openCypher 语句解析
//! - Optimizer：规则 + 代价的轻量查询优化器（投影下推 / 5-hop 空剪枝 / 重排序）
//! - AlgoBridge：Rust 7 算法纯内联实现，精度护栏与 T5 single-source 一致
//! - ResultSet：标准化行列结果集；PropValue 原子类型枚举；LPA 弃用 helper
//!
//! 本 crate 零引用第三方成品图数据库 crate。

pub mod ac15_faults;
pub mod algo_bridge;
pub mod community_cnm;
pub mod cypher_parser;
pub mod error;
pub mod graph_server;
pub mod ngql_parser;
pub mod optimizer;
pub mod projection_20;
pub mod result_set;
pub mod trace_8stages;

/// KG/AI HTTP 适配层：6 KG 真实接口 + 4 AI 引擎接口桩
/// （feature = "http-adapter" 时构建，见 [`http_adapter::build_kg_ai_router`]）
#[cfg(feature = "http-adapter")]
pub mod http_adapter;

pub use ac15_faults::{Ac15Fault, FaultInjector, FaultPoint, FaultReport, QualityGate};
pub use trace_8stages::*;
pub use algo_bridge::{AlgoBridge, Communities, Graph as AlgoGraph};
pub use cypher_parser::CypherParser;
pub use error::{GraphError, GraphResult};
pub use graph_server::{Direction, EdgeRow, GraphServer, Neighbor, StorageEngine};
pub use ngql_parser::{NgqlParser, PlanNode};
pub use crate::optimizer::{Optimizer, PlanOutput};
pub use projection_20::{projection_20_matrix, PROJECTION_OPERATORS, ProjectionContext, ProjectionOperator, ProjectionResult};
pub use result_set::{PropValue, ResultSet};

/// LPA helper（公共 API 已弃用）。
#[allow(deprecated)]
#[deprecated(
    since = "3.0.0",
    note = "LPA public API deprecated; use AlgoBridge::cnm for community detection."
)]
pub fn lpa_communities(_graph: &algo_bridge::Graph) -> Communities {
    Vec::new()
}
