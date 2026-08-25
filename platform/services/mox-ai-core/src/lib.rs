//! # Mox AI Core
//!
//! 从 `D:\a10\mox` 单体原型全维整合的 AI 核心能力 crate。
//!
//! ## 模块结构
//!
//! - [`graph`] — 轻量内存图谱（节点/边/邻接表/统计），为 AI 提供上下文数据基础
//! - [`providers`] — 多 Provider LLM 抽象层（OpenAI / Anthropic / Local LLM），纯 std HTTP 客户端
//! - [`chat`] — 对话会话管理（多轮对话、图谱上下文注入、Session Registry）
//! - [`reasoning`] — AI 增强图谱推理（意图识别、语义搜索、知识抽取、因果推理）
//!
//! ## 设计原则
//!
//! - **零外部 HTTP 依赖**：providers 模块基于 `std::net::TcpStream` 实现 HTTP/1.1，避免 reqwest 大量 transitive deps
//! - **Send + Sync**：所有 Provider 和 Session 均线程安全，支持并发调用
//! - **流式支持**：OpenAI SSE 流式解析器实现 `Read` trait，可逐字节消费
//! - **图谱感知**：对话和推理模块均可注入 MoxGraph 上下文，实现 RAG 式增强
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_ai_core::providers::{OpenAiProvider, AiProvider, ChatMessage, ModelConfig};
//!
//! let provider = OpenAiProvider::new("sk-xxx".into());
//! let messages = vec![
//!     ChatMessage::system("你是助手"),
//!     ChatMessage::user("你好"),
//! ];
//! let config = ModelConfig::default();
//! let response = provider.chat_sync(&messages, &config).unwrap();
//! ```

pub mod chat;
pub mod graph;
pub mod providers;
pub mod reasoning;

// ─── 统一重导出 ──────────────────────────────────────────────────────────────

// 图谱
pub use graph::{
    AssociationType, GraphEdge, GraphNode, GraphStats, NodeId, RelationId, MoxGraph,
};

// AI Providers
pub use providers::{
    AiProvider, AiProviderError, AiStream, AnthropicProvider, ChatMessage, LocalLlmProvider,
    ModelConfig, OpenAiProvider, OpenAiSseStream, Role,
};

// 对话
pub use chat::{ChatHistory, ChatSession, Message, SessionRegistry};

// 推理
pub use reasoning::{
    AiReasoner, CausalAnalysisResult, ExtractedEdge, ExtractedNode, ExtractionSummary,
    GraphAwareReasoner, ReasoningCapability, ReasoningRequest, ReasoningResult, SemanticMatch,
};

// ─── 配置类型 ────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// AI 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// 默认 provider 类型
    pub default_provider: ProviderType,
    /// Provider 配置列表
    pub providers: Vec<ProviderConfig>,
    /// 默认模型
    pub default_model: String,
    /// 图谱上下文注入的最大 token 数
    pub max_context_tokens: usize,
    /// 对话历史保留轮数
    pub max_history_turns: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderType::OpenAI,
            providers: vec![],
            default_model: "gpt-4o-mini".into(),
            max_context_tokens: 8192,
            max_history_turns: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Local, // 本地 LLM（llm crate）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    OpenAI {
        api_key: String,
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
        base_url: Option<String>,
    },
    Local {
        model_path: String,
        context_size: usize,
    },
}

impl Default for ProviderType {
    fn default() -> Self {
        ProviderType::OpenAI
    }
}

impl ProviderConfig {
    /// 根据配置构建对应的 AiProvider
    pub fn build_provider(&self) -> Box<dyn AiProvider> {
        match self {
            ProviderConfig::OpenAI { api_key, base_url } => match base_url {
                Some(url) => Box::new(OpenAiProvider::with_base_url(
                    api_key.clone(),
                    url.clone(),
                )),
                None => Box::new(OpenAiProvider::new(api_key.clone())),
            },
            ProviderConfig::Anthropic { api_key, base_url } => match base_url {
                Some(url) => Box::new(AnthropicProvider::with_base_url(
                    api_key.clone(),
                    url.clone(),
                )),
                None => Box::new(AnthropicProvider::new(api_key.clone())),
            },
            ProviderConfig::Local { model_path, .. } => {
                Box::new(LocalLlmProvider::new(model_path.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_default() {
        let config = AiConfig::default();
        assert_eq!(config.default_provider, ProviderType::OpenAI);
        assert_eq!(config.default_model, "gpt-4o-mini");
        assert_eq!(config.max_context_tokens, 8192);
        assert_eq!(config.max_history_turns, 20);
    }

    #[test]
    fn test_provider_config_openai() {
        let config = ProviderConfig::OpenAI {
            api_key: "test-key".into(),
            base_url: None,
        };
        let provider = config.build_provider();
        assert_eq!(provider.provider_name(), "OpenAI");
    }

    #[test]
    fn test_provider_config_anthropic() {
        let config = ProviderConfig::Anthropic {
            api_key: "test-key".into(),
            base_url: Some("https://custom.anthropic.com".into()),
        };
        let provider = config.build_provider();
        assert_eq!(provider.provider_name(), "Anthropic");
    }

    #[test]
    fn test_provider_config_local() {
        let config = ProviderConfig::Local {
            model_path: "/models/llama".into(),
            context_size: 4096,
        };
        let provider = config.build_provider();
        assert_eq!(provider.provider_name(), "Local LLM");
    }

    #[test]
    fn test_provider_type_serialization() {
        let pt = ProviderType::Anthropic;
        let json = serde_json::to_string(&pt).unwrap();
        let deserialized: ProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, deserialized);
    }

    #[test]
    fn test_provider_config_serialization() {
        let config = ProviderConfig::OpenAI {
            api_key: "key".into(),
            base_url: Some("http://localhost:8080".into()),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("OpenAI"));
        let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            ProviderConfig::OpenAI { api_key, base_url } => {
                assert_eq!(api_key, "key");
                assert_eq!(base_url, Some("http://localhost:8080".into()));
            }
            _ => panic!("wrong variant"),
        }
    }
}
