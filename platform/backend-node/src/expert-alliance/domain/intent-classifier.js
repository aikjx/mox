'use strict';

/**
 * 意图分类器（domain 层 · AINA A2 意图先行公理）
 * ------------------------------------------------------------------
 * 输入输出均为值对象，零 IO、零引擎依赖。
 * 被 expert-alliance.js（路由编排）与 alliance-engine（流水线第一阶段）共用。
 *
 * 匹配算法：
 *   1. 单关键词直接包含匹配
 *   2. 多词短语采用词序不敏感匹配（所有词都出现即命中）
 *   3. 中文词直接包含匹配
 *   4. 支持大小写不敏感
 */
const { INTENT_PATTERNS } = require('./intent-patterns');

/**
 * 检查文本是否包含关键词（支持多词短语）
 * @param {string} text 小写化文本
 * @param {string} keyword 关键词（可能含空格）
 * @returns {boolean}
 */
function keywordMatches(text, keyword) {
  const kw = keyword.toLowerCase().trim();
  if (!kw) return false;
  // 单关键词直接包含匹配
  if (!kw.includes(' ')) {
    return text.includes(kw);
  }
  // 多词短语：所有词都出现（词序不敏感，支持短语拆分匹配）
  const words = kw.split(/\s+/).filter(w => w.length > 1);
  if (words.length === 0) return false;
  return words.every(w => text.includes(w));
}

/**
 * [C3 单一真源 · 轻量映射表] 领域意图 → 统一编排能力（AIEC capability）。
 * 图谱激活扩散与关键词打分共享此映射，确保 top-1 决策一致性（T8 对账约束）。
 * 保持 domain 层零 IO：纯对象字面量，不引用引擎文件。
 */
const INTENT_TO_CAPABILITY = {
  // AI-1 深度推理能力
  'ai': 'reasoning',
  'requirement': 'reasoning',
  // AI-2 专家联盟协作能力
  'fusion': 'expert',
  'automation': 'expert',
  'operator': 'expert',
  'workflow': 'workflow',
  // AI-3 图谱分析能力
  'graph': 'graph',
  'data': 'graph',
  'performance': 'graph',
  'monitor': 'graph',
  'algorithm': 'graph',
  'architecture': 'graph',
  'security': 'graph',
  'mcp': 'graph',
  'market': 'graph',
  // 其余（general/chat 等）→ chat 兜底
};
const CAPABILITY_DEFAULT = 'chat';

/**
 * 将领域意图映射为统一编排能力（单一真源映射）。
 * @param {string} intent 领域意图（来自 intent-patterns）
 * @returns {string} capability
 */
function toCapability(intent) {
  if (!intent) return CAPABILITY_DEFAULT;
  return INTENT_TO_CAPABILITY[intent] || CAPABILITY_DEFAULT;
}

/**
 * 检测问题文本的意图分布。
 * 说明：返回中的 primary 是"领域意图"（intent-patterns 中的 intent 字段）。
 * 若需要统一编排能力，请使用 `toCapability(result.primary)` 或调用 wrapper。
 * @param {string} question 用户问题
 * @returns {{primary:string, secondary:string[], confidence:number, matchedKeywords:string[], allScores:Object, capability:string}}
 */
function detectIntent(question) {
  const text = (question || '').toLowerCase();
  const scores = {};

  for (const pattern of INTENT_PATTERNS) {
    let score = 0;
    const seen = new Set(); // 大小写不敏感去重：中英文区重复登记的同一关键词只计一次分
    const matchedKeywords = [];
    for (const kw of pattern.keywords) {
      const kwLower = kw.toLowerCase().trim();
      if (seen.has(kwLower)) continue;
      if (keywordMatches(text, kw)) {
        seen.add(kwLower);
        // 多词短语权重更高（更精确）
        const weight = kw.includes(' ') ? 2 : 1;
        score += weight;
        matchedKeywords.push(kw);
      }
    }
    if (score > 0) {
      scores[pattern.intent] = { score, matchedKeywords };
    }
  }

  let sorted = Object.entries(scores)
    .sort((a, b) => b[1].score - a[1].score)
    .map(([intent, data]) => ({ intent, ...data }));

  // --- [T8 对账修复] 同义意图按 capability 归并，避免"ai=1"与"general=0"的漂移 ---
  // 同一 capability 下多个 intent 的分数加总后再选 top capability，
  // 然后在该 capability 内取分最高的领域 intent 作为 primary（保持原字段语义）。
  const capBuckets = {}; // capability -> { total, intents: [{intent, score, matchedKeywords}] }
  for (const s of sorted) {
    const cap = toCapability(s.intent);
    if (!capBuckets[cap]) capBuckets[cap] = { total: 0, intents: [] };
    capBuckets[cap].total += s.score;
    capBuckets[cap].intents.push(s);
  }
  const capRanking = Object.entries(capBuckets)
    .sort((a, b) => b[1].total - a[1].total);

  let primary = 'general';
  let matchedKeywords = [];
  if (capRanking.length > 0) {
    const topCap = capRanking[0][0];
    const topIntents = capBuckets[topCap].intents.sort((a, b) => b.score - a.score);
    primary = topIntents[0].intent;
    matchedKeywords = topIntents[0].matchedKeywords;
    // 重排 sorted：按 bucket 总分 优先（保证 secondary 也符合 capability 层的决策顺序）
    const capOrder = Object.fromEntries(capRanking.map(([cap], i) => [cap, i]));
    sorted = sorted.slice().sort((a, b) => {
      const ca = capOrder[toCapability(a.intent)] ?? 99;
      const cb = capOrder[toCapability(b.intent)] ?? 99;
      if (ca !== cb) return ca - cb;
      return b.score - a.score;
    });
  }

  if (sorted.length === 0) {
    return {
      primary: 'general',
      secondary: [],
      confidence: 0,
      matchedKeywords: [],
      allScores: {},
      capability: CAPABILITY_DEFAULT,
    };
  }

  const topScore = capRanking[0][1].total;
  const runnerUpScore = capRanking[1]?.[1].total || 0;
  const confidence = runnerUpScore > 0
    ? Math.min(1, topScore / (topScore + runnerUpScore))
    : Math.min(1, topScore / 3);

  return {
    primary,
    secondary: sorted.slice(1, 3).map(s => s.intent),
    confidence: Math.round(confidence * 100) / 100,
    matchedKeywords,
    allScores: Object.fromEntries(sorted.map(s => [s.intent, s.score])),
    capability: toCapability(primary),
  };
}

/**
 * 轻量级公开 API：直接返回"统一能力"（capability），避免调用方各自映射。
 * @param {string} question
 * @returns {{capability:string, intent:string, confidence:number, matchedKeywords:string[], allScores:Object}}
 */
function intentClassify(question) {
  const r = detectIntent(question);
  return {
    capability: r.capability,
    intent: r.primary,
    confidence: r.confidence,
    matchedKeywords: r.matchedKeywords,
    allScores: r.allScores,
  };
}

module.exports = { detectIntent, keywordMatches, intentClassify, toCapability, INTENT_TO_CAPABILITY };
