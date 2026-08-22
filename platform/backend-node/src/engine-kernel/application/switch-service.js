'use strict';

/**
 * 瞬间切换用例（application 层 · mixin 用例族）
 * ------------------------------------------------------------------
 * 切换引擎 = 换绑定，零代码改动：
 *   校验（槽位存在 + 引擎在候选清单）→ 应用（适配器 apply）→ 探活（契约 health）
 *   → 回报（前后绑定 + 探活结果）。失败自动回滚到原绑定，保证银行级不宕机。
 */

const { SLOT_INDEX } = require('../domain/contract-registry');
const { ADAPTERS, getBinding, setBinding, getBindings } = require('../infrastructure/plugin-repository');

/** 槽位全景：契约摘要 + 当前绑定 + 候选引擎（一屏尽览） */
function getSlots() {
  return Object.values(SLOT_INDEX).map(slot => {
    const adapter = ADAPTERS[slot.adapter];
    const candidates = adapter ? adapter.list() : [];
    const current = adapter ? adapter.current() : null;
    return {
      id: slot.id,
      name: slot.name,
      category: slot.category,
      hotSwap: slot.hotSwap,
      description: slot.description,
      contract: {
        methods: slot.contract.methods.map(m => ({ name: m.name, input: m.input, output: m.output })),
        switchExample: slot.contract.switchExample
      },
      currentEngineId: current,
      currentEngineName: (candidates.find(c => c.id === current) || {}).name || current,
      candidates
    };
  });
}

/** 单槽位契约文档（接口规范原文） */
function getContract(slotId) {
  const slot = SLOT_INDEX[slotId];
  if (!slot) return null;
  const adapter = ADAPTERS[slot.adapter];
  return {
    ...slot,
    currentEngineId: adapter ? adapter.current() : null,
    candidates: adapter ? adapter.list() : []
  };
}

/**
 * 瞬间切换（核心用例）
 * @param {string} slotId 槽位 ID
 * @param {string} engineId 目标引擎 ID
 * @param {object} options { verify: 是否切换后探活（默认 true） }
 */
async function switchEngine(slotId, engineId, options = {}) {
  const slot = SLOT_INDEX[slotId];
  if (!slot) return { ok: false, error: `槽位不存在: ${slotId}（可用: ${Object.keys(SLOT_INDEX).join(',')}）` };

  const adapter = ADAPTERS[slot.adapter];
  if (!adapter) return { ok: false, error: `槽位 ${slotId} 的适配器未实现: ${slot.adapter}` };

  const candidates = adapter.list();
  const target = candidates.find(c => c.id === engineId);
  if (!target) {
    return { ok: false, error: `引擎 ${engineId} 不在槽位 ${slotId} 候选清单（可用: ${candidates.map(c => c.id).join(',')}）` };
  }

  const before = adapter.current();
  let applied = false;
  try {
    applied = adapter.apply(engineId);
  } catch (e) {
    return { ok: false, error: `切换失败: ${e.message}`, before };
  }
  if (!applied) return { ok: false, error: `引擎 ${engineId} 应用失败（适配器拒绝）`, before };

  // 探活（verify=false 跳过；探活失败自动回滚，银行级保障）
  let health = { ok: true, skipped: true };
  if (options.verify !== false && typeof adapter.health === 'function') {
    try {
      health = await adapter.health(engineId);
    } catch (e) {
      health = { ok: false, error: e.message };
    }
    if (!health.ok) {
      adapter.apply(before);
      return { ok: false, error: `切换后探活失败，已回滚至 ${before}`, before, after: engineId, health };
    }
  }

  setBinding(slotId, engineId);
  return {
    ok: true,
    slot: slotId,
    before,
    after: engineId,
    engineName: target.name,
    health,
    switchedAt: new Date().toISOString()
  };
}

/** 全部当前绑定（含未持久化但适配器可推导的实时绑定） */
function getBindingsView() {
  const persisted = getBindings();
  const live = {};
  Object.values(SLOT_INDEX).forEach(slot => {
    const adapter = ADAPTERS[slot.adapter];
    live[slot.id] = adapter ? adapter.current() : null;
  });
  return { persisted, live, consistent: Object.keys(live).every(k => !persisted[k] || persisted[k] === live[k]) };
}

/** 契约兼容性预检：不切换，只探活目标引擎 */
async function validateEngine(slotId, engineId) {
  const slot = SLOT_INDEX[slotId];
  if (!slot) return { ok: false, error: `槽位不存在: ${slotId}` };
  const adapter = ADAPTERS[slot.adapter];
  const candidates = adapter ? adapter.list() : [];
  const target = candidates.find(c => c.id === engineId);
  if (!target) return { ok: false, error: `引擎 ${engineId} 不在候选清单` };
  let health = { ok: false, error: '适配器无探活实现' };
  try { health = await adapter.health(engineId); } catch (e) { health = { ok: false, error: e.message }; }
  return { ok: !!health.ok, slot: slotId, engineId, engineName: target.name, health };
}

module.exports = { getSlots, getContract, switchEngine, getBindingsView, validateEngine };
