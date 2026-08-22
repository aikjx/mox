'use strict';

/**
 * 专家匹配与评分（domain 层 · 纯算法）
 * ------------------------------------------------------------------
 * 匹配打分规则（类型匹配 10 / 次要意图 5 / 能力命中 3 / 关键词 2），
 * 评分 = 匹配分 + 性能加分(成功率×3) + 置信度加分(平均置信度×2)。
 */

/**
 * 基于问题与意图结果筛选候选专家。
 * @param {Array} experts 活跃专家列表（值对象）
 * @param {string} question 用户问题
 * @param {Object} intentResult detectIntent 输出
 * @returns {Array<{expert:Object, matchScore:number, matchReasons:string[]}>}
 */
function matchExperts(experts, question, intentResult) {
  const candidates = [];

  for (const expert of experts) {
    let matchScore = 0;
    const matchReasons = [];

    if (expert.type === intentResult.primary) {
      matchScore += 10;
      matchReasons.push('类型匹配');
    }

    if (intentResult.secondary.includes(expert.type)) {
      matchScore += 5;
      matchReasons.push('次要意图匹配');
    }

    const text = (question || '').toLowerCase();
    for (const cap of expert.capabilities) {
      if (text.includes(cap.toLowerCase())) {
        matchScore += 3;
        matchReasons.push(`能力匹配: ${cap}`);
      }
    }

    for (const kw of intentResult.matchedKeywords) {
      if (expert.capabilities.some(c => c.includes(kw))) {
        matchScore += 2;
      }
      if (expert.name.includes(kw)) {
        matchScore += 2;
      }
    }

    if (matchScore > 0) {
      candidates.push({ expert, matchScore, matchReasons });
    }
  }

  if (candidates.length === 0) {
    return experts.map(e => ({ expert: e, matchScore: 1, matchReasons: ['默认匹配'] }));
  }

  return candidates;
}

/**
 * 综合匹配分与历史表现对候选专家评分排序。
 * @param {Array} candidates matchExperts 输出
 * @param {function(string): Object} statsOf 专家 id → 历史统计（端口注入，避免触碰存储）
 * @returns {Array} 评分降序的 {expert, score, breakdown, reasons}
 */
function scoreExperts(candidates, statsOf) {
  return candidates
    .map(({ expert, matchScore, matchReasons }) => {
      const stats = statsOf(expert.id) || {};
      const performanceBonus = (stats.success_rate || 1.0) * 3;
      const confidenceBonus = (stats.avg_confidence || 0.7) * 2;

      const totalScore = matchScore + performanceBonus + confidenceBonus;

      return {
        expert,
        score: totalScore,
        breakdown: {
          match: matchScore,
          performance: performanceBonus,
          confidence: confidenceBonus
        },
        reasons: matchReasons
      };
    })
    .sort((a, b) => b.score - a.score);
}

module.exports = { matchExperts, scoreExperts };
