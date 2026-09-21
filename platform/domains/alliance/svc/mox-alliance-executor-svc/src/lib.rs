// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Executor Service — 联盟执行器服务
//!
//! 专家联盟执行器的服务层（纯真实执行，无 Mock 路径）：
//! - HTTP API（执行状态查询、节点管理、人工干预）
//! - DAG 执行引擎运行
//! - 节点执行调度（真实 AI 专家）

pub mod app_state;
pub mod routes;
pub mod server;
pub mod state_sink;

pub use app_state::ExecutorAppState;
pub use server::ExecutorServer;
pub use state_sink::{resolve_state_sink, FileExecutionStateSink, SqliteExecutionStateSink};
