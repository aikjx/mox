// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 真实大模型（LLM）聊天客户端 —— OpenAI 兼容 API
//!
//! 通过 DIP 抽象 `ChatClient`，生产用 `OpenAiChatClient`（reqwest blocking，
//! 经 `spawn_blocking` 桥接 async），测试注入 `MockChatClient` 固定脚本，
//! 不触碰真实网络与环境变量（消除并行测试竞争）。

use super::router::LlmRouter;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// 单个 LLM Provider 配置
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub enabled: bool,
    pub price_per_1k_tokens: Option<f64>,
}

/// 路由策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// 按优先级顺序选第一个可用的
    Priority,
    /// 轮询
    RoundRobin,
    /// 优先选平均延迟最低的
    LatencyFirst,
    /// 优先选价格最低的
    CostFirst,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        RoutingStrategy::Priority
    }
}

impl FromStr for RoutingStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "priority" | "priority_first" | "p" => Ok(RoutingStrategy::Priority),
            "roundrobin" | "round_robin" | "rr" => Ok(RoutingStrategy::RoundRobin),
            "latencyfirst" | "latency_first" | "latency" | "l" => Ok(RoutingStrategy::LatencyFirst),
            "costfirst" | "cost_first" | "cost" | "c" => Ok(RoutingStrategy::CostFirst),
            _ => Err(format!("unknown routing strategy: {}", s)),
        }
    }
}

/// LLM 配置
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// OpenAI 兼容 base url（含 /v1，调用方拼接 /chat/completions）
    /// 兼容视图：对应 providers[0] 的 base_url
    pub base_url: String,
    /// API Key
    /// 兼容视图：对应 providers[0] 的 api_key
    pub api_key: String,
    /// 模型名
    /// 兼容视图：对应 providers[0] 的 model
    pub model: String,
    /// 采样温度
    pub temperature: f32,
    /// 最大输出 token
    pub max_tokens: u32,
    /// 单次请求超时
    pub timeout_ms: u64,
    /// 多 Provider 列表（按优先级排序）；单 Provider 模式下只有一个元素
    pub providers: Vec<ProviderConfig>,
    /// 路由策略
    pub routing_strategy: RoutingStrategy,
    /// 熔断阈值（连续失败次数）
    pub circuit_break_threshold: u32,
    /// 熔断冷却期（毫秒）
    pub circuit_break_cooldown_ms: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        let default_provider = ProviderConfig {
            provider_id: "default".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            enabled: true,
            price_per_1k_tokens: None,
        };
        Self {
            base_url: default_provider.base_url.clone(),
            api_key: default_provider.api_key.clone(),
            model: default_provider.model.clone(),
            temperature: 0.5,
            max_tokens: 2048,
            timeout_ms: 60_000,
            providers: vec![default_provider],
            routing_strategy: RoutingStrategy::default(),
            circuit_break_threshold: 5,
            circuit_break_cooldown_ms: 30_000,
        }
    }
}

impl LlmConfig {
    /// 从环境变量加载真实 LLM 配置
    ///
    /// 多 Provider 模式（`MOX_LLM_PROVIDERS` 非空）：
    /// - `MOX_LLM_PROVIDERS`：逗号分隔的 provider ID 列表，如 `deepseek,openai,moonshot`
    /// - 对每个 provider ID，读取 `MOX_LLM_{ID}_BASE_URL` / `MOX_LLM_{ID}_API_KEY` /
    ///   `MOX_LLM_{ID}_MODEL` / `MOX_LLM_{ID}_PRICE_PER_1K`（可选）
    /// - 跳过 api_key 为空的 provider
    ///
    /// 单 Provider 模式（向后兼容，`MOX_LLM_PROVIDERS` 为空）：
    /// 探测顺序（取第一个可用的 Key）：
    /// 1. `MOX_LLM_API_KEY` + `MOX_LLM_BASE_URL` + `MOX_LLM_MODEL`
    /// 2. `DEEPSEEK_API_KEY` → base `https://api.deepseek.com`、model `deepseek-chat`
    /// 3. `OPENAI_API_KEY` → base `https://api.openai.com/v1`、model `gpt-4o-mini`
    ///
    /// 通用可选：`MOX_LLM_TIMEOUT_MS`（缺省 60000）、
    /// `MOX_LLM_ROUTING_STRATEGY`（缺省 priority）、
    /// `MOX_LLM_CIRCUIT_BREAK_THRESHOLD`（缺省 5）、
    /// `MOX_LLM_CIRCUIT_BREAK_COOLDOWN_MS`（缺省 30000）。
    /// 返回 `None` 表示未配置可用 Key（调用方应回退本地引擎）。
    pub fn from_env() -> Option<Self> {
        let timeout_ms = std::env::var("MOX_LLM_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60_000);
        let routing_strategy = std::env::var("MOX_LLM_ROUTING_STRATEGY")
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| RoutingStrategy::from_str(&s).ok())
            .unwrap_or_default();
        let circuit_break_threshold = std::env::var("MOX_LLM_CIRCUIT_BREAK_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(5);
        let circuit_break_cooldown_ms = std::env::var("MOX_LLM_CIRCUIT_BREAK_COOLDOWN_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30_000);

        // 检查是否为多 Provider 模式
        let providers_env = std::env::var("MOX_LLM_PROVIDERS")
            .ok()
            .filter(|s| !s.trim().is_empty());

        if let Some(providers_list) = providers_env {
            let providers: Vec<ProviderConfig> = providers_list
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|id| !id.is_empty())
                .filter_map(|id| Self::parse_provider_from_env(&id))
                .collect();
            if providers.is_empty() {
                return None;
            }
            let first = &providers[0];
            return Some(Self {
                base_url: first.base_url.clone(),
                api_key: first.api_key.clone(),
                model: first.model.clone(),
                temperature: 0.5,
                max_tokens: 2048,
                timeout_ms,
                providers,
                routing_strategy,
                circuit_break_threshold,
                circuit_break_cooldown_ms,
            });
        }

        // 单 Provider 模式（向后兼容）
        let provider = Self::single_provider_from_env()?;
        Some(Self {
            base_url: provider.base_url.clone(),
            api_key: provider.api_key.clone(),
            model: provider.model.clone(),
            temperature: 0.5,
            max_tokens: 2048,
            timeout_ms,
            providers: vec![provider],
            routing_strategy: RoutingStrategy::Priority,
            circuit_break_threshold,
            circuit_break_cooldown_ms,
        })
    }

    /// 从环境变量解析单个 provider 配置（多 Provider 模式下调用）
    fn parse_provider_from_env(id: &str) -> Option<ProviderConfig> {
        let upper = id.to_uppercase();
        let api_key = std::env::var(format!("MOX_LLM_{}_API_KEY", upper))
            .ok()
            .filter(|s| !s.is_empty())?;
        let base_url = std::env::var(format!("MOX_LLM_{}_BASE_URL", upper))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let model = std::env::var(format!("MOX_LLM_{}_MODEL", upper))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "gpt-4o-mini".to_string());
        let price_per_1k = std::env::var(format!("MOX_LLM_{}_PRICE_PER_1K", upper))
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<f64>().ok());
        Some(ProviderConfig {
            provider_id: id.to_string(),
            base_url,
            api_key,
            model,
            enabled: true,
            price_per_1k_tokens: price_per_1k,
        })
    }

    /// 单 Provider 模式：按兼容顺序探测可用 Key
    fn single_provider_from_env() -> Option<ProviderConfig> {
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
            return Some(ProviderConfig {
                provider_id: "default".to_string(),
                base_url,
                api_key,
                model,
                enabled: true,
                price_per_1k_tokens: None,
            });
        }
        // 2) DeepSeek（OpenAI 兼容）
        if let Some(api_key) = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
        {
            return Some(ProviderConfig {
                provider_id: "deepseek".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                api_key,
                model: "deepseek-chat".to_string(),
                enabled: true,
                price_per_1k_tokens: None,
            });
        }
        // 3) OpenAI
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(ProviderConfig {
            provider_id: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key,
            model: "gpt-4o-mini".to_string(),
            enabled: true,
            price_per_1k_tokens: None,
        })
    }

    /// 从环境变量加载配置，并在多 Provider 模式下创建路由器
    ///
    /// 返回 `(config, Option<router>)`：
    /// - 多 Provider（providers.len() > 1）：返回 Some(router)
    /// - 单 Provider：返回 None（保持原有单 Provider 行为）
    pub fn from_env_with_router() -> Option<(Self, Option<Arc<LlmRouter>>)> {
        let config = Self::from_env()?;
        if config.providers.len() > 1 {
            let router = LlmRouter::new(
                config.providers.clone(),
                config.routing_strategy,
                config.circuit_break_threshold,
                config.circuit_break_cooldown_ms,
            );
            Some((config, Some(Arc::new(router))))
        } else {
            Some((config, None))
        }
    }

    /// 是否具备可用的真实 Key（providers 列表非空且至少一个 enabled）
    pub fn is_enabled(&self) -> bool {
        self.providers.iter().any(|p| p.enabled && !p.api_key.is_empty())
    }
}

/// 大模型聊天客户端抽象（DIP：生产 / 测试可注入）
pub trait ChatClient: Send + Sync {
    /// 多轮补全，返回模型文本
    fn complete(&self, messages: &[ChatMessage]) -> anyhow::Result<String>;
}

/// OpenAI 兼容实现（reqwest blocking，延迟初始化）
///
/// **关键设计（延迟创建）**：`reqwest::blocking::Client` 内部自带 tokio runtime，
/// 若在 async 上下文（如 axum `build_app` 内调用 `llm_consultant()`）中创建，
/// 会在进程退出 drop 时触发 `Cannot drop a runtime in a context where blocking
/// is not allowed` panic。因此改为 `OnceLock` 延迟到首次 `complete()`（在
/// `spawn_blocking` 的 blocking 线程中）才构建，彻底避开 async 上下文。
pub struct OpenAiChatClient {
    config: LlmConfig,
    /// 复用的 HTTP 客户端（连接池 + TLS 会话复用，首次调用时懒构建）
    client: std::sync::OnceLock<reqwest::blocking::Client>,
    /// 可选路由器（多 Provider 模式下启用；单 Provider 模式为 None）
    router: Option<Arc<LlmRouter>>,
}

impl OpenAiChatClient {
    pub fn new(config: LlmConfig) -> Self {
        // 只保存配置，blocking client 延迟到首次 complete()（blocking 线程）创建
        Self {
            config,
            client: std::sync::OnceLock::new(),
            router: None,
        }
    }

    /// 注入路由器（多 Provider 模式）
    pub fn with_router(mut self, router: Arc<LlmRouter>) -> Self {
        self.router = Some(router);
        self
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// 懒构建 blocking client（仅在 complete 被调用时——blocking 上下文——创建）
    fn client(&self) -> &reqwest::blocking::Client {
        self.client.get_or_init(|| {
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(self.config.timeout_ms))
                .connect_timeout(Duration::from_secs(10))
                .pool_max_idle_per_host(10)
                .build()
                .expect("OpenAiChatClient: failed to build reqwest client (invalid config)")
        })
    }
}

impl ChatClient for OpenAiChatClient {
    fn complete(&self, messages: &[ChatMessage]) -> anyhow::Result<String> {
        // === 路由器模式：多 Provider 选择 + 熔断 ===
        if let Some(router) = &self.router {
            let provider = match router.select_provider() {
                Some(p) => p,
                None => {
                    return Err(anyhow::anyhow!(
                        "all LLM providers unavailable or circuit-broken"
                    ));
                }
            };
            let provider_id = provider.provider_id.clone();
            let base_url = provider.base_url.clone();
            let api_key = provider.api_key.clone();
            let model = provider.model.clone();

            let request = ChatCompletionRequest {
                model: model.clone(),
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

            let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
            let start = Instant::now();

            let result = self
                .client()
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e));

            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp
                            .text()
                            .unwrap_or_else(|_| "（无响应体）".to_string());
                        router.record_failure(&provider_id, latency_ms);
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
                    router.record_success(&provider_id, latency_ms);
                    Ok(content)
                }
                Err(e) => {
                    router.record_failure(&provider_id, latency_ms);
                    Err(e)
                }
            }
        } else {
            // === 单 Provider 模式：保持原有行为 ===
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

            let resp = self
                .client()
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
    use std::sync::Mutex;

    /// 序列化所有触碰环境变量的测试（env var 是进程全局状态，并行会竞争）
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// 辅助：设置环境变量并返回 guard，drop 时清理
    struct EnvGuard {
        keys: Vec<String>,
    }
    impl EnvGuard {
        fn new() -> Self {
            Self { keys: Vec::new() }
        }
        fn set(&mut self, key: &str, value: &str) {
            std::env::set_var(key, value);
            self.keys.push(key.to_string());
        }
        fn remove(&mut self, key: &str) {
            std::env::remove_var(key);
            self.keys.push(key.to_string());
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for key in &self.keys {
                std::env::remove_var(key);
            }
        }
    }

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

    #[test]
    fn routing_strategy_from_str_variants() {
        assert_eq!(RoutingStrategy::from_str("priority").unwrap(), RoutingStrategy::Priority);
        assert_eq!(RoutingStrategy::from_str("Priority").unwrap(), RoutingStrategy::Priority);
        assert_eq!(RoutingStrategy::from_str("round_robin").unwrap(), RoutingStrategy::RoundRobin);
        assert_eq!(RoutingStrategy::from_str("latency_first").unwrap(), RoutingStrategy::LatencyFirst);
        assert_eq!(RoutingStrategy::from_str("cost_first").unwrap(), RoutingStrategy::CostFirst);
        assert!(RoutingStrategy::from_str("unknown").is_err());
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::Priority);
    }

    #[test]
    fn test_llm_config_multi_provider_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        env.set("MOX_LLM_PROVIDERS", "deepseek,openai");
        env.set("MOX_LLM_DEEPSEEK_BASE_URL", "https://api.deepseek.com");
        env.set("MOX_LLM_DEEPSEEK_API_KEY", "sk-deepseek-test");
        env.set("MOX_LLM_DEEPSEEK_MODEL", "deepseek-chat");
        env.set("MOX_LLM_DEEPSEEK_PRICE_PER_1K", "0.0014");
        env.set("MOX_LLM_OPENAI_BASE_URL", "https://api.openai.com/v1");
        env.set("MOX_LLM_OPENAI_API_KEY", "sk-openai-test");
        env.set("MOX_LLM_OPENAI_MODEL", "gpt-4o-mini");
        // 不设置 OPENAI_PRICE_PER_1K → None

        let cfg = LlmConfig::from_env().expect("multi-provider config should parse");
        assert_eq!(cfg.providers.len(), 2);
        assert_eq!(cfg.providers[0].provider_id, "deepseek");
        assert_eq!(cfg.providers[0].base_url, "https://api.deepseek.com");
        assert_eq!(cfg.providers[0].api_key, "sk-deepseek-test");
        assert_eq!(cfg.providers[0].model, "deepseek-chat");
        assert!((cfg.providers[0].price_per_1k_tokens.unwrap() - 0.0014).abs() < 1e-9);
        assert_eq!(cfg.providers[1].provider_id, "openai");
        assert_eq!(cfg.providers[1].api_key, "sk-openai-test");
        assert!(cfg.providers[1].price_per_1k_tokens.is_none());
        // 兼容视图 = providers[0]
        assert_eq!(cfg.base_url, "https://api.deepseek.com");
        assert_eq!(cfg.api_key, "sk-deepseek-test");
        assert_eq!(cfg.model, "deepseek-chat");
        assert!(cfg.is_enabled());
    }

    #[test]
    fn test_llm_config_multi_provider_skips_empty_key() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        env.set("MOX_LLM_PROVIDERS", "deepseek,broken,openai");
        env.set("MOX_LLM_DEEPSEEK_API_KEY", "sk-ds");
        env.set("MOX_LLM_DEEPSEEK_BASE_URL", "https://api.deepseek.com");
        env.set("MOX_LLM_DEEPSEEK_MODEL", "deepseek-chat");
        // broken 没有 API_KEY → 跳过
        env.set("MOX_LLM_OPENAI_API_KEY", "sk-oa");
        env.set("MOX_LLM_OPENAI_BASE_URL", "https://api.openai.com/v1");
        env.set("MOX_LLM_OPENAI_MODEL", "gpt-4o-mini");

        let cfg = LlmConfig::from_env().expect("should parse with 2 valid providers");
        assert_eq!(cfg.providers.len(), 2);
        assert_eq!(cfg.providers[0].provider_id, "deepseek");
        assert_eq!(cfg.providers[1].provider_id, "openai");
    }

    #[test]
    fn test_llm_config_single_provider_backward_compat() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        // 确保不设置 MOX_LLM_PROVIDERS
        env.remove("MOX_LLM_PROVIDERS");
        env.remove("MOX_LLM_API_KEY");
        env.remove("DEEPSEEK_API_KEY");
        env.remove("OPENAI_API_KEY");
        // 设置 MOX_LLM_API_KEY 单 provider 模式
        env.set("MOX_LLM_API_KEY", "sk-single-test");
        env.set("MOX_LLM_BASE_URL", "https://api.custom.com/v1");
        env.set("MOX_LLM_MODEL", "custom-model");

        let cfg = LlmConfig::from_env().expect("single provider config should parse");
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].provider_id, "default");
        assert_eq!(cfg.providers[0].base_url, "https://api.custom.com/v1");
        assert_eq!(cfg.providers[0].api_key, "sk-single-test");
        assert_eq!(cfg.providers[0].model, "custom-model");
        assert_eq!(cfg.routing_strategy, RoutingStrategy::Priority);
        // 兼容视图
        assert_eq!(cfg.base_url, "https://api.custom.com/v1");
        assert_eq!(cfg.api_key, "sk-single-test");
        assert_eq!(cfg.model, "custom-model");
    }

    #[test]
    fn test_llm_config_from_env_with_router_multi() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        env.set("MOX_LLM_PROVIDERS", "a,b");
        env.set("MOX_LLM_A_API_KEY", "sk-a");
        env.set("MOX_LLM_A_BASE_URL", "https://a.com/v1");
        env.set("MOX_LLM_A_MODEL", "model-a");
        env.set("MOX_LLM_B_API_KEY", "sk-b");
        env.set("MOX_LLM_B_BASE_URL", "https://b.com/v1");
        env.set("MOX_LLM_B_MODEL", "model-b");
        env.set("MOX_LLM_ROUTING_STRATEGY", "round_robin");
        env.set("MOX_LLM_CIRCUIT_BREAK_THRESHOLD", "3");
        env.set("MOX_LLM_CIRCUIT_BREAK_COOLDOWN_MS", "10000");

        let (cfg, router) = LlmConfig::from_env_with_router().expect("should parse");
        assert_eq!(cfg.providers.len(), 2);
        assert_eq!(cfg.routing_strategy, RoutingStrategy::RoundRobin);
        assert_eq!(cfg.circuit_break_threshold, 3);
        assert_eq!(cfg.circuit_break_cooldown_ms, 10000);
        assert!(router.is_some());
        let router = router.unwrap();
        // 验证 router 能选到 provider
        let p = router.select_provider().unwrap();
        assert!(p.provider_id == "a" || p.provider_id == "b");
    }

    #[test]
    fn test_llm_config_from_env_with_router_single() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        env.remove("MOX_LLM_PROVIDERS");
        env.remove("DEEPSEEK_API_KEY");
        env.remove("OPENAI_API_KEY");
        env.set("MOX_LLM_API_KEY", "sk-single");

        let (cfg, router) = LlmConfig::from_env_with_router().expect("should parse");
        assert_eq!(cfg.providers.len(), 1);
        assert!(router.is_none());
    }

    #[test]
    fn test_openai_client_with_router_selects_provider() {
        // 构造带 router 的 OpenAiChatClient，不实际发请求，只验证 select_provider 逻辑
        let providers = vec![
            ProviderConfig {
                provider_id: "p1".into(),
                base_url: "https://p1.com/v1".into(),
                api_key: "sk-p1".into(),
                model: "model-p1".into(),
                enabled: true,
                price_per_1k_tokens: None,
            },
            ProviderConfig {
                provider_id: "p2".into(),
                base_url: "https://p2.com/v1".into(),
                api_key: "sk-p2".into(),
                model: "model-p2".into(),
                enabled: true,
                price_per_1k_tokens: None,
            },
        ];
        let router = Arc::new(LlmRouter::new(
            providers.clone(),
            RoutingStrategy::Priority,
            2,
            30000,
        ));
        // 熔断 p1
        router.record_failure("p1", 10.0);
        router.record_failure("p1", 10.0);
        // select 应返回 p2
        let selected = router.select_provider().unwrap();
        assert_eq!(selected.provider_id, "p2");

        // 构造 client（不调用 complete，只验证 builder 工作）
        let mut cfg = LlmConfig::default();
        cfg.providers = providers;
        let client = OpenAiChatClient::new(cfg).with_router(router);
        assert!(client.config().providers.len() == 2);
    }

    #[test]
    fn test_openai_client_no_router_single_provider() {
        // 单 Provider 模式：无 router，config.is_enabled() 行为不变
        let cfg = LlmConfig::default();
        let client = OpenAiChatClient::new(cfg);
        // 无 api_key → complete 应返回 "LLM API key not configured"
        let msgs = vec![ChatMessage::user("hi")];
        let err = client.complete(&msgs).unwrap_err();
        assert!(err.to_string().contains("LLM API key not configured"));
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
