// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 真实AI大模型客户端 - 支持OpenAI兼容API
//!
//! 支持多种API格式：
//! - OpenAI API (GPT-3.5/4, etc.)
//! - 本地Ollama
//! - 其他OpenAI兼容端点
//! - 内置规则引擎（无API key时降级使用）

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LLM配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enabled: bool,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-3.5-turbo".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            enabled: false,
        }
    }
}

/// Chat消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI格式请求
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<LLMChatMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

/// OpenAI格式响应
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// 真实AI LLM客户端
#[derive(Debug, Clone)]
pub struct LLMClient {
    config: LLMConfig,
    client: reqwest::Client,
    /// 系统提示词
    system_prompt: String,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        let system_prompt = r#"你是算子统一系统的AI智能助手，集成在v3.0 AI驱动mox 模块化系统架构突破平台中。

你的核心能力：
1. 🧮 算子编排执行 - 帮助用户组合和执行数学/AI算子（线性变换、激活函数、归一化、卷积、注意力等）
2. 📊 算法归一化 - 将任意算法分析为标准流程图，给出复杂度分析和优化建议
3. 💎 资源管理 - 监控系统资源（CPU、内存、GPU、插件、工作流）
4. 🔌 插件互通 - 管理插件消息总线，支持插件间发布订阅通信
5. 🎯 流程自动化 - 执行业务工作流（数据管道、算法分析、神经网络训练等）
6. 🌐 浏览器自动化 - 执行网页操作任务（导航、点击、填表、数据提取）

请用中文回答，简洁专业。当用户需要执行操作时，明确告知你可以调用的能力。"#
            .to_string();

        Self {
            config,
            client,
            system_prompt,
        }
    }

    pub fn update_config(&mut self, config: LLMConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &LLMConfig {
        &self.config
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.api_key.is_empty()
    }

    /// 调用真实LLM API
    pub async fn chat(&self, messages: Vec<LLMChatMessage>) -> anyhow::Result<String> {
        if !self.is_enabled() {
            return Err(anyhow::anyhow!("LLM not enabled or no API key configured"));
        }

        let mut full_messages = vec![LLMChatMessage {
            role: "system".to_string(),
            content: self.system_prompt.clone(),
        }];
        full_messages.extend(messages);

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: full_messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "API request failed ({}): {}",
                status,
                error_text
            ));
        }

        let completion: ChatCompletionResponse = response.json().await?;
        let content = completion
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "（无响应内容）".to_string());

        Ok(content)
    }

    /// 测试API连接
    pub async fn test_connection(&self) -> anyhow::Result<serde_json::Value> {
        if !self.is_enabled() {
            return Err(anyhow::anyhow!("LLM not enabled"));
        }

        let messages = vec![LLMChatMessage {
            role: "user".to_string(),
            content: "回复'连接成功'".to_string(),
        }];

        let result = self.chat(messages).await?;
        Ok(serde_json::json!({
            "success": true,
            "model": self.config.model,
            "response": result,
        }))
    }
}
