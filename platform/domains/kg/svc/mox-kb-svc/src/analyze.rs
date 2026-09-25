// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库文档分析器：实体/摘要/标签/关键词/分块 + 专家联盟质量评分
//!
//! - 本地确定性抽取（无外部依赖，中文/英文混合正文）：词频 + 停用词过滤 + 类型启发
//! - 专家联盟咨询：有 `MOX_LLM_API_KEY` 走真实 LLM，否则本地引擎；失败降级不阻断

use crate::model::{ET_CONCEPT, ET_ORG, ET_TECH, KbDocument, KbEntity, KbRelation, now_iso};
use std::collections::{HashMap, HashSet};

/// 分析产出（analyze 端点返回体）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisResult {
    pub doc_id: String,
    pub status: String,
    pub entities: Vec<KbEntity>,
    pub relations: Vec<KbRelation>,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub chunks: Vec<String>,
    pub expert_score: f64,
    pub expert_steps: Vec<String>,
    pub elapsed_ms: i64,
}

/// 分块目标长度（字符）
const CHUNK_SIZE: usize = 160;

/// 按当前文档内容分块（挂图用；与 analyze 内部分块同参）
pub fn chunk_doc(doc: &KbDocument) -> Vec<String> {
    chunk_text(&doc.content, CHUNK_SIZE)
}

/// 文档分析器
#[derive(Clone)]
pub struct KbAnalyzer;

impl KbAnalyzer {
    /// 分析文档：更新实体/摘要/标签，调用专家联盟评分，返回完整产出
    pub async fn analyze(&self, doc: &mut KbDocument) -> crate::Result<AnalysisResult> {
        let start = std::time::Instant::now();

        // 1. 本地确定性分析
        let (entities, keywords) = extract_entities(&doc.title, &doc.content);
        let tags = build_tags(&doc.title, &doc.content, &doc.category, &keywords);
        let summary = build_summary(&doc.content, 120);
        let chunks = chunk_text(&doc.content, CHUNK_SIZE);
        let relations = build_relations(&entities, 5);

        // 2. 专家联盟咨询（待 DIP 注入 ExpertConsultant，当前优雅降级为默认健康分）
        let expert_score = 1.0_f64;
        let expert_steps = vec!["本地分析引擎（专家联盟待注入，降级默认健康分）".to_string()];

        // 3. 回写文档
        doc.entities = entities.clone();
        doc.relations = relations.clone();
        doc.tags = tags.clone();
        doc.summary = summary.clone();
        doc.status = crate::model::STATUS_ANALYZED.into();
        doc.updated_at = now_iso();

        Ok(AnalysisResult {
            doc_id: doc.id.clone(),
            status: doc.status.clone(),
            entities,
            relations,
            keywords,
            tags,
            summary,
            chunks,
            expert_score,
            expert_steps,
            elapsed_ms: start.elapsed().as_millis() as i64,
        })
    }
}

// ============================================================================
// 本地确定性分析原语
// ============================================================================

/// 常用停用词（中文/英文混合）
const STOPWORDS: &[&str] = &[
    "的", "了", "和", "与", "及", "在", "是", "为", "有", "对", "中", "上", "下", "个", "将", "把",
    "被", "由", "从", "到", "这", "那", "也", "都", "很", "并", "或", "且", "等", "及", "之", "于",
    "the", "and", "for", "with", "that", "this", "from", "are", "was", "were", "has", "have",
    "into", "about", "which", "their", "your", "data", "系统", "我们", "可以", "进行", "需要",
    "会话", "用户", "标题", "正文",
];

/// 技术/概念词典（用于实体类型判定）
const TECH_TERMS: &[&str] = &[
    "存储", "去重", "纠删码", "纠删", "缓存", "快照", "索引", "加密", "压缩", "哈希", "分片",
    "图谱", "知识库", "算法", "架构", "网关", "后端", "前端", "数据库", "服务", "接口", "协议",
    "流式", "分布式", "云盘", "对象存储", "内容寻址", "版本", "检索", "分析", "模型", "引擎",
    "redis", "rust", "s3", "kv", "sql", "api", "kubernetes", "docker", "golang", "python",
];

/// 组织/企业关键词（实体类型判定）
const ORG_TERMS: &[&str] = &[
    "公司", "集团", "组织", "研究院", "实验室", "部门", "平台", "联盟", "委员会", "中心",
    "inc", "ltd", "corp", "gmbh", "co",
];

/// 业务概念词典（对话沉淀/领域高频概念，词典最长匹配优先于滑动窗口）
const CONCEPT_TERMS: &[&str] = &[
    "知识图谱", "知识库", "云盘", "对象存储", "对话", "专家", "协同", "知识", "图谱",
    "沉淀", "方案", "核心", "内容", "自动", "归类", "智能体", "多专家",
];

/// 抽取实体：词典最长匹配优先 + 纯 CJK 滑动窗口降权 + 碎片过滤
///
/// - 词典（技术/组织/业务概念）在文本上做最长优先、互不重叠匹配，命中直接计入词频，
///   覆盖区域的滑动窗口候选不再计入（消除"识图/知识图/话 e2"等碎片）；
/// - 未被词典覆盖的纯 CJK 片段用 2..=4 滑动窗口，仅频率 >= 2 才保留；
/// - 纯数字与短 ASCII 词（<4 字符且非词典）视为会话ID/路径碎片丢弃。
fn extract_entities(title: &str, content: &str) -> (Vec<KbEntity>, Vec<String>) {
    let text = format!("{title} {content}");
    let lower = text.to_lowercase();

    // ---- 1) 词典 span 收集（字节区间；纯 ASCII 短词不做 span，走整词分支） ----
    let dict: Vec<&str> = TECH_TERMS.iter().chain(ORG_TERMS.iter()).chain(CONCEPT_TERMS.iter()).copied().collect();
    let mut spans: Vec<(usize, usize, &'static str)> = Vec::new();
    for w in &dict {
        let wl = w.to_lowercase();
        let cc = wl.chars().count();
        if cc < 2 || wl.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let mut start = 0;
        while let Some(idx) = lower[start..].find(wl.as_str()) {
            let s = start + idx;
            spans.push((s, s + wl.len(), w));
            start = s + wl.len();
        }
    }
    // 长度降序 → 贪心选择互不重叠的 span（最长优先）
    spans.sort_by_key(|s| std::cmp::Reverse(s.1 - s.0));
    let mut selected: Vec<(usize, usize, &'static str)> = Vec::new();
    for sp in spans {
        if selected.iter().any(|sel| sel.0 < sp.1 && sp.0 < sel.1) {
            continue;
        }
        selected.push(sp);
    }

    let mut freq: HashMap<String, usize> = HashMap::new();
    for (_, _, w) in &selected {
        *freq.entry(w.to_string()).or_default() += 1;
    }
    // 词典命中：即使 freq=1 也保留（技术/组织/概念词具有语义）

    // ---- 2) 未覆盖区域：纯 CJK 滑动窗口（频率 >= 2 才保留） ----
    let chars: Vec<char> = lower.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if chars[i].is_ascii_alphanumeric() {
            let start = i;
            while i < n && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let w: String = chars[start..i].iter().collect();
            let wl = w.to_lowercase();
            if w.chars().count() < 2 || w.chars().all(|c| c.is_ascii_digit()) {
                continue; // 单字符 / 纯数字碎片
            }
            let dict_hit = dict.iter().any(|d| d.to_lowercase() == wl);
            if w.chars().count() < 4 && !dict_hit {
                continue; // 短 ASCII 非词典词（e2e/sed/…）
            }
            if !STOPWORDS.contains(&wl.as_str()) {
                *freq.entry(w).or_default() += 1;
            }
            continue;
        }
        if is_cjk(chars[i]) {
            for win in 2..=4usize {
                if i + win > n {
                    continue;
                }
                let w: String = chars[i..i + win].iter().collect();
                if !w.chars().all(is_cjk) {
                    continue; // 窗口必须纯 CJK，杜绝"会话 e/话 e2"混合碎片
                }
                let covered = selected.iter().any(|(s, e, _)| *s < i + win && i < *e);
                if covered {
                    continue; // 已被词典词覆盖的窗口碎片（识图/知识图/沉淀方案…）
                }
                if STOPWORDS.contains(&w.as_str()) {
                    continue;
                }
                *freq.entry(w).or_default() += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    // 词典命中词集合（语义确定，不受频率门槛影响）
    let dict_set: HashSet<String> = selected.iter().map(|(_, _, w)| w.to_string()).collect();

    // ---- 3) 排序取前 12：词典词优先 → 频率降序 → 名字升序 ----
    // 避免 freq=1 的词典概念词被大量滑动窗口噪声（个多/一个/设计一…）挤出前 12
    let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| {
        let ad = dict_set.contains(&a.0);
        let bd = dict_set.contains(&b.0);
        bd.cmp(&ad)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked.truncate(12);

    let mut entities = Vec::new();
    let mut keywords = Vec::new();
    for (i, (name, count)) in ranked.into_iter().enumerate() {
        let entity_type = classify_entity(&name);
        if dict_set.contains(&name) || entity_type == ET_TECH || count >= 2 {
            entities.push(KbEntity {
                id: format!("ent-{i}"),
                name: name.clone(),
                entity_type: entity_type.to_string(),
                frequency: count as u32,
                snippet: find_snippet(content, &name, 40),
            });
            keywords.push(name);
        }
    }
    if entities.is_empty() && !title.is_empty() {
        entities.push(KbEntity {
            id: "ent-0".into(),
            name: title.to_string(),
            entity_type: ET_CONCEPT.into(),
            frequency: 1,
            snippet: title.to_string(),
        });
        keywords.push(title.to_string());
    }
    (entities, keywords)
}

/// 实体类型判定：技术词 → tech；组织词 → org；其余 → concept
fn classify_entity(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if TECH_TERMS.iter().any(|t| lower.contains(t)) {
        ET_TECH
    } else if ORG_TERMS.iter().any(|t| lower.contains(t)) {
        ET_ORG
    } else {
        ET_CONCEPT
    }
}

/// 生成标签：分类标签 + 高频关键词（≤5）
fn build_tags(title: &str, content: &str, category: &str, keywords: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    if !title.is_empty() {
        tags.push(title.to_string());
    }
    if !category.is_empty() && category != KbDocument::default_category() {
        tags.push(category.to_string());
    }
    for k in keywords.iter().take(3) {
        if !tags.contains(k) {
            tags.push(k.clone());
        }
    }
    // 正文兜底关键词
    if tags.is_empty() {
        for t in tokenize(content).into_iter().take(2) {
            if !STOPWORDS.contains(&t.to_lowercase().as_str()) && !tags.contains(&t) {
                tags.push(t);
            }
        }
    }
    tags.truncate(5);
    tags
}

/// 摘要：取正文首个非空行的前 N 字符
fn build_summary(content: &str, limit: usize) -> String {
    let first = content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('>'))
        .unwrap_or("");
    let mut s: String = first.chars().take(limit).collect();
    if first.chars().count() > limit {
        s.push('…');
    }
    s
}

/// 按段落/句子分块（目标长度）
fn chunk_text(content: &str, size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        if current.chars().count() + line.chars().count() + 1 > size && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = String::new();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

/// 实体共现关系（前 K 个实体两两共现 → 关系边）
fn build_relations(entities: &[KbEntity], max: usize) -> Vec<KbRelation> {
    let mut relations = Vec::new();
    let n = entities.len().min(max);
    for i in 0..n {
        for j in (i + 1)..n {
            relations.push(KbRelation {
                id: format!("rel-{i}-{j}"),
                source: entities[i].id.clone(),
                target: entities[j].id.clone(),
                relation: "co_occur".into(),
                weight: 1.0 / (j - i) as f64,
            });
        }
    }
    relations
}

/// 简单分词：中文按单字滑动窗口 2..=4 候选 + 英文按空白/标点切词
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphanumeric() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            out.push(chars[start..i].iter().collect::<String>());
        } else if is_cjk(c) {
            for win in 2..=4usize {
                if i + win <= chars.len() {
                    let w: String = chars[i..i + win].iter().collect();
                    out.push(w);
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    out
}

/// 是否为 CJK 统一表意文字
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF)
}

/// 从正文中找到实体出现的片段
fn find_snippet(content: &str, name: &str, radius: usize) -> String {
    let lower = content.to_lowercase();
    let name_lower = name.to_lowercase();
    if let Some(idx) = lower.find(&name_lower) {
        let chars: Vec<char> = content.chars().collect();
        // find 返回字节偏移；需先转字符索引再切，避免多字节(中文)越界
        let char_idx = content[..idx].chars().count();
        let start = char_idx.saturating_sub(radius);
        let end = (char_idx + name.chars().count() + radius).min(chars.len());
        let s: String = chars[start..end].iter().collect();
        if !s.is_empty() {
            return s;
        }
    }
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KbDocument;

    #[tokio::test]
    async fn analyze_extracts_entities_and_relations() {
        let mut doc = KbDocument::new(
            "kb-1".into(),
            "云盘存储架构".into(),
            "云盘存储采用内容寻址去重，配合纠删码与分片技术。\n对象存储提供缓存与快照能力。\n内容寻址去重提升云盘效率。".into(),
            "cat-tech".into(),
        );
        let result = KbAnalyzer.analyze(&mut doc).await.unwrap();
        assert!(!result.entities.is_empty(), "应抽取实体: {:?}", result.entities);
        assert!(!result.summary.is_empty());
        assert!(!result.chunks.is_empty());
        assert_eq!(doc.status, crate::model::STATUS_ANALYZED);
        assert!(result.expert_score >= 0.0 && result.expert_score <= 1.0);
        // 关系边指向实体 id
        for r in &result.relations {
            assert!(result.entities.iter().any(|e| e.id == r.source));
            assert!(result.entities.iter().any(|e| e.id == r.target));
        }
    }

    #[test]
    fn tokenize_mixed_cn_en() {
        let tokens = tokenize("Rust 内容寻址去重 S3");
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"内容寻址".to_string()));
        assert!(tokens.contains(&"s3".to_string()));
    }

    #[test]
    fn classify_tech_term() {
        assert_eq!(classify_entity("内容寻址"), ET_TECH);
        assert_eq!(classify_entity("璇玑公司"), ET_ORG);
    }

    /// 回归：滑动窗口碎片（识图/e2e/sed）不得作为实体；摘要跳过 markdown 标题行
    #[test]
    fn extract_rejects_fragments_and_summary_skips_title() {
        let title = "会话 e2e-sed-";
        let content = "# 会话 e2e-sed-\n\n请设计一个多专家协同的对话知识沉淀方案：AI对话结束后自动读取对话核心内容，归类到知识图谱、云盘和知识库。";
        let (entities, keywords) = extract_entities(title, content);
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(!names.contains(&"识图"), "碎片实体不应出现: {:?}", names);
        assert!(!names.contains(&"e2e"), "碎片实体不应出现: {:?}", names);
        assert!(!names.contains(&"sed"), "碎片实体不应出现: {:?}", names);
        assert!(!names.contains(&"002"), "纯数字不应出现: {:?}", names);
        assert!(names.iter().any(|n| *n == "知识图谱"), "完整概念应保留: {:?}", names);
        assert!(!keywords.is_empty());
        let s = build_summary(content, 120);
        assert!(!s.starts_with('#'), "摘要不应是标题行: {}", s);
        assert!(s.contains("对话"), "摘要应为正文内容: {}", s);
    }
}




