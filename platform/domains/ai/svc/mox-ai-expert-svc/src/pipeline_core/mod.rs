// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一管线核心（Pipeline Core）
//!
//! 为 `pipeline.rs`（全维处理流水线）和 `alliance/gate.rs`（联盟 6 阶段管线）
//! 提供统一的阶段编排、上下文流转、钩子机制和审计基础设施。
//!
//! 设计原则：
//! - 单一管线核心：两套管线共享同一骨架
//! - 阶段可插拔：每个阶段是一个 PhaseHandler，可注册可替换
//! - 上下文贯穿：PipelineContext 在各阶段间流动
//! - 钩子机制：pre_phase / post_phase 瀑布扩展点
//! - 审计统一：统一审计事件模型，内部链 + 外部合规双写
//! - 同步/异步统一：核心支持两种执行模式

pub mod audit;
pub mod context;
pub mod hooks;
pub mod phase;
pub mod pipeline;
pub mod result;

pub use audit::{UnifiedAuditChain, UnifiedAuditEvent};
pub use context::PipelineContext;
pub use hooks::{HookChain, HookError, HookResult, WaterfallHook};
pub use phase::{Phase, PhaseExecution, PhaseHandler, PhaseStatus};
pub use pipeline::{Pipeline, PipelineBuilder, SyncPipeline, AsyncPipeline};
pub use result::{PhaseResult, PhaseResultExt};
