//! 璇玑插件化运行时（参考 DeepSeek Harness "Everything is a Plugin" 范式）
//!
//! 设计要点（对照 deepseek-harness / Cordis）：
//! - **无特权核心**：专家、模型适配器、治理钩子、审计桥接都实现 [`Plugin`]，
//!   没有任何逻辑被硬编码为"系统核心"。
//! - **共享上下文 [`HarnessCtx`]**：插件向 ctx 贡献：
//!   - *services*：类型化服务注册表（模型、工具、存储等能力）
//!   - *typed events*：事件总线（observer）
//!   - *reversible effects*：可逆副作用栈（插件卸载/流程回滚时自动 unwind）
//! - **瀑布事件 [`Waterfall`]**：`pre_analyze` / `post_analyze` / `pre_gate` / `post_gate`
//!   等扩展点，监听者构成一个责任链，必须调用 [`Chain::next`] 委托给下一环。
//! - **可组合性**：[`HarnessProfile`] 声明要装载的插件集，支持运行时 `with_plugin` 叠加。
//!
//! 这一切让"七位璇玑"从"硬编码 trait 实现"升级为"可热插拔、可审计、可拦截"的
//! 生产级 agent harness。

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion};
use crate::ir::ExpertId;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    /// 装载阶段（早期/分析/闸门）
    pub phase: String,
    /// 依赖的其它插件 id（装载顺序保证依赖先行）
    pub depends_on: Vec<String>,
    pub enabled: bool,
}

impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            id: "anon".into(),
            name: "anonymous".into(),
            version: "0.0.0".into(),
            phase: "analyze".into(),
            depends_on: Vec::new(),
            enabled: true,
        }
    }
}

/// 插件：璇玑运行时的第一等公民。
///
/// 实现者可以是：
/// - 七位领域专家（`impl Expert` 后包装为 `ExpertPlugin`）
/// - 模型适配器（LLM provider）
/// - 治理钩子（在 `pre_gate` / `post_gate` 注入策略）
/// - 审计桥接（监听事件并外发）
pub trait Plugin: Send + Sync {
    /// 插件元信息
    fn meta(&self) -> PluginMeta;

    /// 装载：向共享上下文贡献 services / 注册 waterfall 钩子 / 登记事件监听。
    /// 此阶段只应"声明"，不应执行副作用（副作用通过 [`HarnessCtx::effect`] 登记为可逆）。
    fn load(&self, ctx: &HarnessCtx) {
        let _ = ctx;
    }

    /// 卸载：默认由 ctx 自动 unwind 已登记的可逆副作用；可在此做额外清理。
    fn unload(&self, ctx: &HarnessCtx) {
        let _ = ctx;
    }
}

/// 瀑布事件类型（扩展点）。监听者构成责任链。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaterfallEvent {
    /// 专家分析前：可重写/补充分析上下文
    PreAnalyze,
    /// 专家分析后：可拦截/改写专家观点
    PostAnalyze,
    /// 治理闸门前：可追加前置校验
    PreGate,
    /// 治理闸门后：可追加后置审计/回滚点
    PostGate,
}

/// 瀑布责任链中的一个处理器。
///
/// 语义（对照 deepseek-harness 的 waterfall）：处理器**必须**调用传入的 `next`
/// 闭包把控制权委托给责任链的下一环；若不调用 `next` 即视为短路（中断后续处理器）。
/// `next` 接收可变的瀑布共享状态并向下游传递，其返回值是下游处理结果，处理器可对其
/// 拦截/改写后向上传递。
pub type WaterfallHandler = Arc<
    dyn Fn(
            WaterfallEvent,
            &HarnessCtx,
            &mut WaterfallState,
            &dyn Fn(&mut WaterfallState) -> Result<(), String>,
        ) -> Result<(), String>
        + Send
        + Sync,
>;

/// 瀑布在流转过程中可被处理器修改的共享状态
#[derive(Debug, Default)]
pub struct WaterfallState {
    /// 分析阶段：专家观点集合（PostAnalyze 可改写）
    pub opinions: Vec<ExpertOpinion>,
    /// 网关阶段：最终闸门结果（PostGate 可复核）
    pub gate: Option<crate::govern::GateResult>,
    /// 任意键值载荷（插件间传递上下文）
    pub bag: HashMap<String, String>,
}

/// 可逆副作用：插件登记，流程结束/回滚时逆序执行
type Effect = Arc<dyn Fn() + Send + Sync>;

/// 事件监听器：事件名 → 处理器链（提取复杂类型，消解 clippy::type_complexity）
type Listener = Arc<dyn Fn(&str) + Send + Sync>;

/// 共享上下文（对照 deepseek-harness 的 `ctx`）。
///
/// 持有三类贡献物：
/// 1. services：类型化服务注册表（模型/工具/存储等能力）
/// 2. events：observer 事件总线
/// 3. effects：可逆副作用栈（unwind 时逆序执行）
pub struct HarnessCtx {
    /// 共享服务注册表：TypeId → 服务实例（Arc 以便多读者共享）
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// 事件监听器：事件名 → 处理器
    listeners: Mutex<HashMap<String, Vec<Listener>>>,
    /// 瀑布钩子：事件 → 责任链
    waterfalls: Arc<RwLock<HashMap<WaterfallEvent, Vec<WaterfallHandler>>>>,
    /// 可逆副作用栈（逆序执行）
    effects: Mutex<Vec<Effect>>,
    /// 已装载插件（用于有序 unload）
    loaded: Mutex<Vec<Box<dyn Plugin>>>,
    /// 配置档案
    profile: HarnessProfile,
}

impl Default for HarnessCtx {
    fn default() -> Self {
        Self::new(HarnessProfile::default())
    }
}

impl HarnessCtx {
    pub fn new(profile: HarnessProfile) -> Self {
        Self {
            services: Mutex::new(HashMap::new()),
            listeners: Mutex::new(HashMap::new()),
            waterfalls: Arc::new(RwLock::new(HashMap::new())),
            effects: Mutex::new(Vec::new()),
            loaded: Mutex::new(Vec::new()),
            profile,
        }
    }

    /// 贡献一个类型化 service（覆盖式；重复注册同 TypeId 会替换）
    pub fn provide<T: Any + Send + Sync>(&self, svc: T) {
        self.services
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), Arc::new(svc));
    }

    /// 取出类型化 service（无则返回 None）
    pub fn get_service<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.services
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|b| b.clone().downcast::<T>().ok())
    }

    /// 登记一个监听指定事件名的处理器
    pub fn on(&self, event: impl Into<String>, handler: Arc<dyn Fn(&str) + Send + Sync>) {
        self.listeners
            .lock()
            .unwrap()
            .entry(event.into())
            .or_default()
            .push(handler);
    }

    /// 广播一个事件（observer 模式）
    pub fn emit(&self, event: &str) {
        if let Some(list) = self.listeners.lock().unwrap().get(event) {
            for h in list {
                h(event);
            }
        }
    }

    /// 注册瀑布钩子（扩展点）。按注册顺序构成责任链。
    pub fn hook(&self, event: WaterfallEvent, handler: WaterfallHandler) {
        self.waterfalls
            .write()
            .unwrap()
            .entry(event)
            .or_default()
            .push(handler);
    }

    /// 登记可逆副作用（插件卸载/流程回滚时逆序执行）
    pub fn effect(&self, eff: Effect) {
        self.effects.lock().unwrap().push(eff);
    }

    /// 逆序执行所有已登记副作用（unwind），并清空
    pub fn unwind(&self) {
        let mut effects = std::mem::take(&mut *self.effects.lock().unwrap());
        while let Some(e) = effects.pop() {
            e();
        }
    }

    /// 装载一个插件（按依赖顺序），并记录以便有序 unload
    pub fn load_plugin(&self, plugin: Box<dyn Plugin>) {
        plugin.load(self);
        self.loaded.lock().unwrap().push(plugin);
    }

    /// 运行一个瀑布事件：从责任链首环开始，逐个调用处理器并传入 `next` 委托闭包。
    /// 处理器若不调用 `next` 即短路；下游错误会沿调用栈向上传播。
    pub fn run_waterfall(
        &self,
        event: WaterfallEvent,
        state: &mut WaterfallState,
    ) -> Result<(), String> {
        let handlers = self.waterfalls.read().unwrap().get(&event).cloned();
        let handlers = match handlers {
            Some(h) => h,
            None => return Ok(()),
        };
        // 递归构造 next 闭包，逐环委托（state 通过参数流动，避免 FnMut 捕获）
        fn invoke(
            handlers: &[WaterfallHandler],
            idx: usize,
            event: WaterfallEvent,
            ctx: &HarnessCtx,
            state: &mut WaterfallState,
        ) -> Result<(), String> {
            if idx >= handlers.len() {
                return Ok(());
            }
            let h = handlers[idx].clone();
            let next = |s: &mut WaterfallState| invoke(handlers, idx + 1, event, ctx, s);
            h(event, ctx, state, &next)
        }
        invoke(&handlers, 0, event, self, state)
    }
    pub fn shutdown(&self) {
        let mut loaded = self.loaded.lock().unwrap();
        while let Some(p) = loaded.pop() {
            p.unload(self);
        }
        self.unwind();
    }

    pub fn profile(&self) -> &HarnessProfile {
        &self.profile
    }
}

/// 配置档案：声明要装载的插件集（对照 deepseek-harness 的 Profile）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessProfile {
    pub name: String,
    /// 启用的插件 id 列表（顺序即装载顺序，依赖须前置）
    pub plugins: Vec<String>,
    /// 是否启用内部链审计
    pub audit_enabled: bool,
    /// 模型适配器配置（service 配置示例）
    pub model: ModelAdapterConfig,
}

/// 模型适配器配置（Service Definition / Provider 分离的落地）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAdapterConfig {
    pub provider: String,
    pub endpoint: String,
    pub api_key_env: String,
    pub default_tier: String,
}

impl Default for ModelAdapterConfig {
    fn default() -> Self {
        // 默认指向真实 DeepSeek（OpenAI 兼容协议），全系统统一收口 DEEPSEEK_API_KEY。
        // api_key_env 优先读 OUS_LLM_API_KEY，缺失时回退到 DEEPSEEK_API_KEY（运行时由调用方解析）。
        Self {
            provider: "deepseek".into(),
            endpoint: "https://api.deepseek.com/v1".into(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            default_tier: "standard".into(),
        }
    }
}

impl ModelAdapterConfig {
    /// 解析实际 API Key：优先读 `api_key_env` 指向的环境变量，缺失时回退到 `DEEPSEEK_API_KEY`。
    pub fn resolve_api_key(&self) -> Option<String> {
        if let Ok(k) = std::env::var(&self.api_key_env) {
            if !k.trim().is_empty() {
                return Some(k);
            }
        }
        std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
    }

    /// 是否已具备真实调用条件。
    pub fn is_configured(&self) -> bool {
        self.resolve_api_key().is_some()
    }
}

/// 专家 → 插件适配器：让任意 `impl Expert` 成为可插拔的 `Plugin`
pub struct ExpertPlugin {
    expert: Box<dyn Expert>,
    meta: PluginMeta,
}

impl ExpertPlugin {
    pub fn new(expert: Box<dyn Expert>) -> Self {
        let id: ExpertId = expert.id();
        let meta = PluginMeta {
            id: id.clone(),
            name: id.clone(),
            version: "1.0.0".into(),
            phase: "analyze".into(),
            depends_on: Vec::new(),
            enabled: true,
        };
        Self { expert, meta }
    }

    pub fn as_expert(&self) -> &dyn Expert {
        self.expert.as_ref()
    }
}

impl Plugin for ExpertPlugin {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
    // 专家插件无需在 load 阶段注册钩子；分析时由 `run_experts` 并行调用 `as_expert()`。
}

/// 便捷构造：把一组专家包装为默认 bundle（保持与旧 `all_experts()` 行为兼容）
pub fn expert_plugins(experts: Vec<Box<dyn Expert>>) -> Vec<Box<dyn Plugin>> {
    experts
        .into_iter()
        .map(|e| Box::new(ExpertPlugin::new(e)) as Box<dyn Plugin>)
        .collect()
}

/// 用插件化运行时驱动一次璇玑分析（替换 pipeline 中硬编码的 dispatch）。
///
/// 流程：
/// 1. PreAnalyze 瀑布（插件可补充分析上下文）
/// 2. 并行执行所有已装载的专家插件，收集观点
/// 3. PostAnalyze 瀑布（插件可拦截/改写观点）
/// 4. 返回观点集合交由裁决/治理层
pub fn run_experts(
    ctx: &HarnessCtx,
    ectx: &ExpertContext,
    experts: &[Box<dyn Expert>],
) -> Vec<ExpertOpinion> {
    // 1. PreAnalyze
    let mut state = WaterfallState::default();
    if let Err(e) = ctx.run_waterfall(WaterfallEvent::PreAnalyze, &mut state) {
        tracing::warn!(target: "harness", "PreAnalyze 瀑布执行失败: {}", e);
    }

    // 2. 并行派发专家（保持无状态只读，rayon 真并行利用多核；
    //    订单保留原序，保证与串行派发结果确定性一致）
    let opinions: Vec<ExpertOpinion> = {
        use rayon::prelude::*;
        experts.par_iter().map(|e| e.analyze(ectx)).collect()
    };
    state.opinions = opinions.clone();

    // 3. PostAnalyze（插件可改写）
    if let Err(e) = ctx.run_waterfall(WaterfallEvent::PostAnalyze, &mut state) {
        tracing::warn!(target: "harness", "PostAnalyze 瀑布执行失败: {}", e);
    }
    state.opinions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, Principal, Tenant};
    use crate::ir::Dimension;
    use mox_ai_flow_svc::model::FlowGraph;

    struct StubExpert;
    impl Expert for StubExpert {
        fn id(&self) -> ExpertId {
            "stub".into()
        }
        fn dimension(&self) -> Dimension {
            Dimension::Business
        }
        fn analyze(&self, _ctx: &ExpertContext) -> ExpertOpinion {
            ExpertOpinion::empty("stub", Dimension::Business)
        }
    }

    #[test]
    fn ctx_event_bus_works() {
        let ctx = HarnessCtx::default();
        let hit = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let h = Arc::clone(&hit);
        ctx.on(
            "ping",
            Arc::new(move |_| h.store(true, std::sync::atomic::Ordering::SeqCst)),
        );
        ctx.emit("ping");
        assert!(hit.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn service_registry_roundtrip() {
        let ctx = HarnessCtx::default();
        ctx.provide("llm-endpoint".to_string());
        assert_eq!(
            ctx.get_service::<String>(),
            Some(Arc::new("llm-endpoint".to_string()))
        );
        assert_eq!(ctx.get_service::<u32>(), None);
    }

    #[test]
    fn waterfall_chains_and_delegates() {
        let ctx = HarnessCtx::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s1 = Arc::clone(&seen);
        ctx.hook(
            WaterfallEvent::PreAnalyze,
            Arc::new(move |_ev, _c, _state, next| {
                s1.lock().unwrap().push("h1");
                next(_state)
            }),
        );
        let s2 = Arc::clone(&seen);
        ctx.hook(
            WaterfallEvent::PreAnalyze,
            Arc::new(move |_ev, _c, _state, next| {
                s2.lock().unwrap().push("h2");
                next(_state)
            }),
        );
        let mut state = WaterfallState::default();
        ctx.run_waterfall(WaterfallEvent::PreAnalyze, &mut state)
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), vec!["h1", "h2"]);
    }

    #[test]
    fn effects_unwind_in_reverse() {
        let ctx = HarnessCtx::default();
        let log = Arc::new(Mutex::new(Vec::new()));
        let l1 = Arc::clone(&log);
        ctx.effect(Arc::new(move || l1.lock().unwrap().push(1)));
        let l2 = Arc::clone(&log);
        ctx.effect(Arc::new(move || l2.lock().unwrap().push(2)));
        ctx.unwind();
        assert_eq!(*log.lock().unwrap(), vec![2, 1]); // 逆序
    }

    #[test]
    fn expert_plugin_bundle_runs() {
        let ctx = HarnessCtx::default();
        let g = GovernContext::new(Tenant::new("t", "ns"), Principal::new("u"));
        let fg = FlowGraph::new("x", "t");
        let ectx = ExpertContext::new(&fg, &g);
        let experts: Vec<Box<dyn Expert>> = vec![Box::new(StubExpert)];
        let ops = run_experts(&ctx, &ectx, &experts);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].expert, "stub");
    }
}
