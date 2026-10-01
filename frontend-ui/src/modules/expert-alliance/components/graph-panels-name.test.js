/**
 * 图谱页两个面板的名称出口见证。
 *
 * 这三处不吃 store 的候选下拉（那条链路由 store 测试钉），而是各自直接拿到注册表名字：
 * 中心性排行跳转、社区成员串、团队成员跳转。注册表里有两行的 name 在写入侧就丢成了
 * '???????'（探针记录见 contract/graph.test.js 顶部），只要有一处照抄，问号就会印到
 * /alliance/graph 上。只有渲染才暴露，所以单独钉这一枚。
 */
import { describe, expect, it, vi } from 'vitest'
import { h } from 'vue'
import { mount } from '@vue/test-utils'
import GraphMetricsPanel from './GraphMetricsPanel.vue'
import GraphTeamPanel from './GraphTeamPanel.vue'

/** 透传型替身：渲染为同名自定义元素，props 落为属性，默认插槽原样输出 */
const pass = (tag, props = []) => ({
  name: tag,
  props,
  setup(_props, { slots }) {
    return () => h(tag, { class: `ep-${tag}`, ..._props }, slots.default?.())
  }
})
const stubs = {
  'el-alert': pass('el-alert', ['type', 'title', 'description', 'showIcon', 'closable']),
  'el-button': pass('el-button', ['type', 'size', 'plain', 'text', 'loading', 'disabled']),
  'el-select': pass('el-select', ['modelValue', 'multiple', 'filterable', 'placeholder', 'size']),
  'el-option': {
    name: 'el-option',
    props: ['label', 'value'],
    setup(props) {
      return () => h('option', { class: 'ep-option', 'data-value': props.value }, props.label)
    }
  },
  'el-tag': pass('el-tag', ['size', 'effect']),
  'el-input': pass('el-input', ['modelValue', 'type', 'rows', 'placeholder', 'size']),
  'el-input-number': pass('el-input-number', ['modelValue', 'min', 'max', 'step', 'precision', 'size'])
}
const opts = () => ({ global: { stubs, config: { globalProperties: {} } } })

const LOST_ID = 'exp-af875a6f60d943e6964fcef4db0ab73f'
const LOST_SHORT = '未命名节点 af875a6f'

const metricsStore = (memberLabels = ['甲', '???????', '?????????']) => ({
  metrics: {
    totalNodes: 4, expertNodes: 3, domainNodes: 1, totalEdges: 2, collaborationEdges: 1, domainEdges: 1,
    density: 0.33, avgClusteringCoefficient: 0.5, connectedComponents: 1, largestComponentSize: 4,
    topCentralityExperts: [
      { id: 'e1', name: '甲', degree: 3, degreeCentrality: 0.667, betweenness: 1.2 },
      { id: LOST_ID, name: '???????', degree: 2, degreeCentrality: 0.444, betweenness: 0.6 }
    ]
  },
  communities: {
    algorithm: 'label-propagation', iterations: 3, converged: true, modularity: 0.42, totalCommunities: 1,
    communities: [{ communityId: 'c1', size: memberLabels.length, internalEdges: 2, externalEdges: 0, memberLabels }]
  },
  expertOptions: [{ value: 'e1', label: '甲' }, { value: LOST_ID, label: LOST_SHORT }],
  path: null,
  pathDraft: { source: '', target: '' },
  loading: { communities: false, path: false },
  error: { communities: '', path: '' },
  setPathDraft: vi.fn(), findPath: vi.fn(), selectNode: vi.fn()
})

const teamStore = () => ({
  teamDraft: { requiredSkills: [], requiredDomains: [], maxMembers: 5, minRating: 4, goal: '' },
  domainOptions: [{ value: 'ai', label: 'ai' }, { value: 'data-engineering', label: '未命名节点 data-eng' }],
  team: {
    teamId: 'team-1', createdAt: '2026-09-27T00:00:00Z', teamScore: 0.8, strategy: 'greedy',
    members: [
      { id: 'e1', name: '甲', title: '架构师', role: 'leader', avgRating: 4.8, matchScore: 0.9, coveredSkills: ['sql'], coveredDomains: ['ai'] },
      { id: LOST_ID, name: '???????', title: '', role: 'member', avgRating: 4.1, matchScore: 0.7, coveredSkills: [], coveredDomains: [] }
    ],
    coverage: { requiredTotal: 2, coveredCount: 2, coverageRatio: 1, missingSkills: [], missingDomains: [] }
  },
  loading: { team: false },
  error: { team: '' },
  teamProblem: '',
  setTeamValue: vi.fn(), selectNode: vi.fn()
})

describe('中心性排行与社区成员的姓名口径', () => {
  it('跳转按钮改标 id 短码，一个好名字都不误伤', () => {
    const w = mount(GraphMetricsPanel, { props: { store: metricsStore() }, ...opts() })
    expect(w.findAll('.agm-jump').map((b) => b.text())).toEqual(['甲', LOST_SHORT])
  })

  it('社区成员只有名字没有 id，丢码的合并计数而不是各印一串问号', () => {
    const w = mount(GraphMetricsPanel, { props: { store: metricsStore() }, ...opts() })
    expect(w.find('.agm-community-members').text()).toBe('甲、未命名节点 ×2')
    expect(w.text()).not.toContain('?')
  })

  it('成员为空与成员全丢是两种说法', () => {
    const empty = mount(GraphMetricsPanel, { props: { store: metricsStore([]) }, ...opts() })
    expect(empty.find('.agm-community-members').text()).toBe('（无成员）')
    const allLost = mount(GraphMetricsPanel, { props: { store: metricsStore(['???', ' ']) }, ...opts() })
    expect(allLost.find('.agm-community-members').text()).toBe('未命名节点 ×2')
  })

  it('点跳转只上抛意图，面板自己不记账', () => {
    const store = metricsStore()
    const w = mount(GraphMetricsPanel, { props: { store }, ...opts() })
    w.findAll('.agm-jump')[1].trigger('click')
    expect(store.selectNode).toHaveBeenCalledWith(LOST_ID)
  })

  it('路径候选下拉照抄 store 给的短码，不再自己兜一层', () => {
    const w = mount(GraphMetricsPanel, { props: { store: metricsStore() }, ...opts() })
    const labels = w.findAll('.ep-option').map((o) => o.text())
    expect(labels).toEqual(['甲', LOST_SHORT, '甲', LOST_SHORT])
    expect(w.html()).not.toContain('???????')
  })
})

describe('团队成员跳转的姓名口径', () => {
  it('成员标题跳转与技能/域候选都不带问号', () => {
    const store = teamStore()
    const w = mount(GraphTeamPanel, { props: { store, skillOptions: [{ value: 'sql', label: 'sql' }] }, ...opts() })
    expect(w.findAll('.agt-jump').map((b) => b.text())).toEqual(['甲', LOST_SHORT])
    expect(w.findAll('.ep-option').map((o) => o.text())).toEqual(['sql', 'ai', '未命名节点 data-eng'])
    expect(w.text()).not.toContain('?')
  })

  it('点成员跳转把 id 交给 store', () => {
    const store = teamStore()
    const w = mount(GraphTeamPanel, { props: { store, skillOptions: [] }, ...opts() })
    w.findAll('.agt-jump')[1].trigger('click')
    expect(store.selectNode).toHaveBeenCalledWith(LOST_ID)
  })
})
