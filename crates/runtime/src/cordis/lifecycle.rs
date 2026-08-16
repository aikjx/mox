//! Turn/Agent/Step 生命周期管理
//!
//! 核心概念：
//! - Turn: 会话轮次
//! - Step: 执行步骤
//! - Waterfall: 瀑布式事件流

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 生命周期管理器
pub struct LifecycleManager {
    /// 活跃Turn
    active_turns: RwLock<HashMap<String, Turn>>,
    /// Turn历史
    turn_history: RwLock<Vec<TurnSummary>>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        Self {
            active_turns: RwLock::new(HashMap::new()),
            turn_history: RwLock::new(Vec::new()),
        }
    }

    /// 创建新Turn
    pub async fn create_turn(&self, agent_id: &str) -> Result<String, String> {
        let turn_id = uuid::Uuid::new_v4().to_string();
        let turn = Turn::new(turn_id.clone(), agent_id.to_string());

        let mut active_turns = self.active_turns.write();
        active_turns.insert(turn_id.clone(), turn);

        Ok(turn_id)
    }

    /// 执行Step
    pub async fn execute_step(
        &self,
        turn_id: &str,
        step: Step,
    ) -> Result<StepResult, String> {
        let mut active_turns = self.active_turns.write();

        if let Some(turn) = active_turns.get_mut(turn_id) {
            turn.execute_step(step).await
        } else {
            Err(format!("Turn not found: {}", turn_id))
        }
    }

    /// 完成Turn
    pub async fn complete_turn(&self, turn_id: &str) -> Result<TurnSummary, String> {
        let mut active_turns = self.active_turns.write();

        if let Some(turn) = active_turns.remove(turn_id) {
            let summary = turn.complete()?;

            // 保存到历史
            let mut history = self.turn_history.write();
            history.push(summary.clone());

            Ok(summary)
        } else {
            Err(format!("Turn not found: {}", turn_id))
        }
    }

    /// 获取Turn
    pub fn get_turn(&self, turn_id: &str) -> Option<Turn> {
        let active_turns = self.active_turns.read();
        active_turns.get(turn_id).cloned()
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Turn（轮次）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    pub agent_id: String,
    pub steps: Vec<Step>,
    pub state: TurnState,
    pub events: WaterfallEventStream,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Turn {
    pub fn new(id: String, agent_id: String) -> Self {
        Self {
            id,
            agent_id,
            steps: Vec::new(),
            state: TurnState::Pending,
            events: WaterfallEventStream::new(),
            created_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    /// 执行Step
    pub async fn execute_step(&mut self, mut step: Step) -> Result<StepResult, String> {
        // 更新状态
        self.state = TurnState::Running;

        // 执行步骤
        let start_time = std::time::Instant::now();
        step.state = StepState::Running;

        // TODO: 实际执行逻辑
        let result = StepResult {
            step_id: step.id.clone(),
            success: true,
            output: serde_json::json!({"status": "completed"}),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };

        step.state = StepState::Completed;
        step.result = Some(result.clone());

        // 添加事件
        self.events.push(WaterfallEvent::StepComplete {
            step_id: step.id.clone(),
            success: result.success,
        });

        // 添加到步骤列表
        self.steps.push(step);

        Ok(result)
    }

    /// 完成Turn
    pub fn complete(mut self) -> Result<TurnSummary, String> {
        self.state = TurnState::Completed;
        self.completed_at = Some(chrono::Utc::now());

        let total_duration_ms = self.steps.iter()
            .map(|s| s.result.as_ref().map(|r| r.duration_ms).unwrap_or(0))
            .sum();

        Ok(TurnSummary {
            turn_id: self.id.clone(),
            steps: self.steps.len() as u32,
            success: self.steps.iter().all(|s| s.state == StepState::Completed),
            total_duration_ms,
            final_state: serde_json::json!({
                "state": self.state,
                "steps": self.steps.len(),
            }),
        })
    }
}

/// Turn状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnState {
    Pending,
    Running,
    Completed,
    Failed,
    RolledBack,
}

/// Step（步骤）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub action: String,
    pub input: serde_json::Value,
    pub state: StepState,
    pub result: Option<StepResult>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Step {
    pub fn new(action: String, input: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            input,
            state: StepState::Pending,
            result: None,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Step状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepState {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Step结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
}

/// Turn摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: String,
    pub steps: u32,
    pub success: bool,
    pub total_duration_ms: u64,
    pub final_state: serde_json::Value,
}

/// 瀑布式事件流
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallEventStream {
    events: Vec<WaterfallEvent>,
}

impl WaterfallEventStream {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    pub fn push(&mut self, event: WaterfallEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[WaterfallEvent] {
        &self.events
    }
}

impl Default for WaterfallEventStream {
    fn default() -> Self {
        Self::new()
    }
}

/// 瀑布事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaterfallEvent {
    TurnStart { turn_id: String },
    StepStart { step_id: String },
    StepComplete { step_id: String, success: bool },
    TurnComplete { turn_id: String, success: bool },
    Error { turn_id: String, error: String },
    Rollback { turn_id: String, reason: String },
}

/// Step执行器
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute(&self, step: &Step) -> Result<StepResult, String>;
}

/// 内置Step执行器
pub struct BuiltinStepExecutor;

#[async_trait]
impl StepExecutor for BuiltinStepExecutor {
    async fn execute(&self, step: &Step) -> Result<StepResult, String> {
        // TODO: 实现内置执行逻辑
        Ok(StepResult {
            step_id: step.id.clone(),
            success: true,
            output: serde_json::json!({"action": step.action}),
            duration_ms: 0,
        })
    }
}
