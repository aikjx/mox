// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 适配器与增强函数
//!
//! 提供 KgConnector 与上层业务（intent 分类、专家匹配）之间的适配层。
//! 包括：
//!   - spread_fn: 将连接器适配为 classify_intent 需要的闭包
//!   - enhance_expert_matching: 用图谱增强专家匹配排序

use std::collections::BTreeMap;

use super::traits::KgConnector;
use super::types::ExpertGraphBoost;

#[cfg(test)]
use super::types::GraphSearchHit;

// ================== spread_fn 适配器 ==================

/// 将 KgConnector 适配为 `classify_intent()` 需要的 `graph_spread_fn` 闭包
///
/// 用法：
/// ```ignore
/// let connector = HttpKgHubConnector::new("http://localhost:8080");
/// let intent = classify_intent(query, Some(spread_fn(&connector)));
/// ```
///
/// 注意：返回的是 `FnOnce` 闭包，因为 classify_intent 内部只调用一次扩散。
/// 闭包捕获 connector 的引用，调用方需保证 connector 生命周期覆盖闭包使用。
pub fn spread_fn<'a, C: KgConnector + ?Sized>(
    connector: &'a C,
) -> impl FnOnce(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String> + 'a {
    move |seeds, damping, rounds| connector.spread(seeds, damping, rounds)
}

// ================== 专家匹配增强 ==================

/// 用 kg-hub 图谱增强专家匹配排序
///
/// 流程：
///   1. 用 query 搜索图谱，获取相关节点
///   2. 对每个专家，计算其与搜索结果的关联度（专家 dimension/name 出现在搜索结果中的频率和分数）
///   3. 返回 {expert_id: boost_score}，可叠加到 team.rs 的 total 分数中
///
/// - `connector`: kg-hub 连接器
/// - `query`: 用户查询
/// - `expert_ids`: 待增强的专家 ID 列表
/// - `expert_dimensions`: 专家 ID → 维度名映射（用于图谱匹配）
pub fn enhance_expert_matching<C: KgConnector + ?Sized>(
    connector: &C,
    query: &str,
    expert_ids: &[String],
    expert_dimensions: &BTreeMap<String, String>,
) -> ExpertGraphBoost {
    let mut boosts: BTreeMap<String, f64> = BTreeMap::new();
    for id in expert_ids {
        boosts.insert(id.clone(), 0.0);
    }

    let hits = match connector.search(query, 30) {
        Ok(h) if !h.is_empty() => h,
        _ => {
            return ExpertGraphBoost {
                boosts,
                graph_used: false,
            };
        }
    };

    // 计算每个专家的图谱增强分
    // 策略：专家的 dimension/name 出现在搜索结果的 name/path/summary 中，
    // 按命中次数和分数加权
    for expert_id in expert_ids {
        let dim = expert_dimensions
            .get(expert_id)
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let expert_key = expert_id.to_lowercase();

        let mut total_score = 0.0_f64;
        let mut hit_count = 0;

        for hit in &hits {
            let text = format!(
                "{} {} {} {}",
                hit.name.to_lowercase(),
                hit.path.to_lowercase(),
                hit.summary.to_lowercase(),
                hit.kind.to_lowercase()
            );
            if text.contains(&expert_key) || text.contains(&dim) {
                total_score += hit.score;
                hit_count += 1;
            }
        }

        if hit_count > 0 {
            // 归一化：平均分数 × 命中次数衰减（避免单一高分节点过度影响）
            let avg = total_score / hit_count as f64;
            let count_factor = 1.0 - (-hit_count as f64 * 0.5).exp(); // 0..1 饱和
            let boost = (avg * count_factor).min(1.0);
            boosts.insert(expert_id.clone(), boost);
        }
    }

    ExpertGraphBoost {
        boosts,
        graph_used: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alliance::intent::classify_intent;
    use crate::alliance::kg_connector::mock::MockKgHubConnector;

    /// TDD 1: spread_fn 适配器能正确接入 classify_intent
    #[test]
    fn spread_fn_adapter_works_with_classify_intent() {
        let mut spread_result = BTreeMap::new();
        spread_result.insert("code".to_string(), 0.95);
        spread_result.insert("rust".to_string(), 0.80);
        spread_result.insert("programming".to_string(), 0.70);

        let connector = MockKgHubConnector::new().with_spread(spread_result);
        let result = classify_intent(
            "帮我写一个 Rust 函数",
            Some(spread_fn(&connector)),
        );

        // 图谱可用时 degraded 应为 false
        assert!(!result.degraded, "graph available should not be degraded");
        // spread_scores 应包含 code 类的高分
        assert!(
            result.spread_scores.get("code").copied().unwrap_or(0.0) > 0.0,
            "code spread score should be > 0, got {:?}",
            result.spread_scores
        );
    }

    /// TDD 2: kg-hub 不可用时自动降级
    #[test]
    fn unavailable_kg_hub_triggers_degraded() {
        let connector = MockKgHubConnector::new().unavailable();
        let result = classify_intent(
            "测试查询",
            Some(spread_fn(&connector)),
        );
        assert!(result.degraded, "unavailable kg-hub should trigger degraded");
        assert!(result.degrade_reason.is_some());
    }

    /// TDD 3: enhance_expert_matching 返回正确的增强分
    #[test]
    fn enhance_expert_matching_scores_relevant_experts() {
        let hits = vec![
            GraphSearchHit {
                id: "node1".into(),
                name: "Rust 代码质量分析".into(),
                kind: "Function".into(),
                layer: "L3".into(),
                path: "analysis/code_quality".into(),
                summary: "对 Rust 代码进行质量和性能分析".into(),
                score: 0.9,
                keyword_score: 0.8,
                vector_score: 0.7,
                graph_score: 0.6,
                matched_by: vec!["keyword".into()],
            },
            GraphSearchHit {
                id: "node2".into(),
                name: "安全审计".into(),
                kind: "Function".into(),
                layer: "L3".into(),
                path: "security/audit".into(),
                summary: "权限和安全漏洞检测".into(),
                score: 0.7,
                keyword_score: 0.6,
                vector_score: 0.5,
                graph_score: 0.4,
                matched_by: vec!["keyword".into()],
            },
        ];

        let connector = MockKgHubConnector::new().with_search(hits);
        let expert_ids = vec!["code_quality".to_string(), "security".to_string(), "business".to_string()];
        let mut dimensions = BTreeMap::new();
        dimensions.insert("code_quality".to_string(), "CodeQuality".to_string());
        dimensions.insert("security".to_string(), "Security".to_string());
        dimensions.insert("business".to_string(), "Business".to_string());

        let boost = enhance_expert_matching(&connector, "Rust 代码质量", &expert_ids, &dimensions);

        assert!(boost.graph_used, "should use graph");
        // code_quality 应该有较高增强分（命中 node1）
        assert!(
            boost.boosts.get("code_quality").copied().unwrap_or(0.0) > 0.0,
            "code_quality should get boost, got {:?}",
            boost.boosts
        );
        // security 应该有增强分（命中 node2）
        assert!(
            boost.boosts.get("security").copied().unwrap_or(0.0) > 0.0,
            "security should get boost"
        );
        // business 没有命中，应该为 0
        assert_eq!(
            boost.boosts.get("business").copied().unwrap_or(0.0),
            0.0,
            "business should have no boost"
        );
    }

    /// TDD 4: 图谱搜索为空时增强返回全 0
    #[test]
    fn empty_search_returns_zero_boosts() {
        let connector = MockKgHubConnector::new(); // 空搜索结果
        let expert_ids = vec!["code".to_string()];
        let dimensions = BTreeMap::new();
        let boost = enhance_expert_matching(&connector, "test", &expert_ids, &dimensions);
        assert!(!boost.graph_used);
        assert_eq!(boost.boosts.get("code").copied().unwrap_or(-1.0), 0.0);
    }
}
