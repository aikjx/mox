// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 基于规则的专家匹配器
//!
//! 实现 `ExpertMatcher` trait，使用规则匹配算法：
//! 1. 领域匹配（关键词 + 分类标签）
//! 2. 能力匹配（能力声明覆盖度）
//! 3. 健康状态过滤
//! 4. 综合评分排序
//!
//! 这是最简单的匹配实现，后续可以替换为向量匹配或图谱推理。

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceError, AllianceResult, Expert, ExpertStatus};
use mox_alliance_scheduler_proto::{
    ExpertMatchQuery, ExpertMatchResult, ExpertMatcher, MatchScoreBreakdown, MatchedExpert,
};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::debug;

use crate::matching::{description_overlap, tokenize, ExpertTokenCache};

/// 基于规则的专家匹配器
pub struct RuleBasedExpertMatcher {
    /// 专家注册表（内存版，Phase 1 用）
    experts: Arc<RwLock<HashMap<String, Expert>>>,
    /// 专家文本分词缓存（避免匹配时重复 tokenize）
    token_cache: ExpertTokenCache,
}

impl RuleBasedExpertMatcher {
    pub fn new() -> Self {
        Self {
            experts: Arc::new(RwLock::new(HashMap::new())),
            token_cache: ExpertTokenCache::new(),
        }
    }

    /// 使用共享的专家存储创建匹配器
    ///
    /// 用于与 `InMemoryExpertRegistry` 共享同一份专家数据，
    /// 这样同步器写入 registry 后，匹配器可以直接读到最新数据。
    pub fn with_shared_experts(experts: Arc<RwLock<HashMap<String, Expert>>>) -> Self {
        Self {
            experts,
            token_cache: ExpertTokenCache::new(),
        }
    }

    /// 获取内部专家存储的 Arc 引用（供 bridge 共享使用）
    pub fn experts_arc(&self) -> Arc<RwLock<HashMap<String, Expert>>> {
        self.experts.clone()
    }

    /// 注册专家（用于测试和初始化）
    pub fn register_expert(&self, expert: Expert) {
        self.token_cache.invalidate(&expert.expert_id);
        let mut experts = self.experts.write();
        experts.insert(expert.expert_id.clone(), expert);
    }

    /// 批量注册专家
    pub fn register_experts(&self, experts_list: Vec<Expert>) {
        for expert in &experts_list {
            self.token_cache.invalidate(&expert.expert_id);
        }
        let mut experts = self.experts.write();
        for expert in experts_list {
            experts.insert(expert.expert_id.clone(), expert);
        }
    }

    /// 计算领域匹配分
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

    /// 计算能力匹配分
    fn calculate_capability_score(expert: &Expert, query: &ExpertMatchQuery) -> f64 {
        if query.required_capabilities.is_empty() {
            return 0.5; // 没有明确能力需求时给中等分
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

    /// 描述文本相似度（基于中英分词的 token 重叠，解决中文整句永不命中的问题）
    ///
    /// 使用 [`ExpertTokenCache`] 避免对同一份专家文本重复分词。
    fn calculate_description_score(&self, expert: &Expert, query: &ExpertMatchQuery) -> f64 {
        let query_tokens = tokenize(&query.task_description);
        description_overlap(expert, &query_tokens, Some(&self.token_cache)).0
    }

    /// 综合评分
    fn calculate_total_score(breakdown: &MatchScoreBreakdown) -> f64 {
        // 权重：领域 30%、能力 30%、健康 15%、优先级 15%、表现 10%
        breakdown.domain_match * 0.30
            + breakdown.capability_match * 0.30
            + breakdown.health_score * 0.15
            + breakdown.priority_score * 0.15
            + breakdown.performance_score * 0.10
    }
}

impl Default for RuleBasedExpertMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExpertMatcher for RuleBasedExpertMatcher {
    async fn match_experts(&self, query: ExpertMatchQuery) -> AllianceResult<ExpertMatchResult> {
        let start = std::time::Instant::now();
        let experts = self.experts.read();

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

            // 计算各维度分数
            let domain_score = Self::calculate_domain_score(expert, &query);
            let capability_score = Self::calculate_capability_score(expert, &query);
            let description_score = self.calculate_description_score(expert, &query);

            // 描述得分并入能力分（简单加权）
            let capability_final = capability_score * 0.7 + description_score * 0.3;

            let health_score = if expert.health.is_healthy { 1.0 } else { 0.3 };
            let priority_score = expert.priority as f64 / 10.0;
            let performance_score = expert.health.success_rate;

            let breakdown = MatchScoreBreakdown {
                domain_match: domain_score,
                capability_match: capability_final,
                health_score,
                priority_score,
                performance_score,
            };

            let total_score = Self::calculate_total_score(&breakdown);

            // 过滤掉分数太低的
            if total_score < 0.2 {
                continue;
            }

            let match_reason = format!(
                "领域匹配 {:.0}%，能力匹配 {:.0}%，健康 {:.0}%",
                domain_score * 100.0,
                capability_final * 100.0,
                health_score * 100.0
            );

            matched.push(MatchedExpert {
                expert: expert.clone(),
                score: total_score,
                match_reason,
                score_breakdown: breakdown,
            });
        }

        // 按分数降序排序
        matched.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 截取最大结果数
        let total_available = matched.len();
        matched.truncate(query.max_results);

        let match_time_ms = start.elapsed().as_millis() as u64;

        debug!(
            "Matched {} experts for query ({} available), took {}ms",
            matched.len(),
            total_available,
            match_time_ms
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
        // 内存版不需要刷新缓存
        Ok(())
    }

    async fn infer_domains(&self, description: &str) -> Vec<String> {
        let experts: Vec<Expert> = self.experts.read().values().cloned().collect();
        crate::matching::infer_domains(description, &experts, Some(&self.token_cache))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::Expert;

    fn make_test_expert(id: &str, name: &str, domains: Vec<&str>) -> Expert {
        let mut expert = Expert::new_system(name.to_string(), format!("Expert for {}", name));
        expert.expert_id = id.to_string();
        expert.domains = domains.into_iter().map(|s| s.to_string()).collect();
        expert
    }

    #[tokio::test]
    async fn test_match_experts() {
        let matcher = RuleBasedExpertMatcher::new();
        matcher.register_expert(make_test_expert("e1", "Code Quality Expert", vec!["code", "quality"]));
        matcher.register_expert(make_test_expert("e2", "Security Expert", vec!["security", "code"]));
        matcher.register_expert(make_test_expert("e3", "Data Expert", vec!["data", "analysis"]));

        let query = ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "code quality review".to_string(),
            required_domains: vec!["code".to_string()],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 10,
        };

        let result = matcher.match_experts(query).await.unwrap();
        assert!(result.matches.len() >= 2); // e1 和 e2 都匹配 code 领域
        assert!(result.matches[0].score > 0.0);
    }

    #[tokio::test]
    async fn test_get_expert() {
        let matcher = RuleBasedExpertMatcher::new();
        matcher.register_expert(make_test_expert("e1", "Test Expert", vec!["test"]));

        let expert = matcher.get_expert("e1", "system").await.unwrap();
        assert_eq!(expert.name, "Test Expert");

        let result = matcher.get_expert("nonexistent", "system").await;
        assert!(result.is_err());
    }
}
