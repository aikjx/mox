// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 钩子机制（Hook System）
//!
//! 支持在每个阶段前后插入自定义逻辑。
//!
//! # 核心思想
//!
//! 责任链（Chain of Responsibility）模式：
//! - 每个钩子可以修改上下文状态
//! - 每个钩子必须调用 `next` 委托给下一个钩子
//! - 不调用 `next` 即为短路（中断后续钩子）
//!
//! # 钩子类型
//!
//! - `PrePhase(P)` : 阶段执行前，可修改输入、补充上下文
//! - `PostPhase(P)`: 阶段执行后，可修改结果、追加审计
//! - `PrePipeline`     : 管线启动前
//! - `PostPipeline`    : 管线结束后

use std::sync::Arc;

use crate::context::PipelineContext;
use crate::phase::PhaseId;

// ================== 钩子事件 ==================

/// 钩子事件类型
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Debug, Clone)]
pub enum HookEvent<P: PhaseId> {
    /// 管线启动前
    PrePipeline,
    /// 管线结束后
    PostPipeline,
    /// 阶段执行前
    PrePhase(P),
    /// 阶段执行后
    PostPhase(P),
}

impl<P: PhaseId> HookEvent<P> {
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
    pub event_name: String,
    pub message: String,
}

impl HookError {
    pub fn new<P: PhaseId>(event: &HookEvent<P>, message: impl Into<String>) -> Self {
        Self {
            event_name: event.name(),
            message: message.into(),
        }
    }

    /// 从事件名和消息创建
    pub fn from_name(event_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            event_name: event_name.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook error at '{}': {}", self.event_name, self.message)
    }
}

impl std::error::Error for HookError {}

/// 钩子执行结果
pub type HookResult = Result<(), HookError>;

// ================== 钩子处理器 ==================

/// 瀑布钩子处理器
///
/// 语义（责任链模式）：
/// 处理器**必须**调用传入的 `next` 闭包把控制权委托给责任链的下一环；
/// 若不调用 `next` 即视为短路（中断后续处理器）。
///
/// `next` 接收可变的上下文并向下游传递，其返回值是下游处理结果，
/// 处理器可对其拦截/改写后向上传递。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub type WaterfallHook<P> = Arc<
    dyn Fn(
            &HookEvent<P>,
            &mut PipelineContext<P>,
            &dyn Fn(&mut PipelineContext<P>) -> HookResult,
        ) -> HookResult
        + Send
        + Sync,
>;

// ================== 钩子链 ==================

/// 钩子链：管理某一事件的所有钩子，构成责任链
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Clone)]
pub struct HookChain<P: PhaseId> {
    hooks: Vec<WaterfallHook<P>>,
}

impl<P: PhaseId> Default for HookChain<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: PhaseId> HookChain<P> {
    /// 创建空的钩子链
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 添加一个钩子（按添加顺序执行）
    pub fn add(&mut self, hook: WaterfallHook<P>) {
        self.hooks.push(hook);
    }

    /// 运行钩子链：从首环开始，逐个调用处理器并传入 `next` 委托闭包。
    ///
    /// 处理器若不调用 `next` 即短路；下游错误会沿调用栈向上传播。
    pub fn run(&self, event: &HookEvent<P>, ctx: &mut PipelineContext<P>) -> HookResult {
        if self.hooks.is_empty() {
            return Ok(());
        }

        // 递归构造 next 闭包，逐环委托
        fn invoke<P: PhaseId>(
            hooks: &[WaterfallHook<P>],
            idx: usize,
            event: &HookEvent<P>,
            ctx: &mut PipelineContext<P>,
        ) -> HookResult {
            if idx >= hooks.len() {
                return Ok(());
            }
            let h = hooks[idx].clone();
            let next = |c: &mut PipelineContext<P>| invoke(hooks, idx + 1, event, c);
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
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Clone)]
pub struct HookRegistry<P: PhaseId> {
    pre_pipeline: HookChain<P>,
    post_pipeline: HookChain<P>,
    pre_phase: std::collections::HashMap<P, HookChain<P>>,
    post_phase: std::collections::HashMap<P, HookChain<P>>,
}

impl<P: PhaseId> Default for HookRegistry<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: PhaseId> HookRegistry<P> {
    /// 创建空的钩子注册表
    pub fn new() -> Self {
        Self {
            pre_pipeline: HookChain::new(),
            post_pipeline: HookChain::new(),
            pre_phase: std::collections::HashMap::new(),
            post_phase: std::collections::HashMap::new(),
        }
    }

    /// 注册管线启动前钩子
    pub fn on_pre_pipeline(&mut self, hook: WaterfallHook<P>) {
        self.pre_pipeline.add(hook);
    }

    /// 注册管线结束后钩子
    pub fn on_post_pipeline(&mut self, hook: WaterfallHook<P>) {
        self.post_pipeline.add(hook);
    }

    /// 注册阶段前钩子
    pub fn on_pre_phase(&mut self, phase: P, hook: WaterfallHook<P>) {
        self.pre_phase.entry(phase).or_default().add(hook);
    }

    /// 注册阶段后钩子
    pub fn on_post_phase(&mut self, phase: P, hook: WaterfallHook<P>) {
        self.post_phase.entry(phase).or_default().add(hook);
    }

    /// 运行管线启动前钩子
    pub fn run_pre_pipeline(&self, ctx: &mut PipelineContext<P>) -> HookResult {
        self.pre_pipeline.run(&HookEvent::PrePipeline, ctx)
    }

    /// 运行管线结束后钩子
    pub fn run_post_pipeline(&self, ctx: &mut PipelineContext<P>) -> HookResult {
        self.post_pipeline.run(&HookEvent::PostPipeline, ctx)
    }

    /// 运行阶段前钩子
    pub fn run_pre_phase(&self, phase: &P, ctx: &mut PipelineContext<P>) -> HookResult {
        if let Some(chain) = self.pre_phase.get(phase) {
            chain.run(&HookEvent::PrePhase(phase.clone()), ctx)
        } else {
            Ok(())
        }
    }

    /// 运行阶段后钩子
    pub fn run_post_phase(&self, phase: &P, ctx: &mut PipelineContext<P>) -> HookResult {
        if let Some(chain) = self.post_phase.get(phase) {
            chain.run(&HookEvent::PostPhase(phase.clone()), ctx)
        } else {
            Ok(())
        }
    }

    /// 获取 pre_pipeline 钩子数量
    pub fn pre_pipeline_count(&self) -> usize {
        self.pre_pipeline.len()
    }

    /// 获取 post_pipeline 钩子数量
    pub fn post_pipeline_count(&self) -> usize {
        self.post_pipeline.len()
    }
}

// ================== 常用钩子工厂 ==================

/// 内置钩子：审计、日志等通用钩子
pub mod builtin_hooks {
    use super::*;

    /// 创建审计钩子（pre_phase + post_phase 自动审计）
    ///
    /// 这是一个通用的基础钩子，所有管线都应默认装载。
    /// 在阶段开始时记录 phase_start 审计事件，
    /// 在阶段结束时记录 phase_end 审计事件（含耗时和状态）。
    pub fn audit_hook<P: PhaseId>() -> WaterfallHook<P> {
        Arc::new(|event, ctx, next| {
            let phase = match event {
                HookEvent::PrePhase(p) | HookEvent::PostPhase(p) => Some(p),
                _ => None,
            };

            if let Some(phase) = phase {
                match event {
                    HookEvent::PrePhase(_) => {
                        if ctx.options.audit_enabled {
                            ctx.audit.record_phase_start(phase, ctx.trace_id);
                        }
                    }
                    HookEvent::PostPhase(_) => {
                        if ctx.options.audit_enabled {
                            if let Some(exec) = ctx.get_execution(phase) {
                                ctx.audit.record_phase_end(
                                    phase,
                                    exec.status,
                                    exec.latency_ms,
                                    ctx.trace_id,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            next(ctx)
        })
    }

    /// 创建日志钩子（tracing 输出阶段开始/结束）
    ///
    /// 使用 `tracing` crate 输出管线执行过程的日志。
    pub fn tracing_hook<P: PhaseId>() -> WaterfallHook<P> {
        Arc::new(|event, ctx, next| {
            let phase_name = match event {
                HookEvent::PrePhase(p) => {
                    tracing::info!(target: "pipeline", phase = p.name(), trace_id = %ctx.trace_id, "phase start");
                    p.name().to_string()
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
                    p.name().to_string()
                }
                HookEvent::PrePipeline => {
                    tracing::info!(target: "pipeline", trace_id = %ctx.trace_id, "pipeline start");
                    "pipeline".to_string()
                }
                HookEvent::PostPipeline => {
                    tracing::info!(
                        target: "pipeline",
                        trace_id = %ctx.trace_id,
                        total_ms = ctx.total_elapsed().as_millis() as u64,
                        "pipeline end"
                    );
                    "pipeline".to_string()
                }
            };
            let _ = phase_name;
            next(ctx)
        })
    }

    /// 创建指标收集钩子（收集阶段耗时等指标）
    ///
    /// 将阶段执行信息存入 context 的 bag 中，供后续阶段或外部监控使用。
    pub fn metrics_hook<P: PhaseId>() -> WaterfallHook<P> {
        Arc::new(|event, ctx, next| {
            if let HookEvent::PostPhase(phase) = event {
                if let Some(exec) = ctx.get_execution(phase) {
                    let key = format!("metrics.phase.{}.latency_ms", phase.name());
                    ctx.set_bag(key, exec.latency_ms);
                }
            }
            next(ctx)
        })
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{PipelineInput, PipelineOptions};
    use crate::phase::NamedPhase;

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
    fn hook_event_names() {
        assert_eq!(HookEvent::<NamedPhase>::PrePipeline.name(), "pre_pipeline");
        assert_eq!(HookEvent::<NamedPhase>::PostPipeline.name(), "post_pipeline");
        assert_eq!(
            HookEvent::PrePhase(NamedPhase::new("analyze")).name(),
            "pre_analyze"
        );
        assert_eq!(
            HookEvent::PostPhase(NamedPhase::blocking("gate")).name(),
            "post_gate"
        );
    }

    #[test]
    fn empty_hook_chain_returns_ok() {
        let chain = HookChain::<NamedPhase>::new();
        let mut ctx = make_ctx();
        assert!(chain.run(&HookEvent::PrePipeline, &mut ctx).is_ok());
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn single_hook_executes() {
        let mut chain = HookChain::<NamedPhase>::new();
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        chain.add(Arc::new(move |_event, _ctx, next| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            next(_ctx)
        }));

        let mut ctx = make_ctx();
        let result = chain.run(&HookEvent::PrePipeline, &mut ctx);
        assert!(result.is_ok());
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn multiple_hooks_execute_in_order() {
        let mut chain = HookChain::<NamedPhase>::new();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        for i in 0..3 {
            let order_clone = order.clone();
            chain.add(Arc::new(move |_event, _ctx, next| {
                order_clone.lock().unwrap().push(i);
                let result = next(_ctx);
                order_clone.lock().unwrap().push(10 + i); // post-next marker
                result
            }));
        }

        let mut ctx = make_ctx();
        chain.run(&HookEvent::PrePipeline, &mut ctx).unwrap();

        let order = order.lock().unwrap();
        // 执行顺序：0 → 1 → 2 → next(2返回) → 12 → 11 → 10
        // 即 pre 阶段按顺序，post 阶段逆序
        assert_eq!(order[0], 0); // pre_0
        assert_eq!(order[1], 1); // pre_1
        assert_eq!(order[2], 2); // pre_2
        assert_eq!(order[3], 12); // post_2
        assert_eq!(order[4], 11); // post_1
        assert_eq!(order[5], 10); // post_0
    }

    #[test]
    fn hook_short_circuit_stops_chain() {
        let mut chain = HookChain::<NamedPhase>::new();
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        // 第一个钩子短路（不调用 next）
        chain.add(Arc::new(|_event, _ctx, _next| {
            // 不调用 next，直接返回 Ok
            Ok(())
        }));

        // 第二个钩子（不应被调用）
        chain.add(Arc::new(move |_event, _ctx, next| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            next(_ctx)
        }));

        let mut ctx = make_ctx();
        chain.run(&HookEvent::PrePipeline, &mut ctx).unwrap();

        assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn hook_propagates_error() {
        let mut chain = HookChain::<NamedPhase>::new();

        chain.add(Arc::new(|event, _ctx, _next| {
            Err(HookError::new(event, "something went wrong"))
        }));

        let mut ctx = make_ctx();
        let result = chain.run(&HookEvent::PrePhase(NamedPhase::new("analyze")), &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.message, "something went wrong");
    }

    #[test]
    fn hook_registry_manages_all_events() {
        let mut registry = HookRegistry::<NamedPhase>::new();
        let phase = NamedPhase::new("analyze");

        // 注册各种钩子
        registry.on_pre_pipeline(builtin_hooks::audit_hook());
        registry.on_post_pipeline(builtin_hooks::audit_hook());
        registry.on_pre_phase(phase.clone(), builtin_hooks::audit_hook());
        registry.on_post_phase(phase.clone(), builtin_hooks::audit_hook());

        assert_eq!(registry.pre_pipeline_count(), 1);
        assert_eq!(registry.post_pipeline_count(), 1);

        let mut ctx = make_ctx();
        assert!(registry.run_pre_pipeline(&mut ctx).is_ok());
        assert!(registry.run_post_pipeline(&mut ctx).is_ok());
        assert!(registry.run_pre_phase(&phase, &mut ctx).is_ok());
        assert!(registry.run_post_phase(&phase, &mut ctx).is_ok());

        // 未注册钩子的阶段也应返回 Ok
        let other = NamedPhase::new("gate");
        assert!(registry.run_pre_phase(&other, &mut ctx).is_ok());
    }

    #[test]
    fn audit_hook_records_events() {
        let mut registry = HookRegistry::<NamedPhase>::new();
        let phase = NamedPhase::new("analyze");
        registry.on_pre_phase(phase.clone(), builtin_hooks::audit_hook());
        registry.on_post_phase(phase.clone(), builtin_hooks::audit_hook());

        let mut ctx = make_ctx();
        ctx.mark_phase_start(phase.clone());

        // pre_phase 钩子应记录开始事件
        registry.run_pre_phase(&phase, &mut ctx).unwrap();
        let pre_count = ctx.audit.len();

        // post_phase 钩子应记录结束事件
        ctx.mark_phase_end(&phase, crate::phase::PhaseStatus::Success, 100);
        registry.run_post_phase(&phase, &mut ctx).unwrap();

        assert_eq!(ctx.audit.len(), pre_count + 1);
        assert!(ctx.audit.verify());
    }

    #[test]
    fn metrics_hook_stores_latency() {
        let mut registry = HookRegistry::<NamedPhase>::new();
        let phase = NamedPhase::new("analyze");
        registry.on_post_phase(phase.clone(), builtin_hooks::metrics_hook());

        let mut ctx = make_ctx();
        ctx.mark_phase_start(phase.clone());
        ctx.mark_phase_end(&phase, crate::phase::PhaseStatus::Success, 42);

        registry.run_post_phase(&phase, &mut ctx).unwrap();

        let latency = ctx.get_bag::<u64>("metrics.phase.analyze.latency_ms");
        assert_eq!(latency, Some(&42));
    }

    #[test]
    fn hook_error_display() {
        let err = HookError::from_name("pre_pipeline", "test error");
        let display = format!("{err}");
        assert!(display.contains("pre_pipeline"));
        assert!(display.contains("test error"));
    }
}
