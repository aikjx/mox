// 协作结果 → 列表型界面（多专家结果区、辩论逐轮流水、路由候选区）的行投影。
// 存在的理由：视图不许各自摸后端字段名——字段口径在这里出现一次，
// 由 model/normalize.js 的归一化结果喂进来，视图只读投影后的行。
import { answerSourceText, collabMode, collabTemplateNote } from '@/modules/expert-alliance/contract'
import { expertVisualKey } from './display.js'

const SIDE_LABEL = { pro: '正方', con: '反方' }

function answerText(answer) {
  const a = answer || {}
  return [a.analysis, a.solution].filter(Boolean).join('\n\n') || '（该专家未返回正文）'
}

/** 多专家协同 → 每位专家一行；modelBacked=false 表示该条是模板降级回复 */
export function collabContributionItems(result) {
  return (result?.contributions || []).map((c, i) => ({
    key: c.id || `c-${i}`,
    // id 单独留一列：视图要按它回查花名册取配色，不能拿 key 当身份（key 有兜底序）
    id: c.id || '',
    name: c.name || '专家',
    text: answerText(c.answer),
    confidence: Number(c.answer?.confidence) || 0,
    modelBacked: c.answer?.source === 'llm',
    // 文案由契约层单源给出，视图不再各自写一遍「真实模型/模板兜底」
    sourceText: answerSourceText(c.answer),
    blocked: !!c.answer?.blocked || !!c.answer?.vetoed
  }))
}

/** 辩论 → 逐轮正反方发言行（发言人名取自 participants 的 side，缺则只写方名） */
export function collabDebateTurns(result) {
  const namesBySide = {}
  for (const p of result?.participants || []) {
    if (!p?.side || !p.name) continue
    namesBySide[p.side] = namesBySide[p.side] ? `${namesBySide[p.side]}、${p.name}` : p.name
  }
  const turns = []
  for (const r of result?.log || []) {
    for (const side of ['pro', 'con']) {
      const text = side === 'pro' ? r.proArgument : r.conArgument
      if (!text) continue
      turns.push({
        key: `${r.round}-${side}`,
        round: Number(r.round) || 0,
        side,
        sideLabel: SIDE_LABEL[side] || side,
        name: namesBySide[side] || SIDE_LABEL[side] || side,
        text,
        score: Number(side === 'pro' ? r.proScore : r.conScore) || 0
      })
    }
  }
  return turns
}

/** 路由 → 候选专家行；路由只排序不作答，所以行里没有任何"回复"字段 */
export function collabCandidateItems(result) {
  const rec = result?.recommendation || {}
  return (result?.candidates || []).map((c, i) => ({
    key: c.id || `r-${i}`,
    id: c.id,
    name: c.name || '专家',
    subtitle: c.title || (c.domains || []).join(' / '),
    // 配色键取自首个领域（16 色板按领域命名），视图不再自己从展示文案里猜键
    visualKey: expertVisualKey(c),
    matchScore: Number(c.matchScore) || 0,
    recommended: !!rec.expertId && rec.expertId === c.id,
    reason: !!rec.expertId && rec.expertId === c.id ? rec.reason : '',
    status: c.status,
    avgRating: Number(c.avgRating) || 0
  }))
}

/** 结果顶部标注：该模式若为模板产出，界面必须显示一行说明 */
export function collabResultNote(result) {
  return collabTemplateNote(collabMode(result?.mode))
}
