// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Executor Proto — 执行器协议层
//!
//! 联盟执行器的接口契约定义，包括：
//! - DAG 执行引擎接口
//! - 节点执行器接口
//! - 进度追踪接口
//!
//! ## 设计原则
//! - **DIP 依赖倒置**：executor-core 依赖本 crate 的 trait 抽象
//! - **SSOT 单一真相源**：执行器的接口契约只有这里一个权威定义

pub mod dag_engine;
pub mod node_executor;
pub mod types;

// ─── 重导出 ────────────────────────────────────────────────────────────────

pub use dag_engine::{DagEngine, ExecutionOptions, ExecutionStatus};
pub use node_executor::{NodeExecutor, NodeExecutionRequest, NodeExecutionResult};
pub use types::ExecutorConfig;
