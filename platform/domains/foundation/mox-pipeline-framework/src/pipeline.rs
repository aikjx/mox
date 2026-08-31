// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Pipeline trait 与执行器
//!
//! 提供统一的管线编排能力：
//! - `Pipeline<P>` trait：管线的统一抽象（泛型阶段标识）
//! - `PipelineBuilder<P>`：构建器模式，注册阶段和钩子
//! - `SyncPipeline<P>`：同步执行器
//! - `AsyncPipeline<P>`：异步执行器（SSE 友好，需 `async` feature）
//!
//! # 设计原则
//!
//! - 单一管线核心：所有管线共享同一骨架
//! - 阶段可插拔：每个阶段是一个 PhaseHandler，可注册可替换
//! - 上下文贯穿：PipelineContext 在各阶段间流动
//! - 钩子机制：pre_phase / post_phase 瀑布扩展点
//! - 审计统一：统一审计事件模型，内部链 + 外部 sink 双写
//! - 同步/异步统一：核心支持两种执行模式
//! - 泛型阶段标识：通过 `PhaseId` trait 与具体领域阶段解耦

use std::time::Instant;

use crate::context::PipelineContext;
use crate::error::PipelineError;
#[cfg(feature = "async")]
use crate::events::PhaseEvent;
use crate::hooks::{builtin_hooks, HookRegistry, HookResult};
use crate::phase::{PhaseHandler, PhaseId, PhaseStatus};

// ================== Pipeline trait ==================

/// 管线 trait：统一的管线抽象
///
/// 所有管线（优化管线、分析管线、处理管线等）都实现此 trait，
/// 对外提供一致的执行接口。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub trait Pipeline<P: PhaseId> {
    /// 管线名称（用于日志、监控、审计）
    fn name(&self) -> &str;

    /// 获取阶段列表（按执行顺序）
    fn phases(&self) -> &[P];

    /// 获取阶段处理器
    fn handler(&self, phase: &P) -> Option<&dyn PhaseHandler<P>>;

    /// 获取钩子注册表
    fn hooks(&self) -> &HookRegistry<P>;

    /// 是否为异步管线
    fn is_async(&self) -> bool;
}

// ================== PipelineBuilder ==================

/// 管线构建器
///
/// 使用构建器模式组装管线：
/// ```ignore
/// let pipeline = PipelineBuilder::<NamedPhase>::new("my_pipeline")
///     .with_phase(Box::new(NormalizeHandler))
///     .with_phase(Box::new(AnalyzeHandler))
///     .with_phase(Box::new(GateHandler))
///     .with_phase(Box::new(DoneHandler))
///     .with_default_hooks()
///     .build_sync();
/// ```
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub struct PipelineBuilder<P: PhaseId> {
    name: String,
    phases: Vec<P>,
    handlers: Vec<Box<dyn PhaseHandler<P>>>,
    hooks: HookRegistry<P>,
}

impl<P: PhaseId> PipelineBuilder<P> {
    /// 创建新的构建器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            phases: Vec::new(),
            handlers: Vec::new(),
            hooks: HookRegistry::new(),
        }
    }

    /// 注册一个阶段处理器（按注册顺序执行）
    pub fn with_phase(mut self, handler: Box<dyn PhaseHandler<P>>) -> Self {
        let phase = handler.phase();
        self.phases.push(phase);
        self.handlers.push(handler);
        self
    }

    /// 注册多个阶段处理器
    pub fn with_phases(mut self, handlers: Vec<Box<dyn PhaseHandler<P>>>) -> Self {
        for h in handlers {
            let phase = h.phase();
            self.phases.push(phase);
            self.handlers.push(h);
        }
        self
    }

    /// 装载默认钩子（审计 + 日志 + 指标）
    pub fn with_default_hooks(mut self) -> Self {
        self.hooks.on_pre_pipeline(builtin_hooks::audit_hook());
        self.hooks.on_post_pipeline(builtin_hooks::audit_hook());
        self.hooks.on_pre_pipeline(builtin_hooks::tracing_hook());
        self.hooks.on_post_pipeline(builtin_hooks::tracing_hook());

        // 注意：为每个阶段注册的钩子在 build 时补充
        self
    }

    /// 注册一个 pre-phase 钩子
    pub fn with_pre_phase_hook(mut self, phase: P, hook: crate::hooks::WaterfallHook<P>) -> Self {
        self.hooks.on_pre_phase(phase, hook);
        self
    }

    /// 注册一个 post-phase 钩子
    pub fn with_post_phase_hook(mut self, phase: P, hook: crate::hooks::WaterfallHook<P>) -> Self {
        self.hooks.on_post_phase(phase, hook);
        self
    }

    /// 注册 pre-pipeline 钩子
    pub fn with_pre_pipeline_hook(mut self, hook: crate::hooks::WaterfallHook<P>) -> Self {
        self.hooks.on_pre_pipeline(hook);
        self
    }

    /// 注册 post-pipeline 钩子
    pub fn with_post_pipeline_hook(mut self, hook: crate::hooks::WaterfallHook<P>) -> Self {
        self.hooks.on_post_pipeline(hook);
        self
    }

    /// 构建同步管线
    pub fn build_sync(mut self) -> SyncPipeline<P> {
        self.ensure_phase_hooks();
        SyncPipeline {
            name: self.name,
            phases: self.phases,
            handlers: self.handlers,
            hooks: self.hooks,
        }
    }

    /// 构建异步管线
    #[cfg(feature = "async")]
    pub fn build_async(mut self) -> AsyncPipeline<P> {
        self.ensure_phase_hooks();
        AsyncPipeline {
            name: self.name,
            phases: self.phases,
            handlers: self.handlers,
            hooks: self.hooks,
        }
    }

    // 为每个阶段补充审计和日志钩子
    fn ensure_phase_hooks(&mut self) {
        let audit_hook = builtin_hooks::audit_hook();
        let tracing_hook = builtin_hooks::tracing_hook();
        let metrics_hook = builtin_hooks::metrics_hook();
        for phase in &self.phases {
            self.hooks
                .on_pre_phase(phase.clone(), audit_hook.clone());
            self.hooks
                .on_post_phase(phase.clone(), audit_hook.clone());
            self.hooks
                .on_pre_phase(phase.clone(), tracing_hook.clone());
            self.hooks
                .on_post_phase(phase.clone(), tracing_hook.clone());
            self.hooks
                .on_post_phase(phase.clone(), metrics_hook.clone());
        }
    }
}

// ================== SyncPipeline ==================

/// 同步管线执行器
///
/// 顺序执行所有阶段，一次性返回最终上下文。
/// 适用于批处理、离线计算等场景。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub struct SyncPipeline<P: PhaseId> {
    name: String,
    phases: Vec<P>,
    handlers: Vec<Box<dyn PhaseHandler<P>>>,
    hooks: HookRegistry<P>,
}

impl<P: PhaseId> SyncPipeline<P> {
    /// 同步执行完整管线
    pub fn run(&self, mut ctx: PipelineContext<P>) -> Result<PipelineContext<P>, PipelineError<P>> {
        // 1. PrePipeline 钩子
        self.run_hooks(|h| h.run_pre_pipeline(&mut ctx), "pre_pipeline");

        // 2. 逐阶段执行
        for i in 0..self.phases.len() {
            let phase = &self.phases[i];
            let handler = self.handlers[i].as_ref();
            self.run_phase_sync(phase, handler, &mut ctx)?;

            // 检查是否被阻断
            if ctx.is_blocked() {
                tracing::info!(
                    target: "pipeline",
                    pipeline = self.name,
                    phase = phase.name(),
                    "pipeline blocked at phase"
                );
                break;
            }
        }

        // 3. PostPipeline 钩子
        self.run_hooks(|h| h.run_post_pipeline(&mut ctx), "post_pipeline");

        Ok(ctx)
    }

    fn run_phase_sync(
        &self,
        phase: &P,
        handler: &dyn PhaseHandler<P>,
        ctx: &mut PipelineContext<P>,
    ) -> Result<(), PipelineError<P>> {
        // 检查是否跳过
        if handler.should_skip(ctx) {
            ctx.mark_phase_start(phase.clone());
            ctx.mark_phase_end(phase, PhaseStatus::Skipped, 0);
            return Ok(());
        }

        // PrePhase 钩子
        self.run_hooks(
            |h| h.run_pre_phase(phase, ctx),
            &format!("pre_{}", phase.name()),
        );

        // 执行阶段
        ctx.mark_phase_start(phase.clone());
        let start = Instant::now();

        let result = handler.execute(ctx);
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(phase_result) => {
                let success = phase_result.success();
                let status = if success {
                    PhaseStatus::Success
                } else {
                    PhaseStatus::Failed
                };
                ctx.mark_phase_end(phase, status, latency);
                ctx.set_result(phase_result);

                // 阻断性阶段检查
                if phase.is_blocking() && !success {
                    ctx.mark_phase_end(phase, PhaseStatus::Blocked, latency);
                    // 不返回错误，阻断是预期结果
                }
            }
            Err(e) => {
                ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                return Err(PipelineError::PhaseFailed {
                    phase: phase.clone(),
                    message: e,
                });
            }
        }

        // PostPhase 钩子
        self.run_hooks(
            |h| h.run_post_phase(phase, ctx),
            &format!("post_{}", phase.name()),
        );

        Ok(())
    }

    fn run_hooks<F>(&self, f: F, hook_name: &str)
    where
        F: FnOnce(&HookRegistry<P>) -> HookResult,
    {
        if let Err(e) = f(&self.hooks) {
            tracing::warn!(
                target: "pipeline",
                pipeline = self.name,
                hook = hook_name,
                error = %e,
                "hook execution failed"
            );
            // 钩子失败不阻断管线，只记录警告
        }
    }
}

impl<P: PhaseId> Pipeline<P> for SyncPipeline<P> {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> &[P] {
        &self.phases
    }

    fn handler(&self, phase: &P) -> Option<&dyn PhaseHandler<P>> {
        self.phases
            .iter()
            .position(|p| p == phase)
            .map(|i| self.handlers[i].as_ref())
    }

    fn hooks(&self) -> &HookRegistry<P> {
        &self.hooks
    }

    fn is_async(&self) -> bool {
        false
    }
}

// ================== AsyncPipeline ==================

/// 异步管线执行器
///
/// 逐阶段异步执行，每个阶段完成后产生一个事件。
/// 适用于 SSE 流式输出、实时进度展示等场景。
///
/// 需要启用 `async` feature。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[cfg(feature = "async")]
pub struct AsyncPipeline<P: PhaseId> {
    name: String,
    phases: Vec<P>,
    handlers: Vec<Box<dyn PhaseHandler<P>>>,
    hooks: HookRegistry<P>,
}

#[cfg(feature = "async")]
impl<P: PhaseId> AsyncPipeline<P> {
    /// 异步执行完整管线，返回阶段结果流
    ///
    /// 返回 (最终上下文, 阶段事件列表)。
    /// 在实际使用中，可基于此实现 Stream 接口用于 SSE 推送。
    pub async fn run(
        &self,
        mut ctx: PipelineContext<P>,
    ) -> Result<(PipelineContext<P>, Vec<PhaseEvent<P>>), PipelineError<P>> {
        let mut events = Vec::new();

        // 1. PrePipeline 钩子
        let _ = self.hooks.run_pre_pipeline(&mut ctx);

        // 2. 逐阶段执行
        for i in 0..self.phases.len() {
            let phase = &self.phases[i];
            let handler = self.handlers[i].as_ref();
            let event = self.run_phase_async(phase, handler, &mut ctx).await?;
            events.push(event.clone());

            // 检查是否被阻断
            if matches!(event, PhaseEvent::Blocked { .. }) {
                tracing::info!(
                    target: "pipeline",
                    pipeline = self.name,
                    phase = phase.name(),
                    "pipeline blocked at phase"
                );
                break;
            }
        }

        // 3. PostPipeline 钩子
        let _ = self.hooks.run_post_pipeline(&mut ctx);

        Ok((ctx, events))
    }

    async fn run_phase_async(
        &self,
        phase: &P,
        handler: &dyn PhaseHandler<P>,
        ctx: &mut PipelineContext<P>,
    ) -> Result<PhaseEvent<P>, PipelineError<P>> {
        // 检查是否跳过
        if handler.should_skip(ctx) {
            ctx.mark_phase_start(phase.clone());
            ctx.mark_phase_end(phase, PhaseStatus::Skipped, 0);
            return Ok(PhaseEvent::Skipped {
                phase: phase.clone(),
            });
        }

        // PrePhase 钩子
        let _ = self.hooks.run_pre_phase(phase, ctx);

        // 执行阶段
        ctx.mark_phase_start(phase.clone());
        let start = Instant::now();

        let result = handler.execute_async(ctx).await;
        let latency = start.elapsed().as_millis() as u64;

        let event = match result {
            Ok(phase_result) => {
                let success = phase_result.success();
                let payload = phase_result.payload();
                ctx.set_result(phase_result);

                if phase.is_blocking() && !success {
                    ctx.mark_phase_end(phase, PhaseStatus::Blocked, latency);
                    PhaseEvent::Blocked {
                        phase: phase.clone(),
                        latency_ms: latency,
                        payload,
                    }
                } else if success {
                    ctx.mark_phase_end(phase, PhaseStatus::Success, latency);
                    PhaseEvent::Success {
                        phase: phase.clone(),
                        latency_ms: latency,
                        payload,
                    }
                } else {
                    ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                    PhaseEvent::Failed {
                        phase: phase.clone(),
                        latency_ms: latency,
                        payload,
                    }
                }
            }
            Err(e) => {
                ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                return Err(PipelineError::PhaseFailed {
                    phase: phase.clone(),
                    message: e,
                });
            }
        };

        // PostPhase 钩子
        let _ = self.hooks.run_post_phase(phase, ctx);

        Ok(event)
    }
}

#[cfg(feature = "async")]
impl<P: PhaseId> Pipeline<P> for AsyncPipeline<P> {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> &[P] {
        &self.phases
    }

    fn handler(&self, phase: &P) -> Option<&dyn PhaseHandler<P>> {
        self.phases
            .iter()
            .position(|p| p == phase)
            .map(|i| self.handlers[i].as_ref())
    }

    fn hooks(&self) -> &HookRegistry<P> {
        &self.hooks
    }

    fn is_async(&self) -> bool {
        true
    }
}

// ================== 单元测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{PipelineInput, PipelineOptions};
    use crate::phase::NamedPhase;
    use crate::result::{GenericPhaseResult, PhaseResult};

    // 测试用的阶段处理器
    struct TestHandler {
        phase: NamedPhase,
        should_skip: bool,
        succeed: bool,
    }

    impl TestHandler {
        fn new(phase: NamedPhase) -> Self {
            Self {
                phase,
                should_skip: false,
                succeed: true,
            }
        }
        fn with_skip(mut self, skip: bool) -> Self {
            self.should_skip = skip;
            self
        }
        fn with_success(mut self, succeed: bool) -> Self {
            self.succeed = succeed;
            self
        }
    }

    impl PhaseHandler<NamedPhase> for TestHandler {
        fn phase(&self) -> NamedPhase {
            self.phase.clone()
        }

        fn execute(
            &self,
            _ctx: &mut PipelineContext<NamedPhase>,
        ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
            if self.succeed {
                Ok(Box::new(GenericPhaseResult::success(
                    self.phase.clone(),
                    serde_json::json!({"processed": true}),
                    10,
                )))
            } else {
                Ok(Box::new(GenericPhaseResult::failed(
                    self.phase.clone(),
                    "simulated failure",
                    10,
                )))
            }
        }

        fn should_skip(&self, _ctx: &PipelineContext<NamedPhase>) -> bool {
            self.should_skip
        }
    }

    fn make_ctx() -> PipelineContext<NamedPhase> {
        PipelineContext::new(
            PipelineInput::Query {
                query: "test".into(),
                session_id: None,
                context: std::collections::HashMap::new(),
            },
            PipelineOptions::default(),
        )
    }

    #[test]
    fn sync_pipeline_runs_all_phases() {
        let pipeline = PipelineBuilder::new("test_pipeline")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("analyze"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::terminal("done"))))
            .build_sync();

        assert_eq!(pipeline.name(), "test_pipeline");
        assert!(!pipeline.is_async());

        let ctx = pipeline.run(make_ctx()).unwrap();
        assert_eq!(ctx.completed_phases().len(), 3);
        assert!(ctx.all_succeeded());
        assert!(!ctx.is_blocked());
    }

    #[test]
    fn sync_pipeline_audit_events() {
        let pipeline = PipelineBuilder::new("audit_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("analyze"))))
            .with_default_hooks()
            .build_sync();

        let ctx = pipeline.run(make_ctx()).unwrap();

        // 每个阶段有 start + end 两个审计事件
        assert!(ctx.audit.len() > 0);
        assert!(ctx.audit.verify());
    }

    #[test]
    fn gate_phase_blocks_pipeline() {
        // Gate 阶段返回失败（未通过），应阻断管线
        struct GateHandler;
        impl PhaseHandler<NamedPhase> for GateHandler {
            fn phase(&self) -> NamedPhase {
                NamedPhase::blocking("gate")
            }
            fn execute(
                &self,
                _ctx: &mut PipelineContext<NamedPhase>,
            ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
                // 闸门未通过：success=false，且阶段是阻断性的
                Ok(Box::new(GenericPhaseResult::failed(
                    NamedPhase::blocking("gate"),
                    "quality too low",
                    50,
                )))
            }
        }

        let pipeline = PipelineBuilder::new("gate_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(GateHandler))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("done")))) // 不应执行
            .build_sync();

        let ctx = pipeline.run(make_ctx()).unwrap();
        assert!(ctx.is_blocked());
        // Done 阶段不应被执行
        assert!(ctx.get_result(&NamedPhase::new("done")).is_none());
    }

    #[test]
    fn skipped_phase_is_recorded() {
        let pipeline = PipelineBuilder::new("skip_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(
                TestHandler::new(NamedPhase::new("learn")).with_skip(true),
            ))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("done"))))
            .build_sync();

        let ctx = pipeline.run(make_ctx()).unwrap();
        assert!(ctx.all_succeeded()); // 跳过也算成功

        let exec = ctx.get_execution(&NamedPhase::new("learn")).unwrap();
        assert_eq!(exec.status, PhaseStatus::Skipped);
    }

    #[test]
    fn pipeline_handler_lookup() {
        let pipeline = PipelineBuilder::new("lookup_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("analyze"))))
            .build_sync();

        assert!(pipeline.handler(&NamedPhase::new("analyze")).is_some());
        assert!(pipeline.handler(&NamedPhase::new("nonexistent")).is_none());
    }

    #[test]
    fn pipeline_builder_with_phases() {
        let handlers: Vec<Box<dyn PhaseHandler<NamedPhase>>> = vec![
            Box::new(TestHandler::new(NamedPhase::new("normalize"))),
            Box::new(TestHandler::new(NamedPhase::new("analyze"))),
            Box::new(TestHandler::new(NamedPhase::new("done"))),
        ];

        let pipeline = PipelineBuilder::new("multi_test")
            .with_phases(handlers)
            .build_sync();

        assert_eq!(pipeline.phases().len(), 3);
    }

    #[test]
    fn custom_hook_modifies_context() {
        use crate::hooks::WaterfallHook;

        let hook: WaterfallHook<NamedPhase> = std::sync::Arc::new(|_event, ctx, next| {
            ctx.set_bag("hook_called", true);
            next(ctx)
        });

        let pipeline = PipelineBuilder::new("hook_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("analyze"))))
            .with_pre_pipeline_hook(hook)
            .build_sync();

        let ctx = pipeline.run(make_ctx()).unwrap();
        assert_eq!(ctx.get_bag::<bool>("hook_called"), Some(&true));
    }

    #[test]
    fn pipeline_phases_return_safe_slice() {
        let pipeline = PipelineBuilder::new("safe_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("a"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("b"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("c"))))
            .build_sync();

        let phases = pipeline.phases();
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0].name(), "a");
        assert_eq!(phases[1].name(), "b");
        assert_eq!(phases[2].name(), "c");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_pipeline_runs_all_phases() {
        let pipeline = PipelineBuilder::new("async_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("analyze"))))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("done"))))
            .build_async();

        assert!(pipeline.is_async());

        let (ctx, events) = pipeline.run(make_ctx()).await.unwrap();
        assert_eq!(events.len(), 3);
        assert!(ctx.all_succeeded());

        // 所有事件都应该是成功的
        for evt in &events {
            assert!(evt.is_success());
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_pipeline_gate_blocks() {
        struct GateHandler;
        impl PhaseHandler<NamedPhase> for GateHandler {
            fn phase(&self) -> NamedPhase {
                NamedPhase::blocking("gate")
            }
            fn execute(
                &self,
                _ctx: &mut PipelineContext<NamedPhase>,
            ) -> Result<Box<dyn PhaseResult<NamedPhase>>, String> {
                Ok(Box::new(GenericPhaseResult::failed(
                    NamedPhase::blocking("gate"),
                    "blocked",
                    50,
                )))
            }
        }

        let pipeline = PipelineBuilder::new("async_gate_test")
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("normalize"))))
            .with_phase(Box::new(GateHandler))
            .with_phase(Box::new(TestHandler::new(NamedPhase::new("done"))))
            .build_async();

        let (_ctx, events) = pipeline.run(make_ctx()).await.unwrap();
        // 应该只有 2 个事件：Normalize + Gate（被阻断，Done 不执行）
        assert_eq!(events.len(), 2);
        assert!(events[1].is_blocked());
    }
}
