/**
 * 图谱面板渲染见证（工作台外壳 + 模块 store/model/components 的整条链路）。
 *
 * 这里要看的不是"能不能编译"（src/sfc-compile.test.js 已经管了），而是三件只有渲染才暴露的事：
 * 1. 统计条印出的数字与画布上真的画出来的节点/边数一致（口径同源，不是各算一遍）；
 * 2. 编码丢失的标签（后端 label 就是 '???????'）在**每一个**出口都改标为 id 短码——
 *    图上、面板标题、模块检视器标题三处，一处漏了就会把一串问号当真名印给用户；
 * 3. 失败态只红图谱这一区并给出重试，不清空画布以外的东西；空态给的是可操作的下一步。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    graphOverview: vi.fn(), graphStats: vi.fn(), graphCommunities: vi.fn(),
    graphNeighbors: vi.fn(), graphCollaborators: vi.fn(), graphPath: vi.fn(),
    rebuildGraph: vi.fn(), optimalTeam: vi.fn()
  }
}))
vi.mock('@/modules/expert-alliance/api/alliance.api.js', () => ({ allianceApi: api }))

const { useGraphCanvas } = await import('@/composables/workspace/useGraphCanvas.js')
const { useAllianceGraphStore } = await import('@/modules/expert-alliance/store')
const GraphCanvasPanel = (await import('@/views/workspace/panels/GraphCanvasPanel.vue')).default

/** 手写 Element Plus 替身：把 props 与插槽原样落到同名元素上，不把整套 EP 拉进测试环境 */
const ElButton = {
  name: 'el-button',
  props: ['type', 'size', 'plain', 'text', 'loading', 'disabled'],
  setup(_props, { slots, attrs }) {
    return () => h('button', { class: 'ep-btn', ...attrs }, slots.default?.())
  }
}
const ElAlert = {
  name: 'el-alert',
  props: ['type', 'title', 'description', 'showIcon', 'closable'],
  setup(props, { slots }) {
    return () => h('div', { class: 'ep-alert' }, [h('b', props.title), h('span', props.description), slots.default?.()])
  }
}
const pass = (tag) => ({
  name: `stub-${tag}`,
  setup(_props, { slots, attrs }) {
    return () => h(tag, { class: `ep-${tag}`, ...attrs }, slots.default?.())
  }
})
const stubs = {
  'el-button': ElButton,
  'el-alert': ElAlert,
  'el-icon': pass('i'),
  'el-tag': pass('span'),
  'el-select': pass('select'),
  'el-option': pass('option')
}

const LOST_ID = 'exp-af875a6f60d943e6964fcef4db0ab73f'
const nodes = (extraLost = false) => [
  { id: 'domain-ai', label: 'ai', nodeType: 'domain' },
  { id: 'e1', label: '甲', nodeType: 'expert', title: '资深架构师', avgRating: 4.5, availability: 'available' },
  { id: 'e2', label: '乙', nodeType: 'expert', avgRating: null, availability: null },
  ...(extraLost ? [{ id: LOST_ID, label: '?????????', nodeType: 'expert', avgRating: null, availability: null }] : [])
]
const edges = (extraLost = false) => [
  { source: 'e1', target: 'domain-ai', edgeType: 'has_domain', weight: 1 },
  { source: 'e2', target: 'domain-ai', edgeType: 'has_domain', weight: 1 },
  ...(extraLost ? [{ source: LOST_ID, target: 'domain-ai', edgeType: 'has_domain', weight: 1 }] : [])
]

function panelSetup() {
  const c = useGraphCanvas()
  const store = useAllianceGraphStore()
  const w = mount(GraphCanvasPanel, {
    props: {
      store,
      graphStats: c.graphStats.value,
      graphLoading: c.graphLoading.value,
      viewportStyle: c.viewportStyle.value
    },
    global: { stubs }
  })
  return { ...c, store, w }
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.graphOverview.mockResolvedValue({
    nodes: nodes(), edges: edges(),
    stats: { expertCount: 2, domainCount: 1, density: 0.333 },
    builtAt: '2026-09-27T00:00:00Z', version: 4
  })
  api.graphNeighbors.mockResolvedValue({ nodeId: 'e1', neighbors: [{ id: 'domain-ai', label: 'ai', edgeType: 'has_domain', direction: 'out', weight: 1, sharedDomains: [] }], neighborCount: 1 })
  api.graphCollaborators.mockResolvedValue({ expertId: 'e1', collaborators: [], totalCollaborators: 0 })
})

describe('统计条与画布同源', () => {
  it('印出的数字就是画布上画出来的数量', async () => {
    const c = panelSetup()
    await c.loadGraphData()
    await c.w.setProps({ graphStats: c.graphStats.value })
    await flushPromises()
    expect(c.w.findAll('.agc-node')).toHaveLength(3)
    expect(c.w.findAll('.agc-line')).toHaveLength(2)
    const stats = c.w.find('.ws-graph-stats').text()
    expect(stats).toContain('3')
    expect(stats).toContain('2')
    expect(stats).toMatch(/密度\s*0\.333/)
    expect(c.w.find('.agc-svg').attributes('aria-label')).toContain('3 个节点 2 条边')
  })

  it('视口样式落在包裹画布的那一层上', async () => {
    const c = panelSetup()
    await c.loadGraphData()
    c.zoomIn()
    await c.w.setProps({ viewportStyle: c.viewportStyle.value })
    expect(c.w.find('.ws-graph-viewport').attributes('style')).toContain('scale(1.25)')
  })

  it('工具条只有三个动作，且都上抛给外壳', async () => {
    const c = panelSetup()
    const buttons = c.w.findAll('.ws-canvas-tool')
    expect(buttons).toHaveLength(3)
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')
    expect(Object.keys(c.w.emitted())).toEqual(expect.arrayContaining(['zoom-in', 'zoom-out', 'fit-view']))
  })
})

describe('编码丢失的标签在每个出口都改标 id 短码', () => {
  it('图例说出有几枚名字丢了，标题与检视器都不印问号', async () => {
    api.graphOverview.mockResolvedValue({
      nodes: nodes(true), edges: edges(true),
      stats: { expertCount: 3, domainCount: 1, density: 0.2 },
      builtAt: '', version: 4
    })
    const c = panelSetup()
    await c.loadGraphData()
    c.selectNode(LOST_ID)
    await c.w.setProps({ graphStats: c.graphStats.value })
    await flushPromises()

    const short = LOST_ID.replace(/^exp-/, '').slice(0, 8)
    expect(c.w.find('.agc-lost').text()).toContain('1 个节点名称在写入后端时已编码丢失')
    expect(c.w.find('.ws-graph-inspector-title').text()).toBe(`未命名节点 ${short}`)
    expect(c.w.find('.agn-title').text()).toBe(`未命名节点 ${short}`)
    // 全树任何一处都不许留下那串问号（画布标签、title、aria-label 都在树内）
    expect(c.w.text()).not.toContain('?')
    expect(c.w.html()).not.toContain('???????')
    // 好名字不受影响，也不许被加上"未命名"外壳
    expect(c.w.findAll('.agc-node')[1].attributes('aria-label')).toContain('甲')
  })

  it('邻域与协作者列表里的名字同样过一遍口径', async () => {
    api.graphNeighbors.mockResolvedValue({
      nodeId: 'e1',
      neighbors: [{ id: LOST_ID, label: '?????????', edgeType: 'collaborates_with', direction: 'out', weight: 0.4, sharedDomains: [] }],
      neighborCount: 1
    })
    api.graphCollaborators.mockResolvedValue({
      expertId: 'e1',
      collaborators: [{ id: LOST_ID, name: '?????????', rank: 1, collaborationWeight: 0.4, sharedDomains: [] }],
      totalCollaborators: 1
    })
    const c = panelSetup()
    await c.loadGraphData()
    c.selectNode('e1')
    await flushPromises()
    const jumps = c.w.findAll('.agn-jump').map((b) => b.text())
    expect(jumps.length).toBeGreaterThanOrEqual(2)
    for (const text of jumps) {
      expect(text).not.toContain('?')
      expect(text).toMatch(/^未命名节点 [0-9a-f]{8}$|^ai$|^甲$/)
    }
  })
})

describe('选中与动作', () => {
  it('点图上的节点：面板把意图上抛，外壳负责记账', async () => {
    const c = panelSetup()
    await c.loadGraphData()
    await flushPromises()
    const target = c.w.findAll('.agc-node').find((n) => n.attributes('aria-label').includes('甲'))
    await target.trigger('click')
    expect(c.w.emitted('select-node')[0]).toEqual(['e1'])
    // 面板只上抛，不自己改 store；改 store 是外壳/视图的意图落地
    expect(c.store.selectedId).toBe('domain-ai')
    c.selectNode('e1')
    await flushPromises()
    expect(c.store.selectedId).toBe('e1')
    expect(api.graphCollaborators).toHaveBeenCalledWith('e1', c.store.collaboratorLimit)
    await c.w.setProps({ graphStats: c.graphStats.value })
    await c.w.find('.ws-graph-inspector-head .ep-btn').trigger('click')
    expect(c.w.emitted('clear-selected-node')).toBeTruthy()
  })
})

describe('失败态与空态', () => {
  it('取图失败只红这一区，并给出重试', async () => {
    api.graphOverview.mockRejectedValue(Object.assign(new Error('x'), { msg: '网关 502' }))
    const c = panelSetup()
    await c.loadGraphData()
    await c.w.setProps({ graphLoading: false })
    await flushPromises()
    expect(c.w.find('.ep-alert').text()).toContain('协作图谱获取失败')
    expect(c.w.find('.ep-alert').text()).toContain('网关 502')
    await c.w.find('.ep-alert .ep-btn').trigger('click')
    expect(c.w.emitted('retry')).toBeTruthy()
    expect(c.w.findAll('.agc-node')).toHaveLength(0)
  })

  it('图为空时给的是下一步，不是一片空白', async () => {
    api.graphOverview.mockResolvedValue({ nodes: [], edges: [], stats: {}, builtAt: '', version: 0 })
    const c = panelSetup()
    await c.loadGraphData()
    await flushPromises()
    expect(c.w.find('.ws-graph-empty').text()).toContain('图为空')
    expect(c.w.find('.ws-graph-empty').text()).toContain('重建图谱')
  })
})
