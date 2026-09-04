// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AI Provider 核心 trait — 所有AI后端必须实现

use crate::providers::dto::*;
use crate::providers::error::{AiError, AiResult};
use async_trait::async_trait;
use futures::stream::BoxStream;

/// AI Provider 统一接口
///
/// 新增AI提供商只需实现此trait + 注册到 ProviderRegistry，
/// 核心代码零改动。
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Provider唯一标识（如 "openai", "qwen", "anthropic"）
    fn provider_id(&self) -> &'static str;

    /// Provider显示名称
    fn provider_name(&self) -> &'static str;

    /// 支持的能力列表
    fn capabilities(&self) -> Vec<Capability>;

    /// 可用模型列表
    fn available_models(&self) -> Vec<String>;

    /// 检查是否支持指定能力
    fn supports(&self, cap: Capability) -> bool {
        self.capabilities().contains(&cap)
    }

    /// 同步对话
    async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse>;

    /// 流式对话
    async fn chat_stream(&self, req: &ChatRequest) -> AiResult<BoxStream<'static, AiResult<StreamChunk>>>;

    /// 文本嵌入（可选实现）
    async fn embed(&self, _req: &EmbeddingRequest) -> AiResult<EmbeddingResponse> {
        Err(AiError::UnsupportedCapability("embedding".into()))
    }

    /// 健康检查
    async fn health_check(&self) -> HealthStatus;

    /// 估算token数（粗略）
    fn estimate_tokens(&self, text: &str) -> usize {
        let chinese = text.chars().filter(|c| !c.is_ascii()).count();
        let english_words = text.split_whitespace().filter(|w| w.chars().all(|c| c.is_ascii())).count();
        chinese * 2 + english_words / 4
    }
}
