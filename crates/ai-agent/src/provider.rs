//! # AI Gateway —— 组件化、可插拔的 LLM Provider 抽象层
//!
//! 设计目标（贴合企业级诉求）：
//! - **协议无关**：通过 `LlmProvider` trait 屏蔽各家 API 差异；OpenAI 兼容端点（OpenAI /
//!   DeepSeek / 通义千问 Qwen / 智谱 GLM / Kimi / Ollama / vLLM / Azure OpenAI）只需配置即可接入。
//! - **组件化 / 插件化**：每个 Provider 独立构造，经 `ProviderRegistry` 运行时注册，可第三方扩展。
//! - **韧性**：`LlmRouter` 支持 fallback 链（主供应商 429/超时自动降级下一档）与离线规则引擎兜底。
//! - **可观测**：每次调用产出 `LlmCallMeta`（provider/model/tokens/latency/tenant/trace），供审计与成本计量。
//!
//! 本模块不破坏既有 `LLMClient` / `LlmFn` / `compile_requirement_with_llm` / `execute_flow` 的契约。

use crate::llm_client::{LLMChatMessage, LLMClient, LLMConfig};
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Provider 能力标签（用于路由与前端展示）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    Chat,
    Vision,
    FunctionCall,
    Embedding,
    Streaming,
}

/// 标准化对话请求
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<LLMChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    /// 可选租户/用户标识，用于审计与配额
    pub tenant: Option<String>,
    pub user: Option<String>,
    pub trace_id: Option<String>,
}

/// 标准化对话响应
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    /// 用量（如后端返回）；用于成本计量
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// 一次调用的可观测元数据
#[derive(Debug, Clone)]
pub struct LlmCallMeta {
    pub provider: String,
    pub model: String,
    pub latency_ms: u64,
    pub success: bool,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub tenant: Option<String>,
    pub trace_id: Option<String>,
}

impl LlmCallMeta {
    pub fn log(&self) {
        // 预留审计/成本计量接入点（可改为写入 DB 或 tracing span）
        tracing::info!(
            target: "ai_gateway",
            provider = %self.provider,
            model = %self.model,
            latency_ms = self.latency_ms,
            success = self.success,
            prompt_tokens = self.prompt_tokens,
            completion_tokens = self.completion_tokens,
            tenant = ?self.tenant,
            trace_id = ?self.trace_id,
            "llm call"
        );
    }
}

/// 统一 Provider 抽象（组件化核心 trait）。
/// 采用手动 Box-future 风格，避免引入 async-trait 额外依赖。
pub trait LlmProvider: Send + Sync {
    /// 唯一名称，如 "deepseek" / "openai" / "qwen" / "ollama"
    fn name(&self) -> &str;
    /// 支持的模型列表（用于前端下拉 / 路由匹配）
    fn models(&self) -> Vec<String>;
    /// 能力标签
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat]
    }
    /// 是否就绪（如已配置 key）
    fn is_ready(&self) -> bool;
    /// 健康检查
    fn health(&self) -> BoxFut<'_, bool> {
        Box::pin(async { false })
    }
    /// 核心对话
    fn chat(&self, req: ChatRequest) -> BoxFut<'_, Result<ChatResponse>>;
}

/// OpenAI 兼容 Provider：把现有 `LLMClient` 包装为统一抽象。
/// 通过不同 `LLMConfig`（api_base / model / api_key）即可对接任意 OpenAI 兼容厂商。
pub struct OpenAiCompatibleProvider {
    name: String,
    models: Vec<String>,
    client: LLMClient,
}

impl OpenAiCompatibleProvider {
    pub fn new(name: impl Into<String>, config: LLMConfig) -> Self {
        let models = vec![config.model.clone()];
        Self {
            name: name.into(),
            models,
            client: LLMClient::new(config),
        }
    }
}

impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn models(&self) -> Vec<String> {
        self.models.clone()
    }
    fn is_ready(&self) -> bool {
        self.client.is_enabled()
    }
    fn health(&self) -> BoxFut<'_, bool> {
        let client = self.client.clone();
        Box::pin(async move { client.test_connection().await.is_ok() })
    }
    fn chat(&self, req: ChatRequest) -> BoxFut<'_, Result<ChatResponse>> {
        let client = self.client.clone();
        let provider_name = self.name.clone();
        Box::pin(async move {
            let started = Instant::now();
            let result = client.chat(req.messages.clone()).await;
            let latency_ms = started.elapsed().as_millis() as u64;
            match result {
                Ok(content) => {
                    let meta = LlmCallMeta {
                        provider: provider_name.clone(),
                        model: client.get_config().model.clone(),
                        latency_ms,
                        success: true,
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        tenant: req.tenant.clone(),
                        trace_id: req.trace_id.clone(),
                    };
                    meta.log();
                    Ok(ChatResponse {
                        content,
                        model: client.get_config().model.clone(),
                        provider: provider_name,
                        prompt_tokens: 0,
                        completion_tokens: 0,
                    })
                }
                Err(e) => {
                    let meta = LlmCallMeta {
                        provider: provider_name.clone(),
                        model: client.get_config().model.clone(),
                        latency_ms,
                        success: false,
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        tenant: req.tenant.clone(),
                        trace_id: req.trace_id.clone(),
                    };
                    meta.log();
                    Err(e)
                }
            }
        })
    }
}

/// Provider 注册表：运行时收集所有可用 provider（组件化 / 插件化核心）
#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个 provider（同名覆盖）
    pub fn register(&mut self, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.get(name).cloned()
    }

    /// 返回所有已就绪（is_ready）的 provider 名称
    pub fn ready_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .filter(|(_, p)| p.is_ready())
            .map(|(n, _)| n.clone())
            .collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

/// LLM 路由器：按 fallback 链顺序尝试 provider，实现韧性调用。
/// 顺序即优先级（第一个为主供应商）。
#[derive(Clone)]
pub struct LlmRouter {
    registry: Arc<std::sync::RwLock<ProviderRegistry>>,
    /// fallback 链：provider 名称顺序
    chain: Arc<std::sync::RwLock<Vec<String>>>,
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(std::sync::RwLock::new(ProviderRegistry::new())),
            chain: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// 挂载注册表（与全局单例共享）
    pub fn with_registry(registry: Arc<std::sync::RwLock<ProviderRegistry>>) -> Self {
        Self {
            registry,
            chain: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn registry(&self) -> Arc<std::sync::RwLock<ProviderRegistry>> {
        self.registry.clone()
    }

    /// 同步注册一个 provider
    pub fn register_provider(&self, provider: Arc<dyn LlmProvider>) {
        if let Ok(mut reg) = self.registry.write() {
            reg.register(provider);
        }
    }

    /// 设置 fallback 链（按顺序尝试）
    pub fn set_chain(&self, names: Vec<String>) {
        if let Ok(mut c) = self.chain.write() {
            *c = names;
        }
    }

    /// 从环境变量自动注册所有可用 Provider，并按优先级构建 fallback 链。
    /// 返回 `Arc<LlmRouter>`，可在同步构造函数中直接使用。
    pub fn init_from_env() -> Arc<LlmRouter> {
        let router = LlmRouter::new();
        for p in default_providers_from_env() {
            router.register_provider(p);
        }
        let available = router
            .registry
            .read()
            .map(|r| r.names())
            .unwrap_or_default();
        let preferred = ["deepseek", "openai", "qwen", "glm", "ollama"];
        let chain: Vec<String> = preferred
            .iter()
            .filter(|n| available.iter().any(|a| a == *n))
            .map(|s| s.to_string())
            .collect();
        router.set_chain(chain.clone());
        tracing::info!(
            target: "ai_gateway",
            providers = ?available,
            chain = ?chain,
            "AI Gateway 初始化完成（基于环境变量 DEEPSEEK_API_KEY 等自动接入真实 LLM）"
        );
        Arc::new(router)
    }

    pub fn chain(&self) -> Vec<String> {
        self.chain.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// 按 fallback 链执行：任一 provider 成功即返回；全部失败返回最后一个错误。
    pub async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let chain = self.chain();
        // 先在锁内取出 provider 的 Arc（Send+Sync、可跨 await 持有），随后立即释放读锁，
        // 避免 std::sync::RwLockReadGuard（!Send）被跨 await 持有导致 Future 非 Send。
        let candidates: Vec<Arc<dyn LlmProvider>> = {
            let registry = self.registry.read().ok();
            let registry = match registry {
                Some(r) => r,
                None => return Err(anyhow::anyhow!("router registry lock poisoned")),
            };
            chain
                .iter()
                .filter_map(|name| registry.get(name).map(|p| p.clone()))
                .collect()
        };

        let mut last_err: Option<anyhow::Error> = None;
        for provider in &candidates {
            if !provider.is_ready() {
                last_err = Some(anyhow::anyhow!("provider '{}' not ready", provider.name()));
                continue;
            }
            match provider.chat(req.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!(target: "ai_gateway", provider = %provider.name(), error = %e, "provider failed, try next");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no provider available in chain")))
    }

    /// 指定单个 provider 调用（优先于 fallback 链）。provider 未注册或非就绪时返回错误，
    /// 由调用方决定是否降级到 fallback 链（见 `chat`）。
    pub async fn chat_with_provider(&self, name: &str, req: ChatRequest) -> Result<ChatResponse> {
        let provider = {
            let registry = self.registry.read().ok();
            let registry = match registry {
                Some(r) => r,
                None => return Err(anyhow::anyhow!("router registry lock poisoned")),
            };
            registry.get(name).map(|p| p.clone())
        };
        match provider {
            Some(provider) => {
                if !provider.is_ready() {
                    return Err(anyhow::anyhow!("provider '{}' not ready", name));
                }
                provider.chat(req).await
            }
            None => Err(anyhow::anyhow!("provider '{}' not registered", name)),
        }
    }
}

/// 便捷构造：基于 OpenAI 兼容端点注册一个 provider（覆盖开源/开放接口主流厂商）
pub fn make_openai_compatible(
    name: &str,
    api_base: &str,
    api_key: &str,
    model: &str,
) -> Arc<dyn LlmProvider> {
    let config = LLMConfig {
        api_base: api_base.to_string(),
        api_key: api_key.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        max_tokens: 2048,
        enabled: !api_key.trim().is_empty(),
    };
    Arc::new(OpenAiCompatibleProvider::new(name, config))
}

/// 默认 provider 预设（基于环境变量，缺失则 disabled，不影响离线降级）
pub fn default_providers_from_env() -> Vec<Arc<dyn LlmProvider>> {
    let mut list: Vec<Arc<dyn LlmProvider>> = Vec::new();

    // DeepSeek（主线）：DEEPSEEK_API_KEY
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        let base = std::env::var("DEEPSEEK_BASE_URL")
            .ok()
            .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
        let model = std::env::var("DEEPSEEK_MODEL").ok().unwrap_or_else(|| "deepseek-chat".to_string());
        list.push(make_openai_compatible("deepseek", &base, &key, &model));
    }

    // OpenAI（fallback）：OPENAI_API_KEY
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        let model = std::env::var("OPENAI_MODEL").ok().unwrap_or_else(|| "gpt-4o-mini".to_string());
        list.push(make_openai_compatible("openai", "https://api.openai.com/v1", &key, &model));
    }

    // 通义千问 Qwen（fallback）：DASHSCOPE_API_KEY
    if let Ok(key) = std::env::var("DASHSCOPE_API_KEY") {
        list.push(make_openai_compatible(
            "qwen",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            &key,
            "qwen-plus",
        ));
    }

    // 智谱 GLM（fallback）：ZHIPU_API_KEY
    if let Ok(key) = std::env::var("ZHIPU_API_KEY") {
        list.push(make_openai_compatible(
            "glm",
            "https://open.bigmodel.cn/api/paas/v4",
            &key,
            "glm-4",
        ));
    }

    // 本地 Ollama / vLLM（私有化/离线）：OLLAMA_BASE_URL
    if let Ok(base) = std::env::var("OLLAMA_BASE_URL") {
        let key = std::env::var("OLLAMA_API_KEY").ok().unwrap_or_default();
        let model = std::env::var("OLLAMA_MODEL").ok().unwrap_or_else(|| "llama3".to_string());
        list.push(make_openai_compatible("ollama", &base, &key, &model));
    }

    list
}
