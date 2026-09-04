// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 模块化权重专家匹配器
//!
//! 扩展基于规则的匹配器，支持从模块化配置中读取每个专家的独立匹配权重，
//! 实现mox 模块化系统架构动态可配置的专家匹配能力。
//!
//! ## 设计原则
//! - 权重可配置：每个专家的匹配维度权重可以独立调整
//! - 动态生效：配置变更后立即反映到匹配结果
//! - 向后兼容：默认权重与规则匹配器一致

use async_trait::async_trait;
use mox_alliance_common_proto::{
    AllianceError, AllianceResult, Expert, ExpertStatus, MatchingWeights,
};
use mox_alliance_scheduler_proto::{
    ExpertMatchQuery, ExpertMatchResult, ExpertMatcher, MatchScoreBreakdown, MatchedExpert,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::matching::{description_overlap, tokenize};

/// 模块化权重专家匹配器
///
/// 使用模块化配置中的 `MatchingWeights` 来计算匹配分数，
/// 每个专家可以有独立的权重配置。
pub struct ModularWeightMatcher {
    /// 专家注册表
    experts: Arc<RwLock<HashMap<String, Expert>>>,
    /// 每个专家的匹配权重 (expert_id -> weights)
    /// 若专家没有自定义权重，则使用默认权重
    expert_weights: Arc<RwLock<HashMap<String, MatchingWeights>>>,
    /// 默认匹配权重
    default_weights: MatchingWeights,
}

impl ModularWeightMatcher {
    /// 使用默认权重创建匹配器
    pub fn new() -> Self {
        Self {
            experts: Arc::new(RwLock::new(HashMap::new())),
            expert_weights: Arc::new(RwLock::new(HashMap::new())),
            default_weights: MatchingWeights::default(),
        }
    }

    /// 使用共享专家存储创建
    pub fn with_shared_experts(experts: Arc<RwLock<HashMap<String, Expert>>>) -> Self {
        Self {
            experts,
            expert_weights: Arc::new(RwLock::new(HashMap::new())),
            default_weights: MatchingWeights::default(),
        }
    }

    /// 设置默认匹配权重
    pub fn with_default_weights(mut self, weights: MatchingWeights) -> Self {
        self.default_weights = weights;
        self
    }

    /// 获取专家存储的 Arc 引用
    pub fn experts_arc(&self) -> Arc<RwLock<HashMap<String, Expert>>> {
        self.experts.clone()
    }

    /// 获取权重存储的 Arc 引用
    pub fn weights_arc(&self) -> Arc<RwLock<HashMap<String, MatchingWeights>>> {
        self.expert_weights.clone()
    }

    /// 注册专家
    pub fn register_expert(&self, expert: Expert) {
        self.experts
            .write()
            .insert(expert.expert_id.clone(), expert);
    }

    /// 批量注册专家
    pub fn register_experts(&self, experts_list: Vec<Expert>) {
        let mut experts = self.experts.write();
        for expert in experts_list {
            experts.insert(expert.expert_id.clone(), expert);
        }
    }

    /// 设置专家的匹配权重
    pub fn set_expert_weights(&self, expert_id: &str, weights: MatchingWeights) {
        self.expert_weights
            .write()
            .insert(expert_id.to_string(), weights);
        debug!("Set matching weights for expert: {}", expert_id);
    }

    /// 获取专家的匹配权重（不存在则返回默认）
    pub fn get_expert_weights(&self, expert_id: &str) -> MatchingWeights {
        self.expert_weights
            .read()
            .get(expert_id)
            .cloned()
            .unwrap_or_else(|| self.default_weights.clone())
    }

    /// 批量设置专家权重
    pub fn set_expert_weights_batch(&self, weights: HashMap<String, MatchingWeights>) {
        let mut expert_weights = self.expert_weights.write();
        for (id, w) in weights {
            expert_weights.insert(id, w);
        }
    }

    /// 重置为默认权重
    pub fn reset_expert_weights(&self, expert_id: &str) {
        self.expert_weights.write().remove(expert_id);
    }

    // === 评分计算 ===

    /// 计算领域匹配分 (0.0 - 1.0)
    fn calculate_domain_score(expert: &Expert, query: &ExpertMatchQuery) -> f64 {
        if query.required_domains.is_empty() {
            return 1.0;
        }
        let expert_domains: std::collections::HashSet<&str> =
            expert.domains.iter().map(|d| d.as_str()).collect();
        let matched: usize = query
            .required_domains
            .iter()
            .filter(|d| expert_domains.contains(d.as_str()))
            .count();
        matched as f64 / query.required_domains.len() as f64
    }

    /// 计算能力匹配分 (0.0 - 1.0)
    fn calculate_capability_score(expert: &Expert, query: &ExpertMatchQuery) -> f64 {
        if query.required_capabilities.is_empty() {
            return 0.5;
        }
        let expert_caps: std::collections::HashSet<&str> = expert
            .capabilities
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        let matched: usize = query
            .required_capabilities
            .iter()
            .filter(|c| expert_caps.contains(c.as_str()))
            .count();
        matched as f64 / query.required_capabilities.len() as f64
    }

    /// 计算描述文本匹配分 (0.0 - 1.0)
    ///
    /// 基于中英分词（字符 bigram + 英文词）的 token 重叠，
    /// 修复原 `split_whitespace()` 对中文整句永不命中的缺陷。
    fn calculate_description_score(expert: &Expert, query: &ExpertMatchQuery) -> f64 {
        let query_tokens = tokenize(&query.task_description);
        description_overlap(expert, &query_tokens, None).0
    }

    /// 计算健康状态分 (0.0 - 1.0)
    fn calculate_health_score(expert: &Expert) -> f64 {
        if !expert.health.is_healthy {
            return 0.2;
        }
        // 成功率和延迟综合评估
        let health_score = expert.health.success_rate * 0.7
            + if expert.health.avg_latency_ms < 1000.0 {
                0.3
            } else if expert.health.avg_latency_ms < 3000.0 {
                0.2
            } else if expert.health.avg_latency_ms < 5000.0 {
                0.1
            } else {
                0.0
            };
        health_score.clamp(0.0, 1.0)
    }

    /// 计算优先级分 (0.0 - 1.0)
    fn calculate_priority_score(expert: &Expert) -> f64 {
        // priority 范围 1-10，归一化到 0.0-1.0
        (expert.priority as f64 - 1.0) / 9.0
    }

    /// 计算性能分 (0.0 - 1.0)
    fn calculate_performance_score(expert: &Expert) -> f64 {
        // 基于成功率和延迟的综合性能分
        let success_component = expert.health.success_rate;
        let latency_component = if expert.health.avg_latency_ms < 500.0 {
            1.0
        } else if expert.health.avg_latency_ms < 2000.0 {
            0.8
        } else if expert.health.avg_latency_ms < 5000.0 {
            0.5
        } else {
            0.2
        };
        (success_component * 0.6 + latency_component * 0.4).clamp(0.0, 1.0)
    }

    /// 使用指定权重计算综合评分
    fn calculate_weighted_score(
        breakdown: &MatchScoreBreakdown,
        weights: &MatchingWeights,
    ) -> f64 {
        breakdown.domain_match * weights.domain as f64
            + breakdown.capability_match * weights.capability as f64
            + breakdown.health_score * weights.health as f64
            + breakdown.priority_score * weights.rating as f64
            // 注意：这里将 priority_score 映射到 rating 权重
            // 因为现有 Expert 类型中没有独立的 rating 字段
            + breakdown.performance_score * weights.performance as f64
    }
}

impl Default for ModularWeightMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExpertMatcher for ModularWeightMatcher {
    async fn match_experts(&self, query: ExpertMatchQuery) -> AllianceResult<ExpertMatchResult> {
        let start = std::time::Instant::now();
        let experts = self.experts.read();
        let weights = self.expert_weights.read();

        let mut matched: Vec<MatchedExpert> = Vec::new();

        for expert in experts.values() {
            // 租户过滤
            if expert.tenant_id != query.tenant_id && expert.tenant_id != "system" {
                continue;
            }

            // 状态过滤
            if expert.status != ExpertStatus::Active {
                continue;
            }

            // 优先级过滤
            if expert.priority < query.min_priority {
                continue;
            }

            // 领域过滤（硬过滤：完全不匹配则跳过）
            if !query.required_domains.is_empty() {
                let expert_domains: std::collections::HashSet<&str> =
                    expert.domains.iter().map(|d| d.as_str()).collect();
                let has_any = query
                    .required_domains
                    .iter()
                    .any(|d| expert_domains.contains(d.as_str()));
                if !has_any {
                    continue;
                }
            }

            // 计算各维度分数
            let domain_score = Self::calculate_domain_score(expert, &query);
            let capability_score = Self::calculate_capability_score(expert, &query);
            let description_score = Self::calculate_description_score(expert, &query);
            let health_score = Self::calculate_health_score(expert);
            let priority_score = Self::calculate_priority_score(expert);
            let performance_score = Self::calculate_performance_score(expert);

            // 能力分包含描述分的加成
            let capability_combined = capability_score * 0.7 + description_score * 0.3;

            // 获取该专家的权重配置
            let expert_weights = weights
                .get(&expert.expert_id)
                .unwrap_or(&self.default_weights);

            let breakdown = MatchScoreBreakdown {
                domain_match: domain_score,
                capability_match: capability_combined,
                health_score,
                priority_score,
                performance_score,
            };

            let total_score = Self::calculate_weighted_score(&breakdown, expert_weights);

            // 分数下限过滤（过滤明显不相关的专家）
            if total_score < 0.2 {
                continue;
            }

            matched.push(MatchedExpert {
                expert: expert.clone(),
                score: total_score,
                match_reason: Self::generate_match_reason(&breakdown, expert_weights),
                score_breakdown: breakdown,
            });
        }

        // 按匹配分数降序排序
        matched.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 截断到 max_results
        let total_available = matched.len();
        if query.max_results > 0 && matched.len() > query.max_results {
            matched.truncate(query.max_results);
        }

        let elapsed = start.elapsed();
        let match_time_ms = elapsed.as_millis() as u64;

        debug!(
            "Matched {} experts (from {} available) in {}ms, top score: {:.3}",
            matched.len(),
            total_available,
            match_time_ms,
            matched.first().map(|m| m.score).unwrap_or(0.0)
        );

        Ok(ExpertMatchResult {
            query,
            matches: matched,
            total_available,
            match_time_ms,
        })
    }

    async fn get_expert(&self, expert_id: &str, tenant_id: &str) -> AllianceResult<Expert> {
        let experts = self.experts.read();
        experts
            .get(expert_id)
            .filter(|e| e.tenant_id == tenant_id || e.tenant_id == "system")
            .cloned()
            .ok_or_else(|| AllianceError::not_found("Expert", expert_id))
    }

    async fn refresh_cache(&self) -> AllianceResult<()> {
        // 内存版权重直接读取，无需缓存刷新
        Ok(())
    }

    async fn infer_domains(&self, description: &str) -> Vec<String> {
        let experts: Vec<Expert> = self.experts.read().values().cloned().collect();
        crate::matching::infer_domains(description, &experts, None)
    }
}

impl ModularWeightMatcher {
    /// 生成匹配原因的人类可读描述
    fn generate_match_reason(
        breakdown: &MatchScoreBreakdown,
        weights: &MatchingWeights,
    ) -> String {
        let mut reasons = Vec::new();

        if breakdown.domain_match >= 0.8 {
            reasons.push(format!("领域高度匹配 ({:.0}%)", breakdown.domain_match * 100.0));
        } else if breakdown.domain_match >= 0.5 {
            reasons.push(format!("领域部分匹配 ({:.0}%)", breakdown.domain_match * 100.0));
        }

        if breakdown.capability_match >= 0.7 {
            reasons.push(format!("能力强匹配 ({:.0}%)", breakdown.capability_match * 100.0));
        } else if breakdown.capability_match >= 0.4 {
            reasons.push(format!("能力部分匹配 ({:.0}%)", breakdown.capability_match * 100.0));
        }

        if breakdown.health_score >= 0.9 {
            reasons.push("健康状态优秀".to_string());
        } else if breakdown.health_score < 0.5 {
            reasons.push("健康状态一般".to_string());
        }

        if breakdown.performance_score >= 0.8 {
            reasons.push("性能表现优秀".to_string());
        }

        // 添加权重配置说明
        if weights.domain != 0.35
            || weights.capability != 0.30
            || weights.rating != 0.20
            || weights.performance != 0.10
            || weights.health != 0.05
        {
            reasons.push("使用自定义权重配置".to_string());
        }

        if reasons.is_empty() {
            "综合匹配".to_string()
        } else {
            reasons.join("；")
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{Capability, ExpertHealth, ExpertStatus};

    fn make_test_expert(id: &str, name: &str, domains: Vec<&str>, caps: Vec<&str>) -> Expert {
        let now = chrono::Utc::now();
        Expert {
            expert_id: id.to_string(),
            tenant_id: "system".to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("{} expert description", name),
            domains: domains.into_iter().map(|d| d.to_string()).collect(),
            capabilities: caps
                .into_iter()
                .enumerate()
                .map(|(i, c)| Capability {
                    capability_id: format!("{}-cap-{}", id, i),
                    name: c.to_string(),
                    description: format!("{} capability", c),
                    domain: "test".to_string(),
                    version: "1.0.0".to_string(),
                })
                .collect(),
            tools: vec![],
            status: ExpertStatus::Active,
            health: ExpertHealth::default(),
            priority: 5,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_query(
        _id: &str,
        description: &str,
        domains: Vec<&str>,
        caps: Vec<&str>,
    ) -> ExpertMatchQuery {
        ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: description.to_string(),
            required_domains: domains.into_iter().map(|s| s.to_string()).collect(),
            required_capabilities: caps.into_iter().map(|s| s.to_string()).collect(),
            min_priority: 1,
            max_results: 10,
        }
    }

    #[tokio::test]
    async fn basic_matching_works() {
        let matcher = ModularWeightMatcher::new();
        matcher.register_expert(make_test_expert(
            "arch",
            "架构专家",
            vec!["architecture"],
            vec!["系统设计", "微服务"],
        ));
        matcher.register_expert(make_test_expert(
            "algo",
            "算法专家",
            vec!["algorithm"],
            vec!["算法设计", "复杂度分析"],
        ));

        let query = make_query(
            "test-1",
            "系统设计 微服务",
            vec!["architecture"],
            vec!["系统设计"],
        );

        let result = matcher.match_experts(query).await.unwrap();
        assert!(!result.matches.is_empty());
        // 架构专家应该排在前面
        assert_eq!(result.matches[0].expert.expert_id, "arch");
    }

    #[tokio::test]
    async fn custom_weights_change_ranking() {
        let matcher = ModularWeightMatcher::new();

        // 专家 A：领域强匹配但能力一般
        let mut expert_a = make_test_expert(
            "a",
            "专家A",
            vec!["architecture"],
            vec!["系统设计"],
        );
        expert_a.priority = 8;
        matcher.register_expert(expert_a);

        // 专家 B：领域部分匹配但能力强
        let mut expert_b = make_test_expert(
            "b",
            "专家B",
            vec!["architecture", "data"],
            vec!["系统设计", "微服务", "性能优化"],
        );
        expert_b.priority = 3;
        matcher.register_expert(expert_b);

        // 默认权重下：领域权重高，A 应该排前
        let query = make_query(
            "test-2",
            "系统设计 微服务 架构",
            vec!["architecture"],
            vec!["系统设计", "微服务"],
        );

        let result_default = matcher.match_experts(query.clone()).await.unwrap();

        // 设置自定义权重：提高能力权重，降低领域权重
        let capability_heavy = MatchingWeights {
            domain: 0.15,
            capability: 0.50,
            rating: 0.15,
            performance: 0.15,
            health: 0.05,
        };
        matcher.set_expert_weights("b", capability_heavy);

        let result_custom = matcher.match_experts(query).await.unwrap();

        // 在自定义权重下，B 的分数应该变化
        let b_default_score = result_default
            .matches
            .iter()
            .find(|m| m.expert.expert_id == "b")
            .map(|m| m.score)
            .unwrap_or(0.0);
        let b_custom_score = result_custom
            .matches
            .iter()
            .find(|m| m.expert.expert_id == "b")
            .map(|m| m.score)
            .unwrap_or(0.0);

        // 分数应该不同（因为权重变了）
        assert!(
            (b_default_score - b_custom_score).abs() > 0.001,
            "Scores should differ with different weights"
        );
    }

    #[tokio::test]
    async fn inactive_experts_filtered() {
        let matcher = ModularWeightMatcher::new();

        let mut active = make_test_expert("active", "活跃专家", vec!["test"], vec!["能力A"]);
        active.status = ExpertStatus::Active;
        matcher.register_expert(active);

        let mut inactive =
            make_test_expert("inactive", "非活跃专家", vec!["test"], vec!["能力A"]);
        inactive.status = ExpertStatus::Inactive;
        matcher.register_expert(inactive);

        let query = make_query("test-3", "test", vec!["test"], vec![]);

        let result = matcher.match_experts(query).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].expert.expert_id, "active");
    }

    #[tokio::test]
    async fn min_priority_filter_applied() {
        let matcher = ModularWeightMatcher::new();

        let mut low = make_test_expert("low", "低优先级", vec!["test"], vec![]);
        low.priority = 2;
        matcher.register_expert(low);

        let mut high = make_test_expert("high", "高优先级", vec!["test"], vec![]);
        high.priority = 9;
        matcher.register_expert(high);

        let query = ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "test".to_string(),
            required_domains: vec!["test".to_string()],
            required_capabilities: vec![],
            min_priority: 5,
            max_results: 10,
        };

        let result = matcher.match_experts(query).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].expert.expert_id, "high");
    }

    #[tokio::test]
    async fn get_expert_works() {
        let matcher = ModularWeightMatcher::new();
        matcher.register_expert(make_test_expert("e1", "Test Expert", vec!["test"], vec![]));

        let expert = matcher.get_expert("e1", "system").await.unwrap();
        assert_eq!(expert.name, "Test Expert");

        let result = matcher.get_expert("nonexistent", "system").await;
        assert!(result.is_err());
    }

    #[test]
    fn default_weights_sum_to_one() {
        let w = MatchingWeights::default();
        let total = w.domain + w.capability + w.rating + w.performance + w.health;
        assert!((total - 1.0).abs() < 0.001);
    }
}
