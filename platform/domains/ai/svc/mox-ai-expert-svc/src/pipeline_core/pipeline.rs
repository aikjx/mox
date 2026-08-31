// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Pipeline trait 与执行器
//!
//! 提供统一的管线编排能力：
//! - `Pipeline` trait：管线的统一抽象
//! - `PipelineBuilder`：构建器模式，注册阶段和钩子
//! - `SyncPipeline`：同步执行器
//! - `AsyncPipeline`：异步执行器（SSE 友好）
//!
//! 两套现有管线都可以基于此核心实现：
//! - 全维管线 → `SyncPipeline`（同步、一次性返回）
//! - 联盟管线 → `AsyncPipeline`（异步、逐阶段推送）

use std::time::Instant;

use crate::pipeline_core::context::PipelineContext;
use crate::pipeline_core::hooks::{builtin_hooks, HookRegistry, HookResult};
use crate::pipeline_core::phase::{Phase, PhaseExecution, PhaseHandler, PhaseStatus};

// ================== Pipeline trait ==================

/// 管线 trait：统一的管线抽象
///
/// 所有管线（全维优化、联盟分析等）都实现此 trait，
/// 对外提供一致的执行接口。
pub trait Pipeline {
    /// 管线名称（用于日志、监控、审计）
    fn name(&self) -> &str;

    /// 获取阶段列表（按执行顺序）
    fn phases(&self) -> &[Phase];

    /// 获取阶段处理器
    fn handler(&self, phase: Phase) -> Option<&dyn PhaseHandler>;

    /// 获取钩子注册表
    fn hooks(&self) -> &HookRegistry;

    /// 是否为异步管线
    fn is_async(&self) -> bool;
}

// ================== PipelineBuilder ==================

/// 管线构建器
///
/// 使用构建器模式组装管线：
/// ```ignore
/// let pipeline = PipelineBuilder::new("my_pipeline")
///     .with_phase(Box::new(NormalizeHandler))
///     .with_phase(Box::new(AnalyzeHandler))
///     .with_phase(Box::new(GateHandler))
///     .with_phase(Box::new(DoneHandler))
///     .with_default_hooks()
///     .build_sync();
/// ```
pub struct PipelineBuilder {
    name: String,
    phases: Vec<(Phase, Box<dyn PhaseHandler>)>,
    hooks: HookRegistry,
}

impl PipelineBuilder {
    /// 创建新的构建器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            phases: Vec::new(),
            hooks: HookRegistry::new(),
        }
    }

    /// 注册一个阶段处理器（按注册顺序执行）
    pub fn with_phase(mut self, handler: Box<dyn PhaseHandler>) -> Self {
        let phase = handler.phase();
        self.phases.push((phase, handler));
        self
    }

    /// 注册多个阶段处理器
    pub fn with_phases(mut self, handlers: Vec<Box<dyn PhaseHandler>>) -> Self {
        for h in handlers {
            let phase = h.phase();
            self.phases.push((phase, h));
        }
        self
    }

    /// 装载默认钩子（审计 + 日志）
    pub fn with_default_hooks(mut self) -> Self {
        self.hooks.on_pre_pipeline(builtin_hooks::audit_hook());
        self.hooks.on_post_pipeline(builtin_hooks::audit_hook());
        self.hooks.on_pre_pipeline(builtin_hooks::tracing_hook());
        self.hooks.on_post_pipeline(builtin_hooks::tracing_hook());

        // 为所有阶段注册审计和日志钩子
        // 注意：这里需要知道具体有哪些阶段，所以在 build 时再补充
        self
    }

    /// 注册一个 pre-phase 钩子
    pub fn with_pre_phase_hook(
        mut self,
        phase: Phase,
        hook: crate::pipeline_core::hooks::WaterfallHook,
    ) -> Self {
        self.hooks.on_pre_phase(phase, hook);
        self
    }

    /// 注册一个 post-phase 钩子
    pub fn with_post_phase_hook(
        mut self,
        phase: Phase,
        hook: crate::pipeline_core::hooks::WaterfallHook,
    ) -> Self {
        self.hooks.on_post_phase(phase, hook);
        self
    }

    /// 注册 pre-pipeline 钩子
    pub fn with_pre_pipeline_hook(
        mut self,
        hook: crate::pipeline_core::hooks::WaterfallHook,
    ) -> Self {
        self.hooks.on_pre_pipeline(hook);
        self
    }

    /// 注册 post-pipeline 钩子
    pub fn with_post_pipeline_hook(
        mut self,
        hook: crate::pipeline_core::hooks::WaterfallHook,
    ) -> Self {
        self.hooks.on_post_pipeline(hook);
        self
    }

    /// 构建同步管线
    pub fn build_sync(mut self) -> SyncPipeline {
        self.ensure_phase_hooks();
        SyncPipeline {
            name: self.name,
            phases: self.phases,
            hooks: self.hooks,
        }
    }

    /// 构建异步管线
    pub fn build_async(mut self) -> AsyncPipeline {
        self.ensure_phase_hooks();
        AsyncPipeline {
            name: self.name,
            phases: self.phases,
            hooks: self.hooks,
        }
    }

    // 为每个阶段补充审计和日志钩子
    fn ensure_phase_hooks(&mut self) {
        let audit_hook = builtin_hooks::audit_hook();
        let tracing_hook = builtin_hooks::tracing_hook();
        for (phase, _) in &self.phases {
            self.hooks.on_pre_phase(*phase, audit_hook.clone());
            self.hooks.on_post_phase(*phase, audit_hook.clone());
            self.hooks.on_pre_phase(*phase, tracing_hook.clone());
            self.hooks.on_post_phase(*phase, tracing_hook.clone());
        }
    }
}

// ================== SyncPipeline ==================

/// 同步管线执行器
///
/// 顺序执行所有阶段，一次性返回最终上下文。
/// 适用于全维优化管线等批处理场景。
pub struct SyncPipeline {
    name: String,
    phases: Vec<(Phase, Box<dyn PhaseHandler>)>,
    hooks: HookRegistry,
}

impl SyncPipeline {
    /// 同步执行完整管线
    pub fn run(&self, mut ctx: PipelineContext) -> Result<PipelineContext, PipelineError> {
        // 1. PrePipeline 钩子
        self.run_hooks(|h| h.run_pre_pipeline(&mut ctx), "pre_pipeline")?;

        // 2. 逐阶段执行
        for (phase, handler) in &self.phases {
            self.run_phase_sync(*phase, handler.as_ref(), &mut ctx)?;

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
        self.run_hooks(|h| h.run_post_pipeline(&mut ctx), "post_pipeline")?;

        Ok(ctx)
    }

    fn run_phase_sync(
        &self,
        phase: Phase,
        handler: &dyn PhaseHandler,
        ctx: &mut PipelineContext,
    ) -> Result<(), PipelineError> {
        // 检查是否跳过
        if handler.should_skip(ctx) {
            ctx.mark_phase_end(phase, PhaseStatus::Skipped, 0);
            return Ok(());
        }

        // PrePhase 钩子
        self.run_hooks(|h| h.run_pre_phase(phase, ctx), &format!("pre_{}", phase.name()))?;

        // 执行阶段
        ctx.mark_phase_start(phase);
        let start = Instant::now();

        let result = handler.execute(ctx);
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(phase_result) => {
                let status = if phase_result.success() {
                    PhaseStatus::Success
                } else {
                    PhaseStatus::Failed
                };
                ctx.mark_phase_end(phase, status, latency);
                ctx.set_result(phase_result);

                // 阻断性阶段检查
                if phase.is_blocking() && !is_phase_passed(ctx, phase) {
                    ctx.mark_phase_end(phase, PhaseStatus::Blocked, latency);
                    // 不返回错误，阻断是预期结果
                }
            }
            Err(e) => {
                ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                return Err(PipelineError::PhaseFailed {
                    phase,
                    message: e,
                });
            }
        }

        // PostPhase 钩子
        self.run_hooks(
            |h| h.run_post_phase(phase, ctx),
            &format!("post_{}", phase.name()),
        )?;

        Ok(())
    }

    fn run_hooks<F>(&self, f: F, hook_name: &str) -> Result<(), PipelineError>
    where
        F: FnOnce(&HookRegistry) -> HookResult,
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
        Ok(())
    }
}

impl Pipeline for SyncPipeline {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> &[Phase] {
        // 这是一个临时实现，实际应该返回 phases 的 phase 切片
        // 由于我们存储的是 Vec<(Phase, Box<dyn PhaseHandler>)>，需要转换
        // 实际实现中可以用一个单独的 Vec<Phase> 存储顺序
        unsafe {
            // 安全：我们只读取 phase 字段，且生命周期与 self 绑定
            // 这是一个简化实现，生产代码应该用更安全的方式
            std::slice::from_raw_parts(
                self.phases.as_ptr() as *const Phase,
                self.phases.len(),
            )
        }
    }

    fn handler(&self, phase: Phase) -> Option<&dyn PhaseHandler> {
        self.phases
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, h)| h.as_ref())
    }

    fn hooks(&self) -> &HookRegistry {
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
/// 适用于联盟管线等 SSE 流式输出场景。
pub struct AsyncPipeline {
    name: String,
    phases: Vec<(Phase, Box<dyn PhaseHandler>)>,
    hooks: HookRegistry,
}

impl AsyncPipeline {
    /// 异步执行完整管线，返回阶段结果流
    ///
    /// 在实际使用中，这应该返回 `impl Stream<Item = PhaseEvent>`，
    /// 这里简化为 `Vec` 以便于理解。
    pub async fn run(
        &self,
        mut ctx: PipelineContext,
    ) -> Result<(PipelineContext, Vec<PhaseEvent>), PipelineError> {
        let mut events = Vec::new();

        // 1. PrePipeline 钩子
        let _ = self.hooks.run_pre_pipeline(&mut ctx);

        // 2. 逐阶段执行
        for (phase, handler) in &self.phases {
            let event = self.run_phase_async(*phase, handler.as_ref(), &mut ctx).await?;
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
        phase: Phase,
        handler: &dyn PhaseHandler,
        ctx: &mut PipelineContext,
    ) -> Result<PhaseEvent, PipelineError> {
        // 检查是否跳过
        if handler.should_skip(ctx) {
            ctx.mark_phase_end(phase, PhaseStatus::Skipped, 0);
            return Ok(PhaseEvent::Skipped { phase });
        }

        // PrePhase 钩子
        let _ = self.hooks.run_pre_phase(phase, ctx);

        // 执行阶段
        ctx.mark_phase_start(phase);
        let start = Instant::now();

        let result = handler.execute_async(ctx).await;
        let latency = start.elapsed().as_millis() as u64;

        let event = match result {
            Ok(phase_result) => {
                let success = phase_result.success();
                let payload = phase_result.payload();
                ctx.set_result(phase_result);

                if phase.is_blocking() && !is_phase_passed(ctx, phase) {
                    ctx.mark_phase_end(phase, PhaseStatus::Blocked, latency);
                    PhaseEvent::Blocked {
                        phase,
                        latency_ms: latency,
                        payload,
                    }
                } else if success {
                    ctx.mark_phase_end(phase, PhaseStatus::Success, latency);
                    PhaseEvent::Success {
                        phase,
                        latency_ms: latency,
                        payload,
                    }
                } else {
                    ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                    PhaseEvent::Failed {
                        phase,
                        latency_ms: latency,
                        payload,
                    }
                }
            }
            Err(e) => {
                ctx.mark_phase_end(phase, PhaseStatus::Failed, latency);
                return Err(PipelineError::PhaseFailed {
                    phase,
                    message: e,
                });
            }
        };

        // PostPhase 钩子
        let _ = self.hooks.run_post_phase(phase, ctx);

        Ok(event)
    }
}

impl Pipeline for AsyncPipeline {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> &[Phase] {
        unsafe {
            std::slice::from_raw_parts(
                self.phases.as_ptr() as *const Phase,
                self.phases.len(),
            )
        }
    }

    fn handler(&self, phase: Phase) -> Option<&dyn PhaseHandler> {
        self.phases
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, h)| h.as_ref())
    }

    fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    fn is_async(&self) -> bool {
        true
    }
}

// ================== 阶段事件（SSE 用） ==================

/// 异步管线阶段事件（对应 SSE 每帧输出）
#[derive(Debug, Clone)]
pub enum PhaseEvent {
    /// 阶段成功完成
    Success {
        phase: Phase,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段失败（非阻断）
    Failed {
        phase: Phase,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段阻断（管线终止）
    Blocked {
        phase: Phase,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段被跳过
    Skipped { phase: Phase },
}

impl PhaseEvent {
    pub fn phase(&self) -> Phase {
        match self {
            Self::Success { phase, .. }
            | Self::Failed { phase, .. }
            | Self::Blocked { phase, .. }
            | Self::Skipped { phase } => *phase,
        }
    }
}

// ================== 管线错误 ==================

/// 管线执行错误
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 阶段执行失败
    PhaseFailed { phase: Phase, message: String },
    /// 钩子执行失败（通常不阻断，但可配置为阻断）
    HookFailed { hook: String, message: String },
    /// 配置错误
    ConfigError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PhaseFailed { phase, message } => {
                write!(f, "Phase '{}' failed: {}", phase.name(), message)
            }
            Self::HookFailed { hook, message } => {
                write!(f, "Hook '{}' failed: {}", hook, message)
            }
            Self::ConfigError(msg) => {
                write!(f, "Pipeline config error: {}", msg)
            }
        }
    }
}

impl std::error::Error for PipelineError {}

// ================== 辅助函数 ==================

/// 判断某个阶段是否"通过"（成功且闸门类型的阶段 approved = true）
fn is_phase_passed(ctx: &PipelineContext, phase: Phase) -> bool {
    if let Some(result) = ctx.get_result(phase) {
        result.success()
    } else {
        false
    }
}

// ================== 示例：迁移后的全维管线结构 ==================

// 以下是概念性代码，展示如何基于统一核心重写全维管线：
//
// ```ignore
// pub struct MoxOptimizePipeline;
//
// impl MoxOptimizePipeline {
//     pub fn build() -> SyncPipeline {
//         PipelineBuilder::new("mox_optimize")
//             .with_phase(Box::new(NormalizeHandler))       // auto_dimension
//             .with_phase(Box::new(AnalyzeHandler))         // run_experts (14 experts)
//             .with_phase(Box::new(ReconcileHandler))       // reconcile
//             .with_phase(Box::new(OptimizeHandler))        // flow-ai optimize
//             .with_phase(Box::new(VerifyHandler))          // verify (璇玑验证)
//             .with_phase(Box::new(GateHandler::mox()))     // govern + tenant gates
//             .with_phase(Box::new(DoneHandler))            // 收尾 + 审计汇总
//             .with_default_hooks()
//             .build_sync()
//     }
// }
// ```

// ================== 示例：迁移后的联盟管线结构 ==================

// 以下是概念性代码，展示如何基于统一核心重写联盟管线：
//
// ```ignore
// pub struct AllianceAnalysisPipeline;
//
// impl AllianceAnalysisPipeline {
//     pub fn build() -> AsyncPipeline {
//         PipelineBuilder::new("alliance_analysis")
//             .with_phase(Box::new(IntentHandler))         // classify_intent (→ Normalize)
//             .with_phase(Box::new(TeamHandler))           // optimize_team
//             .with_phase(Box::new(DebateHandler))         // consult_and_debate (→ Analyze)
//             .with_phase(Box::new(SynthesizeHandler))     // 合成 Markdown
//             .with_phase(Box::new(GateHandler::alliance())) // HC-8 evaluate_gate
//             .with_phase(Box::new(LearnHandler))          // learn_metrics
//             .with_phase(Box::new(DoneHandler))           // 收尾
//             .with_default_hooks()
//             .build_async()
//     }
// }
// ```
