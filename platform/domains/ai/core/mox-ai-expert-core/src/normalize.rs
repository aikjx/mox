// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 十四维归一化：把各专家产出的原始分数/指标归一化为统一可比的健康分
//!
//! 设计要点：
//! - 每个维度有独立的评分函数（score → [0,1]）
//! - 维度权重由 SSOT 常量 `NORMALIZATION_WEIGHTS` 定义
//! - 综合分 = Σ(score_i × weight_i) / Σweight_i（加权平均）
//! - 归一化不改变原始观点，只派生归一化后的分值供裁决器使用

use mox_ai_expert_proto::{dim_weight, Dimension, ExpertOpinion};
use std::collections::HashMap;

/// 单个维度的归一化结果
#[derive(Debug, Clone)]
pub struct NormalizedScore {
    pub dimension: Dimension,
    pub expert_id: String,
    /// 原始分（专家自评，0..1）
    pub raw_score: f64,
    /// 归一化后的分（0..1，经维度权重调整）
    pub normalized_score: f64,
    /// 维度权重
    pub weight: f64,
    /// 阻断级风险数
    pub blocking_risks: usize,
    /// 警告级风险数
    pub warning_risks: usize,
}

/// 全维归一化结果
#[derive(Debug, Clone)]
pub struct NormalizedReport {
    /// 各维度归一化分数
    pub scores: Vec<NormalizedScore>,
    /// 综合健康分（加权平均，0..1）
    pub overall_score: f64,
    /// 总阻断级风险数
    pub total_blocking: usize,
    /// 总警告级风险数
    pub total_warning: usize,
    /// 是否有任一维度被否决（veto 级风险）
    pub has_veto: bool,
}

/// 获取维度权重（从 SSOT 常量 dim_weight，缺省返回 1.0）
pub fn dimension_weight(dim: Dimension) -> f64 {
    dim_weight(dim)
}

/// 对单个专家观点进行归一化
///
/// 规则：
/// - 基础分 = expert.score（专家自评，0..1）
/// - 每个 Blocking 风险扣 0.3 分（最低 0）
/// - 每个 Warning 风险扣 0.1 分（最低 0）
/// - 有 veto 级风险 → 分数置 0
/// - 最终结果与维度权重相乘得到加权分
pub fn normalize_opinion(op: &ExpertOpinion) -> NormalizedScore {
    let weight = dimension_weight(op.dimension);
    let mut score = op.score.max(0.0).min(1.0);

    let blocking = op
        .risks
        .iter()
        .filter(|r| r.severity == mox_ai_expert_proto::Severity::Blocking)
        .count();
    let warning = op
        .risks
        .iter()
        .filter(|r| r.severity == mox_ai_expert_proto::Severity::Warning)
        .count();
    let has_veto = op.risks.iter().any(|r| r.veto);

    if has_veto {
        score = 0.0;
    } else {
        score = (score - blocking as f64 * 0.3).max(0.0);
        score = (score - warning as f64 * 0.1).max(0.0);
    }

    NormalizedScore {
        dimension: op.dimension,
        expert_id: op.expert.clone(),
        raw_score: op.score,
        normalized_score: score * weight,
        weight,
        blocking_risks: blocking,
        warning_risks: warning,
    }
}

/// 对一组专家观点进行全维归一化，产出综合报告
pub fn normalize_all(opinions: &[ExpertOpinion]) -> NormalizedReport {
    let scores: Vec<NormalizedScore> = opinions.iter().map(normalize_opinion).collect();

    let total_weight: f64 = scores.iter().map(|s| s.weight).sum();
    let total_normalized: f64 = scores.iter().map(|s| s.normalized_score).sum();

    let overall_score = if total_weight > 0.0 {
        (total_normalized / total_weight).max(0.0).min(1.0)
    } else {
        1.0
    };

    let total_blocking = scores.iter().map(|s| s.blocking_risks).sum();
    let total_warning = scores.iter().map(|s| s.warning_risks).sum();
    let has_veto = opinions.iter().any(|o| o.risks.iter().any(|r| r.veto));

    NormalizedReport {
        scores,
        overall_score,
        total_blocking,
        total_warning,
        has_veto,
    }
}

/// 按维度分组的归一化结果（便于按维度查分）
pub fn scores_by_dimension(report: &NormalizedReport) -> HashMap<Dimension, f64> {
    let mut map = HashMap::new();
    for s in &report.scores {
        map.insert(s.dimension, s.normalized_score / s.weight.max(f64::EPSILON));
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::Severity;

    fn make_opinion(id: &str, dim: Dimension, score: f64) -> ExpertOpinion {
        let mut o = ExpertOpinion::empty(id, dim);
        o.score = score;
        o
    }

    #[test]
    fn perfect_score_stays_high() {
        let o = make_opinion("biz", Dimension::Business, 1.0);
        let n = normalize_opinion(&o);
        assert_eq!(n.raw_score, 1.0);
        assert!(n.normalized_score > 0.0);
        assert_eq!(n.blocking_risks, 0);
    }

    #[test]
    fn blocking_risk_lowers_score() {
        let mut o = make_opinion("sec", Dimension::Security, 1.0);
        o.push_risk(Severity::Blocking, vec!["n1".into()], "leak", None);
        let n = normalize_opinion(&o);
        assert!(n.normalized_score < n.weight); // 应被扣减
        assert_eq!(n.blocking_risks, 1);
    }

    #[test]
    fn veto_resets_score_to_zero() {
        let mut o = make_opinion("perm", Dimension::Permission, 1.0);
        o.push_veto(vec!["n1".into()], "越权写敏感库", None);
        let n = normalize_opinion(&o);
        assert!((n.normalized_score - 0.0).abs() < 1e-9);
    }

    #[test]
    fn overall_score_is_weighted_average() {
        let opinions = vec![
            make_opinion("biz", Dimension::Business, 1.0),
            make_opinion("algo", Dimension::Algorithm, 0.8),
        ];
        let report = normalize_all(&opinions);
        assert!(report.overall_score > 0.0 && report.overall_score <= 1.0);
        assert_eq!(report.scores.len(), 2);
    }

    #[test]
    fn empty_opinions_returns_perfect() {
        let report = normalize_all(&[]);
        assert!((report.overall_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dimension_weight_accessible() {
        // 关键维度应有权重
        let w_perm = dimension_weight(Dimension::Permission);
        let w_biz = dimension_weight(Dimension::Business);
        assert!(w_perm > 0.0);
        assert!(w_biz > 0.0);
    }

    #[test]
    fn scores_by_dimension_maps_correctly() {
        let opinions = vec![
            make_opinion("biz", Dimension::Business, 0.9),
            make_opinion("sec", Dimension::Security, 0.7),
        ];
        let report = normalize_all(&opinions);
        let map = scores_by_dimension(&report);
        assert!(map.contains_key(&Dimension::Business));
        assert!(map.contains_key(&Dimension::Security));
        // 分数应在 0..1 之间
        for (_, s) in &map {
            assert!(*s >= 0.0 && *s <= 1.0);
        }
    }

    #[test]
    fn has_veto_flag_propagates() {
        let mut o = make_opinion("sec", Dimension::Security, 1.0);
        o.push_veto(vec!["n1".into()], "veto test", None);
        let report = normalize_all(&[o]);
        assert!(report.has_veto);
    }

    #[test]
    fn risk_counts_accumulate() {
        let mut o1 = make_opinion("sec", Dimension::Security, 1.0);
        o1.push_risk(Severity::Blocking, vec!["a".into()], "b1", None);
        o1.push_risk(Severity::Warning, vec!["b".into()], "w1", None);
        let mut o2 = make_opinion("perm", Dimension::Permission, 1.0);
        o2.push_risk(Severity::Blocking, vec!["c".into()], "b2", None);

        let report = normalize_all(&[o1, o2]);
        assert_eq!(report.total_blocking, 2);
        assert_eq!(report.total_warning, 1);
    }
}
