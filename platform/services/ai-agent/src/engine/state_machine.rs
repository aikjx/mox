//! 引擎状态机 - 定义核心状态与事件流转规则
//!
//! 采用有限状态机（FSM）模式管理引擎生命周期，确保状态转移的合法性与可追溯性。
//! 核心状态覆盖从感知、规划、执行、观察、反思到生成和巩固的完整闭环。

use serde::{Deserialize, Serialize};

/// 引擎核心状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    /// 空闲，等待任务分配
    Idle,
    /// 感知阶段：收集环境信息、解析用户输入
    Perceive,
    /// 召回阶段：从知识图谱/向量库检索相关记忆
    Recall,
    /// 规划阶段：制定执行计划、分解子任务
    Plan,
    /// 执行阶段：调用工具/API/算子完成任务
    Act,
    /// 观察阶段：收集执行结果、检测异常
    Observe,
    /// 反思阶段：评估结果质量、决定是否重试
    Reflect,
    /// 人机协同暂停：等待用户确认或输入
    HitlPause,
    /// 生成阶段：整合结果、产出最终输出
    Generate,
    /// 巩固阶段：持久化经验、更新知识库
    Consolidate,
    /// 完成：所有阶段正常结束
    Done,
    /// 中止：因错误或用户干预而终止
    Abort,
}

/// 触发状态转移的事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EngineEvent {
    /// 启动任务
    Start,
    /// 感知完成
    PerceiveDone,
    /// 召回完成
    RecallDone,
    /// 规划完成
    PlanDone,
    /// 执行完成
    ActDone,
    /// 执行失败
    ActFailed,
    /// 观察完成
    ObserveDone,
    /// 反思完成，继续循环
    ReflectContinue,
    /// 反思完成，进入生成
    ReflectToGenerate,
    /// 需要人工介入
    NeedHumanInput,
    /// 人工确认通过
    HumanApproved,
    /// 人工拒绝
    HumanRejected,
    /// 生成完成
    GenerateDone,
    /// 巩固完成
    ConsolidateDone,
    /// 强制中止
    Abort,
    /// 重置到空闲
    Reset,
}

/// 有限状态机实例
pub struct EngineFSM {
    current: EngineState,
    transition_count: u64,
    history: Vec<(EngineState, EngineEvent, EngineState)>,
}

impl EngineFSM {
    pub fn new() -> Self {
        Self {
            current: EngineState::Idle,
            transition_count: 0,
            history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> &EngineState {
        &self.current
    }

    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }

    pub fn history(&self) -> &[(EngineState, EngineEvent, EngineState)] {
        &self.history
    }

    /// 尝试触发状态转移，返回新状态或拒绝原因
    pub fn trigger(&mut self, event: EngineEvent) -> Result<&EngineState, String> {
        let next = Self::next_state(&self.current, &event)?;
        let prev = std::mem::replace(&mut self.current, next);
        self.history.push((prev, event, self.current.clone()));
        self.transition_count += 1;
        Ok(&self.current)
    }

    /// 定义合法的状态转移规则
    fn next_state(current: &EngineState, event: &EngineEvent) -> Result<EngineState, String> {
        let key = (current.clone(), event.clone());
        match key {
            // ── 启动链路 ──
            (EngineState::Idle, EngineEvent::Start) => Ok(EngineState::Perceive),

            // ── 主循环: Perceive → Recall → Plan → Act → Observe → Reflect ──
            (EngineState::Perceive, EngineEvent::PerceiveDone) => Ok(EngineState::Recall),
            (EngineState::Recall, EngineEvent::RecallDone) => Ok(EngineState::Plan),
            (EngineState::Plan, EngineEvent::PlanDone) => Ok(EngineState::Act),
            (EngineState::Act, EngineEvent::ActDone) => Ok(EngineState::Observe),
            (EngineState::Act, EngineEvent::ActFailed) => Ok(EngineState::Reflect),
            (EngineState::Observe, EngineEvent::ObserveDone) => Ok(EngineState::Reflect),

            // ── Reflect 分支 ──
            (EngineState::Reflect, EngineEvent::ReflectContinue) => Ok(EngineState::Act),
            (EngineState::Reflect, EngineEvent::ReflectToGenerate) => Ok(EngineState::Generate),
            (EngineState::Reflect, EngineEvent::NeedHumanInput) => Ok(EngineState::HitlPause),

            // ── HITL 分支 ──
            (EngineState::HitlPause, EngineEvent::HumanApproved) => Ok(EngineState::Act),
            (EngineState::HitlPause, EngineEvent::HumanRejected) => Ok(EngineState::Abort),

            // ── 收尾链路 ──
            (EngineState::Generate, EngineEvent::GenerateDone) => Ok(EngineState::Consolidate),
            (EngineState::Consolidate, EngineEvent::ConsolidateDone) => Ok(EngineState::Done),

            // ── 强制中止 ──
            (_, EngineEvent::Abort) => Ok(EngineState::Abort),

            // ── 重置 ──
            (EngineState::Done, EngineEvent::Reset) => Ok(EngineState::Idle),
            (EngineState::Abort, EngineEvent::Reset) => Ok(EngineState::Idle),

            // ── 所有非法转移 ──
            (state, event) => Err(format!("非法状态转移: {:?} + {:?}", state, event)),
        }
    }
}

impl Default for EngineFSM {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态转移表（可用于调试/可视化）
pub fn transition_table() -> Vec<(EngineState, EngineEvent, EngineState)> {
    vec![
        (EngineState::Idle, EngineEvent::Start, EngineState::Perceive),
        (
            EngineState::Perceive,
            EngineEvent::PerceiveDone,
            EngineState::Recall,
        ),
        (
            EngineState::Recall,
            EngineEvent::RecallDone,
            EngineState::Plan,
        ),
        (EngineState::Plan, EngineEvent::PlanDone, EngineState::Act),
        (EngineState::Act, EngineEvent::ActDone, EngineState::Observe),
        (
            EngineState::Act,
            EngineEvent::ActFailed,
            EngineState::Reflect,
        ),
        (
            EngineState::Observe,
            EngineEvent::ObserveDone,
            EngineState::Reflect,
        ),
        (
            EngineState::Reflect,
            EngineEvent::ReflectContinue,
            EngineState::Act,
        ),
        (
            EngineState::Reflect,
            EngineEvent::ReflectToGenerate,
            EngineState::Generate,
        ),
        (
            EngineState::Reflect,
            EngineEvent::NeedHumanInput,
            EngineState::HitlPause,
        ),
        (
            EngineState::HitlPause,
            EngineEvent::HumanApproved,
            EngineState::Act,
        ),
        (
            EngineState::HitlPause,
            EngineEvent::HumanRejected,
            EngineState::Abort,
        ),
        (
            EngineState::Generate,
            EngineEvent::GenerateDone,
            EngineState::Consolidate,
        ),
        (
            EngineState::Consolidate,
            EngineEvent::ConsolidateDone,
            EngineState::Done,
        ),
        (EngineState::Done, EngineEvent::Reset, EngineState::Idle),
        (EngineState::Abort, EngineEvent::Reset, EngineState::Idle),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_to_perceive() {
        let mut fsm = EngineFSM::new();
        assert_eq!(*fsm.current_state(), EngineState::Idle);
        fsm.trigger(EngineEvent::Start).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::Perceive);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut fsm = EngineFSM::new();
        fsm.trigger(EngineEvent::Start).unwrap();
        fsm.trigger(EngineEvent::PerceiveDone).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::Recall);
        fsm.trigger(EngineEvent::RecallDone).unwrap();
        fsm.trigger(EngineEvent::PlanDone).unwrap();
        fsm.trigger(EngineEvent::ActDone).unwrap();
        fsm.trigger(EngineEvent::ObserveDone).unwrap();
        fsm.trigger(EngineEvent::ReflectToGenerate).unwrap();
        fsm.trigger(EngineEvent::GenerateDone).unwrap();
        fsm.trigger(EngineEvent::ConsolidateDone).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::Done);
        assert_eq!(fsm.transition_count(), 9);
    }

    #[test]
    fn test_hitl_flow() {
        let mut fsm = EngineFSM::new();
        fsm.trigger(EngineEvent::Start).unwrap();
        fsm.trigger(EngineEvent::PerceiveDone).unwrap();
        fsm.trigger(EngineEvent::RecallDone).unwrap();
        fsm.trigger(EngineEvent::PlanDone).unwrap();
        fsm.trigger(EngineEvent::ActFailed).unwrap();
        fsm.trigger(EngineEvent::NeedHumanInput).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::HitlPause);
        fsm.trigger(EngineEvent::HumanApproved).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::Act);
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let mut fsm = EngineFSM::new();
        let result = fsm.trigger(EngineEvent::ActDone);
        assert!(result.is_err());
    }

    #[test]
    fn test_abort_from_any_state() {
        let mut fsm = EngineFSM::new();
        fsm.trigger(EngineEvent::Start).unwrap();
        fsm.trigger(EngineEvent::PerceiveDone).unwrap();
        fsm.trigger(EngineEvent::Abort).unwrap();
        assert_eq!(*fsm.current_state(), EngineState::Abort);
    }

    #[test]
    fn test_history_tracking() {
        let mut fsm = EngineFSM::new();
        fsm.trigger(EngineEvent::Start).unwrap();
        fsm.trigger(EngineEvent::PerceiveDone).unwrap();
        let history = fsm.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, EngineState::Idle);
        assert_eq!(history[0].1, EngineEvent::Start);
        assert_eq!(history[0].2, EngineState::Perceive);
    }
}
