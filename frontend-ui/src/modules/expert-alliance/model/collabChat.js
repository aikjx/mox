// 协作结果的纯文本投影：聊天型界面（工作台对话栏、AI 对话）只要一段可信文字，
// 结构化的六模式结果渲染仍归 components/ExpertCollabPanel.vue。
// 字段口径全部来自 contract/collab.js 的 resultKind 与 model/normalize.js 的归一化器，
// 这里不重复后端规则，也不补写后端没产出的内容。
import { collabMode, collabTemplateNote } from '@/modules/expert-alliance/contract'
import { collabOutcome } from './normalize.js'

/** 发言者显示名：优先后端给的真实专家名，缺则回落到模式中文名（不编造人名） */
export function collabChatSpeaker(result) {
  const r = result || {}
  if (collabOutcome(r) === 'blocked') return '质量闸门'
  const def = collabMode(r.mode)
  if (def?.resultKind === 'answer') {
    return r.expertName || r.expert?.name || '匹配专家'
  }
  return def?.label || '联盟协作'
}

/** 结果 → 聊天正文；按 resultKind 只取该模式确实产出的字段 */
export function collabChatText(result) {
  const r = result || {}
  if (collabOutcome(r) === 'blocked') {
    return r.answer?.vetoReason || r.fusion?.vetoReason || '结果被质量闸门拦截，未产出可用结论'
  }
  const kind = collabMode(r.mode)?.resultKind || ''
  if (kind === 'routing') {
    const names = (r.candidates || []).map((c) => c.name).filter(Boolean)
    return `路由出 ${names.length} 位候选专家${names.length ? '：' + names.join('、') : ''}。`
      + `${r.recommendation?.reason ? ' 推荐依据：' + r.recommendation.reason : ''}`
      + '（路由只排序作答，不产出回复）'
  }
  if (kind === 'fusion') {
    return `${(r.contributions || []).length} 位专家作答：${r.fusion?.summary || '（后端未返回融合摘要）'}`
  }
  if (kind === 'debate') {
    return `辩论 ${r.rounds ?? 0} 轮，${r.verdict?.winner || '未产生胜方'}胜出：`
      + `${r.verdict?.summary || '（后端未返回裁决摘要）'}`
      + collabTemplateNote(collabMode(r.mode))
  }
  if (kind === 'complexity') {
    return `复杂度 ${r.complexity?.bigO || '未知'}：${r.complexity?.explanation || ''}`
      + `${(r.suggestions || []).length ? ` 另附 ${r.suggestions.length} 条建议，详见智能协作工作台。` : ''}`
  }
  if (kind === 'answer') {
    const a = r.answer || {}
    const body = [a.analysis, a.solution].filter(Boolean).join('\n\n') || '（该专家未返回正文）'
    // source==='llm' 才是真模型；空串＝模板降级，二者可信度不同必须写在正文里
    return a.source === 'llm' ? body : `${body}\n\n（模板降级回复，未经真实模型）`
  }
  return '协作已完成，但结果字段不在前端契约覆盖内，请到智能协作工作台查看。'
}

/** 结果 → 聊天正文里的阶段戳（供带阶段流水线的界面复用，不臆造中间阶段） */
export function collabChatPhase(result) {
  if (collabOutcome(result) === 'blocked') return 'gate'
  const kind = collabMode(result?.mode)?.resultKind || ''
  return { routing: 'team', fusion: 'synthesize', debate: 'debate', complexity: 'synthesize', answer: 'done' }[kind] || 'done'
}
