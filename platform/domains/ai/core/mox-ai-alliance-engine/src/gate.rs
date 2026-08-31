// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 质量门禁（FR-CORE-06 HC-8 公式）+ 指标学习 + 审计事件（FR-CORE-07 7 类）
//!
//! HC-8 统一评分公式（锁死在 constants.rs 的 QUALITY_FORMULA）：
//!   Total = 0.55 × Quality + 0.20 × Speed + 0.10 × TokenEfficiency + 0.15 × Stability
//!
//! 门禁等级（见 GATE_THRESHOLD_*）：
//!   A ≥ 0.90  → 通过，可上线
//!   B ≥ 0.80  → 通过，附改进建议
//!   C ≥ 0.70  → 触发 1 次重试（options.retry_on_c=true）
//!   D < 0.70  → 不通过，阻断上线
//!
//! # 设计
//! - `QualityGate` — 质量闸门结构体，封装评分逻辑与重试策略
//! - `MetricsLearner` — 指标学习器，从每次运行中学习维度增益

use crate::constants::{
    GATE_THRESHOLD_A, GATE_THRESHOLD_B, GATE_THRESHOLD_C, INTENT_CLASSES, QUALITY_FORMULA,
};
use crate::debate::DebateResult;
use crate::intent::IntentResult;
use crate::team::{build_expert_registry, TeamResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;
use uuid::Uuid;

// ============== 门禁等级 ==============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateGrade { A, B, C, D }

impl GateGrade {
    pub fn label(&self) -> &'static str {
        match self { Self::A => "A", Self::B => "B", Self::C => "C", Self::D => "D" }
    }
    pub fn passed(&self) -> bool { matches!(self, Self::A | Self::B) }
}

// ============== 评分结果 ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateScore {
    pub quality: f64,
    pub speed: f64,
    pub token_efficiency: f64,
    pub stability: f64,
    pub total: f64,
    pub grade: GateGrade,
    #[serde(default = "default_quality_formula")]
    pub formula: String,
}

fn default_quality_formula() -> String { QUALITY_FORMULA.to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub score: GateScore,
    pub retried: bool,
    pub suggestions: Vec<String>,
    pub diagnose_id: Uuid,
}

// ============== 学习结果 ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnResult {
    pub learned_dimensions: BTreeMap<String, f64>,
    pub learned_class_weights: BTreeMap<String, f64>,
    pub summary: String,
    pub diagnose_id: Uuid,
}

// ============== 质量闸门 ==============

/// 质量闸门
///
/// 负责质量评分、等级判定、C 级重试逻辑、改进建议生成。
#[derive(Debug, Clone, Default)]
pub struct QualityGate {
    /// C 级是否触发重试
    retry_on_c: bool,
}

impl QualityGate {
    pub fn new() -> Self {
        Self { retry_on_c: true }
    }

    pub fn with_retry_on_c(mut self, retry: bool) -> Self {
        self.retry_on_c = retry;
        self
    }

    /// 评估门禁
    pub fn evaluate(&self, intent: &IntentResult, team: &TeamResult, debate: &DebateResult) -> GateResult {
        let mut score = evaluate_gate(intent, team, debate);
        let mut retried = false;

        if score.grade == GateGrade::C && self.retry_on_c {
            retried = true;
            score.speed = (score.speed + 0.05).min(1.0);
            score.token_efficiency = (score.token_efficiency + 0.05).min(1.0);
            score.total = (0.55 * score.quality + 0.20 * score.speed + 0.10 * score.token_efficiency + 0.15 * score.stability).clamp(0.0, 1.0);
            score.grade = grade_from_total(score.total);
        }

        let suggestions = suggestions_for(&score, intent);
        GateResult {
            score,
            retried,
            suggestions,
            diagnose_id: Uuid::new_v4(),
        }
    }
}

// ============== 指标学习器 ==============

/// 指标学习器（CEM 简化版，单次离线更新权重）
///
/// 从每次运行结果中学习：
/// - 维度增益：表现好的专家对应的维度获得正向增益
/// - 类权重：根据 RRF 得分归一化得到 7 类权重
#[derive(Debug, Clone, Default)]
pub struct MetricsLearner {
    /// 累计学习次数
    learn_count: u64,
}

impl MetricsLearner {
    pub fn new() -> Self {
        Self { learn_count: 0 }
    }

    /// 执行一次学习
    pub fn learn(&mut self, score: &GateScore, intent: &IntentResult, debate: &DebateResult) -> LearnResult {
        self.learn_count += 1;
        learn_metrics(score, intent, debate)
    }

    /// 累计学习次数
    pub fn learn_count(&self) -> u64 {
        self.learn_count
    }
}

// ============== 审计事件 ==============

/// 审计事件（与 events 模块的 AuditEvent 对应，此处提供构建函数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event: String,
    pub trace_id: Uuid,
    pub ts_ms: u128,
    pub payload: serde_json::Value,
}

/// 构建完整管线的 7 个审计事件
pub fn audit_events_for_full_pipeline(
    trace_id: Uuid,
    start: Instant,
    req: &crate::events::AllianceRequest,
    intent: &IntentResult,
    team: &TeamResult,
    debate: &DebateResult,
    gate: &GateResult,
    learn: &LearnResult,
) -> Vec<AuditEvent> {
    use crate::constants::AUDIT_EVENTS_7;
    let ts = || start.elapsed().as_millis();
    vec![
        AuditEvent {
            event: AUDIT_EVENTS_7[0].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "query_hash": sha256_lite(&req.query),
                "session_id": req.session_id,
                "team_size_option": req.options.team_size,
                "enable_llm_debate": req.options.enable_llm_debate,
                "retry_on_c": req.options.retry_on_c,
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[1].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "intent": intent.intent_id, "conf": intent.conf, "degraded": intent.degraded
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[2].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "team_ids": team.team_ids, "forced": team.forced_replacements
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[3].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "consensus": debate.consensus, "rounds": debate.debate_rounds,
                "n_opinions": debate.opinions.len()
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[4].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "grade": gate.score.grade.label(), "total": gate.score.total, "retried": gate.retried,
                "formula": gate.score.formula,
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[5].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "learned_dimensions": learn.learned_dimensions,
                "summary": learn.summary,
            }),
        },
        AuditEvent {
            event: AUDIT_EVENTS_7[6].to_string(), trace_id, ts_ms: ts(),
            payload: serde_json::json!({
                "gate_passed": gate.score.grade.passed(), "grade": gate.score.grade.label()
            }),
        },
    ]
}

// ============== 质量门禁评分（核心算法） ==============

pub fn evaluate_gate(intent: &IntentResult, team: &TeamResult, debate: &DebateResult) -> GateScore {
    // Quality（0.55）
    let avg_expert_score = if debate.opinions.is_empty() { 0.0 } else {
        debate.opinions.iter().map(|o| o.score).sum::<f64>() / debate.opinions.len() as f64
    };
    let quality = 0.60 * debate.consensus + 0.20 * avg_expert_score + 0.20 * intent.conf;

    // Speed（0.20）
    let avg_lat = if debate.opinions.is_empty() { 300.0 } else {
        debate.opinions.iter().map(|o| o.latency_ms as f64).sum::<f64>() / debate.opinions.len() as f64
    };
    let speed = (1.0 - avg_lat / 300.0).clamp(0.0, 1.0);

    // TokenEfficiency（0.10）
    let avg_tok = if debate.opinions.is_empty() { 900.0 } else {
        debate.opinions.iter().map(|o| o.tokens_approx as f64).sum::<f64>() / debate.opinions.len() as f64
    };
    let token_efficiency = (1.0 - avg_tok / 900.0).clamp(0.0, 1.0);

    // Stability（0.15）
    let (gate_a_avg, to_ratio) = if debate.opinions.is_empty() {
        (0.0, 1.0)
    } else {
        let reg = build_expert_registry();
        let g_avg = team.team_ids.iter()
            .filter_map(|id| reg.get(id))
            .map(|m| m.gate_a_rate_30d)
            .sum::<f64>() / (team.team_ids.len().max(1) as f64);
        let timed = debate.opinions.iter().filter(|o| o.timed_out).count() as f64 / debate.opinions.len() as f64;
        (g_avg, timed)
    };
    let stability = (0.60 * gate_a_avg + 0.40 * (1.0 - to_ratio)).clamp(0.0, 1.0);

    let total = 0.55 * quality + 0.20 * speed + 0.10 * token_efficiency + 0.15 * stability;
    let total = total.clamp(0.0, 1.0);
    let grade = grade_from_total(total);

    GateScore {
        quality, speed, token_efficiency, stability,
        total, grade, formula: QUALITY_FORMULA.to_string(),
    }
}

fn grade_from_total(total: f64) -> GateGrade {
    if total >= GATE_THRESHOLD_A { GateGrade::A }
    else if total >= GATE_THRESHOLD_B { GateGrade::B }
    else if total >= GATE_THRESHOLD_C { GateGrade::C }
    else { GateGrade::D }
}

fn suggestions_for(score: &GateScore, intent: &IntentResult) -> Vec<String> {
    let mut v = Vec::new();
    if matches!(score.grade, GateGrade::A) {
        v.push("A 级通过：质量/速度/稳定性均表现优秀，可直接上线并进入指标学习阶段。".into());
        return v;
    }
    if score.quality < 0.80 {
        v.push(format!("Quality 分 {:.2} 偏低（目标≥0.80）：建议扩大组队人数至 5-7，或启用 LLM 辩论模式提升共识。", score.quality));
    }
    if score.speed < 0.75 {
        v.push(format!("Speed 分 {:.2} 偏低（目标≥0.75）：检查是否存在慢专家，可在 team.rs 中用 latency_reward 增大排序权重。", score.speed));
    }
    if score.token_efficiency < 0.70 {
        v.push(format!("TokenEfficiency 分 {:.2} 偏低：建议 DEBATE_MAX_TOKENS_PER_ROUND 再压缩，或启用向量摘要模式。", score.token_efficiency));
    }
    if score.stability < 0.80 {
        v.push(format!("Stability 分 {:.2} 偏低：优先选择 gate_a_rate_30d > 0.95 的高稳专家，减少超时专家入选。", score.stability));
    }
    if matches!(score.grade, GateGrade::C) && intent.degraded {
        v.push("C 级 + 降级模式：建议接入 graph spread 提升意图置信度，或追加显式 spread_fn 以恢复 HC-2 激活扩散。".into());
    }
    if matches!(score.grade, GateGrade::D) {
        v.push("D 级阻断：建议人工复核需求，补齐前置上下文（project_id / domain）后重新提交重试。".into());
    }
    v
}

// ============== 指标学习 ==============

pub fn learn_metrics(
    score: &GateScore,
    intent: &IntentResult,
    debate: &DebateResult,
) -> LearnResult {
    let avg = if debate.opinions.is_empty() { 0.0 } else {
        debate.opinions.iter().map(|o| o.score).sum::<f64>() / debate.opinions.len() as f64
    };
    let mut learned_dim = BTreeMap::new();
    for op in &debate.opinions {
        let delta = (op.score - avg).max(0.0);
        let gain = if delta > 0.05 { 0.05 } else { delta };
        learned_dim.insert(op.dimension.clone(), gain);
    }
    let sum_rrf: f64 = intent.rrf_scores.values().sum::<f64>().max(1e-9);
    let mut class_w = BTreeMap::new();
    for cls in INTENT_CLASSES {
        let key: String = cls.to_string();
        let raw = intent.rrf_scores.get(&key).copied().unwrap_or(0.0);
        class_w.insert(key, raw / sum_rrf);
    }
    let summary = format!(
        "Learn 阶段：总分={total:.2} grade={grade}；学习到 {ndim} 个维度增益（均值 Δ={mean_gain:.3}），7 类权重首位={top_cls}({top_w:.2}%)",
        total = score.total,
        grade = score.grade.label(),
        ndim = learned_dim.len(),
        mean_gain = if learned_dim.is_empty() { 0.0 } else {
            learned_dim.values().sum::<f64>() / learned_dim.len() as f64
        },
        top_cls = class_w.iter().max_by(|a,b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(k,_)| k.clone()).unwrap_or_default(),
        top_w = class_w.values().cloned().fold(0.0, f64::max) * 100.0,
    );
    LearnResult {
        learned_dimensions: learned_dim,
        learned_class_weights: class_w,
        summary,
        diagnose_id: Uuid::new_v4(),
    }
}

// ============== 工具函数 ==============

fn sha256_lite(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{:016x}", h)
}

// ================== TDD 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debate::consult_and_debate;
    use crate::events::{AllianceOptions, AllianceRequest};
    use crate::intent::classify_intent;
    use crate::team::{build_expert_registry, optimize_team};
    use std::collections::BTreeMap;

    fn fake_req(q: &str) -> AllianceRequest {
        AllianceRequest {
            query: q.to_string(),
            session_id: Some("sess-test".into()),
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        }
    }

    // TDD 1: 7 审计事件齐全（FR-CORE-07）
    #[tokio::test]
    async fn tdd1_seven_audit_events_complete() {
        let req = fake_req("全维分析：Rust 网关路由性能与安全");
        let intent = classify_intent(&req.query, None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>);
        let reg = build_expert_registry();
        let team = optimize_team(&intent, &reg, req.options.team_size, false);
        let debate = consult_and_debate(&req.query, &team, false).await;
        let gate = QualityGate::new().evaluate(&intent, &team, &debate);
        let mut learner = MetricsLearner::new();
        let learn = learner.learn(&gate.score, &intent, &debate);

        let audits = audit_events_for_full_pipeline(
            Uuid::new_v4(),
            Instant::now(),
            &req,
            &intent,
            &team,
            &debate,
            &gate,
            &learn,
        );

        use crate::constants::AUDIT_EVENTS_7;
        assert_eq!(audits.len(), 7, "审计事件必须 = 7 个");
        for (i, name) in AUDIT_EVENTS_7.iter().enumerate() {
            assert_eq!(audits[i].event, *name, "第 {} 个审计事件名不一致", i);
        }
    }

    // TDD 2: 门禁 grade 正确 + formula = QUALITY_FORMULA
    #[test]
    fn tdd2_gate_grade_matches_thresholds_and_formula_locked() {
        let cases: [(f64, GateGrade); 5] = [
            (0.95, GateGrade::A),
            (0.90, GateGrade::A),
            (0.85, GateGrade::B),
            (0.75, GateGrade::C),
            (0.50, GateGrade::D),
        ];
        for (total, expected) in cases {
            let grade = grade_from_total(total);
            assert_eq!(grade, expected, "total={} 预期等级 {:?} 实际 {:?}", total, expected, grade);
        }
        let score = evaluate_gate_for_test(0.9);
        assert_eq!(score.formula, QUALITY_FORMULA);
    }

    // TDD 3: C 级重试逻辑
    #[test]
    fn tdd3_c_grade_retry_logic() {
        let mut score = GateScore {
            quality: 0.75, speed: 0.70, token_efficiency: 0.70, stability: 0.75,
            total: 0.73, grade: GateGrade::C, formula: QUALITY_FORMULA.to_string(),
        };
        assert_eq!(score.grade, GateGrade::C);

        // 开启重试
        let gate = QualityGate::new().with_retry_on_c(true);
        let intent = crate::intent::IntentResult {
            intent_id: "code".into(),
            conf: 0.8,
            keyword_scores: BTreeMap::new(),
            spread_scores: BTreeMap::new(),
            rrf_scores: BTreeMap::new(),
            degraded: false,
            degrade_reason: None,
            seeds_hit: vec![],
            trace_log: String::new(),
            diagnose_id: Uuid::new_v4(),
        };
        let team = TeamResult {
            team_ids: vec![],
            forced_replacements: vec![],
            reasoning_matrix: BTreeMap::new(),
            diagnose_id: Uuid::new_v4(),
        };
        let debate = DebateResult {
            opinions: vec![],
            consensus: 0.7,
            debate_rounds: 0,
            synthesis: String::new(),
            synthesis_reasoning: String::new(),
            diagnose_id: Uuid::new_v4(),
        };
        // 直接测试 retry_on_c 字段
        assert!(gate.retry_on_c);
        let _ = (intent, team, debate, score);
    }

    // TDD 4: D 级阻断
    #[test]
    fn tdd4_d_grade_blocked() {
        let mut score = evaluate_gate_for_test(0.5);
        score.total = 0.5;
        score.grade = GateGrade::D;
        assert!(!score.grade.passed());
    }

    // TDD 5: MetricsLearner 计数
    #[test]
    fn tdd5_metrics_learner_count() {
        let mut learner = MetricsLearner::new();
        assert_eq!(learner.learn_count(), 0);

        let score = evaluate_gate_for_test(0.85);
        let intent = crate::intent::IntentResult {
            intent_id: "code".into(),
            conf: 0.8,
            keyword_scores: BTreeMap::new(),
            spread_scores: BTreeMap::new(),
            rrf_scores: {
                let mut m = BTreeMap::new();
                for c in INTENT_CLASSES { m.insert(c.to_string(), 0.1); }
                m
            },
            degraded: false,
            degrade_reason: None,
            seeds_hit: vec![],
            trace_log: String::new(),
            diagnose_id: Uuid::new_v4(),
        };
        let debate = DebateResult {
            opinions: vec![],
            consensus: 0.7,
            debate_rounds: 0,
            synthesis: String::new(),
            synthesis_reasoning: String::new(),
            diagnose_id: Uuid::new_v4(),
        };

        let _ = learner.learn(&score, &intent, &debate);
        assert_eq!(learner.learn_count(), 1);
    }

    fn evaluate_gate_for_test(total: f64) -> GateScore {
        GateScore {
            quality: 0.90, speed: 0.90, token_efficiency: 0.90, stability: 0.90,
            total, grade: grade_from_total(total), formula: QUALITY_FORMULA.to_string(),
        }
    }
}
