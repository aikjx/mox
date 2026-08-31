//! LLM Provider 抽象
//!
//! 定义统一的 LLM 接口，支持多种后端实现

use async_trait::async_trait;
use crate::message::{ChatMessage, ChatCompletion};
use crate::types::{CompletionOptions, Embedding};
use crate::error::AiError;

pub type AiResult<T> = Result<T, AiError>;

/// LLM Provider 接口
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 名称
    fn name(&self) -> &str;

    /// 聊天完成
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: &CompletionOptions,
    ) -> AiResult<ChatCompletion>;

    /// 生成 Embedding
    async fn embed(&self, texts: &[String]) -> AiResult<Vec<Embedding>>;
}

/// Provider 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// 本地 vLLM
    LocalVllm,
    /// OpenAI 兼容
    OpenAi,
    /// Anthropic
    Anthropic,
}
