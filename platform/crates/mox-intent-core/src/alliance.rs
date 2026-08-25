//! 专家联盟打分：matchExperts + scoreExperts 的 Rust 高性能权威实现。
//!
//! - 使用 aho-corasick 对专家 capabilities 做批量关键词匹配（替代 O(expert·cap) 字符串 contains）。
//! - score = matchScore + performanceBonus(成功×3) + confidenceBonus(置信×2)（与 Node 一致）。
//! - rayon 并行逐候选打分，再 TOP-K 排序。

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use ahash::RandomState;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap as StdMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertCandidate {
    pub id: String,
    /// 专家类型（与 intent.primary 匹配得 10 分）
    #[serde(rename = "type")]
    pub expert_type: String,
    /// 专家名称（命中关键词加 2 分）
    pub name: String,
    /// 能力标签（命中 question 加 3，命中 matched_keyword 再加 2）
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub match_score: i32,
    pub performance: f32,
    pub confidence: f32,
    pub total_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredExpert {
    pub id: String,
    pub score: f32,
    pub breakdown: ScoreBreakdown,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AllianceScorer {
    /// 每个 capability 的小写规范化版本
    caps: Vec<(String, Vec<usize>)>, // (cap_lower, Vec<expert_idx>)
    /// Aho-Corasick：pattern 是所有 capability 的小写
    ac: AhoCorasick,
    /// AC pattern index → expert_idx
    pat_to_expert: Vec<(usize, String)>, // (expert_idx, original_cap)
    experts: Vec<ExpertCandidate>,
}

impl AllianceScorer {
    pub fn new(experts: Vec<ExpertCandidate>) -> Self {
        let mut caps_patterns: Vec<String> = Vec::new();
        let mut pat_to_expert: Vec<(usize, String)> = Vec::new();
        let mut caps: Vec<(String, Vec<usize>)> = Vec::new();
        for (ei, ex) in experts.iter().enumerate() {
            for cap in &ex.capabilities {
                let low = cap.to_lowercase();
                caps_patterns.push(low.clone());
                pat_to_expert.push((ei, cap.clone()));
            }
            caps.push((ex.expert_type.to_lowercase(), Vec::new()));
        }
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .build(&caps_patterns);
        Self { caps, ac, pat_to_expert, experts }
    }

    pub fn score(
        &self,
        question: &str,
        intent_primary: &str,
        intent_secondary: &[String],
        matched_keywords: &[String],
        stats_of: impl Fn(&str) -> (f32, f32) + Sync, // expert_id -> (success_rate, avg_confidence)
    ) -> Vec<ScoredExpert> {
        let q = question.to_lowercase();
        // 预处理：匹配 capabilities
        let mut per_expert_hit_caps: Vec<Vec<String>> = vec![Vec::new(); self.experts.len()];
        for m in self.ac.find_iter(q.as_str()) {
            let (ei, cap) = &self.pat_to_expert[m.pattern().as_usize()];
            per_expert_hit_caps[*ei].push(cap.clone());
        }
        // 名字 + 关键词命中
        let matched_lower: Vec<String> = matched_keywords.iter().map(|s| s.to_lowercase()).collect();

        self.experts
            .par_iter()
            .enumerate()
            .map(|(ei, ex)| {
                let mut match_score = 0i32;
                let mut reasons = Vec::new();
                if ex.expert_type == intent_primary {
                    match_score += 10;
                    reasons.push("类型匹配".to_string());
                }
                if intent_secondary.iter().any(|s| s == &ex.expert_type) {
                    match_score += 5;
                    reasons.push("次要意图匹配".to_string());
                }

                // 能力命中 question：去重
                let mut seen_cap = std::collections::BTreeSet::new();
                for cap in &per_expert_hit_caps[ei] {
                    if seen_cap.insert(cap.clone()) {
                        match_score += 3;
                        reasons.push(format!("能力匹配: {}", cap));
                    }
                }
                // 关键词命中 capability 或 name
                let name_lower = ex.name.to_lowercase();
                for kw in &matched_lower {
                    if kw.is_empty() { continue; }
                    for cap in &ex.capabilities {
                        if cap.to_lowercase().contains(kw.as_str()) {
                            match_score += 2;
                        }
                    }
                    if name_lower.contains(kw.as_str()) {
                        match_score += 2;
                    }
                }

                if match_score == 0 {
                    match_score = 1;
                    reasons.push("默认匹配".to_string());
                }

                let (sr, ac_) = stats_of(&ex.id);
                let performance = sr * 3.0;
                let confidence = ac_ * 2.0;
                let total_score = match_score as f32 + performance + confidence;

                ScoredExpert {
                    id: ex.id.clone(),
                    score: total_score,
                    breakdown: ScoreBreakdown {
                        match_score,
                        performance,
                        confidence,
                        total_score,
                    },
                    reasons,
                }
            })
            .collect::<Vec<_>>()
    }
}

/// 便利 API：一次打分并按 score 降序返回（与 Node scoreExperts 同构）
pub fn score_alliance_candidates(
    experts: Vec<ExpertCandidate>,
    question: &str,
    intent_primary: &str,
    intent_secondary: &[String],
    matched_keywords: &[String],
    stats_of: &StdMap<String, (f32, f32)>,
) -> Vec<ScoredExpert> {
    let scorer = AllianceScorer::new(experts);
    let mut out = scorer.score(
        question,
        intent_primary,
        intent_secondary,
        matched_keywords,
        |id| stats_of.get(id).copied().unwrap_or((1.0, 0.7)),
    );
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap().then(a.id.cmp(&b.id)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn experts() -> Vec<ExpertCandidate> {
        vec![
            ExpertCandidate {
                id: "g1".into(), expert_type: "graph".into(),
                name: "图谱专家".into(),
                capabilities: vec!["图谱分析".into(), "算法".into()],
            },
            ExpertCandidate {
                id: "a1".into(), expert_type: "automation".into(),
                name: "自动化专家".into(),
                capabilities: vec!["工作流".into(), "自动化编排".into()],
            },
        ]
    }

    #[test]
    fn type_match_wins_big() {
        let stats = StdMap::new();
        let out = score_alliance_candidates(
            experts(),
            "请分析这个图谱算法",
            "graph",
            &[],
            &["算法".to_string()],
            &stats,
        );
        assert_eq!(out[0].id, "g1");
        assert!(out[0].breakdown.match_score >= 10);
    }

    #[test]
    fn defaults_fallback_gracefully() {
        let stats = StdMap::new();
        let out = score_alliance_candidates(
            experts(),
            "毫无关联的话题",
            "general",
            &[],
            &[],
            &stats,
        );
        // 两者都应是默认匹配 1 分 + performance 3*1 + conf 2*0.7 = 5.4
        for o in &out {
            assert_eq!(o.breakdown.match_score, 1);
            assert!((o.score - 5.4).abs() < 1e-6, "score={}", o.score);
        }
    }
}
