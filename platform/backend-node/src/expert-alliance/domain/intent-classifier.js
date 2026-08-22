'use strict';

/**
 * 意图分类器（domain 层 · AINA A2 意图先行公理）
 * ------------------------------------------------------------------
 * 输入输出均为值对象，零 IO、零引擎依赖。
 * 被 expert-alliance.js（路由编排）与 alliance-engine（流水线第一阶段）共用。
 */
const { INTENT_PATTERNS } = require('./intent-patterns');

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
    const matchedKeywords = [];
    for (const kw of pattern.keywords) {
      const kwLower = kw.toLowerCase();
      if (text.includes(kwLower)) {
        score += 1;
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
    return { primary: 'general', secondary: [], confidence: 0, matchedKeywords: [] };
  }

  return {
    primary: sorted[0].intent,
    secondary: sorted.slice(1, 3).map(s => s.intent),
    confidence: sorted[0].score / (sorted[0].score + sorted[1]?.score || 1),
    matchedKeywords: sorted[0].matchedKeywords,
    allScores: Object.fromEntries(sorted.map(s => [s.intent, s.score]))
  };
}

module.exports = { detectIntent };
