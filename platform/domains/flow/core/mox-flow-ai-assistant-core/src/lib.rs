// Copyright (c) 2026 璇玑 RelGraph · AI对话mox 模块化系统架构自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! AI 对话mox 模块化系统架构自动化核心
//!
//! 六大核心能力：
//! 1. 意图识别 (Intent Recognition) - 理解用户自然语言意图
//! 2. 任务分解 (Task Decomposition) - 将复杂任务拆解为子任务
//! 3. 多智能体协同 (Multi-Agent Collaboration) - 专家分工协作
//! 4. 自主执行 (Autonomous Execution) - 自动调用工具/API
//! 5. 对话管理 (Dialogue Management) - 上下文管理、多轮对话
//! 6. 工具注册 (Tool Registry) - 可扩展的工具/技能注册

pub mod error;
pub mod types;
pub mod intent;
pub mod task_decomposer;
pub mod agent;
pub mod tool_registry;
pub mod dialogue_manager;
pub mod executor;

pub use error::{AiError, AiResult};
pub use types::{
    IntentType, TaskPriority, TaskStatus, AgentRole, MessageType,
    ConversationMessage, Task, SubTask, AgentCapability,
};
pub use intent::{IntentRecognizer, IntentMatch, IntentPattern};
pub use task_decomposer::TaskDecomposer;
pub use agent::{Agent, AgentRegistry, AgentMessage};
pub use tool_registry::{ToolRegistry, ToolDef, ToolParam, ToolResult};
pub use dialogue_manager::{DialogueManager, Conversation, ConversationState};
pub use executor::TaskExecutor;
