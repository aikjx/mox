//! 共享运行状态（bridge 单一事实源）。
//!
//! - `BridgeState`：持有会话流程图录制器、复用模板路由器、算法否决闸门。
//!   中间件（同步）只读/写它；后台 optimize 任务也写它——全部走 Arc<Mutex<..>>。
//! - `GateState`：璇玑验证网关的否决状态（最高权限拦截位）。
//!   内部用 AtomicBool，可在共享只读 Arc 上直接 `set_vetoed`/`is_vetoed`，无需 &mut。

use crate::recorder::Recorder;
use crate::router::Router;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 算法否决闸门（璇玑）。内部可变性。
#[derive(Debug)]
pub struct GateState {
    vetoed: AtomicBool,
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

/// 桥接共享状态。
pub struct BridgeState {
    pub recorder: Recorder,
    pub router: Router,
    pub gate: Arc<GateState>,
}

impl BridgeState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            recorder: Recorder::new(),
            router: Router::new(),
            gate: Arc::new(GateState::new()),
        })
    }
    pub fn set_vetoed(&self, v: bool) {
        self.gate.set_vetoed(v);
    }
    pub fn is_vetoed(&self) -> bool {
        self.gate.is_vetoed()
    }
}
