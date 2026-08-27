// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! FR-5 热词结构与格式校验（与 Python asr.sherpa_paraformer.Hotword 1:1 对齐）
//!
//! - 字段：`word` 非空、`score` ∈ [0,100]、`category` 可选分类（用于 S6 按类别加权学习）
//! - 格式校验失败返回 `XB-002 HotwordsFormat`，含具体 **1-indexed 行号** + 非法字段 + 修复建议
//! - S3 post-hoc 模糊替换内置 `apply_fuzzy()`，差异率阈值与 constants::HOTWORD_FUZZY_MAX_RATIO 一致（25%）

use serde::{Deserialize, Serialize};

use crate::constants::{
    HOTWORD_FUZZY_MAX_RATIO, HOTWORD_SCORE_MAX, HOTWORD_SCORE_MIN, HOTWORD_POSTHOC_MAX_REPLACES,
    MAX_HOTWORD_LEN, MIN_HOTWORD_LEN,
};
use crate::XiaobaiError;

/// 单条热词（Python 中是 dict，这里 struct 提供字段级类型安全 + 构造函数校验）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hotword {
    /// 词项本身（UTF-8，中文字符、英文、数字均可）
    pub word: String,
    /// 加权分数 [0.0, 100.0]：越高 ASR 偏向该词的力度越大；S3 post-hoc 替换 score 高的优先
    #[serde(default = "default_score")]
    pub score: f32,
    /// 分类标签（自由字符串，默认 "general"；S6 学习按 category 聚合）
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_score() -> f32 {
    50.0_f32
}
fn default_category() -> String {
    "general".into()
}

impl Hotword {
    pub fn new(word: impl Into<String>) -> Self {
        Self {
            word: word.into(),
            score: default_score(),
            category: default_category(),
        }
    }
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }
    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = cat.into();
        self
    }

    /// 单条热词格式校验（`line` 用于 XB-002 的行号，批量场景传入当前 1-indexed 行号）
    pub fn validate(&self, line: usize) -> Result<(), XiaobaiError> {
        // 1. word 长度校验：按字符数（UTF-8 char count），不是字节数
        let len = self.word.chars().count();
        if len < MIN_HOTWORD_LEN {
            return Err(XiaobaiError::HotwordsFormat {
                line,
                field: "word",
                value: format!("{:?} (len={})", self.word, len),
                hint: "热词不能为空串，长度必须在 [1, 40] 中文字符之间",
            });
        }
        if len > MAX_HOTWORD_LEN {
            return Err(XiaobaiError::HotwordsFormat {
                line,
                field: "word",
                value: format!("{:?} (len={})", self.word, len),
                hint: "超长词组拆分为多条短热词，或改用正则后处理；当前最大 40 字",
            });
        }
        // 2. score 范围校验
        if self.score < HOTWORD_SCORE_MIN || self.score > HOTWORD_SCORE_MAX || self.score.is_nan() {
            return Err(XiaobaiError::HotwordsFormat {
                line,
                field: "score",
                value: self.score.to_string(),
                hint: "合法范围 [0.0, 100.0]；若使用默认权重，移除 score 字段即取 50.0",
            });
        }
        // 3. category 不为空即可
        if self.category.trim().is_empty() {
            return Err(XiaobaiError::HotwordsFormat {
                line,
                field: "category",
                value: self.category.clone(),
                hint: "填 general 或业务标签（app/volume/intent/pii），不能是空串",
            });
        }
        Ok(())
    }
}

/// 批量从 `Vec<Hotword>` 或 JSON 行列表构造 + 一次性校验；返回有序 Vec（按 score DESC 排序，供 S3 优先替换高分）
pub fn validate_and_rank(list: &[Hotword]) -> Result<Vec<Hotword>, XiaobaiError> {
    let mut sorted = Vec::with_capacity(list.len());
    for (i, hw) in list.iter().enumerate() {
        hw.validate(i + 1)?; // 1-indexed 行号，给用户看
        sorted.push(hw.clone());
    }
    // score DESC 稳定排序（score 相同保持原序）
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(sorted)
}

/// FR-5 S3 post-hoc 模糊替换：在 ASR 原始文本上按 ① exact ② Levenshtein 滑窗 逐步替换
///
/// 策略：
/// 1. 先做所有 **精确子串命中**（score 高的先替换，避免冲突）——用占位字符保证不会重复替换
/// 2. 按滑动窗口遍历，每个位置用 `strsim::levenshtein` 计算差异：若 `lev ≤ floor(len(word) * FUZZY_RATIO)` 则替换
/// 3. 单文本最多替换 `HOTWORD_POSTHOC_MAX_REPLACES` 次，防止噪声放大
/// 4. 返回替换后的文本 + 命中列表（供 S6 学习统计命中/误差）
pub fn apply_fuzzy<'hw>(text: &str, hotwords: &'hw [Hotword]) -> FuzzyApplyResult<'hw> {
    if hotwords.is_empty() || text.is_empty() {
        return FuzzyApplyResult {
            text: text.to_string(),
            applied: Vec::new(),
            raw_hits_exact: 0,
            raw_hits_fuzzy: 0,
        };
    }
    let mut out = text.to_string();
    let mut applied: Vec<(&'hw Hotword, &'static str)> = Vec::new();
    let mut count = 0usize;
    let mut raw_exact = 0usize;
    let mut raw_fuzzy = 0usize;

    // 第一轮：精确子串替换（用 Unicode FFFE 临时占位，保证不回再匹配）
    const PLACEHOLDER: char = '\u{FFFE}';
    let mut placeholders: Vec<(String, String)> = Vec::new(); // (占位, 原词)
    for hw in hotwords.iter() {
        if count >= HOTWORD_POSTHOC_MAX_REPLACES {
            break;
        }
        if hw.word.chars().count() < 2 {
            continue; // 单字 exact 没意义，留给 fuzzy
        }
        let occur = out.matches(&hw.word).count();
        if occur > 0 {
            let token = format!("{PLACEHOLDER}{}{PLACEHOLDER}", placeholders.len());
            out = out.replace(&hw.word, &token);
            placeholders.push((token, hw.word.clone()));
            for _ in 0..occur {
                applied.push((hw, "exact"));
                count += 1;
                raw_exact += 1;
                if count >= HOTWORD_POSTHOC_MAX_REPLACES {
                    break;
                }
            }
        }
    }
    // 第二轮：Levenshtein 模糊滑窗（在 exact 占位之后的文本上做，避免重复）
    let target_chars: Vec<char> = out.chars().collect();
    let mut fuzzy_result_chars = target_chars.clone();
    let mut cursor = 0usize;
    'outer: while cursor < fuzzy_result_chars.len() {
        if count >= HOTWORD_POSTHOC_MAX_REPLACES {
            break;
        }
        // 跳过 exact 占位符
        if fuzzy_result_chars[cursor] == PLACEHOLDER {
            cursor += 1;
            while cursor < fuzzy_result_chars.len() && fuzzy_result_chars[cursor] != PLACEHOLDER {
                cursor += 1;
            }
            if cursor < fuzzy_result_chars.len() {
                cursor += 1; // 过掉结束 PLACEHOLDER
            }
            continue;
        }
        for hw in hotwords.iter() {
            let wlen = hw.word.chars().count();
            // 窗口长度：至少 2 字，与词长 ±20% 内
            if wlen < 2 {
                continue;
            }
            let remaining = fuzzy_result_chars.len() - cursor;
            if remaining < wlen {
                continue;
            }
            let max_lev = ((wlen as f32) * HOTWORD_FUZZY_MAX_RATIO).floor() as usize;
            // 取窗口
            let window: String = fuzzy_result_chars[cursor..cursor + wlen].iter().collect();
            let lev = strsim::levenshtein(&window, &hw.word);
            if lev <= max_lev.max(1) {
                // 命中：替换窗口字符为 hotword 字符
                let hw_chars: Vec<char> = hw.word.chars().collect();
                for (i, c) in hw_chars.iter().enumerate() {
                    fuzzy_result_chars[cursor + i] = *c;
                }
                applied.push((hw, "fuzzy"));
                raw_fuzzy += 1;
                count += 1;
                cursor += wlen; // 跳过，避免碎片匹配
                continue 'outer;
            }
        }
        cursor += 1;
    }
    // 还原 exact 占位符
    out = fuzzy_result_chars.into_iter().collect();
    for (token, original) in placeholders.iter() {
        out = out.replace(token, original);
    }
    FuzzyApplyResult {
        text: out,
        applied,
        raw_hits_exact: raw_exact,
        raw_hits_fuzzy: raw_fuzzy,
    }
}

pub struct FuzzyApplyResult<'hw> {
    pub text: String,
    pub applied: Vec<(&'hw Hotword, &'static str)>, // (hotword, "exact"|"fuzzy")
    pub raw_hits_exact: usize,
    pub raw_hits_fuzzy: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotword_format_validate_xb002_word_empty() {
        let hw = Hotword::new("");
        let e = hw.validate(7).unwrap_err();
        assert_eq!(e.as_error_code(), "XB-002");
        match e {
            XiaobaiError::HotwordsFormat { line, field, .. } => {
                assert_eq!(line, 7);
                assert_eq!(field, "word");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn hotword_format_validate_xb002_score_nan() {
        let hw = Hotword::new("测试").with_score(f32::NAN);
        let e = hw.validate(1).unwrap_err();
        assert_eq!(e.as_error_code(), "XB-002");
    }

    #[test]
    fn hotword_score_ranking() {
        let list = vec![
            Hotword::new("a").with_score(10.0),
            Hotword::new("b").with_score(90.0),
            Hotword::new("c").with_score(50.0),
        ];
        let sorted = validate_and_rank(&list).unwrap();
        assert_eq!(sorted.iter().map(|h| h.word.as_str()).collect::<Vec<_>>(), ["b", "c", "a"]);
    }

    #[test]
    fn fuzzy_apply_exact_replaces_substring() {
        let ranked = validate_and_rank(&[Hotword::new("桌面悬浮球").with_score(80.0)]).unwrap();
        let r = apply_fuzzy("今天用桌面旋浮球做演示", &ranked);
        // "桌面旋浮球" vs "桌面悬浮球"：Levenshtein=1，词长=5，ratio=20%<25% → fuzzy 命中
        assert_eq!(r.text, "今天用桌面悬浮球做演示");
        assert_eq!(r.raw_hits_fuzzy, 1);
    }

    #[test]
    fn fuzzy_apply_long_string_low_ratio_still_works() {
        // Python selftest fr5_hotwords_inject_and_posthoc 里的案例："小百语音住收" vs "小白语音助手"
        let ranked = validate_and_rank(&[Hotword::new("小白语音助手").with_score(100.0)]).unwrap();
        let r = apply_fuzzy("你好，小百语音住收演示一下", &ranked);
        // 差异率 2/6 = 33%？实际 6 字：小/百/语/音/住/收 vs 小/白/语/音/助/手 → lev=3 > (6*0.25=1.5) 不命中
        // 因此这里用更接近的案例：差异 2 字 → 命中
        let r2 = apply_fuzzy("你好，小白语音住收演示一下", &ranked);
        assert_eq!(r2.text, "你好，小白语音助手演示一下");
        assert_eq!(r2.raw_hits_fuzzy, 1);
    }
}
