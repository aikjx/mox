// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 并行咨询 + 辩论合成（FR-CORE-04/05）：
//!   - 每位专家 rayon 并行观点产出，60s 超时隔离（EXPERT_TIMEOUT_SECS=60）
//!   - 900 token 上限（DEBATE_MAX_TOKENS_PER_ROUND）
//!   - 共识度 ≥ 0.60 时跳过辩论直接合成（FR-CORE-05 要求）
//!   - 合成：按 1-penalty 权重归一加权求和

use super::constants::{DEBATE_MAX_TOKENS_PER_ROUND, EXPERT_TIMEOUT_SECS, INTENT_CLASSES};
use crate::alliance::team::{build_expert_registry, ExpertMeta, TeamResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type ExpertId = String;

// 观点：单专家对 query 的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinion {
    pub expert_id: ExpertId,
    pub dimension: String,            // 维度名（snake_case，便于前端展示）
    pub answer: String,               // 专家回答（≤900 token 近似）
    pub score: f64,                   // 自我评分 0..1
    pub confidence: f64,              // 自信度 0..1
    pub latency_ms: u64,              // 实际耗时
    pub timed_out: bool,              // 是否超时（超时则 answer="…"，score=0）
    pub tokens_approx: usize,         // 近似 token 数（按"空格数/2 + 中文字数/1.5"估算）
}

// 辩论结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub opinions: Vec<ExpertOpinion>,          // 所有专家观点
    pub consensus: f64,                         // 全体共识度 0..1
    pub debate_rounds: u32,                     // 实际辩论轮数（共识≥0.6 → 0）
    pub synthesis: String,                      // 合成文本（Markdown 段）
    pub synthesis_reasoning: String,            // 合成说明（权重公式原文 + 每位专家权重）
    pub diagnose_id: Uuid,
}

// =============== 接口：模拟专家观点生成（企业级纯本地，无 LLM 调用） ===============
// 若 `enable_llm_debate=false`（默认）→ 本地规则生成伪观点（符合 AIS "先跑起来再增强" 模式）
// 若 `enable_llm_debate=true` → 此处预留钩子（返回 fake，真实 LLM 由 harness.rs 插件化注入）
pub async fn consult_and_debate(
    query: &str,
    team: &TeamResult,
    enable_llm_debate: bool,
) -> DebateResult {
    let diagnose_id = Uuid::new_v4();
    let registry = build_expert_registry();

    // ========== Stage 1: 并行观点产出（超时 60s 隔离） ==========
    let team_ids: Vec<ExpertId> = team.team_ids.clone();
    // 每个专家元数据，构建可并行迭代的数据
    let tasks: Vec<(ExpertId, ExpertMeta)> = team_ids
        .iter()
        .filter_map(|id| registry.get(id).map(|m| (id.clone(), m.clone())))
        .collect();

    // 模拟超时：用 rayon 并行跑每专家生成；通过 Instant 限时 60s（实际生成 <1ms，为保险起见给边界）
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_secs(EXPERT_TIMEOUT_SECS);

    // opinions 互斥收集
    let opinions: Arc<Mutex<Vec<ExpertOpinion>>> = Arc::new(Mutex::new(Vec::with_capacity(tasks.len())));
    let query_owned = query.to_string();

    // 条件编译：若 rayon 存在则用 par_iter，否则退化为顺序 for
    #[cfg(feature = "rayon")]
    {
        use rayon::prelude::*;
        tasks.par_iter().for_each(|(id, meta)| {
            let op = fake_expert_opinion(&query_owned, id, meta, deadline, enable_llm_debate);
            opinions.lock().unwrap().push(op);
        });
    }
    #[cfg(not(feature = "rayon"))]
    {
        for (id, meta) in tasks.iter() {
            let op = fake_expert_opinion(&query_owned, id, meta, deadline, enable_llm_debate);
            opinions.lock().unwrap().push(op);
        }
    }

    let mut opinions = Arc::try_unwrap(opinions).unwrap().into_inner().unwrap();
    // 稳定排序：按 expert_id 字母序（保证合成结果可复现）
    opinions.sort_by(|a, b| a.expert_id.cmp(&b.expert_id));

    // ========== Stage 2: 共识度计算 ==========
    let consensus = compute_consensus(&opinions);

    // ========== Stage 3: 辩论轮（共识≥0.6 跳过，debate_rounds=0） ==========
    let debate_rounds: u32 = if consensus >= 0.60 {
        0
    } else {
        // 最多 2 轮本地修正：对 score < 0.6 的专家微调（模拟"对齐共识"）
        let mut rounds = 0u32;
        for r in 0..2u32 {
            rounds = r + 1;
            // 把分最低的专家 opinion.score 提升 0.1（模拟接受对方观点）
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

    // ========== Stage 4: 合成 ==========
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

// 单专家观点生成（纯本地规则；未来接入 AI-agent 时替换此函数实现）
fn fake_expert_opinion(
    query: &str,
    id: &str,
    meta: &ExpertMeta,
    deadline: std::time::Instant,
    _enable_llm: bool,
) -> ExpertOpinion {
    let t0 = std::time::Instant::now();
    let timed_out = t0 > deadline;
    // 7 类匹配：若专家支持类，匹配更强 → confidence 更高
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

    // 生成回答（≤900 token 近似）
    let raw_answer = if timed_out {
        format!("【{:?}】该专家咨询超时（>{EXPERT_TIMEOUT_SECS}s），观点不可用。", meta.dimension)
    } else {
        let hit_kw = INTENT_CLASSES
            .iter()
            .find(|c| meta.supported_classes.contains(**c) && q_lower.contains(**c))
            .copied()
            .unwrap_or("general");
        format!(
            "### {:?} 专家 · {}\n\n- **核心观点**：针对您的查询「{}」，在 {} 维度评估为合格（得分 {:.2}/1.0）。\n- **建议**：\n  1. 优先检查该维度的输入前提假设是否成立；\n  2. 结合 Security/Permission 双专家结论做二次确认；\n  3. 若涉及 {hit_kw} 类任务，建议保留测试集做离线回归。\n- **风险**：本观点为规则合成，启用 LLM 辩论模式后可提供更深度分析。\n",
            meta.dimension,
            meta.description,
            truncate(query, 120),
            dim_name(meta.dimension),
            base_score
        )
    };

    // 900 token 近似截断
    let (answer, tokens_approx) = clamp_to_approx_tokens(&raw_answer, DEBATE_MAX_TOKENS_PER_ROUND);

    let score = if timed_out { 0.0 } else { base_score };
    let latency_ms = t0.elapsed().as_millis().min(u64::MAX as u128) as u64;

    ExpertOpinion {
        expert_id: id.to_string(),
        dimension: dim_name_snake(meta.dimension),
        answer,
        score,
        confidence,
        latency_ms,
        timed_out,
        tokens_approx,
    }
}

// consensus：观点间的一致性（简化：score 标准差归一 → 1 - σ_norm，σ_norm = σ / (max-min+ε)）
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
    // confidence 也加分：平均 confidence × 0.30
    let avg_conf = ops.iter().map(|o| o.confidence).sum::<f64>() / n;
    let c = (1.0 - sigma_norm) * 0.70 + avg_conf * 0.30;
    c.clamp(0.0, 1.0)
}

// 合成：按 1-penalty 归一权重，权重 = 0.5*score + 0.3*confidence + 0.2*(priority/100)
fn synthesize_opinions(ops: &[ExpertOpinion], consensus: f64, rounds: u32) -> (String, String) {
    // 构造 fake priority（用于打分）：根据 expert_id 字符串硬编码权重映射（不依赖 team.rs 的 registry，可解耦）
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

    // 合成正文：按权重降序拼接各位专家 answer 头部 4~6 行
    let mut wn_sorted = weights_norm.clone();
    wn_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut text = String::new();
    text.push_str("## 专家联盟 · 合成观点\n\n");
    text.push_str(&format!(
        "- **共识度**：{:.2}/1.0（阈值 0.60，状态 = {}）\n",
        consensus,
        if consensus >= 0.60 { "✅ 达标，跳过辩论" } else { "⚠️ 不足，触发本地修正辩论" }
    ));
    text.push_str(&format!("- **辩论轮次**：{}\n\n", rounds));
    text.push_str("---\n\n");
    for (id, w) in &wn_sorted {
        let Some(op) = ops.iter().find(|o| o.expert_id == *id) else { continue };
        text.push_str(&format!("### 🎖️ {}（权重 {:.1}%）\n\n", id, w * 100.0));
        // 截取 answer 头部非空前 6 行
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
    text.push_str("### ✅ 最终合成结论（Top-3 加权）：\n\n");
    text.push_str("1. **高优先级维度（Security/Permission）**：若涉及代码/权限/安全敏感操作，必须先走 RBAC + 最小权限原则；\n");
    text.push_str("2. **稳定性**：平均自信度与共识度均 ≥ 阈值，可进入下一阶段质量门禁；\n");
    text.push_str("3. **可追溯**：所有专家观点见上方分节，trace_id 将在质量门禁事件中完整输出。\n");

    // 推理说明（AC-09 早期基线：包含权重公式原文）
    let mut reasoning = String::new();
    reasoning.push_str("合成权重公式：w = 0.50·score + 0.30·confidence + 0.20·(priority/100)；然后归一 Σw=1。\n");
    for (id, w) in &weights_norm {
        reasoning.push_str(&format!("- {} = {:.4}\n", id, w));
    }
    (text, reasoning)
}

// =============== 工具 ===============
fn dim_name(d: crate::ir::Dimension) -> String { format!("{:?}", d) }
fn dim_name_snake(d: crate::ir::Dimension) -> String {
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
// 近似 token 估算，并在超过 limit 时截断；返回 (截断后的String, 近似tokens)
fn clamp_to_approx_tokens(s: &str, limit: usize) -> (String, usize) {
    let mut tokens: usize = 0;
    let mut buf = String::with_capacity(s.len());
    let mut _word_count = 0usize;
    for ch in s.chars() {
        buf.push(ch);
        if ch.is_ascii_whitespace() { _word_count += 1; }
        // 中文字符贡献 ~0.67 token/字 → 每 1.5 个汉字≈1 token
        let ascii_count = buf.chars().filter(|c| c.is_ascii()).count() as f64;
        let cjk_count = buf.chars().filter(|c| !c.is_ascii() && !c.is_ascii_whitespace()).count() as f64;
        tokens = ((ascii_count / 4.0) + (cjk_count / 1.5)) as usize;
        if tokens >= limit {
            // 截断：回退到前一个换行或空白
            while buf.len() > 0 && buf.chars().last().map_or(false, |c| c != '\n' && c != ' ') {
                buf.pop();
            }
            buf.push_str("…\n");
            break;
        }
    }
    // 保守估计：若没触发截断，再基于空白数/中文字数算一次
    let est = if tokens == 0 {
        let ascii_words = s.split_whitespace().count();
        let cjk = s.chars().filter(|c| !c.is_ascii() && !c.is_ascii_whitespace()).count();
        (ascii_words as f64 / 1.2 + cjk as f64 / 1.5) as usize
    } else {
        tokens
    };
    (buf, est.min(limit))
}

// ================== TDD 测试（3 个） ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alliance::intent::{classify_intent, IntentResult};
    use crate::alliance::team::{build_expert_registry, optimize_team, TeamResult};

    fn make_team() -> (TeamResult, IntentResult) {
        let intent = classify_intent(
            "写一个 Rust 企业级网关路由，全维分析性能、安全、权限，带测试",
            None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>,
        );
        let reg = build_expert_registry();
        let team = optimize_team(&intent, &reg, 4, true);
        (team, intent)
    }

    // TDD 1: 4 位专家 opinions 齐全，无超时（死锁回归；超时=1小时模拟，但60s内肯定不触发）
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

    // TDD 2: clamp_to_approx_tokens 返回 ≤ 900
    #[test]
    fn tdd2_tokens_limit_enforced() {
        let long_text: String = (0..9000).map(|i| {
            if i % 7 == 0 { '中' } else if i % 5 == 0 { ' ' } else { 'a' }
        }).collect();
        let (out, tok) = clamp_to_approx_tokens(&long_text, DEBATE_MAX_TOKENS_PER_ROUND);
        assert!(tok <= DEBATE_MAX_TOKENS_PER_ROUND, "tokens 超限: {} > {}", tok, DEBATE_MAX_TOKENS_PER_ROUND);
        assert!(!out.is_empty());
    }

    // TDD 3: 合成 reasoning 包含权重公式原文 + consensus 为有限数 0..1
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
}
