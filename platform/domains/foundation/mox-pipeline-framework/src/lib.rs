// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX 平台级管线框架
//!
//! 统一的阶段编排、上下文流转、钩子机制、插件系统和审计基础设施。
//! 适用于所有需要多阶段处理流水线的场景。
//!
//! # 设计原则
//!
//! - **单一管线核心**：所有管线共享同一骨架
//! - **泛型阶段标识**：通过 `PhaseId` trait 与具体领域阶段解耦
//! - **阶段可插拔**：每个阶段是一个 PhaseHandler，可注册可替换
//! - **上下文贯穿**：PipelineContext 在各阶段间流动
//! - **钩子机制**：pre_phase / post_phase 瀑布扩展点
//! - **插件系统**：Everything is a Plugin，支持生命周期管理
//! - **审计统一**：统一审计事件模型，内部链 + 外部 sink 双写
//! - **同步/异步统一**：核心支持两种执行模式
//!
//! # 模块结构
//!
//! - [`phase`] — PhaseId trait + PhaseHandler + PhaseStatus + PhaseExecution
//! - [`result`] — PhaseResult trait + GenericPhaseResult
//! - [`context`] — PipelineContext + PipelineInput + PipelineOptions
//! - [`hooks`] — HookRegistry + HookChain + 内置审计/日志/指标钩子
//! - [`pipeline`] — Pipeline trait + PipelineBuilder + SyncPipeline + AsyncPipeline
//! - [`plugin`] — Plugin trait + PluginRegistry + ExtensionPoint + PluginContext
//! - [`audit`] — UnifiedAuditChain + UnifiedAuditEvent + AuditSink trait
//! - [`error`] — PipelineError（PL05xxx，可集成 `mox-error`）
//! - [`events`] — PipelineEvent + PhaseEvent
//!
//! # Feature Flags
//!
//! - `default` — 核心功能（同步管线 + 内置审计链 + 插件系统）
//! - `mox-error` — 集成 `mox-error` 统一错误码系统
//! - `audit` — 集成 `mox-audit` 平台级审计系统（桥接 Sink）
//! - `async` — 启用异步管线支持（AsyncPipeline）
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mox_pipeline_framework::*;
//!
//! // 1. 定义阶段处理器
//! struct MyHandler;
//! impl PhaseHandler<NamedPhase> for MyHandler {
//!     fn phase(&self) -> NamedPhase {
//!         NamedPhase::new("analyze")
//!     }
//!     fn execute(&self, ctx: &mut PipelineContext<NamedPhase>)
//!         -> Result<Box<dyn PhaseResult<NamedPhase>>, String>
//!     {
//!         // ... 处理逻辑 ...
//!         Ok(Box::new(GenericPhaseResult::success(
//!             NamedPhase::new("analyze"),
//!             serde_json::json!({"result": "ok"}),
//!             100,
//!         )))
//!     }
//! }
//!
//! // 2. 构建管线
//! let pipeline = PipelineBuilder::new("my_pipeline")
//!     .with_phase(Box::new(MyHandler))
//!     .with_default_hooks()
//!     .build_sync();
//!
//! // 3. 执行管线
//! let input = PipelineInput::Query {
//!     query: "hello".into(),
//!     session_id: None,
//!     context: std::collections::HashMap::new(),
//! };
//! let ctx = pipeline.run(PipelineContext::new(input, PipelineOptions::default()))?;
//! ```

// ── 模块声明 ────────────────────────────────────────────────────

pub mod audit;
pub mod context;
pub mod error;
pub mod events;
pub mod hooks;
pub mod phase;
pub mod pipeline;
pub mod plugin;
pub mod result;

// ── 重导出 ──────────────────────────────────────────────────────

// 审计
pub use audit::{
    ActorSource, AuditActor, AuditOutcome, AuditSeverity, AuditSink, UnifiedAuditChain,
    UnifiedAuditEvent,
};

#[cfg(feature = "audit")]
pub use audit::mox_audit_bridge::MoxAuditSink;

// 上下文
pub use context::{PipelineContext, PipelineInput, PipelineOptions};

// 错误
pub use error::PipelineError;

// 事件
pub use events::{PhaseEvent, PipelineEvent};

// 钩子
pub use hooks::{
    builtin_hooks, HookChain, HookError, HookEvent, HookRegistry, HookResult, WaterfallHook,
};

// 阶段
pub use phase::{NamedPhase, PhaseExecution, PhaseHandler, PhaseId, PhaseStatus};

// 管线
pub use pipeline::{Pipeline, PipelineBuilder, SyncPipeline};

#[cfg(feature = "async")]
pub use pipeline::AsyncPipeline;

// 插件
pub use plugin::{
    ExtensionPoint, Plugin, PluginContext, PluginError, PluginMeta, PluginRegistry,
};

// 结果
pub use result::{GenericPhaseResult, PhaseResult, PhaseResultExt};

// ── 便捷类型别名 ────────────────────────────────────────────────

/// 管线结果类型别名
pub type PipelineResult<T, P> = Result<T, PipelineError<P>>;

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod lib_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn reexports_work() {
        // 验证所有重导出类型可用
        let _phase = NamedPhase::new("analyze");
        let _status = PhaseStatus::Success;
        let _event = HookEvent::<NamedPhase>::PrePipeline;
        let _err: PipelineError<NamedPhase> = PipelineError::ConfigError("test".into());

        let input = PipelineInput::Query {
            query: "test".into(),
            session_id: None,
            context: HashMap::new(),
        };
        let _ctx: PipelineContext<NamedPhase> =
            PipelineContext::new(input, PipelineOptions::default());

        let _chain = UnifiedAuditChain::new();

        // 插件系统
        let _registry = PluginRegistry::<NamedPhase>::new();
        let _meta = PluginMeta::default();
    }

    #[test]
    fn pipeline_builder_creates_sync_pipeline() {
        struct TestHandler;
        impl PhaseHandler<NamedPhase> for TestHandler {
            fn phase(&self) -> NamedPhase {
                NamedPhase::new("normalize")
            }
            fn execute(
                &self,
                _ctx: &mut PipelineContext<NamedPhase>,
            ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
                Ok(Box::new(GenericPhaseResult::success(
                    NamedPhase::new("normalize"),
                    serde_json::json!({}),
                    0,
                )))
            }
        }

        let pipeline = PipelineBuilder::new("test")
            .with_phase(Box::new(TestHandler))
            .build_sync();

        assert_eq!(pipeline.name(), "test");
        assert!(!pipeline.is_async());
        assert_eq!(pipeline.phases().len(), 1);
    }

    #[test]
    fn full_pipeline_execution() {
        struct NormalizeHandler;
        impl PhaseHandler<NamedPhase> for NormalizeHandler {
            fn phase(&self) -> NamedPhase {
                NamedPhase::new("normalize")
            }
            fn execute(
                &self,
                ctx: &mut PipelineContext<NamedPhase>,
            ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
                ctx.set_bag("normalized", true);
                Ok(Box::new(GenericPhaseResult::success(
                    NamedPhase::new("normalize"),
                    serde_json::json!({"normalized": true}),
                    10,
                )))
            }
        }

        struct AnalyzeHandler;
        impl PhaseHandler<NamedPhase> for AnalyzeHandler {
            fn phase(&self) -> NamedPhase {
                NamedPhase::new("analyze")
            }
            fn execute(
                &self,
                ctx: &mut PipelineContext<NamedPhase>,
            ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
                assert!(ctx.get_bag::<bool>("normalized") == Some(&true));
                Ok(Box::new(GenericPhaseResult::success(
                    NamedPhase::new("analyze"),
                    serde_json::json!({"analysis": "complete"}),
                    20,
                )))
            }
        }

        let pipeline = PipelineBuilder::new("full_test")
            .with_phase(Box::new(NormalizeHandler))
            .with_phase(Box::new(AnalyzeHandler))
            .build_sync();

        let input = PipelineInput::Query {
            query: "test query".into(),
            session_id: None,
            context: HashMap::new(),
        };
        let ctx = pipeline
            .run(PipelineContext::new(input, PipelineOptions::default()))
            .unwrap();

        assert!(ctx.all_succeeded());
        assert_eq!(ctx.completed_phases().len(), 2);
        assert!(ctx.get_result(&NamedPhase::new("normalize")).is_some());
        assert!(ctx.get_result(&NamedPhase::new("analyze")).is_some());
    }

    #[test]
    fn pipeline_result_type_alias() {
        let ok: PipelineResult<i32, NamedPhase> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: PipelineResult<i32, NamedPhase> = Err(PipelineError::ConfigError("test".into()));
        assert!(err.is_err());
    }

    #[test]
    fn plugin_registry_reexport_works() {
        let registry = PluginRegistry::<NamedPhase>::new();
        assert!(registry.is_empty());

        let meta = PluginMeta {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            ..Default::default()
        };
        assert_eq!(meta.id, "test");
    }
}
