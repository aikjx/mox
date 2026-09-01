// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 真实大模型（LLM）专家咨询子模块
//!
//! - [`chat`]：OpenAI 兼容聊天客户端（生产 `OpenAiChatClient` / 测试 `MockChatClient`）
//! - [`router`]：轻量 LLM Provider 路由器与熔断器（多 Provider 路由 + 熔断降级）
//! - [`tools`]：ReAct 工具注册表（calculate / now / expert_lookup）
//! - [`react`]：ReAct 推理循环（`<tool_call>` 协议）
//! - [`consultant`]：`LlmExpertConsultant` —— 接入 `ExpertConsultant` trait 的入口

pub mod chat;
pub mod consultant;
pub mod react;
pub mod router;
pub mod tools;

pub use chat::{
    ChatClient, ChatMessage, ChatRole, LlmConfig, OpenAiChatClient, ProviderConfig,
    RoutingStrategy,
};
pub use consultant::{expert_role, llm_consultant_from_env, LlmExpertConsultant};
pub use react::{run_react, ReactConfig, ReactResult};
pub use router::{LlmRouter, ProviderHealth, ProviderRuntimeState};
pub use tools::{CalculateTool, NowTool, Tool, ToolRegistry};
