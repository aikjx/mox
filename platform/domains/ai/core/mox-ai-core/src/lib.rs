// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # Mox AI Core — AI Provider Gateway
//!
//! 多Provider LLM抽象层，支持注册表、路由器、自动降级、熔断。
//!
//! ## 核心组件
//! - [`providers`] — AI Provider trait + 内置实现（OpenAI/Anthropic/Qwen/...）
//! - [`registry`] — Provider注册表（运行时动态注册）
//! - [`router`] — 模型路由器（策略路由+自动降级+熔断）
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_ai_core::prelude::*;
//! use std::sync::Arc;
//!
//! // 1. 创建注册表
//! let registry = Arc::new(ProviderRegistry::new());
//!
//! // 2. 注册Provider（新增Provider零改动核心）
//! registry.register(Arc::new(OpenAiProvider::new("sk-xxx".into())));
//! registry.register(Arc::new(QwenProvider::new("sk-xxx".into())));
//! registry.register(Arc::new(AnthropicProvider::new("sk-xxx".into())));
//!
//! // 3. 创建路由器（配置降级链）
//! let router = ModelRouter::new(registry.clone())
//!     .with_strategy(RoutingStrategy::Priority)
//!     .with_fallback_chain(vec!["qwen".into(), "anthropic".into()]);
//!
//! // 4. 注册模型路由
//! router.register_route("gpt-4o", vec![
//!     RouteEntry { provider_id: "openai".into(), priority: 0, weight: 100, enabled: true },
//!     RouteEntry { provider_id: "qwen".into(), priority: 1, weight: 50, enabled: true },
//! ]).await;
//!
//! // 5. 调用（自动降级）
//! let req = ChatRequest {
//!     messages: vec![ChatMessage::user("你好")],
//!     config: ModelConfig::default(),
//! };
//! let resp = router.chat(&req).await.unwrap();
//! println!("{}", resp.content);
//! ```

pub mod chat;
pub mod graph;
pub mod providers;
pub mod reasoning;
pub mod registry;
pub mod router;

// ─── 统一重导出 ──────────────────────────────────────────────────────────────

// 核心类型
pub use providers::{
    AiError, AiProvider, AiResult, AnthropicProvider, Capability, ChatMessage, ChatRequest,
    ChatResponse, EmbeddingRequest, EmbeddingResponse, HealthStatus, ModelConfig, OpenAiProvider,
    QwenProvider, Role, StreamChunk, TokenUsage,
};

// 注册表 + 路由器
pub use registry::ProviderRegistry;
pub use router::{ModelRouter, RouteEntry, RoutingStrategy};

// 对话 + 图谱 + 推理
pub use chat::{ChatHistory, ChatSession, Message, SessionRegistry};
pub use graph::{
    AssociationType, GraphEdge, GraphNode, GraphStats, NodeId, RelationId, MoxGraph,
};
pub use reasoning::{
    AiReasoner, CausalAnalysisResult, ExtractedEdge, ExtractedNode, ExtractionSummary,
    GraphAwareReasoner, ReasoningCapability, ReasoningRequest, ReasoningResult, SemanticMatch,
};

// ─── 配置类型 ────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// AI 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub default_provider: String,
    pub default_model: String,
    pub providers: Vec<ProviderConfig>,
    pub fallback_chain: Vec<String>,
    pub routing_strategy: RoutingStrategyConfig,
    pub max_context_tokens: usize,
    pub max_history_turns: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".into(),
            default_model: "gpt-4o-mini".into(),
            providers: vec![],
            fallback_chain: vec![],
            routing_strategy: RoutingStrategyConfig::Priority,
            max_context_tokens: 8192,
            max_history_turns: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategyConfig {
    Priority,
    RoundRobin,
    LatencyPriority,
    CostPriority,
}

impl From<RoutingStrategyConfig> for RoutingStrategy {
    fn from(c: RoutingStrategyConfig) -> Self {
        match c {
            RoutingStrategyConfig::Priority => RoutingStrategy::Priority,
            RoutingStrategyConfig::RoundRobin => RoutingStrategy::RoundRobin,
            RoutingStrategyConfig::LatencyPriority => RoutingStrategy::LatencyPriority,
            RoutingStrategyConfig::CostPriority => RoutingStrategy::CostPriority,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    Openai { api_key: String, base_url: Option<String> },
    Anthropic { api_key: String, base_url: Option<String> },
    Qwen { api_key: String, base_url: Option<String> },
    Wenxin { api_key: String, secret_key: String },
    Glm { api_key: String, base_url: Option<String> },
    Ollama { base_url: String },
}

impl ProviderConfig {
    /// 根据配置构建Provider并注册到注册表
    pub fn register_to(&self, registry: &ProviderRegistry) {
        match self {
            ProviderConfig::Openai { api_key, base_url } => {
                let p = match base_url {
                    Some(url) => OpenAiProvider::with_base_url(api_key.clone(), url.clone()),
                    None => OpenAiProvider::new(api_key.clone()),
                };
                registry.register(std::sync::Arc::new(p));
            }
            ProviderConfig::Anthropic { api_key, base_url } => {
                let p = match base_url {
                    Some(url) => AnthropicProvider::with_base_url(api_key.clone(), url.clone()),
                    None => AnthropicProvider::new(api_key.clone()),
                };
                registry.register(std::sync::Arc::new(p));
            }
            ProviderConfig::Qwen { api_key, base_url } => {
                let p = match base_url {
                    Some(url) => QwenProvider::with_base_url(api_key.clone(), url.clone()),
                    None => QwenProvider::new(api_key.clone()),
                };
                registry.register(std::sync::Arc::new(p));
            }
            _ => { tracing::warn!("provider config not yet implemented: {:?}", self); }
        }
    }
}

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}
