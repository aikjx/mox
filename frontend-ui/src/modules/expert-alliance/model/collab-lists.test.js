// 协作结果列表投影的判据（FE-MOD-GOV §5.4：视图只读投影行，不摸后端字段名）。
//
// 为什么单独成账：2026-09-27 之前这三只投影零测试、零消费者，工作台却在拿它们不存在
// 的键渲染（results / selected / final_synthesis）——界面静默空白，没人发现字段口径断了。
// 这里把「归一化结果 → 界面行」的口径钉死，视图改字段就会在这里红。
import { describe, it, expect } from 'vitest'

import { COLLAB_MODE } from '@/modules/expert-alliance/contract'
import {
  collabCandidateItems,
  collabContributionItems,
  collabDebateTurns,
  collabResultNote
} from './collabLists.js'
import { normDebate, normMultiConsult, normRouteResult } from './normalize.js'

describe('多专家协同 → 每位专家一行（collabContributionItems）', () => {
  const raw = {
    session_id: 's-1',
    question: '数据仓库选型',
    experts: [
      {
        id: 'e1',
        name: '架构专家',
        match_score: 0.91,
        answer: { analysis: '现状分析', solution: '建议方案', confidence: 0.82, source: 'llm' }
      },
      { id: 'e2', name: '成本专家', answer: { solution: '只有方案', confidence: 0.4 } },
      { id: 'e3', name: '合规专家', answer: { analysis: '越界内容', blocked: true } }
    ],
    fused_answer: { summary: '取第一条', consensus_score: 0.7, confidence: 0.66, dominant_view: '架构专家' }
  }
  const rows = collabContributionItems(normMultiConsult(raw))

  it('行数等于后端 experts 数组长度，且不是从 results/successful 那些不存在的键取的', () => {
    expect(rows.map((r) => r.name)).toEqual(['架构专家', '成本专家', '合规专家'])
  })

  it('正文由 analysis + solution 拼接，缺任一段都不编造', () => {
    expect(rows[0].text).toBe('现状分析\n\n建议方案')
    expect(rows[1].text).toBe('只有方案')
  })

  it('source 决定是否真实模型作答，文案取自契约单源', () => {
    expect(rows[0].modelBacked).toBe(true)
    expect(rows[0].sourceText).toBe('真实模型作答')
    expect(rows[1].modelBacked).toBe(false)
    expect(rows[1].sourceText).toBe('模板兜底作答')
  })

  it('被拦截的行既标 blocked，又给出拦截口径的文案而不是正文', () => {
    expect(rows[2].blocked).toBe(true)
    expect(rows[2].sourceText).toBe('已被治理闸门拦截')
  })

  it('id 与 key 分列：视图回查花名册只认 id，key 允许兜底序', () => {
    expect(rows[0].id).toBe('e1')
    expect(rows[1].id).toBe('e2')
    const noId = collabContributionItems(normMultiConsult({ experts: [{ name: '无名', answer: {} }] }))
    expect(noId[0].id).toBe('')
    expect(noId[0].key).toBe('c-0')
  })

  it('空结果返回空数组而不是抛错，也没有「未返回正文」的假行', () => {
    expect(collabContributionItems(normMultiConsult({}))).toEqual([])
    expect(collabContributionItems(null)).toEqual([])
  })

  it('正文全缺时留痕而非空串（界面不能显示成"这位专家说了空话"）', () => {
    const bare = collabContributionItems(normMultiConsult({ experts: [{ id: 'e9', name: '甲' }] }))
    expect(bare[0].text).toBe('（该专家未返回正文）')
    expect(bare[0].confidence).toBe(0)
  })
})

describe('辩论 → 逐轮正反方发言行（collabDebateTurns）', () => {
  const raw = {
    debate_id: 'd-1',
    topic: '是否微服务',
    rounds: 2,
    participants: [
      { id: 'e1', name: '正方甲', side: 'pro', final_score: 8.5 },
      { id: 'e2', name: '反方乙', side: 'con', final_score: 7.2 }
    ],
    debate_log: [
      { round: 1, pro_argument: 'R1P', con_argument: 'R1C', pro_score: 4.0, con_score: 3.5 },
      { round: 2, pro_argument: 'R2P', con_argument: '', pro_score: 4.6, con_score: 0 }
    ],
    verdict: { winner: 'e1', summary: '正方胜出', key_points: ['成本'], consensus_level: 'partial' }
  }
  const turns = collabDebateTurns(normDebate(raw))

  it('每轮按正反成对展开，缺侧不补空行', () => {
    expect(turns.map((t) => t.key)).toEqual(['1-pro', '1-con', '2-pro'])
  })

  it('发言人名取自 participants 的 side，方名兜底', () => {
    expect(turns[0].name).toBe('正方甲')
    expect(turns[1].name).toBe('反方乙')
    expect(turns[0].sideLabel).toBe('正方')
    expect(turns[1].sideLabel).toBe('反方')
  })

  it('逐轮得分各侧独立取，不串台', () => {
    expect(turns.map((t) => t.score)).toEqual([4, 3.5, 4.6])
  })

  it('没有 participants 时仍出行，名字退化为方名（辩论内容不该整块消失）', () => {
    const nameless = collabDebateTurns(normDebate({ debate_log: [{ round: 1, pro_argument: 'P' }] }))
    expect(nameless).toHaveLength(1)
    expect(nameless[0].name).toBe('正方')
  })

  it('同侧多人则并列显示，不悄悄丢掉第二人', () => {
    const two = collabDebateTurns(normDebate({
      participants: [{ name: '甲', side: 'pro' }, { name: '乙', side: 'pro' }],
      debate_log: [{ round: 1, pro_argument: 'P' }]
    }))
    expect(two[0].name).toBe('甲、乙')
  })

  it('空战报返回空数组（verdict 仍由视图单独渲染）', () => {
    expect(collabDebateTurns(normDebate({}))).toEqual([])
    expect(collabDebateTurns(null)).toEqual([])
  })
})

describe('路由 → 候选专家行（collabCandidateItems）', () => {
  const raw = {
    query: '选型',
    matched_experts: [
      {
        id: 'e1', name: '架构专家', title: '首席架构', domains: ['架构设计', '分布式'],
        match_score: 0.88, availability: { status: 'online' }, metrics: { avg_rating: 4.6 }
      },
      { id: 'e2', name: '成本专家', domains: ['FinOps'], match_score: 0.5 }
    ],
    routing_decision: { recommended_expert_id: 'e1', reason: '领域最匹配', alternative_ids: ['e2'] },
    total_scanned: 16
  }
  const rows = collabCandidateItems(normRouteResult(raw))

  it('候选取自 matched_experts，而不是从不存在的 selected 键', () => {
    expect(rows.map((r) => r.id)).toEqual(['e1', 'e2'])
  })

  it('只有被推荐的那一行拿到 recommended 与 reason', () => {
    expect(rows[0].recommended).toBe(true)
    expect(rows[0].reason).toBe('领域最匹配')
    expect(rows[1].recommended).toBe(false)
    expect(rows[1].reason).toBe('')
  })

  it('副标题优先 title，缺则用领域串联（不再让视图拿展示文案去猜配色键）', () => {
    expect(rows[0].subtitle).toBe('首席架构')
    expect(rows[1].subtitle).toBe('FinOps')
  })

  it('visualKey 取自首个领域，供 16 色板查色', () => {
    expect(rows[0].visualKey).toBe('架构设计')
    expect(rows[1].visualKey).toBe('FinOps')
    expect(rows.find((r) => r.id === 'e2').visualKey).toBe('FinOps')
  })

  it('行里没有"回复"字段：路由只排序不作答', () => {
    for (const row of rows) {
      expect(row).not.toHaveProperty('text')
      expect(row).not.toHaveProperty('answer')
      expect(row).not.toHaveProperty('response')
    }
  })

  it('无推荐结论时全行 recommended=false，而不是拿空串当身份比较', () => {
    const bare = collabCandidateItems(normRouteResult({ matched_experts: [{ id: 'e1', name: '甲' }] }))
    expect(bare[0].recommended).toBe(false)
    expect(bare[0].matchScore).toBe(0)
    expect(bare[0].avgRating).toBe(0)
  })

  it('空候选返回空数组', () => {
    expect(collabCandidateItems(normRouteResult({}))).toEqual([])
    expect(collabCandidateItems(null)).toEqual([])
  })
})

describe('结果顶部标注（collabResultNote）', () => {
  it('辩论由模板生成 ⇒ 必须显示说明', () => {
    expect(collabResultNote(normDebate({}))).toContain('模板生成')
  })

  it('多专家走真实模型链路 ⇒ 不加噪声', () => {
    expect(collabResultNote(normMultiConsult({}))).toBe('')
  })

  it('未知模式不抛错，也不编造标注', () => {
    expect(collabResultNote({ mode: 'not-a-mode' })).toBe('')
    expect(collabResultNote(null)).toBe('')
  })

  it('判据挂在模式定义上：路由/即时咨询都非模板产出', () => {
    expect(collabResultNote({ mode: COLLAB_MODE.ROUTE })).toBe('')
    expect(collabResultNote({ mode: COLLAB_MODE.SINGLE })).toBe('')
  })
})
