//! OpenAI Provider 实现（基于reqwest，支持HTTPS）

use crate::providers::dto::*;
use crate::providers::error::{AiError, AiResult};
use crate::providers::traits::AiProvider;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use std::time::Duration;

pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com".into())
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
        messages.iter().map(|m| {
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), serde_json::Value::String(m.role.as_str().into()));
            obj.insert("content".into(), serde_json::Value::String(m.content.clone()));
            if let Some(name) = &m.name {
                obj.insert("name".into(), serde_json::Value::String(name.clone()));
            }
            serde_json::Value::Object(obj)
        }).collect()
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn provider_id(&self) -> &'static str { "openai" }
    fn provider_name(&self) -> &'static str { "OpenAI" }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::ChatStream, Capability::Embedding, Capability::Vision, Capability::FunctionCalling]
    }

    fn available_models(&self) -> Vec<String> {
        vec!["gpt-4o".into(), "gpt-4o-mini".into(), "gpt-4-turbo".into(), "gpt-4".into(), "gpt-3.5-turbo".into()]
    }

    async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse> {
        let payload = serde_json::json!({
            "model": req.config.model,
            "messages": self.build_messages(&req.messages),
            "max_tokens": req.config.max_tokens,
            "temperature": req.config.temperature,
            "top_p": req.config.top_p,
            "stop": req.config.stop,
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
            return Err(match status.as_u16() {
                401 => AiError::AuthError("invalid api key".into()),
                404 => AiError::ModelNotFound(req.config.model.clone()),
                429 => AiError::RateLimited { retry_after_secs: None, message: "rate limited".into() },
                _ => AiError::Other(format!("HTTP {}: {}", status, &body[..body.len().min(200)])),
            });
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        let content = json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AiError::ParseError("no content in response".into()))?
            .to_string();

        let usage = json.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        }).unwrap_or_default();

        let finish_reason = json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("finish_reason"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ChatResponse {
            content,
            model: req.config.model.clone(),
            provider: self.provider_id().into(),
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(&self, req: &ChatRequest) -> AiResult<BoxStream<'static, AiResult<StreamChunk>>> {
        let mut payload = serde_json::json!({
            "model": req.config.model,
            "messages": self.build_messages(&req.messages),
            "max_tokens": req.config.max_tokens,
            "temperature": req.config.temperature,
            "stream": true,
        });
        if let Some(top_p) = req.config.top_p {
            payload["top_p"] = serde_json::json!(top_p);
        }

        let resp = self.client.post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await?;
            return Err(AiError::Other(format!("HTTP {}: {}", status, &body[..body.len().min(200)])));
        }

        let stream = resp.bytes_stream().map(|result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut contents = Vec::new();
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                contents.push(StreamChunk { content: String::new(), done: true });
                                continue;
                            }
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = json.get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|c| c.as_str())
                                {
                                    contents.push(StreamChunk { content: content.to_string(), done: false });
                                }
                            }
                        }
                    }
                    if contents.is_empty() {
                        Ok(StreamChunk { content: String::new(), done: false })
                    } else {
                        Ok(contents.into_iter().last().unwrap_or(StreamChunk { content: String::new(), done: false }))
                    }
                }
                Err(e) => Err(AiError::NetworkError(e.to_string())),
            }
        });

        Ok(Box::pin(stream))
    }

    async fn embed(&self, req: &EmbeddingRequest) -> AiResult<EmbeddingResponse> {
        let payload = serde_json::json!({
            "model": req.model,
            "input": req.texts,
        });

        let resp = self.client.post(format!("{}/v1/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AiError::Other(format!("HTTP {}", resp.status())));
        }

        let json: serde_json::Value = resp.json().await?;
        let embeddings = json.get("data")
            .and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(|item| {
                item.get("embedding").and_then(|e| e.as_array()).map(|vec| {
                    vec.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect()
                })
            }).collect())
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embeddings,
            model: req.model.clone(),
            usage: TokenUsage::default(),
        })
    }

    async fn health_check(&self) -> HealthStatus {
        let test_req = ChatRequest {
            messages: vec![ChatMessage::user("ping")],
            config: ModelConfig { model: "gpt-4o-mini".into(), max_tokens: 5, ..Default::default() },
        };
        match self.chat(&test_req).await {
            Ok(_) => HealthStatus::Healthy,
            Err(AiError::Timeout) => HealthStatus::Degraded,
            Err(_) => HealthStatus::Unhealthy,
        }
    }
}
