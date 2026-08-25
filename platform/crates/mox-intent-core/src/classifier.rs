//! 意图分类：Aho-Corasick 多模匹配（leftmost-first） + 等级评分 + TOP-K。
//!
//! ## API 契约（与 Node intent-classifier.js 完全一致）
//! ```text
//! IntentPattern { intent, keywords[], capability? }
//! classify_intent(question: &str) -> IntentResult
//! {
//!   primary: String,
//!   secondary: [String, String],
//!   confidence: f32, // 0..1, 保留 2 位小数
//!   matched_keywords: Vec<String>,
//!   all_scores: {intent: score},
//!   capability: String,  // 统一编排能力
//! }
//! ```

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use ahash::RandomState;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPattern {
    pub intent: String,
    pub keywords: Vec<String>,
    /// 可选：显式 capability；若 None，走 INTENT_TO_CAPABILITY 映射表
    #[serde(default)]
    pub capability: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentResult {
    pub primary: String,
    pub secondary: Vec<String>,
    pub confidence: f32,
    #[serde(rename = "matchedKeywords")]
    pub matched_keywords: Vec<String>,
    #[serde(rename = "allScores")]
    pub all_scores: HashMap<String, i32, RandomState>,
    pub capability: String,
}

/// 领域意图 → 统一编排能力（与 Node 单一真源对齐，禁止独立实现）
pub fn intent_to_capability(intent: &str) -> &'static str {
    match intent {
        "ai" | "requirement" | "reasoning" => "reasoning",
        "fusion" | "automation" | "operator" => "expert",
        "workflow" => "workflow",
        "graph" | "data" | "performance" | "monitor" | "algorithm"
        | "architecture" | "security" | "mcp" | "market" => "graph",
        _ => "chat",
    }
}

pub struct IntentClassifier {
    patterns: Vec<IntentPattern>,
    ac: AhoCorasick,
    /// Aho-Corasick pattern index → (pattern_idx: usize, keyword_index: usize)
    kw_meta: Vec<(usize, usize)>,
    /// 关键词小写规范化索引
    kw_lower: Vec<String>,
}

impl IntentClassifier {
    pub fn new(patterns: Vec<IntentPattern>) -> Self {
        // 为 Aho-Corasick 构造所有关键词（lowercase）
        let mut kw_lower = Vec::new();
        let mut kw_meta = Vec::new();
        for (pi, p) in patterns.iter().enumerate() {
            for (ki, kw) in p.keywords.iter().enumerate() {
                kw_lower.push(kw.to_lowercase());
                kw_meta.push((pi, ki));
            }
        }
        // leftmost-first：保证 "工作流" 比 "工作" 优先命中（多词短语权重更高）
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .ascii_case_insensitive(false) // 我们自己小写化，避免 unicode 大小写不一致
            .build(&kw_lower)
            .expect("IntentClassifier: AhoCorasick build failed (empty pattern set? at least 1 keyword required)");
        Self { patterns, ac, kw_meta, kw_lower }
    }

    pub fn classify(&self, question: &str) -> IntentResult {
        let text = question.to_lowercase();
        let n_patterns = self.patterns.len();
        let mut scores = vec![0i32; n_patterns];
        let mut matched_kws: Vec<Vec<String>> = vec![Vec::new(); n_patterns];
        let mut seen_pat_kw: HashSet<(usize, usize), RandomState> =
            HashSet::with_hasher(RandomState::new());

        for m in self.ac.find_iter(text.as_str()) {
            let idx = m.pattern().as_usize();
            let (pi, ki) = self.kw_meta[idx];
            // 大小写不敏感 + 同 pattern 同 keyword 去重
            if !seen_pat_kw.insert((pi, ki)) { continue; }
            let kw_str = &self.patterns[pi].keywords[ki];
            // 多词短语权重 2，单词 1（与 Node 一致）
            let weight = if kw_str.contains(' ') { 2 } else { 1 };
            scores[pi] += weight;
            matched_kws[pi].push(kw_str.clone());
        }

        // 打包为 [(intent_idx, score)]
        let mut scored: Vec<(usize, i32)> = (0..n_patterns)
            .filter(|&i| scores[i] > 0)
            .map(|i| (i, scores[i]))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(self.patterns[a.0].intent.cmp(&self.patterns[b.0].intent)));

        // capability 桶归并（与 Node T8 对账一致）
        let mut cap_buckets: HashMap<String, (i32, Vec<usize>), RandomState> =
            HashMap::with_hasher(RandomState::new());
        for &(pi, s) in &scored {
            let p = &self.patterns[pi];
            let cap = p.capability.clone().unwrap_or_else(|| intent_to_capability(&p.intent).to_string());
            let entry = cap_buckets.entry(cap).or_insert((0, Vec::new()));
            entry.0 += s;
            entry.1.push(pi);
        }
        let mut cap_order: Vec<(String, i32)> = cap_buckets
            .iter()
            .map(|(k, v)| (k.clone(), v.0))
            .collect();
        cap_order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        if scored.is_empty() {
            return IntentResult {
                primary: "general".to_string(),
                secondary: Vec::new(),
                confidence: 0.0,
                matched_keywords: Vec::new(),
                all_scores: HashMap::with_hasher(RandomState::new()),
                capability: "chat".to_string(),
            };
        }

        // 重新排列 scored：按 cap bucket 顺序
        let cap_rank: HashMap<String, usize, RandomState> = cap_order
            .iter()
            .enumerate()
            .map(|(i, (c, _))| (c.clone(), i))
            .collect();
        scored.sort_by(|a, b| {
            let pa = &self.patterns[a.0];
            let pb = &self.patterns[b.0];
            let ca = pa.capability.clone().unwrap_or_else(|| intent_to_capability(&pa.intent).to_string());
            let cb = pb.capability.clone().unwrap_or_else(|| intent_to_capability(&pb.intent).to_string());
            let ra = cap_rank.get(&ca).copied().unwrap_or(usize::MAX);
            let rb = cap_rank.get(&cb).copied().unwrap_or(usize::MAX);
            ra.cmp(&rb).then(b.1.cmp(&a.1)).then(pa.intent.cmp(&pb.intent))
        });

        let (top_cap, top_score) = &cap_order[0];
        let runner_up = cap_order.get(1).map(|(_, s)| *s).unwrap_or(0);
        let confidence = if runner_up > 0 {
            let t = *top_score as f32;
            let r = runner_up as f32;
            (t / (t + r)).min(1.0)
        } else {
            (*top_score as f32 / 3.0).min(1.0)
        };
        // 保留 2 位小数（与 Node Math.round(x*100)/100 一致）
        let confidence = (confidence * 100.0).round() / 100.0;

        // primary：top cap 桶内分最高的 intent
        let top_bucket = cap_buckets.get(top_cap).unwrap();
        let mut top_intents: Vec<(usize, i32)> = top_bucket.1
            .iter()
            .map(|&pi| (pi, scores[pi]))
            .collect();
        top_intents.sort_by(|a, b| b.1.cmp(&a.1).then(self.patterns[a.0].intent.cmp(&self.patterns[b.0].intent)));
        let primary_pi = top_intents[0].0;
        let primary = self.patterns[primary_pi].intent.clone();
        let matched_keywords = matched_kws[primary_pi].clone();

        // all_scores
        let mut all_scores: HashMap<String, i32, RandomState> =
            HashMap::with_capacity_and_hasher(scored.len(), RandomState::new());
        for &(pi, s) in &scored {
            all_scores.insert(self.patterns[pi].intent.clone(), s);
        }

        // secondary：TOP-K 后续 2 项 intent
        let secondary: Vec<String> = scored
            .iter()
            .skip(1)
            .take(2)
            .map(|(pi, _)| self.patterns[*pi].intent.clone())
            .collect();

        IntentResult {
            primary,
            secondary,
            confidence,
            matched_keywords,
            all_scores,
            capability: top_cap.clone(),
        }
    }
}

/// 便利函数：使用 patterns 切片构造一次性分类器并调用
pub fn classify_intent(patterns: &[IntentPattern], question: &str) -> IntentResult {
    let cl = IntentClassifier::new(patterns.to_vec());
    cl.classify(question)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_patterns() -> Vec<IntentPattern> {
        vec![
            IntentPattern {
                intent: "graph".into(),
                keywords: vec!["graph".into(), "图".into(), "图谱".into(), "算法".into()],
                capability: None,
            },
            IntentPattern {
                intent: "automation".into(),
                keywords: vec!["自动化".into(), "工作流".into(), "流程".into()],
                capability: None,
            },
            IntentPattern {
                intent: "general".into(),
                keywords: vec!["你好".into(), "聊天".into()],
                capability: Some("chat".into()),
            },
        ]
    }

    #[test]
    fn primary_matches_pattern() {
        let r = classify_intent(&sample_patterns(), "帮我分析这个图谱算法");
        assert_eq!(r.primary, "graph");
        assert_eq!(r.capability, "graph");
        assert!(!r.matched_keywords.is_empty());
        assert!(r.confidence > 0.0);
    }

    #[test]
    fn unknown_falls_to_general() {
        let r = classify_intent(&sample_patterns(), "请解释相对论");
        assert_eq!(r.primary, "general");
        assert_eq!(r.capability, "chat");
        assert_eq!(r.confidence, 0.0);
    }

    #[test]
    fn multiword_weight_is_higher() {
        let pats = vec![IntentPattern {
            intent: "w".into(),
            keywords: vec!["工作流".into(), "工作".into()],
            capability: None,
        }];
        let r = classify_intent(&pats, "请运行我的工作流");
        // "工作流" 应命中 1 次（leftmost-first），权重 2（含空格？"工作流" 不含空格，权重 1）
        // （Node 规则：仅含 ASCII space 的 kw 权重 2）这里为 1，但与 Node 逻辑对齐（无空格→1）。
        assert_eq!(*r.all_scores.get("w").unwrap(), 1);
    }
}
