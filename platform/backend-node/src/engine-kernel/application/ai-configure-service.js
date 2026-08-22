'use strict';

/**
 * AI 自动配置用例（application 层 · mixin 用例族）
 * ------------------------------------------------------------------
 * 「叫 AI 去配置」：把槽位契约 + 候选引擎 + 当前绑定作为上下文交给 LLM，
 * 要求输出严格 JSON 的绑定方案；支持 dryRun（仅出方案）与 autoApply（直接切换）。
 * LLM 输出经 JSON 解析容错 + 候选合法性校验，非法绑定一律拒绝——AI 只能
 * 在机器验证过的候选清单内做决策，不能凭空造引擎。
 */

const { SLOT_INDEX } = require('../domain/contract-registry');
const { ADAPTERS, getBindings } = require('../infrastructure/plugin-repository');
const switchService = require('./switch-service');

/** 构建给 LLM 的槽位上下文（契约 + 候选 + 当前绑定） */
function _buildSlotContext() {
  return Object.values(SLOT_INDEX).map(slot => {
    const adapter = ADAPTERS[slot.adapter];
    const candidates = adapter ? adapter.list().map(c => ({ id: c.id, name: c.name, enabled: c.enabled })) : [];
    return {
      slot: slot.id,
      用途: slot.description,
      当前引擎: adapter ? adapter.current() : null,
      候选引擎: candidates
    };
  });
}

function _systemPrompt() {
  return [
    '你是本系统的引擎配置专家。系统采用"槽位契约"架构：每类能力是一个槽位，',
    '槽位内的引擎可瞬间切换（零代码改动）。你的任务：根据用户需求，从候选引擎清单中选择最合适的引擎绑定方案。',
    '',
    '规则：',
    '1. 只能从各槽位的候选引擎中选择，不得发明不存在的引擎',
    '2. 只输出需要变更的槽位，无需变更的不要输出',
    '3. 输出必须是纯 JSON（无 markdown 代码块包裹），格式：',
    '   {"plan":[{"slot":"槽位ID","engineId":"引擎ID","reason":"一句话理由"}]}',
    '4. 若需求与引擎配置无关，输出 {"plan":[]}'
  ].join('\n');
}

/** 解析 LLM 输出的 JSON（容错：剥离代码块围栏/前后杂文） */
function _parsePlan(text) {
  if (!text) return [];
  let t = String(text).trim();
  t = t.replace(/```(?:json)?/g, '').trim();
  const start = t.indexOf('{');
  const end = t.lastIndexOf('}');
  if (start < 0 || end <= start) return [];
  try {
    const obj = JSON.parse(t.slice(start, end + 1));
    return Array.isArray(obj.plan) ? obj.plan : [];
  } catch (e) {
    return [];
  }
}

/**
 * AI 自动配置
 * @param {string} requirement 自然语言需求（如"我要最省钱的中文对话引擎，搜索用国内的"）
 * @param {object} options { dryRun: true 仅出方案不切换（默认）, llmEngineId: 指定用哪个 LLM 决策 }
 */
async function aiConfigure(requirement, options = {}) {
  if (!requirement) return { ok: false, error: 'requirement 为必填' };
  const gateway = require('../../llm-gateway').getGateway();

  const messages = [
    { role: 'system', content: _systemPrompt() },
    { role: 'user', content: `用户需求：${requirement}\n\n系统槽位与候选引擎：\n${JSON.stringify(_buildSlotContext(), null, 2)}\n\n当前绑定：${JSON.stringify(getBindings())}` }
  ];

  let raw;
  try {
    const params = { messages, temperature: 0.2 };
    if (options.llmEngineId) {
      raw = await gateway.chatWithProvider(options.llmEngineId, params);
    } else {
      raw = await gateway.chat(params);
    }
  } catch (e) {
    return { ok: false, error: `LLM 决策失败: ${e.message}` };
  }

  const content = raw && (raw.content || raw.choices?.[0]?.message?.content) || '';
  const plan = _parsePlan(content);

  // 合法性校验：槽位存在 + 引擎在候选清单
  const validated = [];
  const rejected = [];
  for (const item of plan) {
    const slot = SLOT_INDEX[item.slot];
    const adapter = slot && ADAPTERS[slot.adapter];
    const candidate = adapter && adapter.list().find(c => c.id === item.engineId);
    if (slot && candidate) validated.push({ slot: item.slot, engineId: item.engineId, reason: item.reason || '', engineName: candidate.name });
    else rejected.push({ slot: item.slot, engineId: item.engineId, reason: '槽位不存在或引擎不在候选清单，已拒绝' });
  }

  const dryRun = options.dryRun !== false;
  const applied = [];
  if (!dryRun) {
    for (const v of validated) {
      const r = await switchService.switchEngine(v.slot, v.engineId);
      applied.push({ slot: v.slot, engineId: v.engineId, ok: r.ok, before: r.before, after: r.after, health: r.health });
    }
  }

  return {
    ok: true,
    dryRun,
    requirement,
    plan: validated,
    rejected,
    applied,
    raw: content.slice(0, 500)
  };
}

module.exports = { aiConfigure };
