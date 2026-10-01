// 图谱 store 单元测试：分區错误隔离、按节点类型省调用、重建后重取。
// wire 名与出参面由 contract/graph.test.js 守住，这里只验 store 是否把它们交给对的接口。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    graphOverview: vi.fn(), graphStats: vi.fn(), graphCommunities: vi.fn(),
    graphNeighbors: vi.fn(), graphCollaborators: vi.fn(), graphPath: vi.fn(),
    rebuildGraph: vi.fn(), optimalTeam: vi.fn()
  }
}))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceGraphStore } = await import('./alliance-graph.store.js')
const { COLLABORATOR_LIMIT_DEFAULT } = await import('../contract/graph.js')

const fail = (msg) => Object.assign(new Error(msg), { name: 'ApiError', msg })

const graphFixture = () => ({
  nodes: [
    { id: 'domain-ai', label: 'ai', nodeType: 'domain' },
    { id: 'e1', label: '甲', nodeType: 'expert' },
    { id: 'e2', label: '乙', nodeType: 'expert' }
  ],
  edges: [
    { source: 'e1', target: 'domain-ai', edgeType: 'has_domain', weight: 1 },
    { source: 'e1', target: 'e2', edgeType: 'collaborates_with', weight: 0.5 }
  ],
  stats: { nodeCount: 3, edgeCount: 2, version: 4 },
  builtAt: '2026-09-23T00:00:00Z',
  version: 4
})

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.graphOverview.mockResolvedValue(graphFixture())
  api.graphNeighbors.mockResolvedValue({ nodeId: 'e1', neighbors: [], neighborCount: 0 })
  api.graphCollaborators.mockResolvedValue({ expertId: 'e1', collaborators: [], totalCollaborators: 0 })
})

describe('加载与分区错误隔离', () => {
  it('取到图后自动选中首个节点，域节点不查协作者', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    expect(store.selectedId).toBe('domain-ai')
    expect(api.graphNeighbors).toHaveBeenCalledWith('domain-ai')
    expect(api.graphCollaborators).not.toHaveBeenCalled()
  })

  it('统计失败不清图，三区各自记账', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    api.graphStats.mockRejectedValue(fail('统计不可用'))
    await store.loadMetrics()
    expect(store.error.metrics).toBe('统计不可用')
    expect(store.graph).toBeTruthy()
    expect(store.loading.metrics).toBe(false)
    expect(store.error.graph).toBe('')
  })

  it('图获取失败时归零而不是留下半张图', async () => {
    const store = useAllianceGraphStore()
    api.graphOverview.mockRejectedValue(fail('图不可用'))
    await store.loadGraph()
    expect(store.graph).toBe(null)
    expect(store.error.graph).toBe('图不可用')
    expect(store.layout.nodes).toEqual([])
  })

  it('选项来自图本身：域候选去掉 domain- 前缀', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    expect(store.domainOptions).toEqual([{ value: 'ai', label: 'ai' }])
    expect(store.expertOptions.map((o) => o.value)).toEqual(['e1', 'e2'])
  })
})

describe('节点详情', () => {
  it('专家节点才查协作者，并带上当前 limit', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    await store.selectNode('e1')
    expect(api.graphCollaborators).toHaveBeenCalledWith('e1', COLLABORATOR_LIMIT_DEFAULT)
    expect(store.error.node).toBe('')
  })

  it('改 limit 只重取协作者；域节点根本不发请求', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    await store.selectNode('e1')
    api.graphNeighbors.mockClear()
    await store.changeCollaboratorLimit(20)
    expect(api.graphCollaborators).toHaveBeenLastCalledWith('e1', 20)
    expect(api.graphNeighbors).not.toHaveBeenCalled()

    await store.selectNode('domain-ai')
    api.graphCollaborators.mockClear()
    await store.changeCollaboratorLimit(5)
    expect(api.graphCollaborators).not.toHaveBeenCalled()
  })

  it('limit 生效时截断事实可被界面读到', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    api.graphCollaborators.mockResolvedValue({
      expertId: 'e1',
      collaborators: [{ rank: 1, id: 'e2', name: '乙', collaborationWeight: 0.5, sharedDomains: ['ai'] }],
      totalCollaborators: 9
    })
    await store.selectNode('e1')
    expect(store.collaboratorsTruncated).toBe(true)
  })

  it('邻域 404 时错误落在 node 区，已选中的节点仍在图上高亮', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    api.graphNeighbors.mockRejectedValue(fail('node not found: x'))
    await store.selectNode('e1')
    expect(store.error.node).toBe('node not found: x')
    expect(store.selectedId).toBe('e1')
    expect(store.neighbors).toBe(null)
  })
})

describe('路径与重建', () => {
  it('两端未选齐不发请求', async () => {
    const store = useAllianceGraphStore()
    await store.findPath()
    expect(api.graphPath).not.toHaveBeenCalled()
    expect(store.error.path).toContain('请先选择')
  })

  it('查询走 path/:source/:target 两个参数', async () => {
    const store = useAllianceGraphStore()
    api.graphPath.mockResolvedValue({ source: 'e1', target: 'e2', path: [], pathLength: 1, found: true })
    store.setPathDraft('source', 'e1')
    store.setPathDraft('target', 'e2')
    await store.findPath()
    expect(api.graphPath).toHaveBeenCalledWith('e1', 'e2')
    expect(store.path.found).toBe(true)
    expect(store.error.path).toBe('')
  })

  it('重建成功后必须重取三份数据，否则界面停在旧图上', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    api.graphStats.mockResolvedValue({ totalNodes: 3 })
    api.graphCommunities.mockResolvedValue({ totalCommunities: 1 })
    api.rebuildGraph.mockResolvedValue({ rebuilt: true, previousVersion: 4, newVersion: 5, nodeCount: 3, edgeCount: 2, durationMs: 12 })
    await store.rebuild()
    expect(store.rebuildResult.newVersion).toBe(5)
    expect(api.graphOverview.mock.calls.length).toBeGreaterThan(1)
    expect(api.graphStats).toHaveBeenCalled()
    expect(api.graphCommunities).toHaveBeenCalled()
    expect(store.loading.rebuild).toBe(false)
  })

  it('重建失败不动已有图', async () => {
    const store = useAllianceGraphStore()
    await store.loadGraph()
    api.rebuildGraph.mockRejectedValue(fail('重建失败'))
    expect(await store.rebuild()).toBe(null)
    expect(store.error.rebuild).toBe('重建失败')
    expect(store.graph.version).toBe(4)
    expect(api.graphOverview).toHaveBeenCalledTimes(1)
  })
})

describe('最优团队', () => {
  it('校验不过就不发请求，错误直接落在 team 区', async () => {
    const store = useAllianceGraphStore()
    expect(store.teamProblem).toContain('至少')
    expect(await store.formTeam()).toBe(null)
    expect(api.optimalTeam).not.toHaveBeenCalled()
    // 校验文案直接写进 error.team，面板不再另设一套提示
    expect(store.error.team).toBe(store.teamProblem)
  })

  it('草稿整份交给 api，body 拼装是 api 层的事', async () => {
    const store = useAllianceGraphStore()
    api.optimalTeam.mockResolvedValue({ teamId: 'team-1', members: [], coverage: { requiredTotal: 0 } })
    store.setTeamValue('requiredDomains', ['ai'])
    store.setTeamValue('maxMembers', 7)
    await store.formTeam()
    expect(api.optimalTeam).toHaveBeenCalledWith({
      requiredSkills: [], requiredDomains: ['ai'], maxMembers: 7, minRating: 4, goal: ''
    })
    expect(store.team.teamId).toBe('team-1')
  })

  it('后端报错时保留上一次团队结果', async () => {
    const store = useAllianceGraphStore()
    api.optimalTeam.mockResolvedValue({ teamId: 'team-1', members: [] })
    store.setTeamValue('requiredDomains', ['ai'])
    await store.formTeam()
    api.optimalTeam.mockRejectedValue(fail('无候选'))
    expect(await store.formTeam()).toBe(null)
    expect(store.error.team).toBe('无候选')
    expect(store.team.teamId).toBe('team-1')
  })
})
