//! 通义千问 Provider（政企AI示例，OpenAI兼容API）
//!
//! 阿里云通义千问兼容OpenAI API格式，只需改base_url和api_key。
//! 这展示了"新增Provider零改动核心"的模式。

use crate::providers::dto::*;
use crate::providers::error::{AiError, AiResult};
use crate::providers::traits::AiProvider;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::time::Duration;

pub struct QwenProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl QwenProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://dashscope.aliyuncs.com/compatible-mode".into())
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self { api_key, base_url: base_url.trim_end_matches('/').to_string(), client }
    }

    fn build_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages.iter().map(|m| serde_json::json!({
            "role": m.role.as_str(),
            "content": m.content,
        })).collect()
    }
}

#[async_trait]
impl AiProvider for QwenProvider {
    fn provider_id(&self) -> &'static str { "qwen" }
    fn provider_name(&self) -> &'static str { "通义千问" }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::ChatStream, Capability::Embedding, Capability::Vision]
    }

    fn available_models(&self) -> Vec<String> {
        vec!["qwen-max".into(), "qwen-plus".into(), "qwen-turbo".into(), "qwen-vl-max".into(), "text-embedding-v3".into()]
    }

    async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse> {
        let payload = serde_json::json!({
            "model": req.config.model,
            "messages": self.build_messages(&req.messages),
            "max_tokens": req.config.max_tokens,
            "temperature": req.config.temperature,
            "stream": false,
        });

        let resp = self.client.post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(AiError::Other(format!("HTTP {}: {}", status, &body[..body.len().min(200)])));
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        let content = json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AiError::ParseError("no content".into()))?
            .to_string();

        let usage = json.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        }).unwrap_or_default();

        Ok(ChatResponse {
            content,
            model: req.config.model.clone(),
            provider: self.provider_id().into(),
            usage,
            finish_reason: None,
        })
    }

    async fn chat_stream(&self, _req: &ChatRequest) -> AiResult<BoxStream<'static, AiResult<StreamChunk>>> {
        Err(AiError::UnsupportedCapability("qwen stream TODO".into()))
    }

    async fn health_check(&self) -> HealthStatus {
        let test_req = ChatRequest {
            messages: vec![ChatMessage::user("ping")],
            config: ModelConfig { model: "qwen-turbo".into(), max_tokens: 5, ..Default::default() },
        };
        match self.chat(&test_req).await {
            Ok(_) => HealthStatus::Healthy,
            Err(AiError::Timeout) => HealthStatus::Degraded,
            Err(_) => HealthStatus::Unhealthy,
        }
    }
}
