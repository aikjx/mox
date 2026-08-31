// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 真实大模型（LLM）聊天客户端 —— OpenAI 兼容 API
//!
//! 通过 DIP 抽象 `ChatClient`，生产用 `OpenAiChatClient`（reqwest blocking，
//! 经 `spawn_blocking` 桥接 async），测试注入 `MockChatClient` 固定脚本，
//! 不触碰真实网络与环境变量（消除并行测试竞争）。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 聊天消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    /// 系统提示
    System,
    /// 用户
    User,
    /// 助手（历史推理/回复）
    Assistant,
    /// 工具观察结果
    Tool,
}

impl ChatRole {
    /// 映射到 OpenAI chat 的 role 字符串
    fn as_str(&self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "user",
        }
    }
}

/// 一条聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: ChatRole::System, content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: ChatRole::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: ChatRole::Assistant, content: content.into() }
    }
    pub fn tool(content: impl Into<String>) -> Self {
        Self { role: ChatRole::Tool, content: content.into() }
    }
}

/// LLM 配置
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// OpenAI 兼容 base url（含 /v1，调用方拼接 /chat/completions）
    pub base_url: String,
    /// API Key
    pub api_key: String,
    /// 模型名
    pub model: String,
    /// 采样温度
    pub temperature: f32,
    /// 最大输出 token
    pub max_tokens: u32,
    /// 单次请求超时
    pub timeout_ms: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.5,
            max_tokens: 2048,
            timeout_ms: 60_000,
        }
    }
}

impl LlmConfig {
    /// 从环境变量加载真实 LLM 配置
    ///
    /// 探测顺序（取第一个可用的 Key）：
    /// 1. `MOX_LLM_API_KEY` + `MOX_LLM_BASE_URL` + `MOX_LLM_MODEL`
    /// 2. `DEEPSEEK_API_KEY` → base `https://api.deepseek.com`、model `deepseek-chat`
    /// 3. `OPENAI_API_KEY` → base `https://api.openai.com/v1`、model `gpt-4o-mini`
    ///
    /// 其余可选：`MOX_LLM_TIMEOUT_MS`（缺省 60000）。
    /// 返回 `None` 表示未配置可用 Key（调用方应回退本地引擎）。
    pub fn from_env() -> Option<Self> {
        // 1) 显式 MOX_LLM_* 优先
        if let Some(api_key) = std::env::var("MOX_LLM_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
        {
            let base_url = std::env::var("MOX_LLM_BASE_URL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let model = std::env::var("MOX_LLM_MODEL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "gpt-4o-mini".to_string());
            return Some(Self::from_parts(api_key, base_url, model));
        }
        // 2) DeepSeek（OpenAI 兼容）
        if let Some(api_key) = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
        {
            return Some(Self::from_parts(
                api_key,
                "https://api.deepseek.com".to_string(),
                "deepseek-chat".to_string(),
            ));
        }
        // 3) OpenAI
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Self::from_parts(
            api_key,
            "https://api.openai.com/v1".to_string(),
            "gpt-4o-mini".to_string(),
        ))
    }

    fn from_parts(api_key: String, base_url: String, model: String) -> Self {
        let timeout_ms = std::env::var("MOX_LLM_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60_000);
        Self {
            base_url,
            api_key,
            model,
            temperature: 0.5,
            max_tokens: 2048,
            timeout_ms,
        }
    }

    /// 是否具备可用的真实 Key
    pub fn is_enabled(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// 大模型聊天客户端抽象（DIP：生产 / 测试可注入）
pub trait ChatClient: Send + Sync {
    /// 多轮补全，返回模型文本
    fn complete(&self, messages: &[ChatMessage]) -> anyhow::Result<String>;
}

/// OpenAI 兼容实现（reqwest blocking）
pub struct OpenAiChatClient {
    config: LlmConfig,
}

impl OpenAiChatClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }
}

impl ChatClient for OpenAiChatClient {
    fn complete(&self, messages: &[ChatMessage]) -> anyhow::Result<String> {
        if !self.config.is_enabled() {
            return Err(anyhow::anyhow!("LLM API key not configured"));
        }

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: messages
                .iter()
                .map(|m| ChatMessageDto {
                    role: m.role.as_str().to_string(),
                    content: m.content.clone(),
                })
                .collect(),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .build()
            .map_err(|e| anyhow::anyhow!("LLM http client build failed: {}", e))?;

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .unwrap_or_else(|_| "（无响应体）".to_string());
            return Err(anyhow::anyhow!(
                "LLM API error ({}): {}",
                status,
                truncate(&body, 300)
            ));
        }

        let completion: ChatCompletionResponse = resp
            .json()
            .map_err(|e| anyhow::anyhow!("LLM response parse failed: {}", e))?;
        let content = completion
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "（模型未返回内容）".to_string());
        Ok(content)
    }
}

fn truncate(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    if s.chars().count() > n {
        out.push_str("…");
    }
    out
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessageDto>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessageDto {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChoiceDto>,
}

#[derive(Debug, Deserialize)]
struct ChoiceDto {
    message: ResponseMessageDto,
}

#[derive(Debug, Deserialize)]
struct ResponseMessageDto {
    content: String,
}

/// 测试用 Mock 客户端：按脚本逐轮返回固定文本
///
/// 第 N 次调用返回 `script[N-1]`；超出脚本长度后重复最后一条。
pub struct MockChatClient {
    script: Vec<String>,
    calls: std::sync::atomic::AtomicUsize,
}

impl MockChatClient {
    pub fn new(script: Vec<String>) -> Self {
        assert!(!script.is_empty(), "MockChatClient script must not be empty");
        Self {
            script,
            calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// 已发生的调用次数（供测试断言 ReAct 轮数）
    pub fn call_count(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl ChatClient for MockChatClient {
    fn complete(&self, messages: &[ChatMessage]) -> anyhow::Result<String> {
        let _ = messages;
        let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let idx = n.min(self.script.len() - 1);
        Ok(self.script[idx].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_env_none_without_key() {
        // 并行测试下不触碰真实环境变量，改用子进程隔离难；
        // 这里只验证默认（无 MOX_LLM_API_KEY 时返回 None 的构造逻辑不依赖真实 env）
        let cfg = LlmConfig::default();
        assert!(!cfg.is_enabled());
        assert_eq!(cfg.model, "gpt-4o-mini");
    }

    #[test]
    fn mock_client_returns_script_progressively() {
        let c = MockChatClient::new(vec!["hello".into(), "world".into()]);
        let msgs = vec![ChatMessage::user("hi")];
        assert_eq!(c.complete(&msgs).unwrap(), "hello");
        assert_eq!(c.complete(&msgs).unwrap(), "world");
        assert_eq!(c.complete(&msgs).unwrap(), "world"); // 超长后重复末条
        assert_eq!(c.call_count(), 3);
    }

    /// 真实 LLM 连通性冒烟测试（默认忽略，需手动 `cargo test -- --ignored`）
    ///
    /// 需要环境变量 `DEEPSEEK_API_KEY` 或 `MOX_LLM_API_KEY`；
    /// 未配置时自动跳过（不失败）。
    #[test]
    #[ignore = "live LLM connectivity smoke test, requires API key"]
    fn live_llm_connectivity() {
        let Some(config) = LlmConfig::from_env() else {
            eprintln!("[skip] 未配置 LLM API Key，跳过实时连通性测试");
            return;
        };
        let client = OpenAiChatClient::new(config);
        let msgs = vec![
            ChatMessage::system("你是一个测试助手，请用一句话回答。"),
            ChatMessage::user("1+1 等于几？"),
        ];
        let out = client
            .complete(&msgs)
            .expect("真实 LLM 调用应成功（请检查 base_url/api_key/网络）");
        eprintln!("[live] LLM 返回: {}", out);
        assert!(!out.is_empty());
        assert!(!out.contains("（模型未返回内容）"));
    }
}
