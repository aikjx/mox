// MatchExplainPanel（U2 匹配透明化）真实挂载测试：它是纯呈现组件（无 Element Plus、无 store），
// 这里用 @vue/test-utils 真 mount，断言：scores 缺省不渲染、逐维条按权重/贡献画出、且驱动数据
// （props，由 AllianceExpertsView 从 store.expertMatches 喂入）一变 DOM 跟着重渲染。
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import MatchExplainPanel from './MatchExplainPanel.vue'

// 形状对齐 model/normalize.js normScores：五维各 { value, weight, contrib }，健康度权重 0.05。
const SCORES = {
  domain: { value: 0.8, weight: 0.4, contrib: 0.32 },
  capability: { value: 0.7, weight: 0.25, contrib: 0.175 },
  priority: { value: 0.6, weight: 0.15, contrib: 0.09 },
  performance: { value: 0.5, weight: 0.05, contrib: 0.025 },
  health: { value: 1.0, weight: 0.05, contrib: 0.05 }
}

describe('MatchExplainPanel 匹配透明面板', () => {
  it('scores 为空（旧上游/降级路径）时整个面板不渲染，卡片维持原总分', () => {
    const w = mount(MatchExplainPanel, { props: { scores: null, matchScore: 0 } })
    expect(w.find('.mxe-root').exists()).toBe(false)
  })

  it('带 scores 时渲染五个维度条，并如实标注健康度 0.05 加权口径', () => {
    const w = mount(MatchExplainPanel, {
      props: { scores: SCORES, matchScore: 0.66, matchReason: '领域重合最高' }
    })
    // 五维各一条
    expect(w.findAll('.mxe-row')).toHaveLength(5)
    // 总分演算行与匹配原因
    expect(w.text()).toContain('总分 0.660')
    expect(w.text()).toContain('领域重合最高')
    // 健康度 0.05 口径说明（U2 纠错后文案）
    expect(w.text()).toContain('0.05')
    // 维度标签齐全
    expect(w.text()).toContain('领域')
    expect(w.text()).toContain('健康度')
  })

  it('驱动数据变化时 DOM 跟随重渲染（总分与健康度条随 props 更新）', async () => {
    const w = mount(MatchExplainPanel, {
      props: { scores: SCORES, matchScore: 0.5, matchReason: '' }
    })
    expect(w.text()).toContain('总分 0.500')

    // 后端重算后总分变了 → 面板总分行跟着变
    await w.setProps({ matchScore: 0.923 })
    expect(w.text()).toContain('总分 0.923')
    expect(w.text()).not.toContain('总分 0.500')

    // 健康度维得分从 1.0 掉到 0.2（不健康）→ 该行值与贡献同步刷新
    await w.setProps({ scores: { ...SCORES, health: { value: 0.2, weight: 0.05, contrib: 0.01 } } })
    const rows = w.findAll('.mxe-row')
    const healthRow = rows[4]
    expect(healthRow.find('.mxe-val').text()).toContain('0.200')
    expect(healthRow.find('.mxe-contrib').text()).toContain('0.010')
  })
})
