//! OUS-Cordis 插件化运行时内核 - 完整实现

// 此文件内容过多，已拆分为多个子模块
// 核心结构保持不变，修复编译错误

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod bundle;
pub mod context;
pub mod event_bus;
pub mod lifecycle;
pub mod profile;
pub mod seam;

pub use bundle::{Bundle, BundleError, BundleManager, BundleManifest, PluginMeta};
pub use context::{AgentRegistry, OperatorRegistry, PluginContext, SessionEntry, SessionLog};
pub use event_bus::{Event, EventBus, EventDomain, Subscription};
pub use lifecycle::{LifecycleManager, Step, StepResult, Turn, TurnState, TurnSummary};
pub use profile::{Profile, ProfileError, ProfileLoader};
pub use seam::{Seam, SeamCapability, SeamError, SeamRegistry};

/// Turn标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(String);

impl TurnId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TurnId {
    fn default() -> Self {
        Self::new()
    }
}

/// OUS-Cordis 核心运行时
#[allow(dead_code)] // 预留运行时能力面：profile_loader/bundle_manager/seam_registry 待接入管线后启用
pub struct OUSCordis {
    /// 插件上下文树
    ctx: Arc<PluginContext>,
    /// Profile加载器
    profile_loader: ProfileLoader,
    /// Bundle管理器
    bundle_manager: BundleManager,
    /// 事件总线
    event_bus: Arc<EventBus>,
    /// Seam注册表
    seam_registry: Arc<SeamRegistry>,
    /// 生命周期管理器
    lifecycle: Arc<LifecycleManager>,
    /// 状态
    state: RwLock<CordisState>,
}

/// 运行时状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CordisState {
    /// 已加载Profile列表
    loaded_profiles: Vec<String>,
    /// 已挂载Bundle列表
    mounted_bundles: Vec<String>,
    /// 运行时统计
    stats: RuntimeStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeStats {
    /// 总Turn数
    pub total_turns: u64,
    /// 总Step数
    pub total_steps: u64,
    /// 插件调用次数
    pub plugin_invocations: u64,
    /// 事件处理次数
    pub event_handlers_called: u64,
}

impl OUSCordis {
    /// 创建新的运行时实例
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(PluginContext::new()),
            profile_loader: ProfileLoader::new(),
            bundle_manager: BundleManager::new(),
            event_bus: Arc::new(EventBus::new()),
            seam_registry: Arc::new(SeamRegistry::new()),
            lifecycle: Arc::new(LifecycleManager::new()),
            state: RwLock::new(CordisState::default()),
        }
    }

    /// 创建新的Turn
    pub async fn create_turn(&self, agent_id: &str) -> Result<String, String> {
        let turn_id = self.lifecycle.create_turn(agent_id).await?;

        // 追加到Session Log
        self.ctx
            .sessions
            .append(SessionEntry::TurnStart {
                turn_id: turn_id.clone(),
                agent_id: agent_id.to_string(),
                timestamp: chrono::Utc::now(),
            })
            .await?;

        // 发送事件
        self.event_bus
            .emit(Event::TurnStarted {
                turn_id: turn_id.clone(),
                agent_id: agent_id.to_string(),
            })
            .await;

        // 更新统计
        self.state.write().stats.total_turns += 1;

        Ok(turn_id)
    }

    /// 执行Step
    pub async fn execute_step(&self, turn_id: &str, step: Step) -> Result<StepResult, String> {
        // 执行步骤
        let result = self.lifecycle.execute_step(turn_id, step.clone()).await?;

        // 追加到Session Log
        self.ctx
            .sessions
            .append(SessionEntry::StepStart {
                step_id: step.id.clone(),
                turn_id: turn_id.to_string(),
                action: step.action.clone(),
            })
            .await?;

        // 更新统计
        self.state.write().stats.total_steps += 1;

        Ok(result)
    }

    /// 完成Turn
    pub async fn complete_turn(&self, turn_id: &str) -> Result<TurnSummary, String> {
        let summary = self.lifecycle.complete_turn(turn_id).await?;

        // 追加到Session Log
        self.ctx
            .sessions
            .append(SessionEntry::TurnComplete {
                turn_id: turn_id.to_string(),
                summary: serde_json::to_string(&summary).unwrap_or_default(),
            })
            .await?;

        // 发送事件
        self.event_bus
            .emit(Event::TurnCompleted {
                turn_id: turn_id.to_string(),
                summary: summary.clone(),
            })
            .await;

        Ok(summary)
    }

    /// 获取运行时状态
    pub fn state(&self) -> CordisState {
        self.state.read().clone()
    }
}

impl Default for OUSCordis {
    fn default() -> Self {
        Self::new()
    }
}
