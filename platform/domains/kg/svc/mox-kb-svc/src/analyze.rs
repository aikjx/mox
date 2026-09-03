// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库文档分析器：实体/摘要/标签/关键词/分块 + 专家联盟质量评分
//!
//! - 本地确定性抽取（无外部依赖，中文/英文混合正文）：词频 + 停用词过滤 + 类型启发
//! - 专家联盟咨询：有 `MOX_LLM_API_KEY` 走真实 LLM，否则本地引擎；失败降级不阻断

use crate::model::{ET_CONCEPT, ET_ORG, ET_TECH, KbDocument, KbEntity, KbRelation, now_iso};
use mox_ai_expert_proto::types::ConsultQuery;
use std::collections::HashMap;

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

        // 2. 专家联盟咨询（失败降级为默认健康分）
        let mut expert_score = 1.0_f64;
        let mut expert_steps = Vec::new();
        let consultant = mox_ai_expert_svc::expert_traits::llm_consultant();
        let query = ConsultQuery {
            id: doc.id.clone(),
            query: format!("知识库文档分析：{}。{}", doc.title, summary),
            ctx: {
                let mut m = HashMap::new();
                m.insert("doc_id".into(), doc.id.clone());
                m.insert("category".into(), doc.category.clone());
                m.insert("doc_type".into(), "knowledge_base".into());
                m
            },
        };
        if let Ok(report) = consultant.consult(&query).await {
            expert_score = report.score;
            expert_steps = report.steps;
            if report.vetoed {
                expert_steps.push(format!("治理否决：{}", report.reason.unwrap_or_default()));
            }
        } else {
            expert_steps.push("本地分析引擎（专家联盟不可用，降级默认健康分）".into());
        }

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

/// 抽取实体：词频统计 + 停用词过滤 + 类型启发
fn extract_entities(title: &str, content: &str) -> (Vec<KbEntity>, Vec<String>) {
    let text = format!("{title} {content}");
    let tokens = tokenize(&text);
    let mut freq = HashMap::<String, usize>::new();
    for t in &tokens {
        if t.chars().count() < 2 || t.chars().count() > 8 {
            continue;
        }
        if STOPWORDS.contains(&t.to_lowercase().as_str()) {
            continue;
        }
        *freq.entry(t.clone()).or_default() += 1;
    }
    // 排序取前 12
    let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(12);

    let mut entities = Vec::new();
    let mut keywords = Vec::new();
    for (i, (name, count)) in ranked.into_iter().enumerate() {
        let entity_type = classify_entity(&name);
        if entity_type == ET_TECH || count >= 2 {
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
        .find(|l| !l.is_empty())
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
}




