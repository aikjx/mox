// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Turn/Agent/Step 生命周期管理
//!
//! 核心概念：
//! - Turn: 会话轮次
//! - Step: 执行步骤
//! - Waterfall: 瀑布式事件流

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub async fn execute_step(&self, turn_id: &str, step: Step) -> Result<StepResult, String> {
        // 先取出 turn 并立即释放写锁，避免持同步锁跨 await（死锁风险）
        let mut turn = self
            .active_turns
            .write()
            .remove(turn_id)
            .ok_or_else(|| format!("Turn not found: {}", turn_id))?;
        let result = turn.execute_step(step).await;
        // 执行完毕写回活动表（execute_step 未完成 turn，需保留）
        self.active_turns.write().insert(turn_id.to_string(), turn);
        result
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

    /// 执行Step（委托内置执行器按动作协议真实执行）
    pub async fn execute_step(&mut self, mut step: Step) -> Result<StepResult, String> {
        // 更新状态
        self.state = TurnState::Running;

        // 执行步骤
        let start_time = std::time::Instant::now();
        step.state = StepState::Running;

        // 委托内置执行器执行动作协议；失败时真实标记 Failed 并推送错误事件
        let executor = BuiltinStepExecutor;
        let result = match executor.execute(&step).await {
            Ok(mut r) => {
                r.duration_ms = start_time.elapsed().as_millis() as u64;
                step.state = StepState::Completed;
                step.result = Some(r.clone());
                self.events.push(WaterfallEvent::StepComplete {
                    step_id: step.id.clone(),
                    success: r.success,
                });
                r
            }
            Err(e) => {
                step.state = StepState::Failed;
                step.result = Some(StepResult {
                    step_id: step.id.clone(),
                    success: false,
                    output: serde_json::json!({ "error": e }),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
                self.events.push(WaterfallEvent::Error {
                    turn_id: self.id.clone(),
                    error: e.clone(),
                });
                self.steps.push(step);
                return Err(e);
            }
        };

        // 添加到步骤列表
        self.steps.push(step);

        Ok(result)
    }

    /// 完成Turn
    pub fn complete(mut self) -> Result<TurnSummary, String> {
        self.state = TurnState::Completed;
        self.completed_at = Some(chrono::Utc::now());

        let total_duration_ms = self
            .steps
            .iter()
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
        Self { events: Vec::new() }
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
///
/// 内置动作协议（确定性算法，非占位）：
/// - `echo`：回显输入（连通性验证）
/// - `math:add` / `math:sub` / `math:mul` / `math:div`：输入 `{"a": 1, "b": 2}` 数值运算
/// - `json:pick`：输入 `{"path": "/k", "input": {...}}` 按 JSON Pointer 提取字段
/// - 未知动作：明确报错（绝不静默成功）
pub struct BuiltinStepExecutor;

impl BuiltinStepExecutor {
    /// 动作协议分发：确定性算法动作
    pub fn dispatch(action: &str, input: &serde_json::Value) -> Result<serde_json::Value, String> {
        match action {
            "echo" => Ok(input.clone()),
            "math:add" | "math:sub" | "math:mul" | "math:div" => {
                let a = input
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| format!("math 动作需要数值字段 a，收到: {}", input))?;
                let b = input
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| format!("math 动作需要数值字段 b，收到: {}", input))?;
                let value = match action {
                    "math:add" => a + b,
                    "math:sub" => a - b,
                    "math:mul" => a * b,
                    "math:div" => {
                        if b == 0.0 {
                            return Err("math:div 除数为零".to_string());
                        }
                        a / b
                    }
                    _ => unreachable!("matched action"),
                };
                Ok(serde_json::json!({ "action": action, "a": a, "b": b, "result": value }))
            }
            "json:pick" => {
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("json:pick 需要字符串字段 path，收到: {}", input))?;
                let obj = input
                    .get("input")
                    .ok_or_else(|| format!("json:pick 需要 input 对象，收到: {}", input))?;
                let picked = obj
                    .pointer(path)
                    .ok_or_else(|| format!("json:pick 路径 {} 不存在", path))?;
                Ok(serde_json::json!({ "action": action, "path": path, "value": picked }))
            }
            other => Err(format!("未知内置动作: {}", other)),
        }
    }
}

#[async_trait]
impl StepExecutor for BuiltinStepExecutor {
    async fn execute(&self, step: &Step) -> Result<StepResult, String> {
        let start_time = std::time::Instant::now();
        let output = Self::dispatch(&step.action, &step.input)?;
        Ok(StepResult {
            step_id: step.id.clone(),
            success: true,
            output,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_echo() {
        let out =
            BuiltinStepExecutor::dispatch("echo", &serde_json::json!({"k": "v"})).expect("echo ok");
        assert_eq!(out, serde_json::json!({"k": "v"}));
    }

    #[test]
    fn test_dispatch_math() {
        let out =
            BuiltinStepExecutor::dispatch("math:add", &serde_json::json!({"a": 2.0, "b": 3.0}))
                .expect("add ok");
        assert_eq!(out["result"], 5.0);

        let out =
            BuiltinStepExecutor::dispatch("math:mul", &serde_json::json!({"a": 4.0, "b": 0.5}))
                .expect("mul ok");
        assert_eq!(out["result"], 2.0);

        // 除零真实报错
        let err =
            BuiltinStepExecutor::dispatch("math:div", &serde_json::json!({"a": 1.0, "b": 0.0}))
                .expect_err("div by zero rejected");
        assert!(err.contains("除数为零"));
    }

    #[test]
    fn test_dispatch_math_missing_field() {
        let err = BuiltinStepExecutor::dispatch("math:add", &serde_json::json!({"a": 1.0}))
            .expect_err("missing b rejected");
        assert!(err.contains("字段 b"));
    }

    #[test]
    fn test_dispatch_json_pick() {
        let out = BuiltinStepExecutor::dispatch(
            "json:pick",
            &serde_json::json!({"path": "/user/name", "input": {"user": {"name": "alice"}}}),
        )
        .expect("pick ok");
        assert_eq!(out["value"], "alice");

        let err = BuiltinStepExecutor::dispatch(
            "json:pick",
            &serde_json::json!({"path": "/nope", "input": {"user": {"name": "alice"}}}),
        )
        .expect_err("missing path rejected");
        assert!(err.contains("不存在"));
    }

    #[test]
    fn test_dispatch_unknown_action() {
        let err = BuiltinStepExecutor::dispatch("no:such", &serde_json::json!({}))
            .expect_err("unknown action rejected");
        assert!(err.contains("未知内置动作"));
    }

    #[tokio::test]
    async fn test_turn_execute_step_success() {
        let mut turn = Turn::new("t1".to_string(), "agent-a".to_string());
        let step = Step::new(
            "math:add".to_string(),
            serde_json::json!({"a": 10.0, "b": 5.0}),
        );

        let result = turn.execute_step(step).await.expect("step ok");
        assert!(result.success);
        assert_eq!(result.output["result"], 15.0);
        assert_eq!(turn.steps.len(), 1);
        assert_eq!(turn.steps[0].state, StepState::Completed);
    }

    #[tokio::test]
    async fn test_turn_execute_step_failure_marks_failed() {
        let mut turn = Turn::new("t2".to_string(), "agent-a".to_string());
        let step = Step::new(
            "math:div".to_string(),
            serde_json::json!({"a": 1.0, "b": 0.0}),
        );

        let err = turn.execute_step(step).await.expect_err("should fail");
        assert!(err.contains("除数为零"));
        assert_eq!(turn.steps.len(), 1);
        assert_eq!(turn.steps[0].state, StepState::Failed);
        // 失败事件已入瀑布流
        let has_error = turn
            .events
            .events()
            .iter()
            .any(|e| matches!(e, WaterfallEvent::Error { .. }));
        assert!(has_error);
    }

    #[tokio::test]
    async fn test_lifecycle_manager_roundtrip() {
        let mgr = LifecycleManager::new();
        let turn_id = mgr.create_turn("agent-x").await.expect("turn created");

        let step = Step::new("echo".to_string(), serde_json::json!({"ping": true}));
        let result = mgr.execute_step(&turn_id, step).await.expect("step ok");
        assert!(result.success);
        assert_eq!(result.output["ping"], true);

        let summary = mgr.complete_turn(&turn_id).await.expect("turn completed");
        assert!(summary.success);
        assert_eq!(summary.steps, 1);
    }
}
