// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! mox-flow-unified-process-core —— 统一流程引擎核心库
//!
//! 本库抽象了 mox-ai-agent-svc、mox-ai-expert-svc、mox-ai-flow-svc
//! 三套流程引擎的公共类型与执行逻辑，提供：
//!
//! - **统一类型**：`UnifiedFlowGraph` / `UnifiedNodeKind` / `UnifiedToolKind`
//! - **统一执行器**：`FlowExecutor` trait + `DagFlowExecutor` 内置实现
//! - **节点扩展**：`NodeHandler` trait，各服务注入差异化能力
//! - **通用工具**：模板替换、条件求值、DAG 校验、循环检测
//!
//! # 设计原则
//! - 核心只做通用事，差异化通过扩展实现
//! - 向后兼容：各服务通过 From/Into 桥接
//! - 零性能损失：静态分发优先

pub mod error;
pub mod types;
pub mod executor;
pub mod handlers;
pub mod utils;
pub mod extension;

/// 适配层参考实现（各服务如何桥接到统一核心）
///
/// 注意：实际的转换代码应放在各业务服务的 crate 中，
/// 这里仅作为参考和文档。
#[cfg(feature = "adapters")]
pub mod adapters;

// 常用类型重导出
pub use error::{UnifiedFlowError, FlowResult};
pub use types::*;
pub use executor::{FlowExecutor, NodeHandler, ExecutionContext, DagFlowExecutor};
pub use extension::ExtensionRegistry;
