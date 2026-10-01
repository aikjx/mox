/**
 * 工作台图谱外壳的行为测试 + 结构性钉。
 *
 * 行为面：统计条数字来自后端 stats（缺省时才按类型现算）、选中态读写 store、
 * 缩放有上下界、平移必须先按下鼠标、滚轮必须吃掉默认行为。
 *
 * 结构性钉守的是**已经付出过代价**的那几条：本文件曾经绕过模块自己拉数，
 * 于是 node_type 读成 type（类型统计恒 1）、doc_count/expert_count/rank 后端从不产出
 * （信息卡三行永远 0/0/'-'）、Math.random() 当坐标（同一张图每次刷新都在换位置）、
 * .slice(0, 4) 裁标签（architecture→clou，data 与 database 撞名，丢码专家→'????'）、
 * 空态分支引用作用域里不存在的 ref（一跑就 ReferenceError）、
 * setTimeout 假扮"图谱分析"。这几条一旦被写回来，下面的用例必须变红。
 */
import path from 'node:path'
import { readFileSync } from 'node:fs'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    graphOverview: vi.fn(), graphStats: vi.fn(), graphCommunities: vi.fn(),
    graphNeighbors: vi.fn(), graphCollaborators: vi.fn(), graphPath: vi.fn(),
    rebuildGraph: vi.fn(), optimalTeam: vi.fn()
  }
}))
vi.mock('@/modules/expert-alliance/api/alliance.api.js', () => ({ allianceApi: api }))

const { useGraphCanvas } = await import('./useGraphCanvas.js')
const { useAllianceGraphStore } = await import('@/modules/expert-alliance/store')

// vitest 下 import.meta.url 是 http 协议不是 file（见 palette.test.js:139 同一坑），
// 沿用本仓约定：从 process.cwd()（= frontend-ui 根）定位源文件；读不到会整节红而不是静默放过。
const SRC = path.resolve(process.cwd(), 'src')
const sourceOf = (rel) => readFileSync(path.join(SRC, rel), 'utf8')
/**
 * 负向扫描必须先把注释剥掉：这两个文件的头部注释逐条记录了退役写法（'Math.random()'、
 * '.slice(0, 4)'、'setTimeout(1500)'、'doc_count'……），直接对全文扫会让"不许回来"的针
 * 一上手就红在自己写的说明上——那不是缺陷，是判据没分清散文与代码。
 */
const codeOf = (rel) => sourceOf(rel)
  .replace(/<!--[\s\S]*?-->/g, '')
  .replace(/\/\*[\s\S]*?\*\//g, '')
  .replace(/^\s*\/\/.*$/gm, '')

const graphFixture = (stats = { expertCount: 2, domainCount: 1, density: 0.5 }) => ({
  nodes: [
    { id: 'domain-ai', label: 'ai', nodeType: 'domain' },
    { id: 'e1', label: '甲', nodeType: 'expert' },
    { id: 'e2', label: '乙', nodeType: 'expert' }
  ],
  edges: [
    { source: 'e1', target: 'domain-ai', edgeType: 'has_domain', weight: 1 },
    { source: 'e1', target: 'e2', edgeType: 'collaborates_with', weight: 0.5 }
  ],
  stats,
  builtAt: '2026-09-27T00:00:00Z',
  version: 4
})

function mount() {
  const c = useGraphCanvas()
  return { ...c, store: useAllianceGraphStore() }
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.graphOverview.mockResolvedValue(graphFixture())
  api.graphNeighbors.mockResolvedValue({ nodeId: 'e1', neighbors: [], neighborCount: 0 })
  api.graphCollaborators.mockResolvedValue({ expertId: 'e1', collaborators: [], totalCollaborators: 0 })
})

describe('统计条与图数据同源', () => {
  it('未加载时是明确的空，不是 NaN 也不是 undefined', () => {
    const c = mount()
    expect(c.graphStats.value).toEqual({
      nodes: 0, edges: 0, expertNodes: 0, domainNodes: 0, density: 0, version: 0, builtAt: ''
    })
    expect(c.selectedNode.value).toBe(null)
    expect(c.layout.value.nodes).toEqual([])
  })

  it('后端给了 stats 就照后端口径报，不自己数一遍', async () => {
    const c = mount()
    await c.loadGraphData()
    expect(c.graphStats.value).toMatchObject({
      nodes: 3, edges: 2, expertNodes: 2, domainNodes: 1, density: 0.5, version: 4
    })
    expect(c.graphStats.value.builtAt).toBe('2026-09-27T00:00:00Z')
  })

  it('后端缺 expertCount 时按节点类型现算，而不是印 0', async () => {
    api.graphOverview.mockResolvedValue(graphFixture({}))
    const c = mount()
    await c.loadGraphData()
    expect(c.graphStats.value.expertNodes).toBe(2)
    expect(c.graphStats.value.domainNodes).toBe(1)
    expect(c.graphStats.value.density).toBe(0)
  })

  it('loadGraphData 返回布尔，调用方据此决定要不要提示空图', async () => {
    const c = mount()
    expect(await c.loadGraphData()).toBe(true)
    api.graphOverview.mockRejectedValue(Object.assign(new Error('boom'), { msg: '图谱不可用' }))
    expect(await c.loadGraphData()).toBe(false)
    expect(c.graphError.value).toBe('图谱不可用')
    expect(c.graphLoading.value).toBe(false)
  })
})

describe('选中态归 store', () => {
  it('selectNode 把 id 交给 store，由 store 去取邻域', async () => {
    const c = mount()
    await c.loadGraphData()
    c.selectNode('e1')
    expect(c.store.selectedId).toBe('e1')
    expect(api.graphCollaborators).toHaveBeenCalledWith('e1', c.store.collaboratorLimit)
    expect(c.selectedNode.value.id).toBe('e1')
  })

  it('clearSelectedNode 只清选中，不清图', async () => {
    const c = mount()
    await c.loadGraphData()
    c.clearSelectedNode()
    expect(c.store.selectedId).toBe('')
    expect(c.selectedNode.value).toBe(null)
    expect(c.graphStats.value.nodes).toBe(3)
  })
})

describe('视口缩放与平移', () => {
  it('缩放被夹在 [0.5, 2.5]，连续点按钮不会缩到看不见或无限大', () => {
    const c = mount()
    for (let i = 0; i < 30; i++) c.zoomIn()
    expect(c.viewport.value.scale).toBe(2.5)
    for (let i = 0; i < 40; i++) c.zoomOut()
    expect(c.viewport.value.scale).toBe(0.5)
    expect(c.viewportStyle.value.transform).toBe('translate(0px, 0px) scale(0.5)')
  })

  it('fitView 复位平移与缩放', () => {
    const c = mount()
    c.onCanvasMouseDown({ button: 0, clientX: 100, clientY: 100 })
    c.onCanvasMouseMove({ clientX: 160, clientY: 40 })
    c.zoomIn()
    expect(c.viewport.value.x).not.toBe(0)
    c.fitView()
    expect(c.viewport.value).toEqual({ x: 0, y: 0, scale: 1 })
  })

  it('未按下鼠标时移动不改视口，右键按下也不拖动', () => {
    const c = mount()
    c.onCanvasMouseMove({ clientX: 500, clientY: 500 })
    expect(c.viewport.value.x).toBe(0)
    c.onCanvasMouseDown({ button: 2, clientX: 0, clientY: 0 })
    c.onCanvasMouseMove({ clientX: 500, clientY: 500 })
    expect(c.viewport.value).toEqual({ x: 0, y: 0, scale: 1 })
  })

  it('按下后按屏幕位移平移，抬手即停', () => {
    const c = mount()
    c.onCanvasMouseDown({ button: 0, clientX: 100, clientY: 100 })
    c.onCanvasMouseMove({ clientX: 130, clientY: 90 })
    expect([c.viewport.value.x, c.viewport.value.y]).toEqual([30, -10])
    c.onCanvasMouseUp()
    c.onCanvasMouseMove({ clientX: 900, clientY: 900 })
    expect([c.viewport.value.x, c.viewport.value.y]).toEqual([30, -10])
  })

  it('滚轮放大缩小且必须 preventDefault，否则页面跟着滚', () => {
    const c = mount()
    const up = { deltaY: -100, preventDefault: vi.fn() }
    const down = { deltaY: 100, preventDefault: vi.fn() }
    c.onCanvasWheel(up)
    expect(up.preventDefault).toHaveBeenCalled()
    expect(c.viewport.value.scale).toBeCloseTo(1.1, 10)
    c.onCanvasWheel(down)
    expect(down.preventDefault).toHaveBeenCalled()
    expect(c.viewport.value.scale).toBeCloseTo(0.99, 10)
    for (let i = 0; i < 60; i++) c.onCanvasWheel({ deltaY: -100, preventDefault: () => {} })
    expect(c.viewport.value.scale).toBe(2.5)
  })
})

describe('不许再把模块那一套复制回工作台', () => {
  const composable = sourceOf('composables/workspace/useGraphCanvas.js')
  const composableCode = codeOf('composables/workspace/useGraphCanvas.js')
  const panel = sourceOf('views/workspace/panels/GraphCanvasPanel.vue')
  const panelCode = codeOf('views/workspace/panels/GraphCanvasPanel.vue')

  it('外壳不直连 @/api，也不自己拉数、猜字段、随机摆点', () => {
    expect(composable).not.toMatch(/from ['"]@\/api['"]/)
    expect(composableCode).not.toMatch(/\baxios\b|\bfetch\(/)
    expect(composableCode).toMatch(/useAllianceGraphStore/)
    for (const retired of ['Math.random', 'doc_count', 'expert_count', '.slice(0, 4)', 'setTimeout']) {
      expect(composableCode, `已退役的写法又回来了：${retired}`).not.toContain(retired)
    }
  })

  it('画布与检视器用模块那一份（走 components 出口），不自己画 SVG', () => {
    const importLine = panel.match(/import\s*\{[^}]*\}\s*from\s*'@\/modules\/expert-alliance\/components'/)
    expect(importLine, '面板不再从模块出口取画布组件').not.toBe(null)
    expect(importLine[0]).toMatch(/GraphCanvas/)
    expect(importLine[0]).toMatch(/GraphNodeInspector/)
    expect(panel).toMatch(/graphNodeLabel\s*\}\s*from\s*'@\/modules\/expert-alliance\/contract'/)
    expect(panelCode).not.toMatch(/<svg|preserveAspectRatio|feDropShadow/)
    // 假功能一律不许回来：没有后端的工具按钮、假布局切换、后端从不产出的信息卡行
    for (const fake of ['力导向', '添加节点', '添加关系', '关联文档', '中心性排名', '#6366f1', '#94a3b8']) {
      expect(panelCode, `假界面又回来了：${fake}`).not.toContain(fake)
    }
  })

  it('面板的 props 只有一个 store，视口样式由外壳算', () => {
    expect(panelCode).toMatch(/:store="store"/)
    expect(panelCode).toMatch(/viewportStyle/)
    expect(composableCode).toMatch(/graphStore:\s*store/)
  })
})
