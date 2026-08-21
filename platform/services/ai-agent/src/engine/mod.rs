//! AI Agent 引擎模块
//!
//! 提供核心运行时能力：
//! - 状态机 (state_machine): 定义 EngineState / EngineEvent / EngineFSM
//! - 守卫系统 (guards): BudgetGuard / ProgressGuard / RiskGuard / CompositeGuard
//! - 主循环 (engine_loop): Engine 结构体串联完整 PERCEIVE→PLAN→ACT→OBSERVE→REFLECT→GENERATE→CONSOLIDATE 链路

pub mod state_machine;
pub mod guards;
pub mod engine_loop;

pub use state_machine::{EngineEvent, EngineFSM, EngineState, transition_table};
pub use guards::{
    BudgetGuard, CompositeGuard, Guard, GuardContext, GuardResult, ProgressGuard, RiskGuard,
    RiskLevel,
};
pub use engine_loop::{Engine, EngineConfig, EngineContext, EngineResult};
pub use kg_hub::consolidator::ConsolidationResult;