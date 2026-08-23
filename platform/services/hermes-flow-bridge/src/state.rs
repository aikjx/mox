//! 共享运行状态（bridge 单一事实源）。
//!
//! - `BridgeState`：持有会话流程图录制器、复用模板路由器、算法否决闸门、以及
//!   **专家咨询 trait object（`Arc<dyn ExpertConsultant>`）**。
//!   DIP：中间件（同步）只依赖 trait，不依赖 xuanji-expert 任何 concrete struct 名字。
//! - `GateState`：璇玑验证网关的否决状态（最高权限拦截位）。
//!   内部用 AtomicBool，可在共享只读 Arc 上直接 `set_vetoed`/`is_vetoed`，无需 &mut。

use crate::recorder::Recorder;
use crate::router::Router;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use xuanji_expert::expert_traits::ExpertConsultant;

/// 算法否决闸门（璇玑）。内部可变性。
#[derive(Debug)]
pub struct GateState {
    vetoed: AtomicBool,
}

impl Clone for GateState {
    fn clone(&self) -> Self {
        Self { vetoed: AtomicBool::new(self.vetoed.load(Ordering::SeqCst)) }
    }
}

impl Default for GateState {
    fn default() -> Self {
        Self::new()
    }
}

impl GateState {
    pub fn new() -> Self {
        Self { vetoed: AtomicBool::new(false) }
    }
    pub fn set_vetoed(&self, v: bool) {
        self.vetoed.store(v, Ordering::SeqCst);
    }
    pub fn is_vetoed(&self) -> bool {
        self.vetoed.load(Ordering::SeqCst)
    }
}

/// 桥接共享状态（DIP 版：通过 `Arc<dyn ExpertConsultant>` 调用璇玑引擎，
/// 不直接引用 xuanji-expert 的 concrete 实现）。
pub struct BridgeState {
    pub recorder: Recorder,
    pub router: Router,
    pub gate: Arc<GateState>,
    /// DIP：依赖专家咨询抽象，不依赖具体实现。
    /// 默认由 xuanji_expert::expert_traits::default_consultant() 注入真实引擎；
    /// 测试可替换为 MockExpert。
    pub consultant: Arc<dyn ExpertConsultant>,
}

impl BridgeState {
    /// 创建共享状态，默认装配 xuanji-expert 内置 concrete 实现（通过工厂函数，
    /// 不出现 concrete struct 名字，满足 DIP 静态检查）。
    pub fn new() -> Arc<Self> {
        Self::with_consultant(xuanji_expert::expert_traits::default_consultant())
    }
    /// 自定义 consultant：测试时可替换 Mock 实现，不依赖 xuanji-expert 引擎。
    pub fn with_consultant(consultant: Arc<dyn ExpertConsultant>) -> Arc<Self> {
        Arc::new(Self {
            recorder: Recorder::new(),
            router: Router::new(),
            gate: Arc::new(GateState::new()),
            consultant,
        })
    }
    pub fn set_vetoed(&self, v: bool) {
        self.gate.set_vetoed(v);
    }
    pub fn is_vetoed(&self) -> bool {
        self.gate.is_vetoed()
    }
}
