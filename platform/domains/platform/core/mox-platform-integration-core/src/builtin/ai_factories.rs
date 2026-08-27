// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 内置AI Provider Factory — OpenAI / Qwen / Anthropic

use crate::ai::providers::anthropic::AnthropicProvider;
use crate::ai::providers::openai::OpenAiProvider;
use crate::ai::providers::qwen::QwenProvider;
use crate::ai::providers::traits::AiProvider;
use crate::factory::{AiProviderFactory, FactoryConfig};
use async_trait::async_trait;
use std::sync::Arc;

/// OpenAI Factory
pub struct OpenAiFactory;

#[async_trait]
impl AiProviderFactory for OpenAiFactory {
    fn factory_type(&self) -> &'static str { "openai" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn AiProvider>> {
        let api_base = config.get_str("api_base").unwrap_or("https://api.openai.com").to_string();
        let api_key = config.get_str("api_key").unwrap_or("").to_string();
        let provider = OpenAiProvider::with_base_url(api_key, api_base);
        Ok(Arc::new(provider))
    }
}

/// 通义千问 Factory
pub struct QwenFactory;

#[async_trait]
impl AiProviderFactory for QwenFactory {
    fn factory_type(&self) -> &'static str { "qwen" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn AiProvider>> {
        let api_base = config.get_str("api_base").unwrap_or("https://dashscope.aliyuncs.com/compatible-mode").to_string();
        let api_key = config.get_str("api_key").unwrap_or("").to_string();
        let provider = QwenProvider::with_base_url(api_key, api_base);
        Ok(Arc::new(provider))
    }
}

/// Anthropic Claude Factory
pub struct AnthropicFactory;

#[async_trait]
impl AiProviderFactory for AnthropicFactory {
    fn factory_type(&self) -> &'static str { "anthropic" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn AiProvider>> {
        let api_base = config.get_str("api_base").unwrap_or("https://api.anthropic.com").to_string();
        let api_key = config.get_str("api_key").unwrap_or("").to_string();
        let provider = AnthropicProvider::with_base_url(api_key, api_base);
        Ok(Arc::new(provider))
    }
}
