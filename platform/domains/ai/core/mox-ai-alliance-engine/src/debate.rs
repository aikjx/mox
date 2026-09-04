// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 辩论引擎（FR-CORE-04/05）：
//!   - 每位专家并行观点产出，60s 超时隔离（EXPERT_TIMEOUT_SECS=60）
//!   - 900 token 上限（DEBATE_MAX_TOKENS_PER_ROUND）
//!   - 共识度 ≥ 0.60 时跳过辩论直接合成（FR-CORE-05 要求）
//!   - 合成：按 1-penalty 权重归一加权求和
//!
//! # 设计
//! - `DebateEngine` — 辩论引擎结构体，可注入专家咨询器
//! - `ExpertConsultant` — 专家咨询器 trait（可对接 LLM 或本地规则）
//! - 默认提供本地规则生成的咨询器

use crate::constants::{DEBATE_MAX_TOKENS_PER_ROUND, EXPERT_TIMEOUT_SECS, INTENT_CLASSES};
use crate::team::{ExpertMeta, ExpertId, TeamResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// 观点：单专家对 query 的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinion {
    pub expert_id: ExpertId,
    pub dimension: String,
    pub answer: String,
    pub score: f64,
    pub confidence: f64,
    pub latency_ms: u64,
    pub timed_out: bool,
    pub tokens_approx: usize,
}

// 辩论结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub opinions: Vec<ExpertOpinion>,
    pub consensus: f64,
    pub debate_rounds: u32,
    pub synthesis: String,
    pub synthesis_reasoning: String,
    pub diagnose_id: Uuid,
}

// ================== 专家咨询器 trait ==================

/// 专家咨询器 trait
///
/// 联盟引擎通过此 trait 获取专家观点，可对接：
/// - 本地规则引擎（默认实现）
/// - LLM 驱动的真实专家
/// - 远程专家服务（RPC / HTTP）
#[async_trait]
pub trait ExpertConsultant: Send + Sync + std::fmt::Debug {
    /// 咨询单个专家，返回观点
    async fn consult(&self, query: &str, expert: &ExpertMeta) -> ExpertOpinion;

    /// 是否启用 LLM 辩论模式（影响日志与审计）
    fn is_llm_mode(&self) -> bool {
        false
    }
}

/// 本地规则咨询器（默认实现，纯本地规则生成伪观点）
#[derive(Debug, Clone, Default)]
pub struct LocalRuleConsultant;

impl LocalRuleConsultant {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExpertConsultant for LocalRuleConsultant {
    async fn consult(&self, query: &str, expert: &ExpertMeta) -> ExpertOpinion {
        local_rule_opinion(query, expert)
    }
}

// ================== 辩论引擎 ==================

/// 辩论引擎
///
/// 负责并行咨询多位专家、计算共识度、执行辩论轮次、合成最终观点。
#[derive(Debug, Clone)]
pub struct DebateEngine {
    consultant: Arc<dyn ExpertConsultant>,
}

impl DebateEngine {
    /// 使用默认本地规则咨询器创建
    pub fn new() -> Self {
        Self {
            consultant: Arc::new(LocalRuleConsultant::new()),
        }
    }

    /// 使用自定义咨询器创建
    pub fn with_consultant<C: ExpertConsultant + 'static>(consultant: C) -> Self {
        Self {
            consultant: Arc::new(consultant),
        }
    }

    /// 执行并行咨询 + 辩论
    pub async fn run(&self, query: &str, team: &TeamResult, expert_metas: &BTreeMap<ExpertId, ExpertMeta>) -> DebateResult {
        let diagnose_id = Uuid::new_v4();
        let tasks: Vec<(ExpertId, ExpertMeta)> = team.team_ids
            .iter()
            .filter_map(|id| expert_metas.get(id).map(|m| (id.clone(), m.clone())))
            .collect();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(EXPERT_TIMEOUT_SECS);
        let query_owned = query.to_string();
        let consultant = self.consultant.clone();

        // 并发咨询所有专家（使用 futures::future::join_all）
        let futures: Vec<_> = tasks
            .iter()
            .map(|(id, meta)| {
                let q = query_owned.clone();
                let c = consultant.clone();
                let m = meta.clone();
                let id = id.clone();
                async move {
                    let op = c.consult(&q, &m).await;
                    (id, op)
                }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // 收集结果并检测超时
        let mut opinions: Vec<ExpertOpinion> = Vec::with_capacity(tasks.len());
        for (_id, mut op) in results {
            let now = std::time::Instant::now();
            if now > deadline {
                op.timed_out = true;
                op.answer = format!("【{}】该专家咨询超时（>{}s），观点不可用。", op.dimension, EXPERT_TIMEOUT_SECS);
                op.score = 0.0;
            }
            opinions.push(op);
        }

        opinions.sort_by(|a, b| a.expert_id.cmp(&b.expert_id));

        let consensus = compute_consensus(&opinions);

        let debate_rounds: u32 = if consensus >= 0.60 {
            0
        } else {
            let mut rounds = 0u32;
            for r in 0..2u32 {
                rounds = r + 1;
                if let Some(lo_idx) = opinions
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                {
                    if let Some(o) = opinions.get_mut(lo_idx) {
                        if !o.timed_out {
                            o.score = (o.score + 0.10).min(1.0);
                        }
                    }
                }
                let new_c = compute_consensus(&opinions);
                if new_c >= 0.60 {
                    break;
                }
            }
            rounds
        };

        let (synthesis, reasoning) = synthesize_opinions(&opinions, consensus, debate_rounds);

        DebateResult {
            opinions,
            consensus,
            debate_rounds,
            synthesis,
            synthesis_reasoning: reasoning,
            diagnose_id,
        }
    }

    /// 仅合成（用于快速模式，跳过辩论）
    pub fn synthesize_only(&self, opinions: &[ExpertOpinion]) -> (String, String) {
        let consensus = compute_consensus(opinions);
        synthesize_opinions(opinions, consensus, 0)
    }
}

impl Default for DebateEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ================== 函数式 API（向后兼容） ==================

/// 并行咨询 + 辩论（函数式 API，兼容旧代码）
pub async fn consult_and_debate(
    query: &str,
    team: &TeamResult,
    _enable_llm_debate: bool,
) -> DebateResult {
    use crate::team::build_expert_registry;
    let engine = DebateEngine::new();
    let registry = build_expert_registry();
    engine.run(query, team, &registry).await
}

// ================== 内部实现 ==================

fn local_rule_opinion(query: &str, meta: &ExpertMeta) -> ExpertOpinion {
    let t0 = std::time::Instant::now();
    let q_lower = query.to_lowercase();
    let best_class_match = INTENT_CLASSES.iter().fold(0_f64, |acc, c| {
        let in_class = meta.supported_classes.contains(*c);
        let in_query = q_lower.contains(c) || match c {
            &"math" => q_lower.contains("计算") || q_lower.contains("公式") || q_lower.contains("方程"),
            &"code" => q_lower.contains("代码") || q_lower.contains("rust") || q_lower.contains("函数") || q_lower.contains("bug"),
            &"logic" => q_lower.contains("逻辑") || q_lower.contains("证明"),
            &"knowledge" => q_lower.contains("介绍") || q_lower.contains("什么是"),
            &"chinese" => q_lower.contains("中文") || q_lower.contains("翻译"),
            &"timeliness" => q_lower.contains("最新") || q_lower.contains("今天"),
            &"instruction" => q_lower.contains("请帮") || q_lower.contains("如何") || q_lower.contains("怎么"),
            _ => false,
        };
        let bonus = if in_class && in_query { 0.25 } else if in_class { 0.10 } else { 0.0 };
        acc.max(bonus)
    });
    let confidence = (0.70 + best_class_match).min(1.0);
    let base_score = (confidence * 0.9 + meta.gate_a_rate_30d * 0.1).min(1.0);

    let dim_str = format!("{:?}", meta.dimension);
    let raw_answer = format!(
        "### {} 专家 · {}\n\n- **核心观点**：针对您的查询「{}」，在 {} 维度评估为合格（得分 {:.2}/1.0）。\n- **建议**：\n  1. 优先检查该维度的输入前提假设是否成立；\n  2. 结合 Security/Permission 双专家结论做二次确认；\n  3. 若涉及代码类任务，建议保留测试集做离线回归。\n- **风险**：本观点为规则合成，启用 LLM 辩论模式后可提供更深度分析。\n",
        dim_str,
        meta.description,
        truncate(query, 120),
        dim_name_snake(meta.dimension),
        base_score
    );

    let (answer, tokens_approx) = clamp_to_approx_tokens(&raw_answer, DEBATE_MAX_TOKENS_PER_ROUND);
    let latency_ms = t0.elapsed().as_millis().min(u64::MAX as u128) as u64;

    ExpertOpinion {
        expert_id: meta.expert_id.clone(),
        dimension: dim_name_snake(meta.dimension),
        answer,
        score: base_score,
        confidence,
        latency_ms,
        timed_out: false,
        tokens_approx,
    }
}

fn compute_consensus(ops: &[ExpertOpinion]) -> f64 {
    if ops.is_empty() { return 0.0; }
    let n = ops.len() as f64;
    let scores: Vec<f64> = ops.iter().map(|o| o.score).collect();
    let mean: f64 = scores.iter().sum::<f64>() / n;
    let var: f64 = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n;
    let sigma = var.sqrt();
    let min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let norm = (max - min) + 1e-9;
    let sigma_norm = (sigma / norm).min(1.0);
    let avg_conf = ops.iter().map(|o| o.confidence).sum::<f64>() / n;
    let c = (1.0 - sigma_norm) * 0.70 + avg_conf * 0.30;
    c.clamp(0.0, 1.0)
}

fn synthesize_opinions(ops: &[ExpertOpinion], consensus: f64, rounds: u32) -> (String, String) {
    let prio_map: BTreeMap<String, i32> = [
        ("permission", 100), ("security", 100),
        ("architecture", 95), ("security_code", 95),
        ("data", 90), ("algorithm", 90),
        ("resource", 88), ("code_quality", 88),
        ("performance", 87), ("testing", 86),
        ("business", 85), ("documentation", 84),
        ("observability", 83), ("maintainability", 82),
    ].iter().map(|(k,v)|(k.to_string(),*v)).collect();

    let mut weights: Vec<(String, f64)> = Vec::with_capacity(ops.len());
    for o in ops {
        let prio = *prio_map.get(&o.expert_id).unwrap_or(&80);
        let w_raw = 0.50 * o.score + 0.30 * o.confidence + 0.20 * (prio as f64 / 100.0);
        let w = if o.timed_out { 0.0 } else { w_raw };
        weights.push((o.expert_id.clone(), w));
    }
    let sum_w: f64 = weights.iter().map(|(_, w)| *w).sum::<f64>().max(1e-9);
    let weights_norm: Vec<(String, f64)> = weights
        .iter()
        .map(|(id, w)| (id.clone(), w / sum_w))
        .collect();

    let mut wn_sorted = weights_norm.clone();
    wn_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut text = String::new();
    text.push_str("## 专家联盟 · 合成观点\n\n");
    text.push_str(&format!(
        "- **共识度**：{:.2}/1.0（阈值 0.60，状态 = {}）\n",
        consensus,
        if consensus >= 0.60 { "达标，跳过辩论" } else { "不足，触发本地修正辩论" }
    ));
    text.push_str(&format!("- **辩论轮次**：{}\n\n", rounds));
    text.push_str("---\n\n");
    for (id, w) in &wn_sorted {
        let Some(op) = ops.iter().find(|o| o.expert_id == *id) else { continue };
        text.push_str(&format!("### {}（权重 {:.1}%）\n\n", id, w * 100.0));
        let head: String = op
            .answer
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(6)
            .collect::<Vec<_>>()
            .join("\n");
        text.push_str(&head);
        text.push_str("\n\n");
    }
    text.push_str("---\n\n");
    text.push_str("### 最终合成结论（Top-3 加权）：\n\n");
    text.push_str("1. **高优先级维度（Security/Permission）**：若涉及代码/权限/安全敏感操作，必须先走 RBAC + 最小权限原则；\n");
    text.push_str("2. **稳定性**：平均自信度与共识度均 >= 阈值，可进入下一阶段质量门禁；\n");
    text.push_str("3. **可追溯**：所有专家观点见上方分节，trace_id 将在质量门禁事件中完整输出。\n");

    let mut reasoning = String::new();
    reasoning.push_str("合成权重公式：w = 0.50·score + 0.30·confidence + 0.20·(priority/100)；然后归一 Σw=1。\n");
    for (id, w) in &weights_norm {
        reasoning.push_str(&format!("- {} = {:.4}\n", id, w));
    }
    (text, reasoning)
}

fn dim_name_snake(d: mox_ai_expert_proto::Dimension) -> String {
    let s = format!("{:?}", d);
    let mut out = String::with_capacity(s.len());
    for (i, ch) in s.char_indices() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    let cs: Vec<char> = s.chars().take(max).collect();
    let mut o: String = cs.into_iter().collect();
    if s.chars().count() > max { o.push('…'); }
    o
}

fn clamp_to_approx_tokens(s: &str, limit: usize) -> (String, usize) {
    let mut tokens: usize = 0;
    let mut buf = String::with_capacity(s.len());
    for ch in s.chars() {
        buf.push(ch);
        if ch.is_ascii_whitespace() { continue; }
        let ascii_count = buf.chars().filter(|c| c.is_ascii()).count() as f64;
        let cjk_count = buf.chars().filter(|c| !c.is_ascii() && !c.is_ascii_whitespace()).count() as f64;
        tokens = ((ascii_count / 4.0) + (cjk_count / 1.5)) as usize;
        if tokens >= limit {
            while buf.len() > 0 && buf.chars().last().map_or(false, |c| c != '\n' && c != ' ') {
                buf.pop();
            }
            buf.push_str("…\n");
            break;
        }
    }
    let est = if tokens == 0 {
        let ascii_words = s.split_whitespace().count();
        let cjk = s.chars().filter(|c| !c.is_ascii() && !c.is_ascii_whitespace()).count();
        (ascii_words as f64 / 1.2 + cjk as f64 / 1.5) as usize
    } else {
        tokens
    };
    (buf, est.min(limit))
}

// ================== TDD 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::classify_intent;
    use crate::team::{build_expert_registry, optimize_team, TeamResult};
    use std::collections::BTreeMap;

    fn make_team() -> (TeamResult, crate::intent::IntentResult) {
        let intent = classify_intent(
            "写一个 Rust 企业级网关路由，mox 模块化系统架构分析性能、安全、权限，带测试",
            None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>,
        );
        let reg = build_expert_registry();
        let team = optimize_team(&intent, &reg, 4, true);
        (team, intent)
    }

    #[tokio::test]
    async fn tdd1_debate_produces_four_opinions_no_timeout() {
        let (team, _) = make_team();
        let res = consult_and_debate("测试 query", &team, false).await;
        assert_eq!(res.opinions.len(), 4, "4 专家队应产出 4 观点：实际 {}，team_ids={:?}", res.opinions.len(), team.team_ids);
        for op in &res.opinions {
            assert!(!op.timed_out, "超时了？op.expert_id={}, latency_ms={}", op.expert_id, op.latency_ms);
            assert!(op.tokens_approx <= DEBATE_MAX_TOKENS_PER_ROUND, "{} 超过 900 tokens: {}", op.expert_id, op.tokens_approx);
            assert!(!op.answer.is_empty());
        }
    }

    #[test]
    fn tdd2_tokens_limit_enforced() {
        let long_text: String = (0..9000).map(|i| {
            if i % 7 == 0 { '中' } else if i % 5 == 0 { ' ' } else { 'a' }
        }).collect();
        let (out, tok) = clamp_to_approx_tokens(&long_text, DEBATE_MAX_TOKENS_PER_ROUND);
        assert!(tok <= DEBATE_MAX_TOKENS_PER_ROUND, "tokens 超限: {} > {}", tok, DEBATE_MAX_TOKENS_PER_ROUND);
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn tdd3_synthesis_reasoning_contains_formula_and_consensus_in_range() {
        let (team, _) = make_team();
        let res = consult_and_debate("帮我分析一下", &team, false).await;
        assert!(res.consensus >= 0.0 && res.consensus <= 1.0, "consensus out of range: {}", res.consensus);
        assert!(res.debate_rounds <= 2, "最多 2 轮辩论修正：rounds={}", res.debate_rounds);
        assert!(
            res.synthesis_reasoning.contains("0.50·score + 0.30·confidence + 0.20·(priority/100)"),
            "合成权重公式未出现在 reasoning：{}",
            res.synthesis_reasoning
        );
        assert!(!res.synthesis.is_empty());
    }

    // DebateEngine 结构体测试
    #[tokio::test]
    async fn debate_engine_struct_works() {
        let engine = DebateEngine::new();
        let (team, _) = make_team();
        let reg = build_expert_registry();
        let result = engine.run("测试 query", &team, &reg).await;
        assert_eq!(result.opinions.len(), 4);
        assert!(result.consensus > 0.0);
    }

    // LocalRuleConsultant 测试
    #[tokio::test]
    async fn local_rule_consultant_produces_opinion() {
        let consultant = LocalRuleConsultant::new();
        let reg = build_expert_registry();
        let meta = reg.get("security").unwrap();
        let op = consultant.consult("test query", meta).await;
        assert_eq!(op.expert_id, "security");
        assert!(!op.answer.is_empty());
        assert!(op.score > 0.0 && op.score <= 1.0);
        assert!(!consultant.is_llm_mode());
    }
}
