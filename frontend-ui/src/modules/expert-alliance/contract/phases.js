// 联盟 7 阶段管线契约。权威源为 Rust 单源，本文件只做前端投影，不得增删改序：
// platform/shared/mox-unified-contract/src/event.rs (PHASE_NAMES / AUDIT_EVENTS_7)
// 漂移由 contract/contract.test.js 直接读取 Rust 源做断言守护。

export const PHASE_IDS = Object.freeze([
  'intent',
  'team',
  'debate',
  'synthesize',
  'gate',
  'learn',
  'done'
])

export const PHASE_META = Object.freeze({
  intent: { index: 1, label: '意图识别', audit: 'INTENT_DONE' },
  team: { index: 2, label: '组队路由', audit: 'TEAM_DONE' },
  debate: { index: 3, label: '并行咨询 + 辩论', audit: 'DEBATE_DONE' },
  // 7 类审计以 ALLIANCE_START 起始、无 SYNTHESIZE_DONE，故合成阶段不带审计事件。
  synthesize: { index: 4, label: '归一合成', audit: null },
  gate: { index: 5, label: '质量门禁', audit: 'GATE_DONE' },
  learn: { index: 6, label: '指标学习', audit: 'LEARN_DONE' },
  done: { index: 7, label: '终态', audit: 'ALLIANCE_DONE' }
})

// event.rs AUDIT_EVENTS_7（FR-CORE-07，缺任意一项企业基线不过）
export const AUDIT_EVENTS_7 = Object.freeze([
  'ALLIANCE_START',
  'INTENT_DONE',
  'TEAM_DONE',
  'DEBATE_DONE',
  'GATE_DONE',
  'LEARN_DONE',
  'ALLIANCE_DONE'
])

export const ALLIANCE_START_AUDIT = 'ALLIANCE_START'

export function isPhaseId(v) {
  return PHASE_IDS.includes(v)
}

export function phaseMeta(id) {
  return PHASE_META[id] ?? null
}

export function phaseLabel(id) {
  return PHASE_META[id]?.label ?? id ?? ''
}

/**
 * 由阶段序号（1..7）取 id，越界返回 null。
 */
export function phaseIdByIndex(index) {
  return PHASE_IDS[index - 1] ?? null
}

export function phaseProgress(doneCount) {
  const ratio = Math.max(0, Math.min(PHASE_IDS.length, Number(doneCount) || 0)) / PHASE_IDS.length
  return Math.round(ratio * 100)
}
