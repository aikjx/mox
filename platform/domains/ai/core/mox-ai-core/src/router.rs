// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AI 模型路由器 — 多模型路由、负载均衡、自动降级

use crate::providers::dto::*;
use crate::providers::error::{AiError, AiResult};
use crate::providers::traits::AiProvider;
use crate::registry::ProviderRegistry;
use futures::stream::BoxStream;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// 路由策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// 按优先级顺序（第一个可用）
    Priority,
    /// 轮询
    RoundRobin,
    /// 最低延迟（基于历史性能）
    LatencyPriority,
    /// 最低成本
    CostPriority,
}

impl Default for RoutingStrategy {
    fn default() -> Self { RoutingStrategy::Priority }
}

/// 路由条目（一个Provider + 权重 + 优先级）
#[derive(Clone)]
pub struct RouteEntry {
    pub provider_id: String,
    pub priority: u8,       // 0最高
    pub weight: u32,        // 负载均衡权重
    pub enabled: bool,
}

/// Provider性能统计（用于延迟路由）
#[derive(Debug, Clone, Default)]
struct ProviderStats {
    success_count: u64,
    failure_count: u64,
    total_latency_ms: u64,
    last_failure: Option<std::time::Instant>,
    circuit_open_until: Option<std::time::Instant>,
}

impl ProviderStats {
    fn avg_latency_ms(&self) -> f64 {
        if self.success_count == 0 { return f64::MAX; }
        self.total_latency_ms as f64 / self.success_count as f64
    }

    fn failure_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 { return 0.0; }
        self.failure_count as f64 / total as f64
    }

    fn is_circuit_open(&self) -> bool {
        self.circuit_open_until
            .map(|until| until > std::time::Instant::now())
            .unwrap_or(false)
    }
}

/// AI 模型路由器
///
/// 功能：
/// - 按策略选择Provider
/// - 自动降级（主Provider失败→备用链）
/// - 熔断（连续失败后暂时断开）
/// - 负载均衡（权重轮询）
pub struct ModelRouter {
    registry: Arc<ProviderRegistry>,
    strategy: RoutingStrategy,
    /// 降级链：按优先级排列的provider_id列表
    fallback_chain: Vec<String>,
    /// 路由表：model_name -> RouteEntry列表
    routes: RwLock<std::collections::HashMap<String, Vec<RouteEntry>>>,
    /// 性能统计
    stats: RwLock<std::collections::HashMap<String, ProviderStats>>,
    /// 熔断阈值：连续失败次数
    circuit_breaker_threshold: u64,
    /// 熔断恢复时间
    circuit_breaker_timeout: Duration,
    /// 最大降级次数
    max_fallback_attempts: usize,
}

impl ModelRouter {
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        Self {
            registry,
            strategy: RoutingStrategy::default(),
            fallback_chain: vec![],
            routes: RwLock::new(std::collections::HashMap::new()),
            stats: RwLock::new(std::collections::HashMap::new()),
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(30),
            max_fallback_attempts: 3,
        }
    }

    pub fn with_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_fallback_chain(mut self, chain: Vec<String>) -> Self {
        self.fallback_chain = chain;
        self
    }

    pub fn with_circuit_breaker(mut self, threshold: u64, timeout: Duration) -> Self {
        self.circuit_breaker_threshold = threshold;
        self.circuit_breaker_timeout = timeout;
        self
    }

    /// 注册模型路由
    pub async fn register_route(&self, model: &str, entries: Vec<RouteEntry>) {
        self.routes.write().await.insert(model.to_string(), entries);
    }

    /// 选择Provider（按策略+熔断状态）
    async fn select_provider(&self, model: &str) -> AiResult<Arc<dyn AiProvider>> {
        let routes = self.routes.read().await;
        let entries = routes.get(model)
            .ok_or_else(|| AiError::ModelNotFound(model.into()))?;

        let stats = self.stats.read().await;

        // 过滤：启用 + 未熔断
        let available: Vec<&RouteEntry> = entries.iter()
            .filter(|e| e.enabled)
            .filter(|e| !stats.get(&e.provider_id).map(|s| s.is_circuit_open()).unwrap_or(false))
            .collect();

        if available.is_empty() {
            return Err(AiError::AllProvidersFailed);
        }

        // 按策略选择
        let selected = match self.strategy {
            RoutingStrategy::Priority => {
                available.iter().min_by_key(|e| e.priority).copied()
            }
            RoutingStrategy::RoundRobin => {
                // 简单轮询：按总请求数取模
                let total: u64 = stats.values().map(|s| s.success_count + s.failure_count).sum();
                let idx = (total % available.len() as u64) as usize;
                available.get(idx).copied()
            }
            RoutingStrategy::LatencyPriority => {
                available.iter()
                    .min_by(|a, b| {
                        let la = stats.get(&a.provider_id).map(|s| s.avg_latency_ms()).unwrap_or(f64::MAX);
                        let lb = stats.get(&b.provider_id).map(|s| s.avg_latency_ms()).unwrap_or(f64::MAX);
                        la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
            RoutingStrategy::CostPriority => {
                // 简化：按provider_id字典序（实际应按成本配置）
                available.first().copied()
            }
        };

        let entry = selected.ok_or(AiError::AllProvidersFailed)?;
        self.registry.get(&entry.provider_id)
    }

    /// 记录调用结果（更新统计+熔断）
    async fn record_result(&self, provider_id: &str, success: bool, latency_ms: u64) {
        let mut stats = self.stats.write().await;
        let s = stats.entry(provider_id.to_string()).or_default();
        if success {
            s.success_count += 1;
            s.total_latency_ms += latency_ms;
        } else {
            s.failure_count += 1;
            s.last_failure = Some(std::time::Instant::now());
            // 熔断检查
            if s.failure_count >= self.circuit_breaker_threshold {
                s.circuit_open_until = Some(std::time::Instant::now() + self.circuit_breaker_timeout);
                tracing::warn!("circuit breaker opened for provider: {}", provider_id);
            }
        }
    }

    /// 对话（带自动降级）
    pub async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse> {
        let model = &req.config.model;
        let mut last_error: Option<AiError> = None;

        // 尝试1：按路由策略选择
        match self.select_provider(model).await {
            Ok(provider) => {
                let start = std::time::Instant::now();
                match provider.chat(req).await {
                    Ok(resp) => {
                        self.record_result(provider.provider_id(), true, start.elapsed().as_millis() as u64).await;
                        return Ok(resp);
                    }
                    Err(e) => {
                        self.record_result(provider.provider_id(), false, start.elapsed().as_millis() as u64).await;
                        tracing::warn!("provider {} failed: {}, attempting fallback", provider.provider_id(), e);
                        last_error = Some(e);
                    }
                }
            }
            Err(e) => { last_error = Some(e); }
        }

        // 尝试2-N：降级链
        for (i, provider_id) in self.fallback_chain.iter().enumerate() {
            if i >= self.max_fallback_attempts { break; }
            if let Ok(provider) = self.registry.get(provider_id) {
                if !provider.supports(Capability::Chat) { continue; }
                let start = std::time::Instant::now();
                // 降级时用provider的默认模型
                let mut fallback_req = req.clone();
                if let Some(default_model) = provider.available_models().first() {
                    fallback_req.config.model = default_model.clone();
                }
                match provider.chat(&fallback_req).await {
                    Ok(mut resp) => {
                        resp.provider = format!("{} (fallback)", provider.provider_id());
                        self.record_result(provider.provider_id(), true, start.elapsed().as_millis() as u64).await;
                        tracing::info!("fallback to provider {} succeeded", provider.provider_id());
                        return Ok(resp);
                    }
                    Err(e) => {
                        self.record_result(provider.provider_id(), false, start.elapsed().as_millis() as u64).await;
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or(AiError::AllProvidersFailed))
    }

    /// 流式对话
    pub async fn chat_stream(&self, req: &ChatRequest) -> AiResult<BoxStream<'_, AiResult<StreamChunk>>> {
        let provider = self.select_provider(&req.config.model).await?;
        if !provider.supports(Capability::ChatStream) {
            return Err(AiError::UnsupportedCapability("chat_stream".into()));
        }
        provider.chat_stream(req).await
    }

    /// 健康检查（所有Provider）
    pub async fn health_check_all(&self) -> std::collections::HashMap<String, HealthStatus> {
        let mut results = std::collections::HashMap::new();
        for provider in self.registry.list() {
            let status = provider.health_check().await;
            results.insert(provider.provider_id().to_string(), status);
        }
        results
    }
}
