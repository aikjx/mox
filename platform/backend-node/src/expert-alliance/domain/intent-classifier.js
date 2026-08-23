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
 * 检测问题文本的意图分布。
 * @param {string} question 用户问题
 * @returns {{primary:string, secondary:string[], confidence:number, matchedKeywords:string[], allScores:Object}}
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

  const sorted = Object.entries(scores)
    .sort((a, b) => b[1].score - a[1].score)
    .map(([intent, data]) => ({ intent, ...data }));

  if (sorted.length === 0) {
    return { primary: 'general', secondary: [], confidence: 0, matchedKeywords: [], allScores: {} };
  }

  const topScore = sorted[0].score;
  const runnerUpScore = sorted[1]?.score || 0;
  // 置信度计算：top分与次高分的差距占比
  const confidence = runnerUpScore > 0
    ? Math.min(1, topScore / (topScore + runnerUpScore))
    : Math.min(1, topScore / 3); // 无竞争时根据命中数计算

  return {
    primary: sorted[0].intent,
    secondary: sorted.slice(1, 3).map(s => s.intent),
    confidence: Math.round(confidence * 100) / 100,
    matchedKeywords: sorted[0].matchedKeywords,
    allScores: Object.fromEntries(sorted.map(s => [s.intent, s.score]))
  };
}

module.exports = { detectIntent, keywordMatches };
