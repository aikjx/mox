// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 轻量 LLM Provider 路由器与熔断器
//!
//! 独立于 scheduler-core 的 `LlmRouter`，专为 mox-ai-expert-svc 设计：
//! - 4 种路由策略：Priority / RoundRobin / LatencyFirst / CostFirst
//! - 三级健康状态：Healthy / Degraded / CircuitBroken
//! - 熔断冷却后自动探测恢复
//! - 所有状态通过 `RwLock<HashMap>` 内部可变性，`&self` 即可调用

use super::chat::{ProviderConfig, RoutingStrategy};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Provider 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderHealth {
    /// 正常
    Healthy,
    /// 降级（连续失败 >1 但未达熔断阈值）
    Degraded,
    /// 熔断（连续失败达阈值，冷却期内不可用）
    CircuitBroken,
}

/// Provider 运行时状态
#[derive(Debug, Clone)]
pub struct ProviderRuntimeState {
    pub provider_id: String,
    pub health: ProviderHealth,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub avg_latency_ms: f64,
    pub last_call_at: Option<Instant>,
    pub circuit_break_until: Option<Instant>,
    pub total_calls: u64,
    pub total_failures: u64,
}

impl ProviderRuntimeState {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
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

    /// 是否当前可用（考虑熔断冷却期过后的探测机会）
    fn is_available(&self, now: Instant) -> bool {
        match self.health {
            ProviderHealth::Healthy | ProviderHealth::Degraded => true,
            ProviderHealth::CircuitBroken => {
                // 冷却期过后允许一次探测
                matches!(self.circuit_break_until, Some(t) if now >= t)
            }
        }
    }
}

/// 轻量 LLM 路由器（含熔断器）
pub struct LlmRouter {
    provider_states: Arc<RwLock<HashMap<String, ProviderRuntimeState>>>,
    providers: Vec<ProviderConfig>,
    routing_strategy: RoutingStrategy,
    circuit_break_threshold: u32,
    circuit_break_cooldown: Duration,
    round_robin_counter: AtomicUsize,
}

impl LlmRouter {
    /// 创建路由器，自动为每个 enabled provider 初始化运行时状态
    pub fn new(
        providers: Vec<ProviderConfig>,
        strategy: RoutingStrategy,
        threshold: u32,
        cooldown_ms: u64,
    ) -> Self {
        let mut states = HashMap::new();
        for p in &providers {
            if p.enabled && !p.api_key.is_empty() {
                states.insert(p.provider_id.clone(), ProviderRuntimeState::new(&p.provider_id));
            }
        }
        Self {
            provider_states: Arc::new(RwLock::new(states)),
            providers,
            routing_strategy: strategy,
            circuit_break_threshold: threshold.max(1),
            circuit_break_cooldown: Duration::from_millis(cooldown_ms),
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    /// 按路由策略选择一个可用 Provider
    pub fn select_provider(&self) -> Option<&ProviderConfig> {
        let now = Instant::now();
        let states = self.provider_states.read().ok()?;

        // 收集可用 provider 索引（按 providers 顺序）
        let available: Vec<usize> = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, p)| p.enabled && !p.api_key.is_empty())
            .filter(|(_, p)| {
                states
                    .get(&p.provider_id)
                    .map(|s| s.is_available(now))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();

        if available.is_empty() {
            return None;
        }

        let idx = match self.routing_strategy {
            RoutingStrategy::Priority => available[0],
            RoutingStrategy::RoundRobin => {
                let n = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
                available[n % available.len()]
            }
            RoutingStrategy::LatencyFirst => {
                // 选 avg_latency_ms 最低的；无记录的排最前（优先探测）
                available
                    .iter()
                    .copied()
                    .min_by(|&a, &b| {
                        let la = states
                            .get(&self.providers[a].provider_id)
                            .map(|s| s.avg_latency_ms)
                            .unwrap_or(f64::INFINITY);
                        let lb = states
                            .get(&self.providers[b].provider_id)
                            .map(|s| s.avg_latency_ms)
                            .unwrap_or(f64::INFINITY);
                        la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(available[0])
            }
            RoutingStrategy::CostFirst => {
                // 选 price_per_1k_tokens 最低的；无价格的排最后
                available
                    .iter()
                    .copied()
                    .min_by(|&a, &b| {
                        let pa = self.providers[a].price_per_1k_tokens.unwrap_or(f64::INFINITY);
                        let pb = self.providers[b].price_per_1k_tokens.unwrap_or(f64::INFINITY);
                        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(available[0])
            }
        };

        Some(&self.providers[idx])
    }

    /// 记录一次成功调用
    pub fn record_success(&self, provider_id: &str, latency_ms: f64) {
        let now = Instant::now();
        let mut states = match self.provider_states.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        let Some(state) = states.get_mut(provider_id) else {
            return;
        };
        state.total_calls += 1;
        state.consecutive_failures = 0;
        state.consecutive_successes += 1;
        state.last_call_at = Some(now);
        // 指数移动平均延迟（α=0.3）
        if state.avg_latency_ms <= 0.0 {
            state.avg_latency_ms = latency_ms;
        } else {
            state.avg_latency_ms = state.avg_latency_ms * 0.7 + latency_ms * 0.3;
        }
        // 状态恢复：连续成功 >= 3 → Healthy
        if state.consecutive_successes >= 3 {
            state.health = ProviderHealth::Healthy;
            state.circuit_break_until = None;
        }
    }

    /// 记录一次失败调用
    pub fn record_failure(&self, provider_id: &str, latency_ms: f64) {
        let now = Instant::now();
        let mut states = match self.provider_states.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        let Some(state) = states.get_mut(provider_id) else {
            return;
        };
        state.total_calls += 1;
        state.total_failures += 1;
        state.consecutive_successes = 0;
        state.consecutive_failures += 1;
        state.last_call_at = Some(now);
        // 更新延迟（失败也记录，便于 LatencyFirst 规避慢节点）
        if state.avg_latency_ms <= 0.0 {
            state.avg_latency_ms = latency_ms;
        } else {
            state.avg_latency_ms = state.avg_latency_ms * 0.7 + latency_ms * 0.3;
        }
        // 状态机转换（顺序级联：Healthy → Degraded → CircuitBroken 可在同一次调用内完成）
        // 1) Healthy → Degraded：连续失败 > 1
        if state.health == ProviderHealth::Healthy && state.consecutive_failures > 1 {
            state.health = ProviderHealth::Degraded;
        }
        // 2) Degraded → CircuitBroken：连续失败 >= threshold（含刚从 Healthy 降级的情况）
        if state.health == ProviderHealth::Degraded
            && state.consecutive_failures >= self.circuit_break_threshold
        {
            state.health = ProviderHealth::CircuitBroken;
            state.circuit_break_until = Some(now + self.circuit_break_cooldown);
        }
        // 3) CircuitBroken：探测调用也失败 → 重置冷却期
        if state.health == ProviderHealth::CircuitBroken {
            state.circuit_break_until = Some(now + self.circuit_break_cooldown);
        }
    }

    /// 检查是否所有 provider 都处于熔断状态（且冷却期未过）
    pub fn all_circuit_broken(&self) -> bool {
        let now = Instant::now();
        let states = match self.provider_states.read() {
            Ok(s) => s,
            Err(_) => return true,
        };
        if states.is_empty() {
            return true;
        }
        states.values().all(|s| !s.is_available(now))
    }

    /// 获取指定 provider 的运行时状态快照
    pub fn get_state(&self, provider_id: &str) -> Option<ProviderRuntimeState> {
        let states = self.provider_states.read().ok()?;
        states.get(provider_id).cloned()
    }

    /// 重置指定 provider 的运行时状态（恢复 Healthy）
    pub fn reset_provider(&self, provider_id: &str) {
        let mut states = match self.provider_states.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        if let Some(state) = states.get_mut(provider_id) {
            *state = ProviderRuntimeState::new(provider_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                provider_id: "primary".into(),
                base_url: "https://api.primary.com/v1".into(),
                api_key: "key-primary".into(),
                model: "primary-model".into(),
                enabled: true,
                price_per_1k_tokens: Some(0.0015),
            },
            ProviderConfig {
                provider_id: "secondary".into(),
                base_url: "https://api.secondary.com/v1".into(),
                api_key: "key-secondary".into(),
                model: "secondary-model".into(),
                enabled: true,
                price_per_1k_tokens: Some(0.0020),
            },
            ProviderConfig {
                provider_id: "tertiary".into(),
                base_url: "https://api.tertiary.com/v1".into(),
                api_key: "key-tertiary".into(),
                model: "tertiary-model".into(),
                enabled: true,
                price_per_1k_tokens: Some(0.0010),
            },
        ]
    }

    #[test]
    fn test_router_priority_selection() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 5, 30000);
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "primary");
    }

    #[test]
    fn test_router_round_robin() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::RoundRobin, 5, 30000);
        let p1 = router.select_provider().unwrap();
        let p2 = router.select_provider().unwrap();
        let p3 = router.select_provider().unwrap();
        let p4 = router.select_provider().unwrap();
        assert_eq!(p1.provider_id, "primary");
        assert_eq!(p2.provider_id, "secondary");
        assert_eq!(p3.provider_id, "tertiary");
        assert_eq!(p4.provider_id, "primary"); // 循环
    }

    #[test]
    fn test_router_latency_first() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::LatencyFirst, 5, 30000);
        // 记录不同延迟
        router.record_success("primary", 100.0);
        router.record_success("secondary", 50.0);
        router.record_success("tertiary", 200.0);
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "secondary");
    }

    #[test]
    fn test_router_circuit_break() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 3, 30000);
        // 连续失败 3 次达到阈值
        router.record_failure("primary", 10.0);
        router.record_failure("primary", 10.0);
        router.record_failure("primary", 10.0);
        let state = router.get_state("primary").unwrap();
        assert_eq!(state.health, ProviderHealth::CircuitBroken);
        // select_provider 应跳过熔断的 primary，选 secondary
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "secondary");
    }

    #[test]
    fn test_router_circuit_break_recovery() {
        // 用极短冷却期便于测试
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 2, 50);
        // 熔断 primary
        router.record_failure("primary", 10.0);
        router.record_failure("primary", 10.0);
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::CircuitBroken
        );
        // 等待冷却期
        std::thread::sleep(Duration::from_millis(60));
        // 冷却后应可被选中（探测）
        let p = router.select_provider();
        assert!(p.is_some());
        assert_eq!(p.unwrap().provider_id, "primary");
        // 探测成功 3 次后恢复 Healthy
        router.record_success("primary", 20.0);
        router.record_success("primary", 20.0);
        router.record_success("primary", 20.0);
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::Healthy
        );
    }

    #[test]
    fn test_router_fallback_to_secondary() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 2, 30000);
        // 熔断 primary
        router.record_failure("primary", 10.0);
        router.record_failure("primary", 10.0);
        // 应自动选 secondary
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "secondary");
        // secondary 也熔断
        router.record_failure("secondary", 10.0);
        router.record_failure("secondary", 10.0);
        // 应选 tertiary
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "tertiary");
    }

    #[test]
    fn test_router_all_broken() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 2, 30000);
        // 熔断所有 provider
        for id in ["primary", "secondary", "tertiary"] {
            router.record_failure(id, 10.0);
            router.record_failure(id, 10.0);
        }
        assert!(router.all_circuit_broken());
        assert!(router.select_provider().is_none());
    }

    #[test]
    fn test_router_cost_first() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::CostFirst, 5, 30000);
        // tertiary 价格最低 (0.0010)
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "tertiary");
    }

    #[test]
    fn test_router_degraded_state() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 5, 30000);
        // 1 次失败仍 Healthy
        router.record_failure("primary", 10.0);
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::Healthy
        );
        // 2 次失败 → Degraded
        router.record_failure("primary", 10.0);
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::Degraded
        );
        // Degraded 仍可被选中
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "primary");
    }

    #[test]
    fn test_router_disabled_provider_skipped() {
        let mut providers = make_providers();
        providers[0].enabled = false;
        let router = LlmRouter::new(providers, RoutingStrategy::Priority, 5, 30000);
        let p = router.select_provider().unwrap();
        assert_eq!(p.provider_id, "secondary");
    }

    #[test]
    fn test_router_reset_provider() {
        let router = LlmRouter::new(make_providers(), RoutingStrategy::Priority, 2, 30000);
        router.record_failure("primary", 10.0);
        router.record_failure("primary", 10.0);
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::CircuitBroken
        );
        router.reset_provider("primary");
        assert_eq!(
            router.get_state("primary").unwrap().health,
            ProviderHealth::Healthy
        );
    }
}
