// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! LLM 路由选择器
//!
//! 负责根据模块配置选择最优的大模型 Provider，
//! 支持多种路由策略和降级回退链。
//!
//! ## 核心能力
//! - **优先级路由**：按配置的优先级顺序选择可用的 Provider
//! - **轮询路由**：在可用 Provider 之间轮询，实现负载均衡
//! - **延迟优先路由**：选择延迟最低的 Provider
//! - **成本优先路由**：选择成本最低的 Provider
//! - **降级回退**：主 Provider 不可用时自动切换到降级链
//! - **健康检查**：自动检测 Provider 可用性
//! - **熔断保护**：连续失败后暂时熔断该 Provider

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mox_alliance_common_proto::{
    ApiKeySource, LlmProviderOption, LlmRoutingStrategy, MergedLlmConfig,
};
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// Provider 健康状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderHealth {
    /// 健康可用
    Healthy,
    /// 降级中（有失败但未熔断）
    Degraded,
    /// 已熔断（暂时不可用）
    CircuitBroken,
}

/// Provider 运行时状态
#[derive(Debug, Clone)]
pub struct ProviderRuntimeState {
    /// Provider ID
    pub provider_id: String,
    /// 健康状态
    pub health: ProviderHealth,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 连续成功次数
    pub consecutive_successes: u32,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 最近一次调用时间
    pub last_call_at: Option<Instant>,
    /// 熔断结束时间（如果被熔断）
    pub circuit_break_until: Option<Instant>,
    /// 总调用次数
    pub total_calls: u64,
    /// 总失败次数
    pub total_failures: u64,
}

impl ProviderRuntimeState {
    fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            health: ProviderHealth::Healthy,
            consecutive_failures: 0,
            consecutive_successes: 0,
            avg_latency_ms: 0.0,
            last_call_at: None,
            circuit_break_until: None,
            total_calls: 0,
            total_failures: 0,
        }
    }

    /// 是否可用
    fn is_available(&self) -> bool {
        if self.health == ProviderHealth::CircuitBroken {
            if let Some(until) = self.circuit_break_until {
                if Instant::now() < until {
                    return false;
                }
                // 熔断时间已过，恢复为降级状态
                // （实际恢复逻辑在记录成功时处理）
            }
        }
        true
    }
}

/// LLM 路由选择结果
#[derive(Debug, Clone)]
pub struct RouterSelection {
    /// 选中的 Provider ID
    pub provider_id: String,
    /// 选中的模型名
    pub model: String,
    /// API Key（如果能解析到）
    pub api_key: Option<String>,
    /// Base URL
    pub base_url: Option<String>,
    /// 选择的路由策略
    pub strategy: LlmRoutingStrategy,
    /// 是否为降级选择（非首选）
    pub is_fallback: bool,
}

/// LLM 路由选择器
///
/// 负责根据模块配置和运行时状态选择最优的 LLM Provider。
pub struct LlmRouter {
    /// Provider 运行时状态（provider_id -> state）
    provider_states: Arc<RwLock<HashMap<String, ProviderRuntimeState>>>,
    /// 熔断阈值（连续失败次数）
    pub circuit_break_threshold: u32,
    /// 熔断持续时间
    pub circuit_break_duration: Duration,
    /// 半开状态探测次数（熔断后需要多少次成功才恢复健康）
    pub half_open_probes: u32,
    /// API Key 解析器（DIP：隔离环境变量依赖，便于测试注入与密钥管理器接入）
    api_key_resolver: Arc<dyn ApiKeyResolver>,
}

/// API Key 解析器抽象
///
/// 通过依赖注入将「API Key 从哪解析」与路由逻辑解耦：
/// - 生产默认 `EnvApiKeyResolver`：读环境变量/明文
/// - 测试注入 `FakeApiKeyResolver`：固定映射，不触碰全局环境变量（消除并行测试竞争）
/// - 未来可无缝接入密钥管理服务（Vault/KMS），无须改动路由逻辑
pub trait ApiKeyResolver: Send + Sync {
    /// 解析指定来源的 API Key；无法解析返回 `None`
    fn resolve(&self, source: &ApiKeySource) -> Option<String>;
}

/// 默认实现：委托 `ApiKeySource::resolve_api_key`（环境变量/明文）
#[derive(Default, Clone, Copy)]
pub struct EnvApiKeyResolver;

impl ApiKeyResolver for EnvApiKeyResolver {
    fn resolve(&self, source: &ApiKeySource) -> Option<String> {
        source.resolve_api_key()
    }
}

/// 测试实现：按环境变量名查固定映射，绝不触碰真实全局环境变量
#[derive(Default, Clone)]
pub struct FakeApiKeyResolver {
    env_overrides: HashMap<String, String>,
}

impl FakeApiKeyResolver {
    /// 从键值对构造（环境变量名 -> API Key）
    pub fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            env_overrides: entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl ApiKeyResolver for FakeApiKeyResolver {
    fn resolve(&self, source: &ApiKeySource) -> Option<String> {
        match source {
            ApiKeySource::EnvVar { env_name } => self.env_overrides.get(env_name).cloned(),
            ApiKeySource::PlainText { api_key } => Some(api_key.clone()),
            ApiKeySource::SecretRef { .. } | ApiKeySource::Inherit => None,
        }
    }
}

impl LlmRouter {
    /// 创建新的 LLM 路由选择器
    pub fn new() -> Self {
        Self {
            provider_states: Arc::new(RwLock::new(HashMap::new())),
            circuit_break_threshold: 5,
            circuit_break_duration: Duration::from_secs(60),
            half_open_probes: 3,
            api_key_resolver: Arc::new(EnvApiKeyResolver),
        }
    }

    /// 创建带自定义参数的路由选择器
    pub fn with_config(
        circuit_break_threshold: u32,
        circuit_break_duration: Duration,
        half_open_probes: u32,
    ) -> Self {
        Self {
            provider_states: Arc::new(RwLock::new(HashMap::new())),
            circuit_break_threshold,
            circuit_break_duration,
            half_open_probes,
            api_key_resolver: Arc::new(EnvApiKeyResolver),
        }
    }

    /// 注入自定义 API Key 解析器（生产接密钥管理 / 测试隔离环境变量）
    pub fn with_api_key_resolver(mut self, resolver: Arc<dyn ApiKeyResolver>) -> Self {
        self.api_key_resolver = resolver;
        self
    }

    /// 选择最佳的 LLM Provider
    ///
    /// 根据模块的合并配置和路由策略，选择一个可用的 Provider。
    /// 如果首选 Provider 不可用，会按降级链依次尝试。
    pub fn select_provider(&self, config: &MergedLlmConfig) -> Option<RouterSelection> {
        let route_order = config.provider_route_order();
        debug!(
            module_id = %config.module_id,
            strategy = ?config.routing_strategy,
            route_order = ?route_order,
            "Selecting LLM provider"
        );

        let result = match config.routing_strategy {
            LlmRoutingStrategy::Priority => {
                self.select_by_priority(config, &route_order)
            }
            LlmRoutingStrategy::RoundRobin => {
                self.select_by_round_robin(config, &route_order)
            }
            LlmRoutingStrategy::LatencyPriority => {
                self.select_by_latency(config, &route_order)
            }
            LlmRoutingStrategy::CostPriority => {
                self.select_by_cost(config, &route_order)
            }
        };

        if let Some(selection) = &result {
            debug!(
                provider_id = %selection.provider_id,
                model = %selection.model,
                is_fallback = selection.is_fallback,
                "Selected LLM provider"
            );
        } else {
            warn!(
                module_id = %config.module_id,
                "No available LLM provider found"
            );
        }

        result
    }

    /// 按优先级选择（顺序遍历，选第一个可用的）
    fn select_by_priority(
        &self,
        config: &MergedLlmConfig,
        route_order: &[String],
    ) -> Option<RouterSelection> {
        for (index, provider_id) in route_order.iter().enumerate() {
            if let Some(provider) = config.get_provider(provider_id) {
                if self.is_provider_available(provider) {
                    return Some(self.build_selection(provider, config, index > 0));
                }
            }
        }
        None
    }

    /// 按轮询选择
    fn select_by_round_robin(
        &self,
        config: &MergedLlmConfig,
        route_order: &[String],
    ) -> Option<RouterSelection> {
        let available_providers: Vec<&LlmProviderOption> = route_order
            .iter()
            .filter_map(|pid| config.get_provider(pid))
            .filter(|p| self.is_provider_available(p))
            .collect();

        if available_providers.is_empty() {
            return None;
        }

        // 简单实现：选择总调用次数最少的
        let states = self.provider_states.read();
        let selected = available_providers
            .iter()
            .min_by_key(|p| {
                states
                    .get(&p.provider_id)
                    .map(|s| s.total_calls)
                    .unwrap_or(0)
            })
            .copied()?;

        let is_fallback = selected.provider_id != config.primary_provider;
        Some(self.build_selection(selected, config, is_fallback))
    }

    /// 按延迟优先选择
    fn select_by_latency(
        &self,
        config: &MergedLlmConfig,
        route_order: &[String],
    ) -> Option<RouterSelection> {
        let states = self.provider_states.read();
        let mut available: Vec<(&LlmProviderOption, f64)> = route_order
            .iter()
            .filter_map(|pid| {
                let provider = config.get_provider(pid)?;
                if !self.is_provider_available(provider) {
                    return None;
                }
                let latency = states
                    .get(&provider.provider_id)
                    .map(|s| s.avg_latency_ms)
                    .unwrap_or(f64::MAX);
                Some((provider, latency))
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        available.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let (selected, _) = available[0];
        let is_fallback = selected.provider_id != config.primary_provider;
        Some(self.build_selection(selected, config, is_fallback))
    }

    /// 按成本优先选择
    fn select_by_cost(
        &self,
        config: &MergedLlmConfig,
        route_order: &[String],
    ) -> Option<RouterSelection> {
        let mut available: Vec<&LlmProviderOption> = route_order
            .iter()
            .filter_map(|pid| {
                let provider = config.get_provider(pid)?;
                if !self.is_provider_available(provider) {
                    return None;
                }
                Some(provider)
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        // 按价格排序（有价格的排前面，价格低的排前面）
        available.sort_by(|a, b| {
            let price_a = a.price_per_1k_tokens.unwrap_or(f64::MAX);
            let price_b = b.price_per_1k_tokens.unwrap_or(f64::MAX);
            price_a.partial_cmp(&price_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let selected = available[0];
        let is_fallback = selected.provider_id != config.primary_provider;
        Some(self.build_selection(selected, config, is_fallback))
    }

    /// 构建选择结果
    fn build_selection(
        &self,
        provider: &LlmProviderOption,
        config: &MergedLlmConfig,
        is_fallback: bool,
    ) -> RouterSelection {
        let model = provider
            .default_model
            .clone()
            .unwrap_or_else(|| {
                if provider.provider_id == config.primary_provider {
                    config.primary_model.clone()
                } else {
                    provider.provider_id.clone()
                }
            });

        RouterSelection {
            provider_id: provider.provider_id.clone(),
            model,
            api_key: self.api_key_resolver.resolve(&provider.api_key_source),
            base_url: provider.base_url.clone(),
            strategy: config.routing_strategy,
            is_fallback,
        }
    }

    /// 检查 Provider 是否可用
    fn is_provider_available(&self, provider: &LlmProviderOption) -> bool {
        if !provider.enabled {
            return false;
        }
        // 密钥必须实际可解析：
        // - EnvVar：环境变量必须已设置（修复"未设置也被视为可用"的缺陷）
        // - PlainText：恒可用
        // - Inherit / SecretRef（当前路由无本地密钥管理器）：视为不可用
        if self.api_key_resolver.resolve(&provider.api_key_source).is_none() {
            return false;
        }
        // 检查运行时状态
        let states = self.provider_states.read();
        if let Some(state) = states.get(&provider.provider_id) {
            if !state.is_available() {
                return false;
            }
        }
        true
    }

    /// 记录 Provider 调用成功
    pub fn record_success(&self, provider_id: &str, latency_ms: f64) {
        let mut states = self.provider_states.write();
        let state = states
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderRuntimeState::new(provider_id.to_string()));

        state.total_calls += 1;
        state.consecutive_failures = 0;
        state.consecutive_successes += 1;
        state.last_call_at = Some(Instant::now());

        // 更新平均延迟（指数移动平均）
        if state.avg_latency_ms == 0.0 {
            state.avg_latency_ms = latency_ms;
        } else {
            state.avg_latency_ms = state.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }

        // 检查是否需要从熔断/降级恢复
        match state.health {
            ProviderHealth::CircuitBroken => {
                // 熔断后第一次成功，进入降级状态
                if state.consecutive_successes >= self.half_open_probes {
                    state.health = ProviderHealth::Healthy;
                    state.circuit_break_until = None;
                    info!(provider_id, "Provider recovered from circuit break");
                } else {
                    state.health = ProviderHealth::Degraded;
                }
            }
            ProviderHealth::Degraded => {
                if state.consecutive_successes >= self.half_open_probes {
                    state.health = ProviderHealth::Healthy;
                    debug!(provider_id, "Provider recovered to healthy");
                }
            }
            ProviderHealth::Healthy => {}
        }
    }

    /// 记录 Provider 调用失败
    pub fn record_failure(&self, provider_id: &str, latency_ms: f64) {
        let mut states = self.provider_states.write();
        let state = states
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderRuntimeState::new(provider_id.to_string()));

        state.total_calls += 1;
        state.total_failures += 1;
        state.consecutive_failures += 1;
        state.consecutive_successes = 0;
        state.last_call_at = Some(Instant::now());

        // 更新平均延迟
        if state.avg_latency_ms == 0.0 {
            state.avg_latency_ms = latency_ms;
        } else {
            state.avg_latency_ms = state.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }

        // 检查是否需要熔断
        if state.consecutive_failures >= self.circuit_break_threshold
            && state.health != ProviderHealth::CircuitBroken
        {
            state.health = ProviderHealth::CircuitBroken;
            state.circuit_break_until = Some(Instant::now() + self.circuit_break_duration);
            warn!(
                provider_id,
                consecutive_failures = state.consecutive_failures,
                "Provider circuit broken"
            );
        } else if state.health == ProviderHealth::Healthy && state.consecutive_failures > 1 {
            state.health = ProviderHealth::Degraded;
        }
    }

    /// 获取 Provider 运行时状态
    pub fn get_provider_state(&self, provider_id: &str) -> Option<ProviderRuntimeState> {
        self.provider_states.read().get(provider_id).cloned()
    }

    /// 获取所有 Provider 状态
    pub fn get_all_states(&self) -> Vec<ProviderRuntimeState> {
        self.provider_states.read().values().cloned().collect()
    }

    /// 手动重置 Provider 状态
    pub fn reset_provider(&self, provider_id: &str) {
        let mut states = self.provider_states.write();
        if let Some(state) = states.get_mut(provider_id) {
            state.health = ProviderHealth::Healthy;
            state.consecutive_failures = 0;
            state.consecutive_successes = 0;
            state.circuit_break_until = None;
            info!(provider_id, "Provider state reset manually");
        }
    }

    /// 检查 API Key 是否已配置（能通过注入的解析器解析到）
    pub fn has_usable_api_key(&self, source: &ApiKeySource) -> bool {
        self.api_key_resolver.resolve(source).is_some()
    }
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{ModelConfig, ModuleLlmConfig};

    fn make_test_config() -> MergedLlmConfig {
        let module_config = ModuleLlmConfig {
            module_id: "test-module".to_string(),
            primary_provider: "openai".to_string(),
            primary_model: "gpt-4".to_string(),
            fallback_chain: vec!["anthropic".to_string(), "deepseek".to_string()],
            routing_strategy: LlmRoutingStrategy::Priority,
            model_config: ModelConfig::default(),
            provider_options: vec![
                LlmProviderOption {
                    provider_id: "openai".to_string(),
                    display_name: Some("OpenAI".to_string()),
                    api_key_source: ApiKeySource::from_env("OPENAI_API_KEY"),
                    base_url: None,
                    default_model: Some("gpt-4".to_string()),
                    supported_models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
                    price_per_1k_tokens: Some(0.03),
                    rpm_limit: Some(500),
                    tpm_limit: Some(80000),
                    enabled: true,
                },
                LlmProviderOption {
                    provider_id: "anthropic".to_string(),
                    display_name: Some("Anthropic".to_string()),
                    api_key_source: ApiKeySource::from_env("ANTHROPIC_API_KEY"),
                    base_url: None,
                    default_model: Some("claude-3-opus".to_string()),
                    supported_models: vec!["claude-3-opus".to_string()],
                    price_per_1k_tokens: Some(0.015),
                    rpm_limit: Some(1000),
                    tpm_limit: Some(100000),
                    enabled: true,
                },
                LlmProviderOption {
                    provider_id: "deepseek".to_string(),
                    display_name: Some("DeepSeek".to_string()),
                    api_key_source: ApiKeySource::from_env("DEEPSEEK_API_KEY"),
                    base_url: Some("https://api.deepseek.com".to_string()),
                    default_model: Some("deepseek-chat".to_string()),
                    supported_models: vec!["deepseek-chat".to_string()],
                    price_per_1k_tokens: Some(0.001),
                    rpm_limit: Some(2000),
                    tpm_limit: Some(200000),
                    enabled: true,
                },
            ],
            system_prompt_template: Some("You are a helpful assistant.".to_string()),
            use_global_prompt_prefix: true,
            version: 1,
            updated_at: chrono::Utc::now(),
        };

        let global = mox_alliance_common_proto::GlobalLlmConfig::default();
        module_config.merge_with_global(&global)
    }

    #[test]
    fn test_router_selection_priority() {
        // 注入固定 Key 解析器，避免触碰全局环境变量（消除并行测试竞争）
        let router = LlmRouter::new().with_api_key_resolver(Arc::new(FakeApiKeyResolver::with(&[
            ("OPENAI_API_KEY", "test-key-openai"),
            ("ANTHROPIC_API_KEY", "test-key-anthropic"),
        ])));
        let config = make_test_config();

        let selection = router.select_provider(&config).unwrap();
        assert_eq!(selection.provider_id, "openai");
        assert_eq!(selection.model, "gpt-4");
        assert!(!selection.is_fallback);
        assert!(selection.api_key.is_some());
    }

    #[test]
    fn test_router_fallback_when_primary_unavailable() {
        let router = LlmRouter::with_config(2, Duration::from_secs(60), 3)
            .with_api_key_resolver(Arc::new(FakeApiKeyResolver::with(&[
                ("OPENAI_API_KEY", "test-key-openai"),
                ("ANTHROPIC_API_KEY", "test-key-anthropic"),
            ])));
        let config = make_test_config();

        // 模拟 openai 连续失败达到熔断阈值
        router.record_failure("openai", 100.0);
        router.record_failure("openai", 100.0);

        // 此时 openai 应该被熔断，选择 anthropic
        let selection = router.select_provider(&config).unwrap();
        assert_eq!(selection.provider_id, "anthropic");
        assert!(selection.is_fallback);
    }

    #[test]
    fn test_router_cost_priority() {
        let router = LlmRouter::new().with_api_key_resolver(Arc::new(FakeApiKeyResolver::with(&[
            ("OPENAI_API_KEY", "test-key-openai"),
            ("ANTHROPIC_API_KEY", "test-key-anthropic"),
            ("DEEPSEEK_API_KEY", "test-key-deepseek"),
        ])));
        let mut config = make_test_config();
        config.routing_strategy = LlmRoutingStrategy::CostPriority;

        // deepseek 最便宜，应该被选中
        let selection = router.select_provider(&config).unwrap();
        assert_eq!(selection.provider_id, "deepseek");
    }

    #[test]
    fn test_provider_recovery() {
        let router = LlmRouter::with_config(2, Duration::from_secs(60), 2);

        // 熔断
        router.record_failure("test-provider", 100.0);
        router.record_failure("test-provider", 100.0);

        let state = router.get_provider_state("test-provider").unwrap();
        assert_eq!(state.health, ProviderHealth::CircuitBroken);

        // 手动重置
        router.reset_provider("test-provider");
        let state = router.get_provider_state("test-provider").unwrap();
        assert_eq!(state.health, ProviderHealth::Healthy);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_no_available_provider() {
        // 空映射：所有 EnvVar 来源均无法解析 → 无可用 Provider（不依赖真实环境变量）
        let router = LlmRouter::new().with_api_key_resolver(Arc::new(FakeApiKeyResolver::default()));
        let config = make_test_config();

        let selection = router.select_provider(&config);
        assert!(selection.is_none());
    }
}
