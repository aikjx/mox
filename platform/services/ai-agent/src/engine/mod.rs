//! AI Agent 引擎模块
//!
//! 提供核心运行时能力：
//! - 状态机 (state_machine): 定义 EngineState / EngineEvent / EngineFSM
//! - 守卫系统 (guards): BudgetGuard / ProgressGuard / RiskGuard / CompositeGuard
//! - 工具扩展 (tools): Tool trait / ToolRegistry / 内置工具集
//! - 主循环 (engine_loop): Engine 结构体串联完整 PERCEIVE→PLAN→ACT→OBSERVE→REFLECT→GENERATE→CONSOLIDATE 链路
//! - 多 Agent 协作 (multi_agent): AgentRole / SubAgent / MultiAgentOrchestrator

pub mod state_machine;
pub mod guards;
pub mod tools;
pub mod multi_agent;
pub mod engine_loop;

pub use state_machine::{EngineEvent, EngineFSM, EngineState, transition_table};
pub use guards::{
    BudgetGuard, CompositeGuard, Guard, GuardContext, GuardResult, ProgressGuard, RiskGuard,
    RiskLevel,
};
pub use tools::{
    CalculatorTool, CodeSandboxTool, DatabaseTool, FileOperationTool, HttpRequestTool, Tool,
    ToolRegistry, ToolResult,
};
pub use multi_agent::{AgentRole, MultiAgentOrchestrator, SubAgent};
pub use engine_loop::{Engine, EngineConfig, EngineContext, EngineResult};
pub use kg_hub::consolidator::ConsolidationResult;