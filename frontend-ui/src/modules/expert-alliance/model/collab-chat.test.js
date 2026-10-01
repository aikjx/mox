// 聊天型界面（工作台对话栏、AI 对话）拿到的段落文本只在 model/collabChat.js 产一次，
// 本文件钉住三件事：每种 resultKind 都真的走到自己的口径（不许掉进兜底分支）、
// 后端没产出的字段必须显式写成「未返回」而不是被省略、阶段戳取自 contract/phases.js。
import { describe, it, expect } from 'vitest'
import { COLLAB_MODE, COLLAB_MODES, collabMode } from '../contract/collab.js'
import { PHASE_IDS } from '../contract/phases.js'
import { collabChatPhase, collabChatSpeaker, collabChatText } from './collabChat.js'

const FALLBACK_TEXT = '协作已完成，但结果字段不在前端契约覆盖内，请到智能协作工作台查看。'

// 每个模式一份「后端确实会产出的形状」，字段名取自 contract/collab.js 的 resultKind 口径
const FIXTURES = {
  [COLLAB_MODE.ROUTE]: {
    result: { mode: 'route', candidates: [{ name: '甲' }, { name: '乙' }], recommendation: { reason: '评分最高' } },
    textIncludes: ['路由出 2 位候选专家', '甲、乙', '推荐依据：评分最高', '（路由只排序作答，不产出回复）'],
    phase: 'team'
  },
  [COLLAB_MODE.SINGLE]: {
    result: { mode: 'single', expertName: '专家甲', answer: { analysis: '分析段', solution: '方案段', source: 'llm' } },
    textIncludes: ['分析段', '方案段'],
    textExcludes: ['（模板降级回复，未经真实模型）'],
    speaker: '专家甲',
    phase: 'done'
  },
  [COLLAB_MODE.MULTI]: {
    result: { mode: 'multi', contributions: [{}, {}, {}], fusion: { summary: '融合结论' } },
    textIncludes: ['3 位专家作答', '融合结论'],
    phase: 'synthesize'
  },
  [COLLAB_MODE.DEBATE]: {
    result: { mode: 'debate', rounds: 2, verdict: { winner: '正方', summary: '论据更实' } },
    textIncludes: ['辩论 2 轮', '正方胜出', '论据更实'],
    phase: 'debate'
  },
  [COLLAB_MODE.SMART]: {
    result: { mode: 'smart', expert: { name: '自动匹配专家' }, answer: { analysis: '仅分析', source: '' } },
    textIncludes: ['仅分析', '（模板降级回复，未经真实模型）'],
    speaker: '自动匹配专家',
    phase: 'done'
  },
  [COLLAB_MODE.ALGORITHM]: {
    result: { mode: 'algorithm', complexity: { bigO: 'O(n)', explanation: '线性' }, suggestions: [{}, {}] },
    textIncludes: ['复杂度 O(n)', '线性', '另附 2 条建议'],
    phase: 'synthesize'
  }
}

describe('协作结果的聊天文本投影（model/collabChat.js）', () => {
  it('夹具覆盖契约里的全部模式，一个都不许漏', () => {
    const declared = COLLAB_MODES.map((m) => m.key).sort()
    expect(Object.keys(FIXTURES).sort()).toEqual(declared)
  })

  it('每种模式都走到自己的口径：不掉兜底、含约定片段、阶段戳在七阶段内', () => {
    for (const [mode, fx] of Object.entries(FIXTURES)) {
      const text = collabChatText(fx.result)
      expect(text, mode).not.toBe(FALLBACK_TEXT)
      for (const frag of fx.textIncludes) expect(text, `${mode} 缺片段 ${frag}`).toContain(frag)
      for (const frag of fx.textExcludes || []) expect(text, `${mode} 不该出现 ${frag}`).not.toContain(frag)
      expect(text, mode).not.toContain('undefined')
      expect(text, mode).not.toContain('[object')
      const phase = collabChatPhase(fx.result)
      expect(PHASE_IDS, `阶段戳 ${phase} 不在契约内`).toContain(phase)
      expect(phase, mode).toBe(fx.phase)
      if (fx.speaker) expect(collabChatSpeaker(fx.result), mode).toBe(fx.speaker)
    }
  })

  it('发言者标签：answer 类取后端专家名，其余取模式中文名，绝不编造人名', () => {
    for (const mode of Object.keys(FIXTURES)) {
      const def = collabMode(mode)
      const named = collabChatSpeaker({ ...FIXTURES[mode].result, expertName: '有名字' })
      if (def.resultKind === 'answer') expect(named, mode).toBe('有名字')
      else expect(named, `${mode} 不该把专家名当发言者`).toBe(def.label)
    }
    expect(collabChatSpeaker({ mode: 'single', answer: {} })).toBe('匹配专家')
  })

  it('后端缺字段时显式说明「未返回」，而不是把空值当内容渲染', () => {
    expect(collabChatText({ mode: 'multi', contributions: [{}] })).toContain('（后端未返回融合摘要）')
    expect(collabChatText({ mode: 'debate' })).toContain('未产生胜方')
    expect(collabChatText({ mode: 'debate' })).toContain('（后端未返回裁决摘要）')
    expect(collabChatText({ mode: 'algorithm' })).toContain('复杂度 未知')
    expect(collabChatText({ mode: 'single', answer: { source: 'llm' } })).toContain('（该专家未返回正文）')
    expect(collabChatText({ mode: 'route' })).toContain('路由出 0 位候选专家')
  })

  it('质量闸门拦截优先于模式：发言者、正文、阶段都指向 gate', () => {
    const blocked = { mode: 'multi', fusion: { blocked: true, vetoReason: '共识度低于门槛' } }
    expect(collabChatSpeaker(blocked)).toBe('质量闸门')
    expect(collabChatText(blocked)).toBe('共识度低于门槛')
    expect(collabChatPhase(blocked)).toBe('gate')
  })

  it('契约外的 mode 与空结果都安全降级，不抛异常也不假装成功', () => {
    for (const r of [null, undefined, {}, { mode: 'not_in_contract' }]) {
      const text = collabChatText(r)
      expect(text).toBe(FALLBACK_TEXT)
      expect(collabChatSpeaker(r)).toBe('联盟协作')
      expect(PHASE_IDS).toContain(collabChatPhase(r))
    }
  })
})
