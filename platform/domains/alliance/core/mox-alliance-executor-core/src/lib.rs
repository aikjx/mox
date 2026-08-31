// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Executor Core — 执行器核心
//!
//! 执行器的核心业务逻辑实现：
//! - DAG 执行引擎（节点调度、依赖管理、状态追踪）
//! - Mock 节点执行器（用于测试）
//!
//! ## 设计原则
//! - 依赖 proto 层的 trait 抽象（DIP）
//! - 核心逻辑无状态，状态通过外部存储管理
//! - 可测试：所有核心算法都有对应的单测

pub mod dag_engine;
pub mod mock_executor;

pub use dag_engine::DagEngineImpl;
pub use mock_executor::{MockExecutorConfig, MockNodeExecutor};
