// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识沉淀与反馈（Knowledge Learning & Feedback）
//!
//! 从每次联盟运行中学习并沉淀知识，持续优化：
//! - 维度增益：表现好的专家维度获得正向增益
//! - 类权重：7 类意图的权重自适应
//! - 反馈闭环：用户反馈回传，修正学习结果
//! - 知识图谱：沉淀成功案例和失败教训
//!
//! # 设计
//! - `KnowledgeLearner` — 知识学习器主结构体
//! - `FeedbackRecord` — 用户反馈记录
//! - `LearnedKnowledge` — 沉淀的知识快照

use crate::constants::INTENT_CLASSES;
use crate::debate::DebateResult;
use crate::gate::{GateScore, LearnResult};
use crate::intent::IntentResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ================== 反馈类型 ==================

/// 用户反馈类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    /// 点赞（结果有用）
    ThumbUp,
    /// 点踩（结果无用）
    ThumbDown,
    /// 修正（用户修改了结果）
    Correction,
    /// 报告问题
    ReportIssue,
}

impl FeedbackType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ThumbUp => "thumb_up",
            Self::ThumbDown => "thumb_down",
            Self::Correction => "correction",
            Self::ReportIssue => "report_issue",
        }
    }

    /// 反馈的增益系数（正向为正，负向为负）
    pub fn gain_factor(&self) -> f64 {
        match self {
            Self::ThumbUp => 0.05,
            Self::ThumbDown => -0.05,
            Self::Correction => 0.03,
            Self::ReportIssue => -0.03,
        }
    }
}

/// 用户反馈记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub feedback_id: String,
    pub trace_id: Uuid,
    pub feedback_type: FeedbackType,
    pub comment: Option<String>,
    pub session_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ================== 沉淀的知识 ==================

/// 沉淀的知识快照（可序列化，用于持久化）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearnedKnowledge {
    /// 维度增益累积：dimension -> gain (0..1)
    pub dimension_gains: BTreeMap<String, f64>,
    /// 7 类权重累积：class -> weight (归一化)
    pub class_weights: BTreeMap<String, f64>,
    /// 成功案例数
    pub success_count: u64,
    /// 失败案例数
    pub failure_count: u64,
    /// 总学习次数
    pub total_learnings: u64,
    /// 最后更新时间
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

impl LearnedKnowledge {
    pub fn new() -> Self {
        let mut class_weights = BTreeMap::new();
        for cls in INTENT_CLASSES {
            class_weights.insert(cls.to_string(), 1.0 / INTENT_CLASSES.len() as f64);
        }
        Self {
            dimension_gains: BTreeMap::new(),
            class_weights,
            success_count: 0,
            failure_count: 0,
            total_learnings: 0,
            last_updated: None,
        }
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failures_count();
        if total == 0 {
            return 0.0;
        }
        self.success_count as f64 / total as f64
    }

    fn failures_count(&self) -> u64 {
        self.failure_count
    }

    /// 归一化类权重
    pub fn normalized_class_weights(&self) -> BTreeMap<String, f64> {
        let sum: f64 = self.class_weights.values().sum::<f64>().max(1e-9);
        self.class_weights
            .iter()
            .map(|(k, v)| (k.clone(), v / sum))
            .collect()
    }
}

// ================== 知识学习器 ==================

/// 知识学习器
///
/// 负责从每次联盟运行结果和用户反馈中学习，沉淀知识。
/// 支持增量学习和知识快照导出/导入。
#[derive(Debug, Clone)]
pub struct KnowledgeLearner {
    knowledge: LearnedKnowledge,
    feedback_history: Vec<FeedbackRecord>,
}

impl Default for KnowledgeLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeLearner {
    pub fn new() -> Self {
        Self {
            knowledge: LearnedKnowledge::new(),
            feedback_history: Vec::new(),
        }
    }

    /// 从一次运行结果中学习
    pub fn learn_from_run(
        &mut self,
        score: &GateScore,
        intent: &IntentResult,
        debate: &DebateResult,
    ) -> LearnResult {
        self.knowledge.total_learnings += 1;

        // 学习维度增益
        let avg = if debate.opinions.is_empty() { 0.0 } else {
            debate.opinions.iter().map(|o| o.score).sum::<f64>() / debate.opinions.len() as f64
        };
        for op in &debate.opinions {
            let delta = (op.score - avg).max(0.0);
            let gain = if delta > 0.05 { 0.05 } else { delta };
            let entry = self.knowledge.dimension_gains.entry(op.dimension.clone()).or_insert(0.0);
            *entry = (*entry + gain * 0.1).min(1.0); // 增量学习，每次只更新 10%
        }

        // 学习类权重
        let sum_rrf: f64 = intent.rrf_scores.values().sum::<f64>().max(1e-9);
        for cls in INTENT_CLASSES {
            let key = cls.to_string();
            let raw = intent.rrf_scores.get(&key).copied().unwrap_or(0.0);
            let run_weight = raw / sum_rrf;
            let entry = self.knowledge.class_weights.entry(key).or_insert(1.0 / INTENT_CLASSES.len() as f64);
            // 指数移动平均（EMA）：保留 90% 旧值，10% 新值
            *entry = 0.9 * *entry + 0.1 * run_weight;
        }

        // 成功/失败计数
        if score.grade.passed() {
            self.knowledge.success_count += 1;
        } else {
            self.knowledge.failure_count += 1;
        }

        self.knowledge.last_updated = Some(chrono::Utc::now());

        // 返回 LearnResult
        crate::gate::learn_metrics(score, intent, debate)
    }

    /// 从用户反馈中学习
    pub fn learn_from_feedback(&mut self, feedback: FeedbackRecord) {
        let factor = feedback.feedback_type.gain_factor();

        // 根据反馈调整类权重（正向反馈增强，负向反馈减弱）
        // 这里简化处理：整体调整所有维度的增益
        for (_dim, gain) in self.knowledge.dimension_gains.iter_mut() {
            *gain = (*gain + factor * 0.1).clamp(0.0, 1.0);
        }

        match feedback.feedback_type {
            FeedbackType::ThumbUp | FeedbackType::Correction => {
                self.knowledge.success_count += 1;
            }
            FeedbackType::ThumbDown | FeedbackType::ReportIssue => {
                self.knowledge.failure_count += 1;
            }
        }

        self.feedback_history.push(feedback);
        self.knowledge.last_updated = Some(chrono::Utc::now());
    }

    /// 获取当前知识快照
    pub fn knowledge(&self) -> &LearnedKnowledge {
        &self.knowledge
    }

    /// 导入知识快照
    pub fn import_knowledge(&mut self, knowledge: LearnedKnowledge) {
        self.knowledge = knowledge;
    }

    /// 导出知识快照
    pub fn export_knowledge(&self) -> LearnedKnowledge {
        self.knowledge.clone()
    }

    /// 获取反馈历史
    pub fn feedback_history(&self) -> &[FeedbackRecord] {
        &self.feedback_history
    }

    /// 总学习次数
    pub fn total_learnings(&self) -> u64 {
        self.knowledge.total_learnings
    }

    /// 重置学习器
    pub fn reset(&mut self) {
        self.knowledge = LearnedKnowledge::new();
        self.feedback_history.clear();
    }
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debate::ExpertOpinion;
    use crate::gate::GateGrade;
    use std::collections::BTreeMap;

    fn make_gate_score(total: f64) -> GateScore {
        let grade = if total >= 0.90 { GateGrade::A }
        else if total >= 0.80 { GateGrade::B }
        else if total >= 0.70 { GateGrade::C }
        else { GateGrade::D };
        GateScore {
            quality: 0.85,
            speed: 0.85,
            token_efficiency: 0.85,
            stability: 0.85,
            total,
            grade,
            formula: crate::constants::QUALITY_FORMULA.to_string(),
        }
    }

    fn make_intent(winner: &str) -> IntentResult {
        let mut rrf = BTreeMap::new();
        for c in INTENT_CLASSES {
            rrf.insert(c.to_string(), if c == winner { 0.8 } else { 0.05 });
        }
        IntentResult {
            intent_id: winner.into(),
            conf: 0.8,
            keyword_scores: BTreeMap::new(),
            spread_scores: BTreeMap::new(),
            rrf_scores: rrf,
            degraded: false,
            degrade_reason: None,
            seeds_hit: vec![],
            trace_log: String::new(),
            diagnose_id: Uuid::new_v4(),
        }
    }

    fn make_debate() -> DebateResult {
        DebateResult {
            opinions: vec![
                ExpertOpinion {
                    expert_id: "security".into(),
                    dimension: "security".into(),
                    answer: "test".into(),
                    score: 0.90,
                    confidence: 0.85,
                    latency_ms: 100,
                    timed_out: false,
                    tokens_approx: 100,
                },
                ExpertOpinion {
                    expert_id: "performance".into(),
                    dimension: "performance".into(),
                    answer: "test".into(),
                    score: 0.75,
                    confidence: 0.70,
                    latency_ms: 150,
                    timed_out: false,
                    tokens_approx: 120,
                },
            ],
            consensus: 0.75,
            debate_rounds: 1,
            synthesis: "test".into(),
            synthesis_reasoning: "test".into(),
            diagnose_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn knowledge_learner_initial_state() {
        let learner = KnowledgeLearner::new();
        assert_eq!(learner.total_learnings(), 0);
        assert_eq!(learner.knowledge().success_count, 0);
        assert_eq!(learner.feedback_history().len(), 0);
        // 类权重初始均匀分布
        let weights = learner.knowledge().normalized_class_weights();
        assert_eq!(weights.len(), 7);
    }

    #[test]
    fn learn_from_passed_run_increments_success() {
        let mut learner = KnowledgeLearner::new();
        let score = make_gate_score(0.95);
        let intent = make_intent("code");
        let debate = make_debate();

        let result = learner.learn_from_run(&score, &intent, &debate);
        assert!(!result.summary.is_empty());
        assert_eq!(learner.total_learnings(), 1);
        assert_eq!(learner.knowledge().success_count, 1);
        assert_eq!(learner.knowledge().failure_count, 0);
    }

    #[test]
    fn learn_from_failed_run_increments_failure() {
        let mut learner = KnowledgeLearner::new();
        let score = make_gate_score(0.5);
        let intent = make_intent("code");
        let debate = make_debate();

        let _ = learner.learn_from_run(&score, &intent, &debate);
        assert_eq!(learner.knowledge().failure_count, 1);
        assert_eq!(learner.knowledge().success_count, 0);
    }

    #[test]
    fn learn_from_thumb_up_feedback() {
        let mut learner = KnowledgeLearner::new();
        let feedback = FeedbackRecord {
            feedback_id: "fb-1".into(),
            trace_id: Uuid::new_v4(),
            feedback_type: FeedbackType::ThumbUp,
            comment: Some("很有用".into()),
            session_id: Some("sess-1".into()),
            created_at: chrono::Utc::now(),
        };
        learner.learn_from_feedback(feedback);
        assert_eq!(learner.feedback_history().len(), 1);
        assert_eq!(learner.knowledge().success_count, 1);
    }

    #[test]
    fn learn_from_thumb_down_feedback() {
        let mut learner = KnowledgeLearner::new();
        let feedback = FeedbackRecord {
            feedback_id: "fb-2".into(),
            trace_id: Uuid::new_v4(),
            feedback_type: FeedbackType::ThumbDown,
            comment: None,
            session_id: None,
            created_at: chrono::Utc::now(),
        };
        learner.learn_from_feedback(feedback);
        assert_eq!(learner.knowledge().failure_count, 1);
    }

    #[test]
    fn dimension_gains_accumulate() {
        let mut learner = KnowledgeLearner::new();
        let score = make_gate_score(0.85);
        let intent = make_intent("code");
        let debate = make_debate();

        let _ = learner.learn_from_run(&score, &intent, &debate);
        let gains1 = learner.knowledge().dimension_gains.clone();

        let _ = learner.learn_from_run(&score, &intent, &debate);
        let gains2 = learner.knowledge().dimension_gains.clone();

        // 第二次学习后增益应该有所增加（或保持不变）
        for (dim, g2) in &gains2 {
            if let Some(g1) = gains1.get(dim) {
                assert!(g2 >= g1, "维度增益不应减少: {} {} < {}", dim, g2, g1);
            }
        }
    }

    #[test]
    fn export_import_knowledge() {
        let mut learner = KnowledgeLearner::new();
        let score = make_gate_score(0.9);
        let intent = make_intent("code");
        let debate = make_debate();
        let _ = learner.learn_from_run(&score, &intent, &debate);

        let exported = learner.export_knowledge();
        assert_eq!(exported.total_learnings, 1);
        assert_eq!(exported.success_count, 1);

        let mut learner2 = KnowledgeLearner::new();
        learner2.import_knowledge(exported);
        assert_eq!(learner2.total_learnings(), 1);
    }

    #[test]
    fn reset_clears_everything() {
        let mut learner = KnowledgeLearner::new();
        let score = make_gate_score(0.9);
        let intent = make_intent("code");
        let debate = make_debate();
        let _ = learner.learn_from_run(&score, &intent, &debate);

        learner.reset();
        assert_eq!(learner.total_learnings(), 0);
        assert_eq!(learner.knowledge().success_count, 0);
        assert!(learner.feedback_history().is_empty());
    }

    #[test]
    fn feedback_type_gain_factors() {
        assert!(FeedbackType::ThumbUp.gain_factor() > 0.0);
        assert!(FeedbackType::ThumbDown.gain_factor() < 0.0);
        assert!(FeedbackType::Correction.gain_factor() > 0.0);
        assert!(FeedbackType::ReportIssue.gain_factor() < 0.0);
    }

    #[test]
    fn learned_knowledge_success_rate() {
        let mut k = LearnedKnowledge::new();
        assert_eq!(k.success_rate(), 0.0);

        k.success_count = 80;
        k.failure_count = 20;
        assert!((k.success_rate() - 0.8).abs() < 0.01);
    }
}
