// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 智能路由（IntelligentRouter）
//!
//! 根据请求特征（意图、上下文、历史表现）智能路由到最佳处理路径：
//! - 快速路径：简单查询 → 单专家 + 快速响应
//! - 标准路径：常规查询 → 4 专家并行 + 质量门禁
//! - 深度路径：复杂/敏感查询 → 7 专家 + LLM 辩论 + 全维度分析
//!
//! # 设计
//! - `IntelligentRouter` — 智能路由器结构体
//! - `RouteDecision` — 路由决策结果
//! - `RoutePath` — 三种路径枚举

use crate::events::{AllianceOptions, AllianceRequest};
use crate::intent::IntentResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

// ================== 路由路径枚举 ==================

/// 路由路径：决定使用哪种处理模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePath {
    /// 快速路径：1~2 专家，低延迟，适合简单查询
    Fast,
    /// 标准路径：4 专家并行 + 质量门禁（默认）
    Standard,
    /// 深度路径：7 专家 + LLM 辩论 + 全维度分析，适合复杂/敏感查询
    Deep,
}

impl RoutePath {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }

    /// 推荐组队大小
    pub fn recommended_team_size(&self) -> usize {
        match self {
            Self::Fast => 2,
            Self::Standard => 4,
            Self::Deep => 7,
        }
    }

    /// 是否启用 LLM 辩论
    pub fn should_enable_llm(&self) -> bool {
        matches!(self, Self::Deep)
    }

    /// 是否进行全维度算法分析
    pub fn should_full_algo_analysis(&self) -> bool {
        matches!(self, Self::Deep)
    }
}

// ================== 路由决策结果 ==================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// 选择的路径
    pub path: RoutePath,
    /// 决策置信度 0..1
    pub confidence: f64,
    /// 决策原因（可解释）
    pub reason: String,
    /// 决策因子明细
    pub factors: BTreeMap<String, f64>,
    /// 推荐的组队大小
    pub recommended_team_size: usize,
    /// 是否启用 LLM 辩论
    pub enable_llm: bool,
    /// 是否需要算法分析
    pub need_algo_analysis: bool,
}

// ================== 智能路由器 ==================

/// 智能路由器
///
/// 根据查询特征和意图分类结果，智能选择最佳处理路径。
/// 支持基于规则的路由，后续可扩展为基于 ML 的路由。
#[derive(Debug, Clone)]
pub struct IntelligentRouter {
    /// 快速路径阈值（查询越简单分越高）
    fast_threshold: f64,
    /// 深度路径阈值（查询越复杂分越高）
    deep_threshold: f64,
    /// 路由统计
    route_stats: BTreeMap<String, u64>,
}

impl Default for IntelligentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentRouter {
    pub fn new() -> Self {
        Self {
            fast_threshold: 0.70,
            deep_threshold: 0.65,
            route_stats: BTreeMap::new(),
        }
    }

    /// 自定义阈值
    pub fn with_thresholds(fast: f64, deep: f64) -> Self {
        Self {
            fast_threshold: fast,
            deep_threshold: deep,
            route_stats: BTreeMap::new(),
        }
    }

    /// 根据请求和意图结果进行路由决策
    pub fn route(&mut self, req: &AllianceRequest, intent: &IntentResult) -> RouteDecision {
        let mut factors = BTreeMap::new();

        // 因子 1: 查询复杂度（长度、关键词数量）
        let complexity_score = compute_complexity(&req.query);
        factors.insert("complexity".into(), complexity_score);

        // 因子 2: 意图置信度（置信度高 → 可能简单）
        let confidence_factor = 1.0 - intent.conf;
        factors.insert("intent_uncertainty".into(), confidence_factor);

        // 因子 3: 敏感标志
        let sensitive = is_sensitive_query(&req.query, &req.context, intent);
        let sensitive_score = if sensitive { 1.0 } else { 0.0 };
        factors.insert("sensitive".into(), sensitive_score);

        // 因子 4: 上下文丰富度
        let context_score = (req.context.len() as f64 / 5.0).min(1.0);
        factors.insert("context_richness".into(), context_score);

        // 因子 5: 代码/算法类查询深度处理
        let code_depth_score = match intent.intent_id.as_str() {
            "code" | "math" | "logic" => 0.8,
            _ => 0.2,
        };
        factors.insert("domain_depth".into(), code_depth_score);

        // 综合"深度分"：越高越需要深度处理
        let deep_score = 0.30 * complexity_score
            + 0.20 * confidence_factor
            + 0.25 * sensitive_score
            + 0.10 * context_score
            + 0.15 * code_depth_score;

        // 综合"简单分"：越高越可以快速处理
        let fast_score = 1.0 - deep_score;

        let (path, confidence, reason) = if sensitive || deep_score >= self.deep_threshold {
            // 敏感或复杂 → 深度路径
            let reason = if sensitive {
                format!("敏感场景触发深度路径（安全/权限需要严格审查），deep_score={:.2}", deep_score)
            } else {
                format!("查询复杂度较高（{:.2}），启用深度路径", deep_score)
            };
            (RoutePath::Deep, deep_score.min(1.0), reason)
        } else if fast_score >= self.fast_threshold && intent.conf > 0.8 {
            // 简单且置信度高 → 快速路径
            (
                RoutePath::Fast,
                fast_score.min(1.0),
                format!("查询简单（fast_score={:.2}）且意图置信度高（{:.2}），走快速路径", fast_score, intent.conf),
            )
        } else {
            // 默认 → 标准路径
            (
                RoutePath::Standard,
                0.5 + (fast_score - 0.5).abs() * 0.5,
                format!("标准路径（fast_score={:.2}, deep_score={:.2}）", fast_score, deep_score),
            )
        };

        // 记录统计
        *self.route_stats.entry(path.label().to_string()).or_insert(0) += 1;

        RouteDecision {
            path,
            confidence,
            reason,
            factors,
            recommended_team_size: path.recommended_team_size(),
            enable_llm: path.should_enable_llm(),
            need_algo_analysis: path.should_full_algo_analysis(),
        }
    }

    /// 应用路由决策到请求选项
    pub fn apply_decision(&self, decision: &RouteDecision, options: &mut AllianceOptions) {
        options.team_size = decision.recommended_team_size;
        options.enable_llm_debate = decision.enable_llm;
    }

    /// 获取路由统计
    pub fn route_stats(&self) -> &BTreeMap<String, u64> {
        &self.route_stats
    }

    /// 总路由次数
    pub fn total_routes(&self) -> u64 {
        self.route_stats.values().sum()
    }
}

// ================== 内部函数 ==================

fn compute_complexity(query: &str) -> f64 {
    let len = query.chars().count();
    let len_score = (len as f64 / 200.0).min(1.0);

    // 复杂关键词
    let complex_keywords = [
        "优化", "性能", "架构", "设计", "方案", "全维",
        "对比", "比较", "分析", "review", "重构",
        "企业级", "生产", "高可用", "分布式",
    ];
    let q_lower = query.to_lowercase();
    let keyword_count = complex_keywords.iter().filter(|k| q_lower.contains(*k)).count();
    let kw_score = (keyword_count as f64 / 3.0).min(1.0);

    // 标点/分句数
    let sentence_count = query.matches('。').count() + query.matches('？').count()
        + query.matches('!').count() + query.matches('.').count();
    let sentence_score = (sentence_count as f64 / 5.0).min(1.0);

    (0.40 * len_score + 0.35 * kw_score + 0.25 * sentence_score).clamp(0.0, 1.0)
}

fn is_sensitive_query(query: &str, context: &BTreeMap<String, String>, intent: &IntentResult) -> bool {
    // 上下文显式标记
    if context.get("sensitive").map(|s| s == "1").unwrap_or(false) {
        return true;
    }

    // 代码类 + 高置信度 → 可能敏感
    if matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6 {
        return true;
    }

    // 敏感关键词
    let sensitive_keywords = [
        "密码", "密钥", "token", "认证", "授权",
        "权限", "安全", "漏洞", "攻击", "注入",
        "admin", "root", "sudo",
    ];
    let q_lower = query.to_lowercase();
    sensitive_keywords.iter().any(|k| q_lower.contains(k))
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{classify_intent, IntentResult};
    use std::collections::BTreeMap;

    fn make_request(q: &str) -> AllianceRequest {
        AllianceRequest {
            query: q.to_string(),
            session_id: None,
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        }
    }

    fn fake_intent(winner: &str, conf: f64) -> IntentResult {
        let mut rrf = BTreeMap::new();
        rrf.insert(winner.into(), conf);
        for c in crate::constants::INTENT_CLASSES {
            rrf.entry(c.to_string()).or_insert(0.0);
        }
        IntentResult {
            intent_id: winner.into(),
            conf,
            keyword_scores: Default::default(),
            spread_scores: Default::default(),
            rrf_scores: rrf,
            degraded: false,
            degrade_reason: None,
            seeds_hit: vec![],
            trace_log: String::new(),
            diagnose_id: uuid::Uuid::new_v4(),
        }
    }

    #[test]
    fn simple_query_routes_fast() {
        let mut router = IntelligentRouter::new();
        let req = make_request("你好");
        let intent = fake_intent("knowledge", 0.85);
        let decision = router.route(&req, &intent);
        // 简单查询 + 高置信度应走快速路径
        assert!(
            decision.path == RoutePath::Fast || decision.path == RoutePath::Standard,
            "简单查询应走 fast 或 standard，实际 {:?}",
            decision.path
        );
    }

    #[test]
    fn code_query_routes_standard_or_deep() {
        let mut router = IntelligentRouter::new();
        let req = make_request("写一个 Rust 函数实现冒泡排序并优化性能");
        let intent = fake_intent("code", 0.9);
        let decision = router.route(&req, &intent);
        // 代码类查询不应走 fast
        assert_ne!(decision.path, RoutePath::Fast);
        assert!(
            decision.path == RoutePath::Standard || decision.path == RoutePath::Deep,
            "代码类查询应走 standard 或 deep"
        );
    }

    #[test]
    fn sensitive_context_routes_deep() {
        let mut router = IntelligentRouter::new();
        let mut req = make_request("检查权限配置");
        req.context.insert("sensitive".into(), "1".into());
        let intent = fake_intent("code", 0.7);
        let decision = router.route(&req, &intent);
        assert_eq!(decision.path, RoutePath::Deep);
        assert!(decision.enable_llm);
        assert!(decision.need_algo_analysis);
    }

    #[test]
    fn route_decision_factors_present() {
        let mut router = IntelligentRouter::new();
        let req = make_request("test");
        let intent = fake_intent("knowledge", 0.5);
        let decision = router.route(&req, &intent);
        assert!(decision.factors.contains_key("complexity"));
        assert!(decision.factors.contains_key("intent_uncertainty"));
        assert!(decision.factors.contains_key("sensitive"));
        assert!(!decision.reason.is_empty());
    }

    #[test]
    fn apply_decision_modifies_options() {
        let router = IntelligentRouter::new();
        let mut options = AllianceOptions::default();
        let decision = RouteDecision {
            path: RoutePath::Deep,
            confidence: 0.9,
            reason: "test".into(),
            factors: BTreeMap::new(),
            recommended_team_size: 7,
            enable_llm: true,
            need_algo_analysis: true,
        };
        router.apply_decision(&decision, &mut options);
        assert_eq!(options.team_size, 7);
        assert!(options.enable_llm_debate);
    }

    #[test]
    fn route_stats_accumulate() {
        let mut router = IntelligentRouter::new();
        let req = make_request("test query");
        let intent = fake_intent("knowledge", 0.8);
        for _ in 0..5 {
            let _ = router.route(&req, &intent);
        }
        assert_eq!(router.total_routes(), 5);
        assert!(!router.route_stats().is_empty());
    }

    #[test]
    fn route_path_recommended_sizes() {
        assert_eq!(RoutePath::Fast.recommended_team_size(), 2);
        assert_eq!(RoutePath::Standard.recommended_team_size(), 4);
        assert_eq!(RoutePath::Deep.recommended_team_size(), 7);
    }

    #[test]
    fn route_path_labels() {
        assert_eq!(RoutePath::Fast.label(), "fast");
        assert_eq!(RoutePath::Standard.label(), "standard");
        assert_eq!(RoutePath::Deep.label(), "deep");
    }
}
