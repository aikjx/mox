// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 钩子机制（Hook System）
//!
//! 从 `harness.rs` 的 Waterfall 模式泛化而来，支持在每个阶段前后插入自定义逻辑。
//!
//! 核心思想：责任链（Chain of Responsibility）
//! - 每个钩子可以修改上下文状态
//! - 每个钩子必须调用 `next` 委托给下一个钩子
//! - 不调用 `next` 即为短路（中断后续钩子）
//!
//! 钩子类型：
//! - `PrePhase(Phase)` : 阶段执行前，可修改输入、补充上下文
//! - `PostPhase(Phase)`: 阶段执行后，可修改结果、追加审计
//! - `PrePipeline`     : 管线启动前
//! - `PostPipeline`    : 管线结束后

use std::sync::Arc;

use crate::pipeline_core::context::PipelineContext;
use crate::pipeline_core::phase::Phase;

// ================== 钩子事件 ==================

/// 钩子事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    /// 管线启动前
    PrePipeline,
    /// 管线结束后
    PostPipeline,
    /// 阶段执行前
    PrePhase(Phase),
    /// 阶段执行后
    PostPhase(Phase),
}

impl HookEvent {
    pub fn name(&self) -> String {
        match self {
            Self::PrePipeline => "pre_pipeline".to_string(),
            Self::PostPipeline => "post_pipeline".to_string(),
            Self::PrePhase(p) => format!("pre_{}", p.name()),
            Self::PostPhase(p) => format!("post_{}", p.name()),
        }
    }
}

// ================== 钩子错误 ==================

/// 钩子执行错误
#[derive(Debug, Clone)]
pub struct HookError {
    pub event: HookEvent,
    pub message: String,
}

impl HookError {
    pub fn new(event: HookEvent, message: impl Into<String>) -> Self {
        Self {
            event,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook error at {:?}: {}", self.event, self.message)
    }
}

impl std::error::Error for HookError {}

/// 钩子执行结果
pub type HookResult = Result<(), HookError>;

// ================== 钩子处理器 ==================

/// 瀑布钩子处理器
///
/// 语义（对照 deepseek-harness 的 waterfall）：
/// 处理器**必须**调用传入的 `next` 闭包把控制权委托给责任链的下一环；
/// 若不调用 `next` 即视为短路（中断后续处理器）。
///
/// `next` 接收可变的上下文并向下游传递，其返回值是下游处理结果，
/// 处理器可对其拦截/改写后向上传递。
pub type WaterfallHook = Arc<
    dyn Fn(
            HookEvent,
            &mut PipelineContext,
            &dyn Fn(&mut PipelineContext) -> HookResult,
        ) -> HookResult
        + Send
        + Sync,
>;

// ================== 钩子链 ==================

/// 钩子链：管理某一事件的所有钩子，构成责任链
#[derive(Clone)]
pub struct HookChain {
    hooks: Vec<WaterfallHook>,
}

impl Default for HookChain {
    fn default() -> Self {
        Self::new()
    }
}

impl HookChain {
    /// 创建空的钩子链
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 添加一个钩子（按添加顺序执行）
    pub fn add(&mut self, hook: WaterfallHook) {
        self.hooks.push(hook);
    }

    /// 运行钩子链：从首环开始，逐个调用处理器并传入 `next` 委托闭包。
    ///
    /// 处理器若不调用 `next` 即短路；下游错误会沿调用栈向上传播。
    pub fn run(&self, event: HookEvent, ctx: &mut PipelineContext) -> HookResult {
        if self.hooks.is_empty() {
            return Ok(());
        }

        // 递归构造 next 闭包，逐环委托
        fn invoke(
            hooks: &[WaterfallHook],
            idx: usize,
            event: HookEvent,
            ctx: &mut PipelineContext,
        ) -> HookResult {
            if idx >= hooks.len() {
                return Ok(());
            }
            let h = hooks[idx].clone();
            let next = |c: &mut PipelineContext| invoke(hooks, idx + 1, event, c);
            h(event, ctx, &next)
        }

        invoke(&self.hooks, 0, event, ctx)
    }

    /// 钩子数量
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

// ================== 钩子注册表 ==================

/// 钩子注册表：管理所有事件的钩子链
#[derive(Clone, Default)]
pub struct HookRegistry {
    pre_pipeline: HookChain,
    post_pipeline: HookChain,
    pre_phase: std::collections::HashMap<Phase, HookChain>,
    post_phase: std::collections::HashMap<Phase, HookChain>,
}

impl HookRegistry {
    /// 创建空的钩子注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册管线启动前钩子
    pub fn on_pre_pipeline(&mut self, hook: WaterfallHook) {
        self.pre_pipeline.add(hook);
    }

    /// 注册管线结束后钩子
    pub fn on_post_pipeline(&mut self, hook: WaterfallHook) {
        self.post_pipeline.add(hook);
    }

    /// 注册阶段前钩子
    pub fn on_pre_phase(&mut self, phase: Phase, hook: WaterfallHook) {
        self.pre_phase.entry(phase).or_default().add(hook);
    }

    /// 注册阶段后钩子
    pub fn on_post_phase(&mut self, phase: Phase, hook: WaterfallHook) {
        self.post_phase.entry(phase).or_default().add(hook);
    }

    /// 运行管线启动前钩子
    pub fn run_pre_pipeline(&self, ctx: &mut PipelineContext) -> HookResult {
        self.pre_pipeline.run(HookEvent::PrePipeline, ctx)
    }

    /// 运行管线结束后钩子
    pub fn run_post_pipeline(&self, ctx: &mut PipelineContext) -> HookResult {
        self.post_pipeline.run(HookEvent::PostPipeline, ctx)
    }

    /// 运行阶段前钩子
    pub fn run_pre_phase(&self, phase: Phase, ctx: &mut PipelineContext) -> HookResult {
        if let Some(chain) = self.pre_phase.get(&phase) {
            chain.run(HookEvent::PrePhase(phase), ctx)
        } else {
            Ok(())
        }
    }

    /// 运行阶段后钩子
    pub fn run_post_phase(&self, phase: Phase, ctx: &mut PipelineContext) -> HookResult {
        if let Some(chain) = self.post_phase.get(&phase) {
            chain.run(HookEvent::PostPhase(phase), ctx)
        } else {
            Ok(())
        }
    }
}

// ================== 常用钩子工厂 ==================

/// 审计钩子：在每个阶段前后自动追加审计事件
///
/// 这是一个通用的基础钩子，所有管线都应默认装载。
pub mod builtin_hooks {
    use super::*;

    /// 创建审计钩子（pre_phase + post_phase 自动审计）
    pub fn audit_hook() -> WaterfallHook {
        Arc::new(|event, ctx, next| {
            let phase = match event {
                HookEvent::PrePhase(p) | HookEvent::PostPhase(p) => p,
                _ => return next(ctx),
            };

            match event {
                HookEvent::PrePhase(_) => {
                    ctx.audit.record_phase_start(phase, ctx.trace_id);
                }
                HookEvent::PostPhase(_) => {
                    if let Some(exec) = ctx.get_execution(phase) {
                        ctx.audit.record_phase_end(
                            phase,
                            exec.status,
                            exec.latency_ms,
                            ctx.trace_id,
                        );
                    }
                }
                _ => {}
            }
            next(ctx)
        })
    }

    /// 创建日志钩子（tracing 输出阶段开始/结束）
    pub fn tracing_hook() -> WaterfallHook {
        Arc::new(|event, ctx, next| {
            let phase_name = match event {
                HookEvent::PrePhase(p) => {
                    tracing::info!(target: "pipeline", phase = p.name(), trace_id = %ctx.trace_id, "phase start");
                    p.name()
                }
                HookEvent::PostPhase(p) => {
                    if let Some(exec) = ctx.get_execution(p) {
                        tracing::info!(
                            target: "pipeline",
                            phase = p.name(),
                            trace_id = %ctx.trace_id,
                            status = ?exec.status,
                            latency_ms = exec.latency_ms,
                            "phase end"
                        );
                    }
                    p.name()
                }
                HookEvent::PrePipeline => {
                    tracing::info!(target: "pipeline", trace_id = %ctx.trace_id, "pipeline start");
                    "pipeline"
                }
                HookEvent::PostPipeline => {
                    tracing::info!(
                        target: "pipeline",
                        trace_id = %ctx.trace_id,
                        total_ms = ctx.total_elapsed().as_millis() as u64,
                        "pipeline end"
                    );
                    "pipeline"
                }
            };
            let _ = phase_name;
            next(ctx)
        })
    }
}
