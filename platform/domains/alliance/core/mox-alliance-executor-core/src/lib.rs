// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Executor Core — 执行器核心
//!
//! 执行器的核心业务逻辑实现（纯真实执行）：
//! - DAG 执行引擎（节点调度、依赖管理、状态追踪）
//! - 真实专家执行器
//!
//! ## 设计原则
//! - 依赖 proto 层的 trait 抽象（DIP）
//! - 核心逻辑无状态，状态通过外部存储管理
//! - 可测试：所有核心算法都有对应的单测

pub mod condition;
pub mod dag_engine;
pub mod expert_executor;
pub mod fusion;
/// Mock 节点执行器：仅测试 / 开发用，生产库默认不编译。
///
/// 启用方式：
/// - 本 crate 自身单元测试（`cfg(test)`）自动包含；
/// - 外部 crate（如 executor-svc 集成测试）需显式启用 `mock` feature：
///   `mox-alliance-executor-core = { features = ["mock"] }`；
/// - 本 crate 集成测试（tests/）经 `#[path]` 直接引用本文件，无需 feature。
#[cfg(any(test, feature = "mock"))]
pub mod mock_executor;
pub mod state_sink;

pub use condition::{Condition, CompareOp, Operator, Operand};
pub use dag_engine::DagEngineImpl;
pub use expert_executor::{
    ExecutorStatsView, ExpertExecutorConfig, ExpertNodeExecutor,
};
pub use fusion::{FusionEngine, FusionInput, FusionItem};
pub use state_sink::{ExecutionStateSink, ExecutionView, RestorableTask};

// 融合产出类型的权威定义在协议层（它是 `DagEngine` 契约的返回类型），此处仅转出
pub use mox_alliance_executor_proto::FusionOutput;
