// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! napi-rs 绑定：mox-norm-core + mox-intent-core → @infotopograph/mox-norm-intent-native
//!
//! 反序列化策略（跨外部类型零侵入）：
//!   JsUnknown → env JSON.stringify → &str → serde_json::from_str

use mox_ai_intent_core::{
    classify_intent, score_alliance_candidates, ExpertCandidate, IntentPattern,
};
use mox_data_norm_core::{
    dedup_records, merge_conflicts, merge_records, resolve_rules, ConflictMergeFn, MergeStrategy,
    NormRecord, Rule, RuleEngine,
};
use napi::{Env, JsUnknown, Result};
use napi_derive::napi;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap as StdMap;

// ======================================================================
// Serde 输入结构（直接对应核心 crate 类型）
// ======================================================================
#[derive(Debug, Deserialize)]
pub struct NormInput {
    pub records: Vec<NormRecord>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub strategy: Option<String>,
    /// authority 策略的来源优先级排序（可选）
    #[serde(default)]
    pub src_order: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ClassifyInput {
    pub question: String,
    pub patterns: Vec<IntentPattern>,
}

#[derive(Debug, Deserialize)]
pub struct AllianceInput {
    pub question: String,
    pub experts: Vec<ExpertCandidate>,
    #[serde(default, alias = "intentPrimary")]
    pub intent_primary: String,
    #[serde(default, alias = "intentSecondary")]
    pub intent_secondary: Vec<String>,
    #[serde(default, alias = "matchedKeywords")]
    pub matched_keywords: Vec<String>,
    /// expert_id → (success_rate, avg_confidence)
    #[serde(default)]
    pub stats: StdMap<String, (f64, f64)>,
}

fn parse_input<T: DeserializeOwned>(env: Env, input: JsUnknown) -> Result<T> {
    let s = json_stringify(env, input)?;
    let v: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| napi::Error::from_reason(format!("parse json: {e}")))?;
    serde_json::from_value(v).map_err(|e| napi::Error::from_reason(format!("parse schema: {e}")))
}

fn json_stringify(env: Env, value: JsUnknown) -> Result<String> {
    let global = env.get_global()?;
    let json: napi::JsObject = global.get_named_property("JSON")?;
    let stringify_fn: napi::JsFunction = json.get_named_property("stringify")?;
    let result: napi::JsUnknown = stringify_fn.call(None, &[value])?;
    result.coerce_to_string()?.into_utf8()?.as_str().map(|s| s.to_string())
}

fn to_value<T: Serialize>(t: &T) -> Result<serde_json::Value> {
    serde_json::to_value(t).map_err(|e| napi::Error::from_reason(format!("ser: {e}")))
}

// ======================================================================
// Meta
// ======================================================================
#[napi]
pub fn mox_norm_intent_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ======================================================================
// F-N1 归一化流水线：dedup → rules → merge_conflicts (自定义)
// ======================================================================
#[derive(Serialize)]
struct NormOut {
    records: Vec<NormRecord>,
    report: mox_data_norm_core::DedupReport,
    rules_applied: i64,
    merges: i64,
}

#[napi]
pub fn normalize_records(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let inp: NormInput = parse_input(env, input)?;
    let (deduped, report) = dedup_records(&inp.records);

    let after_rules = if let Some(rules) = &inp.rules {
        let engine = RuleEngine::new(rules.clone());
        resolve_rules(&deduped, &engine)
    } else {
        deduped
    };

    let strategy_str = inp.strategy.as_deref().unwrap_or("lww");
    let strategy = match strategy_str {
        "lww" => MergeStrategy::LastWriteWins,
        "union" | "union_fields" | "unionFields" => MergeStrategy::UnionFields,
        "majority" => MergeStrategy::Majority,
        "conf" | "highest_confidence" => MergeStrategy::HighestConfidenceFirst,
        "authority" => MergeStrategy::SourceAuthority { src_order: inp.src_order.clone().unwrap_or_default() },
        _ => MergeStrategy::LastWriteWins,
    };

    // 零捕获 fn pointer 分派
    let m_fn: ConflictMergeFn = match strategy {
        MergeStrategy::LastWriteWins => lww_fn,
        MergeStrategy::UnionFields | MergeStrategy::UnionAttributes => union_fields_fn,
        MergeStrategy::Majority => majority_fn,
        MergeStrategy::HighestConfidenceFirst => lww_fn,
        MergeStrategy::SourceAuthority { .. } => lww_fn,
    };
    let mut m_fn_copy = m_fn;
    let merged = merge_conflicts(&after_rules, &mut m_fn_copy);

    let groups = merged.len();
    let rules_applied = inp.rules.as_ref().map(|r| r.len()).unwrap_or(0) as i64;

    to_value(&NormOut {
        records: merged,
        report,
        rules_applied,
        merges: (after_rules.len().saturating_sub(groups)) as i64,
    })
}

// ======================================================================
// F-N2 基于 MergeStrategy 内置语义的归一化（更完整的 merge_records 入口）
// ======================================================================
#[derive(Serialize)]
struct StratOut {
    merged: Vec<NormRecord>,
    merged_groups: i64,
    conflicts_resolved: i64,
}

#[napi]
pub fn normalize_records_strategy(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let inp: NormInput = parse_input(env, input)?;
    let (deduped, _report) = dedup_records(&inp.records);
    let after_rules = if let Some(rules) = &inp.rules {
        let engine = RuleEngine::new(rules.clone());
        resolve_rules(&deduped, &engine)
    } else {
        deduped
    };
    let strategy = match inp.strategy.as_deref().unwrap_or("lww") {
        "lww" => MergeStrategy::LastWriteWins,
        "union" | "union_fields" | "unionFields" => MergeStrategy::UnionFields,
        "majority" => MergeStrategy::Majority,
        "conf" | "highest_confidence" => MergeStrategy::HighestConfidenceFirst,
        "authority" => MergeStrategy::SourceAuthority { src_order: inp.src_order.unwrap_or_default() },
        _ => MergeStrategy::LastWriteWins,
    };
    let r = merge_records(&after_rules, &strategy);
    to_value(&StratOut {
        merged: r.merged,
        merged_groups: r.merged_groups as i64,
        conflicts_resolved: r.conflicts_resolved as i64,
    })
}

// ======================================================================
// 静态 merge_conflicts 函数指针实现（零捕获）
// ======================================================================
fn lww_fn(_k: &str, vs: &[serde_json::Value], _s: &str) -> serde_json::Value {
    vs.last().cloned().unwrap_or(serde_json::Value::Null)
}
fn union_fields_fn(_k: &str, vs: &[serde_json::Value], _s: &str) -> serde_json::Value {
    vs.last().cloned().unwrap_or(serde_json::Value::Null)
}
fn majority_fn(_k: &str, vs: &[serde_json::Value], _s: &str) -> serde_json::Value {
    if vs.is_empty() { return serde_json::Value::Null; }
    let mut counts: StdMap<String, (usize, usize)> = StdMap::new();
    for (i, v) in vs.iter().enumerate() {
        let key = v.to_string();
        let e = counts.entry(key).or_insert((0, usize::MAX));
        e.0 += 1;
        if i < e.1 { e.1 = i; }
    }
    let best = counts
        .into_iter()
        .max_by(|a, b| a.1 .0.cmp(&b.1 .0).then(b.1 .1.cmp(&a.1 .1)))
        .map(|(s, _)| s);
    if let Some(target) = best {
        for v in vs {
            if v.to_string() == target {
                return v.clone();
            }
        }
    }
    vs[0].clone()
}

// ======================================================================
// F-I1 意图分类
// ======================================================================
#[napi]
pub fn classify_question(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let inp: ClassifyInput = parse_input(env, input)?;
    let r = classify_intent(&inp.patterns, &inp.question);
    to_value(&r)
}

// ======================================================================
// F-I2 联盟打分
// ======================================================================
#[napi]
pub fn score_alliance(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let inp: AllianceInput = parse_input(env, input)?;
    let stats32: StdMap<String, (f32, f32)> = inp
        .stats
        .into_iter()
        .map(|(k, (a, b))| (k, (a as f32, b as f32)))
        .collect();
    let out = score_alliance_candidates(
        inp.experts,
        &inp.question,
        &inp.intent_primary,
        &inp.intent_secondary,
        &inp.matched_keywords,
        &stats32,
    );
    to_value(&out)
}
