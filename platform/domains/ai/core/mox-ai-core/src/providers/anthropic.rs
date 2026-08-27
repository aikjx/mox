// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Anthropic Provider 实现（Claude系列，基于reqwest）

use crate::providers::dto::*;
use crate::providers::error::{AiError, AiResult};
use crate::providers::traits::AiProvider;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::time::Duration;

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://api.anthropic.com".into())
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self { api_key, base_url: base_url.trim_end_matches('/').to_string(), client }
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn provider_id(&self) -> &'static str { "anthropic" }
    fn provider_name(&self) -> &'static str { "Anthropic" }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::Vision, Capability::FunctionCalling]
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".into(),
            "claude-3-5-haiku-20241022".into(),
            "claude-3-opus-20240229".into(),
        ]
    }

    async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse> {
        // Anthropic: system单独字段，messages不含system
        let system = req.messages.iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<serde_json::Value> = req.messages.iter()
            .filter(|m| m.role != Role::System)
            .map(|m| serde_json::json!({
                "role": m.role.as_str(),
                "content": m.content,
            }))
            .collect();

        let mut payload = serde_json::json!({
            "model": req.config.model,
            "max_tokens": req.config.max_tokens,
            "messages": messages,
            "temperature": req.config.temperature,
        });
        if let Some(s) = system {
            payload["system"] = serde_json::Value::String(s);
        }

        let resp = self.client.post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => AiError::AuthError("invalid api key".into()),
                404 => AiError::ModelNotFound(req.config.model.clone()),
                429 => AiError::RateLimited { retry_after_secs: None, message: "rate limited".into() },
                _ => AiError::Other(format!("HTTP {}: {}", status, &body[..body.len().min(200)])),
            });
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        let content = json.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| AiError::ParseError("no text in response".into()))?
            .to_string();

        let usage = json.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            total_tokens: 0,
        }).unwrap_or_default();

        Ok(ChatResponse {
            content,
            model: req.config.model.clone(),
            provider: self.provider_id().into(),
            usage,
            finish_reason: json.get("stop_reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    async fn chat_stream(&self, _req: &ChatRequest) -> AiResult<BoxStream<'static, AiResult<StreamChunk>>> {
        Err(AiError::UnsupportedCapability("anthropic streaming not yet implemented, use chat".into()))
    }

    async fn health_check(&self) -> HealthStatus {
        let test_req = ChatRequest {
            messages: vec![ChatMessage::user("ping")],
            config: ModelConfig { model: "claude-3-5-haiku-20241022".into(), max_tokens: 5, ..Default::default() },
        };
        match self.chat(&test_req).await {
            Ok(_) => HealthStatus::Healthy,
            Err(AiError::Timeout) => HealthStatus::Degraded,
            Err(_) => HealthStatus::Unhealthy,
        }
    }
}
