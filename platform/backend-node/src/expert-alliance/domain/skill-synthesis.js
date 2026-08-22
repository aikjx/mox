'use strict';

/**
 * 学习技能沉淀（domain 层纯函数 · 零 IO）
 * ------------------------------------------------------------------
 * 职责：从一次联盟处理的完整结果（意图/团队/审议/综合/门禁）中
 *       提炼结构化学习技能记录，供 infrastructure 层持久化。
 *
 * 沉淀规则（企业级可审计语义）：
 *   1. 仅质量门禁 passed 的处理才产生技能（失败经验进 trace，不进技能库）
 *   2. 技能键 = 意图 + 团队签名（类型集合）：同键重复出现 → 强化（count+1，
 *      累计置信度），不产生重复记录（防技能库膨胀）
 *   3. 技能内容 = 关键洞察 + 建议摘要 + 门禁级别（供后续组队先验参考）
 *   4. 输出兼容既有 learned_skills 记录形态（id/name/extractedAt），
 *      叠加联盟特有字段（intent/team/gate）
 */

/** 团队签名：按专家类型排序去重（同构团队视为同一技能键） */
function teamSignature(team) {
  const types = (team || []).map(m => m.type || 'unknown').sort();
  return types.join('+') || 'solo';
}

/** 从综合结果提取建议摘要（截断防膨胀） */
function digestOf(synthesis) {
  if (!synthesis) return { insights: [], recommendations: [] };
  const clip = (arr) => (Array.isArray(arr) ? arr.slice(0, 3).map(s => String(s).slice(0, 120)) : []);
  return { insights: clip(synthesis.key_insights), recommendations: clip(synthesis.recommendations) };
}

/**
 * 提炼学习技能记录。
 * @param {object} params { question, intent, team, deliberation, synthesis, gate }
 * @param {Map|object} existing 既有技能库（键 → 记录），用于去重与强化
 * @returns {{ records: object[], merged: Map|object }} 新记录列表 + 合并后的技能库
 */
function synthesizeSkills(params, existing = new Map()) {
  const { question, intent, team, synthesis, gate } = params || {};
  if (!gate || !gate.passed) return { records: [], merged: existing };

  const key = `${intent.primary}::${teamSignature(team)}`;
  const digest = digestOf(synthesis);
  const now = new Date().toISOString();

  const prev = existing instanceof Map ? existing.get(key) : existing[key];
  if (prev) {
    // 强化：同键出现次数 +1，置信度指数平滑，摘要取最新（近因）
    prev.count = (prev.count || 1) + 1;
    prev.confidence = Math.round(((prev.confidence || 0.5) * 0.7 + (synthesis.confidence || 0.5) * 0.3) * 100) / 100;
    prev.lastSeenAt = now;
    prev.insights = digest.insights;
    prev.recommendations = digest.recommendations;
    if (existing instanceof Map) existing.set(key, prev); else existing[key] = prev;
    return { records: [], merged: existing };
  }

  const record = {
    id: `ea_skill_${Buffer.from(key).toString('base64url').slice(0, 16)}`,
    key,
    name: `${intent.primary} 咨询经验（${teamSignature(team)}）`,
    intent: intent.primary,
    team_signature: teamSignature(team),
    team_ids: (team || []).map(m => m.id),
    gate_level: gate.level,
    confidence: synthesis.confidence || 0.5,
    question_brief: String(question || '').slice(0, 120),
    insights: digest.insights,
    recommendations: digest.recommendations,
    count: 1,
    extractedAt: now,
    lastSeenAt: now
  };

  if (existing instanceof Map) existing.set(key, record); else existing[key] = record;
  return { records: [record], merged: existing };
}

/** 技能库视图：按强化次数排序（组队先验参考） */
function rankSkills(store, limit = 20) {
  const list = store instanceof Map ? Array.from(store.values()) : Object.values(store || {});
  return list.sort((a, b) => (b.count || 1) - (a.count || 1)).slice(0, limit);
}

module.exports = { synthesizeSkills, rankSkills, teamSignature };
