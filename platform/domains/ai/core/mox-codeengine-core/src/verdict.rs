// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Verdict 阶段：多专家意见的加权投票融合裁决（自研）。
//!
//! 规则：
//! - 总分 = Σ weight(expert) × score(expert)，权重和为 1；
//! - 任一专家出 Blocking 意见 → 禁止出码；其中 auditor 的 Blocking 另记为**一票否决**
//!   （与 mox-ai-expert-svc 的最高权限语义一致）；
//! - 总分 ≥ threshold 且零阻断 → approved。

use crate::experts::ExpertOpinion;
use mox_ai_flow_core::model::Severity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerdictPolicy {
    /// expert id → 权重
    pub weights: BTreeMap<String, f64>,
    /// 通过分数线
    pub threshold: f64,
}

impl Default for VerdictPolicy {
    fn default() -> Self {
        let mut weights = BTreeMap::new();
        weights.insert("analyst".into(), 0.25);
        weights.insert("builder".into(), 0.20);
        weights.insert("auditor".into(), 0.35);
        weights.insert("coordinator".into(), 0.20);
        Self {
            weights,
            threshold: 0.75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub score: f64,
    pub threshold: f64,
    pub approved: bool,
    /// 全体专家阻断级意见数
    pub blocking_findings: usize,
    /// auditor 一票否决是否触发
    pub vetoed: bool,
    pub votes: BTreeMap<String, f64>,
    pub summary: String,
}

/// 融合裁决（confidence_weighting 模式的自研实现）
pub fn fuse(opinions: &[ExpertOpinion], policy: &VerdictPolicy) -> Verdict {
    let mut votes = BTreeMap::new();
    let mut weighted = 0.0f64;
    let mut weight_sum = 0.0f64;
    let mut blocking = 0usize;
    let mut vetoed = false;

    for op in opinions {
        let w = policy.weights.get(op.expert).copied().unwrap_or(0.0);
        weighted += w * op.score;
        weight_sum += w;
        blocking += op.blocking_count();
        if op.findings.iter().any(|f| f.severity == Severity::Blocking) && op.expert == "auditor" {
            vetoed = true;
        }
        votes.insert(op.expert.to_string(), op.score);
    }

    let score = if weight_sum > 0.0 {
        weighted / weight_sum
    } else {
        0.0
    };
    // 任一专家的 Blocking 意见都禁止出码（auditor 的 Blocking 另记为一票否决）
    let approved = blocking == 0 && !vetoed && score >= policy.threshold;

    let summary = format!(
        "联盟裁决: score={:.3} threshold={:.2} 阻断={} 一票否决={} → {}",
        score,
        policy.threshold,
        blocking,
        vetoed,
        if approved { "通过出码" } else { "拒绝出码" }
    );

    Verdict {
        score,
        threshold: policy.threshold,
        approved,
        blocking_findings: blocking,
        vetoed,
        votes,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experts::Finding;
    use mox_ai_flow_core::model::Severity;

    fn opinion(expert: &'static str, score: f64, blocking: bool) -> ExpertOpinion {
        ExpertOpinion {
            expert,
            role: "test",
            score,
            findings: if blocking {
                vec![Finding {
                    code: "X",
                    expert,
                    severity: Severity::Blocking,
                    message: "m".into(),
                    suggestion: "s".into(),
                    confidence: 1.0,
                    target: None,
                }]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn approves_high_confidence_panel() {
        let panel = vec![
            opinion("analyst", 0.95, false),
            opinion("builder", 0.9, false),
            opinion("auditor", 0.95, false),
            opinion("coordinator", 0.9, false),
        ];
        let v = fuse(&panel, &VerdictPolicy::default());
        assert!(v.approved);
        assert!(!v.vetoed);
    }

    #[test]
    fn auditor_blocking_vetoes_even_with_high_score() {
        let panel = vec![
            opinion("analyst", 1.0, false),
            opinion("builder", 1.0, false),
            opinion("auditor", 0.0, true),
            opinion("coordinator", 1.0, false),
        ];
        let v = fuse(&panel, &VerdictPolicy::default());
        assert!(v.vetoed);
        assert!(!v.approved);
    }

    #[test]
    fn low_score_below_threshold_rejected() {
        let panel = vec![
            opinion("analyst", 0.5, false),
            opinion("builder", 0.5, false),
            opinion("auditor", 0.6, false),
            opinion("coordinator", 0.5, false),
        ];
        let v = fuse(&panel, &VerdictPolicy::default());
        assert!(!v.approved);
        assert!(!v.vetoed);
    }
}
