// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家匹配器 trait 抽象

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceResult, Expert};
use serde::{Deserialize, Serialize};

/// 专家匹配查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMatchQuery {
    pub tenant_id: String,
    pub task_description: String,
    pub required_domains: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub min_priority: u8,
    pub max_results: usize,
}

/// 匹配到的专家
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedExpert {
    pub expert: Expert,
    /// 匹配分数 0.0 ~ 1.0
    pub score: f64,
    /// 匹配原因描述
    pub match_reason: String,
    /// 各维度评分明细
    pub score_breakdown: MatchScoreBreakdown,
}

/// 匹配分数明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchScoreBreakdown {
    /// 领域匹配度
    pub domain_match: f64,
    /// 能力匹配度
    pub capability_match: f64,
    /// 健康状态分
    pub health_score: f64,
    /// 优先级分
    pub priority_score: f64,
    /// 历史表现分
    pub performance_score: f64,
}

/// 专家匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMatchResult {
    pub query: ExpertMatchQuery,
    pub matches: Vec<MatchedExpert>,
    pub total_available: usize,
    pub match_time_ms: u64,
}

/// 专家匹配器 trait
///
/// 负责根据任务描述和需求，从专家注册中心匹配最合适的专家。
/// 匹配策略可插拔：规则匹配 / 向量相似度 / 图谱推理 / 混合。
#[async_trait]
pub trait ExpertMatcher: Send + Sync {
    /// 匹配专家
    async fn match_experts(&self, query: ExpertMatchQuery) -> AllianceResult<ExpertMatchResult>;

    /// 获取单个专家（用于精确匹配）
    async fn get_expert(&self, expert_id: &str, tenant_id: &str) -> AllianceResult<Expert>;

    /// 刷新匹配缓存
    async fn refresh_cache(&self) -> AllianceResult<()>;
}
