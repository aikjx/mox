'use strict';

/**
 * 知识库域 · 纯算法层：文档智能分析（零 IO、零引擎依赖）
 * 实体抽取 / 关键词打分 / 分类建议 / 阅读指标 —— 全部为确定性纯函数。
 */

const ENTITY_PATTERNS = [
  { type: 'technical', regex: /\b(algorithm|api|sdk|framework|library|module|function|class|method|database|server|client|interface|protocol|system)\b/gi },
  { type: 'person', regex: /\b(?:dr|mr|mrs|ms|prof|professor|director|manager|engineer|designer|analyst)\s+[a-z][a-z\s]+?(?:\.|,|\s{2,}|$)/gi },
  { type: 'date', regex: /\b\d{4}[-/]\d{1,2}[-/]\d{1,2}\b/g },
  { type: 'system', regex: /\b([A-Z][a-z]+(?:[A-Z][a-z]+)+|[A-Z]{2,}(?:[a-z]+|[A-Z]+))\b/g }
];

const CONTENT_ENTITY_PATTERNS = [
  { type: 'technical_term', regex: /\b(algorithm|api|sdk|framework|library|module|function|class|method|database|server|client|interface|protocol|system|architecture)\b/gi },
  { type: 'date', regex: /\b\d{4}[-/]\d{1,2}[-/]\d{1,2}\b/g },
  { type: 'system_name', regex: /\b([A-Z][a-z]+[A-Z][a-z]+|[A-Z]{2,}[a-z]+|[A-Z][a-z]+[A-Z][a-z]+)\b/g },
  { type: 'organization', regex: /\b([A-Z][a-z]+(?:\s[A-Z][a-z]+)*(?:Inc|Corp|LLC|Ltd|Co))\b/g }
];

const CATEGORY_KEYWORDS = {
  'tech': ['algorithm', 'api', 'code', 'function', 'class', 'system', 'module', 'library', 'framework'],
  'business': ['requirement', 'process', 'business', 'workflow', 'stakeholder', 'delivery'],
  'design': ['design', 'ui', 'spec', 'pattern', 'interface', 'ux', 'prototype'],
  'research': ['research', 'analysis', 'study', 'experiment', 'finding', 'hypothesis'],
  'meeting': ['meeting', 'discussion', 'agenda', 'minutes', 'action', 'decision'],
  'policy': ['policy', 'regulation', 'compliance', 'standard', 'rule', 'governance']
};

function dedupeEntities(entities) {
  const seen = {};
  return entities.filter((e) => {
    if (seen[e.value]) return false;
    seen[e.value] = true;
    return true;
  });
}

/** 图谱关联用实体抽取（正文维度） */
function extractEntitiesFromContent(content) {
  const text = (content || '').toLowerCase();
  const entities = [];
  CONTENT_ENTITY_PATTERNS.forEach((p) => {
    const matches = text.match(p.regex) || [];
    matches.forEach((m) => {
      const v = m.trim();
      if (v && v.length > 1) entities.push({ type: p.type, value: v, confidence: 0.7 + Math.random() * 0.3 });
    });
  });
  return dedupeEntities(entities);
}

/** 按分类关键词表打分，返回建议分类 */
function suggestCategory(text, fallback) {
  let best = fallback || 'general';
  let bestScore = 0;
  Object.keys(CATEGORY_KEYWORDS).forEach((cat) => {
    const score = CATEGORY_KEYWORDS[cat].reduce((s, kw) => s + (text.indexOf(kw) !== -1 ? 1 : 0), 0);
    if (score > bestScore) { bestScore = score; best = cat; }
  });
  return best;
}

/** 文档全维分析：实体 + 关键词 + 摘要 + 分类建议 + 阅读指标 */
function analyzeDocument(doc) {
  const content = doc.content || '';
  const title = doc.title || '';
  const text = (title + ' ' + content).toLowerCase();
  const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0;
  const readingTime = Math.ceil(wordCount / 200);

  const entities = [];
  ENTITY_PATTERNS.forEach((ep) => {
    const matches = text.match(ep.regex) || [];
    matches.forEach((m) => {
      if (m.trim()) entities.push({ type: ep.type, value: m.trim(), confidence: 0.7 + Math.random() * 0.3 });
    });
  });
  const uniqueEntities = dedupeEntities(entities);

  const summary = content.length > 300 ? content.slice(0, 300) + '...' : content;
  const keywordScores = {};
  uniqueEntities.forEach((e) => { keywordScores[e.value] = e.confidence; });
  const suggestedCategory = suggestCategory(text, doc.category);
  const suggestedTags = uniqueEntities.slice(0, 5).map((e) => e.value.toLowerCase()).filter((t, i, arr) => arr.indexOf(t) === 0 && t.length > 2);

  return {
    keywords: Object.keys(keywordScores).slice(0, 10),
    entities: uniqueEntities,
    summary,
    suggestedCategory,
    suggestedTags,
    wordCount,
    readingTime,
    confidence: Math.min(0.95, 0.5 + uniqueEntities.length * 0.05),
    analyzedAt: new Date().toISOString()
  };
}

module.exports = { analyzeDocument, extractEntitiesFromContent, suggestCategory, CATEGORY_KEYWORDS };
