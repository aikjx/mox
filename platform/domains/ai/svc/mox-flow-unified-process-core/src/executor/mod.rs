// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 流程执行器模块
//!
//! 提供统一的流程执行抽象：
//! - `FlowExecutor` trait：执行器标准接口
//! - `NodeHandler` trait：节点处理器扩展点
//! - `DagFlowExecutor`：内置 DAG 执行器实现
//! - `ExecutionContext`：节点执行上下文

pub mod context;
pub mod r#trait;
pub mod dag;

pub use context::ExecutionContext;
pub use r#trait::{FlowExecutor, NodeHandler};
pub use dag::DagFlowExecutor;
