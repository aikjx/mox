// 协作图谱状态：唯一持有图数据的前序状态。视图只读绑定、只发意图。
// 图与统计/社区是三份独立接口，各自可失败，因此逐区记账 loading/error，
// 一处失败不得连带清空已取到的部分。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import { COLLABORATOR_LIMIT_DEFAULT, DOMAIN_NODE_ID_PREFIX, GRAPH_NODE_TYPE, graphNodeLabel, optimalTeamProblem, ragExpandProblem } from '@/modules/expert-alliance/contract'
import { graphLayout } from '@/modules/expert-alliance/model'

export const useAllianceGraphStore = defineStore('allianceGraph', () => {
  const api = allianceApi

  const graph = ref(null)
  const metrics = ref(null)
  const communities = ref(null)
  const selectedId = ref('')
  const neighbors = ref(null)
  const collaborators = ref(null)
  const path = ref(null)
  const team = ref(null)
  const rebuildResult = ref(null)
  // T2 图 RAG 多跳邻域扩展结果（独立 loading/error，与图本身互不连带）
  const ragResults = ref(null)

  const pathDraft = reactive({ source: '', target: '' })
  const collaboratorLimit = ref(COLLABORATOR_LIMIT_DEFAULT)
  const teamDraft = reactive({
    requiredSkills: [],
    requiredDomains: [],
    maxMembers: 5,
    minRating: 4,
    goal: ''
  })
  // RAG 输入草稿：seeds 为节点 id 数组
  const ragDraft = reactive({
    seeds: [],
    maxDepth: 2,
    topK: 20,
    nodeTypes: [],
    minWeight: 0
  })

  // ── U1 画布编辑（视觉态，不落后端）─────────────────────────────
  // editMode/linkSource/pendingEdge/nodeDraft 都是"画布操作态"，不是图数据：
  // 拖出来的坐标只活在 dragPositions 里（reload 即弃），绝不写进 graph.value 那份派生图。
  const editMode = ref(false)
  const dragPositions = ref({})
  const linkSourceId = ref('')
  const pendingEdge = reactive({ source: '', target: '' })
  // 新增节点表单（管理写面）：id/label/node_type
  const nodeDraft = reactive({ id: '', label: '', nodeType: 'expert' })

  const loading = reactive({
    graph: false, metrics: false, communities: false, node: false, path: false, team: false, rebuild: false, rag: false
  })
  const error = reactive({
    graph: '', metrics: '', communities: '', node: '', path: '', team: '', rebuild: '', rag: ''
  })

  // 布局是纯函数的产物；dragPositions 只是在它之上叠加一层"人手动挪过的位置"。
  // 不回写 graph.value——视觉坐标不属于图谱数据模型（MVP 取舍，见 frontend-fix-report U1 节）。
  const layout = computed(() => {
    const base = graphLayout(graph.value?.nodes || [], graph.value?.edges || [])
    const overrides = dragPositions.value || {}
    const posIds = new Set(Object.keys(overrides))
    if (!posIds.size) return base
    const positions = { ...base.positions }
    for (const id of posIds) {
      if (positions[id]) positions[id] = { ...overrides[id] }
    }
    const nodes = base.nodes.map((n) => (overrides[n.id] ? { ...n, ...overrides[n.id] } : n))
    return { ...base, positions, nodes }
  })
  const nodesById = computed(() => {
    const map = {}
    for (const n of layout.value.nodes) map[n.id] = n
    return map
  })
  // 两个候选下拉都过 graphNodeLabel：下拉里印一串问号等于让使用者拿丢码的 id 去提问。
  const expertOptions = computed(() =>
    layout.value.nodes
      .filter((n) => n.nodeType === GRAPH_NODE_TYPE.expert)
      .map((n) => ({ value: n.id, label: graphNodeLabel(n) }))
  )
  const domainOptions = computed(() =>
    graph.value?.nodes
      ?.filter((n) => n.nodeType === GRAPH_NODE_TYPE.domain)
      .map((n) => ({ value: n.id.slice(DOMAIN_NODE_ID_PREFIX.length), label: graphNodeLabel(n) })) || []
  )
  const selectedNode = computed(() => nodesById.value[selectedId.value] || null)
  /** 截断事实要说给界面：limit 生效后返回条数会小于 total_collaborators */
  const collaboratorsTruncated = computed(() => {
    const c = collaborators.value
    return !!c && c.collaborators.length < c.totalCollaborators
  })
  const teamProblem = computed(() => optimalTeamProblem(teamDraft))
  const ragProblem = computed(() => ragExpandProblem(ragDraft))

  async function loadGraph() {
    loading.graph = true
    error.graph = ''
    try {
      graph.value = await api.graphOverview()
      // 重取即新图：人手动挪过的位置对新节点集合无意义，丢弃避免旧坐标贴到新 id 上
      dragPositions.value = {}
      if (!selectedId.value && graph.value.nodes.length) await selectNode(graph.value.nodes[0].id)
    } catch (e) {
      error.graph = e?.msg || e?.message || '协作图谱获取失败'
      graph.value = null
    } finally {
      loading.graph = false
    }
  }

  async function loadMetrics() {
    loading.metrics = true
    error.metrics = ''
    try {
      metrics.value = await api.graphStats()
    } catch (e) {
      error.metrics = e?.msg || e?.message || '图谱统计获取失败'
      metrics.value = null
    } finally {
      loading.metrics = false
    }
  }

  async function loadCommunities() {
    loading.communities = true
    error.communities = ''
    try {
      communities.value = await api.graphCommunities()
    } catch (e) {
      error.communities = e?.msg || e?.message || '社区划分获取失败'
      communities.value = null
    } finally {
      loading.communities = false
    }
  }

  function setPathDraft(side, value) {
    pathDraft[side] = value
  }

  function setTeamValue(key, value) {
    teamDraft[key] = value
  }

  /**
   * 取邻居 + 协作者。collaborators 只对专家节点有意义：
   * handler 只统计 collaborates_with 边，问域节点会 200 返回空表，白跑一趟。
   */
  async function selectNode(id) {
    if (!id) return
    selectedId.value = id
    loading.node = true
    error.node = ''
    neighbors.value = null
    collaborators.value = null
    try {
      const settled = await Promise.allSettled([
        api.graphNeighbors(id),
        nodesById.value[id]?.nodeType === GRAPH_NODE_TYPE.expert
          ? api.graphCollaborators(id, collaboratorLimit.value)
          : Promise.resolve(null)
      ])
      const [nb, col] = settled
      if (nb.status === 'rejected') throw nb.reason
      neighbors.value = nb.value
      collaborators.value = col.status === 'fulfilled' ? col.value : null
    } catch (e) {
      error.node = e?.msg || e?.message || '节点邻域获取失败'
    } finally {
      loading.node = false
    }
  }

  /** 改 limit 只对专家节点重取协作者：域节点根本没有协作边，重取也是空跑 */
  async function changeCollaboratorLimit(limit) {
    collaboratorLimit.value = limit
    if (!selectedId.value) return
    if (nodesById.value[selectedId.value]?.nodeType !== GRAPH_NODE_TYPE.expert) return
    loading.node = true
    error.node = ''
    try {
      collaborators.value = await api.graphCollaborators(selectedId.value, limit)
    } catch (e) {
      error.node = e?.msg || e?.message || '协作者获取失败'
    } finally {
      loading.node = false
    }
  }

  async function findPath() {
    const { source, target } = pathDraft
    if (!source || !target) {
      error.path = '请先选择起点与终点'
      return
    }
    loading.path = true
    error.path = ''
    try {
      path.value = await api.graphPath(source, target)
    } catch (e) {
      error.path = e?.msg || e?.message || '路径查询失败'
      path.value = null
    } finally {
      loading.path = false
    }
  }

  /** 重建是写操作：版本号 +1，之后必须重取图与统计，否则界面停留在旧图上 */
  async function rebuild() {
    loading.rebuild = true
    error.rebuild = ''
    try {
      rebuildResult.value = await api.rebuildGraph()
      await Promise.all([loadGraph(), loadMetrics(), loadCommunities()])
      return rebuildResult.value
    } catch (e) {
      error.rebuild = e?.msg || e?.message || '图谱重建失败'
      return null
    } finally {
      loading.rebuild = false
    }
  }

  // ── 节点级 CRUD（N4，管理写面；成功后重取图保持画布一致。U1 画布留接 UI）──
  async function createGraphNode(body) {
    try {
      const res = await api.createGraphNode(body)
      await loadGraph()
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '新增节点失败'
      return null
    }
  }

  async function updateGraphNode(id, body) {
    try {
      const res = await api.updateGraphNode(id, body)
      await loadGraph()
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '更新节点失败'
      return null
    }
  }

  async function deleteGraphNode(id) {
    try {
      const res = await api.deleteGraphNode(id)
      await Promise.all([loadGraph(), loadMetrics()])
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '删除节点失败'
      return null
    }
  }

  async function createGraphEdge(body) {
    try {
      const res = await api.createGraphEdge(body)
      await loadGraph()
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '新增边失败'
      return null
    }
  }

  async function updateGraphEdge(seq, body) {
    try {
      const res = await api.updateGraphEdge(seq, body)
      await loadGraph()
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '更新边失败'
      return null
    }
  }

  async function deleteGraphEdge(seq) {
    try {
      const res = await api.deleteGraphEdge(seq)
      await loadGraph()
      return res
    } catch (e) {
      error.graph = e?.msg || e?.message || '删除边失败'
      return null
    }
  }

  async function formTeam() {
    if (teamProblem.value) {
      error.team = teamProblem.value
      return null
    }
    loading.team = true
    error.team = ''
    try {
      team.value = await api.optimalTeam({ ...teamDraft })
      return team.value
    } catch (e) {
      error.team = e?.msg || e?.message || '最优团队组建失败'
      return null
    } finally {
      loading.team = false
    }
  }

  /**
   * T2 图 RAG 多跳邻域扩展。读面公开，不写图。
   * 结果落在 ragResults（含 query/results/stats/rerank），UI 由消费方后续接。
   */
  async function expandNeighborhood() {
    if (ragProblem.value) {
      error.rag = ragProblem.value
      return null
    }
    loading.rag = true
    error.rag = ''
    try {
      ragResults.value = await api.expandGraphNeighborhood({ ...ragDraft })
      return ragResults.value
    } catch (e) {
      error.rag = e?.msg || e?.message || '邻域扩展失败'
      ragResults.value = null
      return null
    } finally {
      loading.rag = false
    }
  }

  // ── U1 画布编辑动作 ────────────────────────────────────────────
  function setEditMode(v) {
    editMode.value = !!v
    if (!editMode.value) {
      linkSourceId.value = ''
      pendingEdge.source = ''
      pendingEdge.target = ''
    }
  }

  /** 拖拽落位：只写视觉覆盖，不碰后端（坐标不入图谱数据模型） */
  function setNodePosition(id, x, y) {
    if (!id) return
    dragPositions.value = { ...dragPositions.value, [id]: { x, y } }
  }

  /**
   * 编辑模式下点节点的意图分发：
   * - 没有连线源 → 记下它当连线起点；
   * - 已有连线源且点了别的节点 → 交给视图弹边类型选择（pendingEdge）；
   * - 否则退化为普通选中。
   */
  function canvasClickNode(id) {
    if (!editMode.value) return 'select'
    if (!linkSourceId.value) {
      linkSourceId.value = id
      return 'link-source'
    }
    if (linkSourceId.value === id) {
      linkSourceId.value = ''
      return 'select'
    }
    pendingEdge.source = linkSourceId.value
    pendingEdge.target = id
    linkSourceId.value = ''
    return 'pending-edge'
  }

  function cancelLink() {
    linkSourceId.value = ''
    pendingEdge.source = ''
    pendingEdge.target = ''
  }

  function resetNodeDraft() {
    nodeDraft.id = ''
    nodeDraft.label = ''
    nodeDraft.nodeType = 'expert'
  }

  /**
   * T2 RAG 结果增量并入画布（读面操作，不写后端图）：
   * - results 里出现、当前图没有的节点 → 追加（幂等，已存在跳过）；
   * - 每条首跳边 → 按 source|target|edgeType 去重后追加；
   * - 原节点不动，layout 是 computed，graph.value 一换它自动重排。
   * 不调 loadGraph：那会把手动并入的邻域冲掉（派生图 rebuild 才含它们）。
   */
  function mergeRagResults(rows = []) {
    const g = graph.value
    if (!g) return { addedNodes: 0, addedEdges: 0 }
    const nodes = [...(g.nodes || [])]
    const edges = [...(g.edges || [])]
    const nodeIds = new Set(nodes.map((n) => n.id))
    const edgeKey = (e) => `${e.source}|${e.target}|${e.edgeType}`
    const edgeKeys = new Set(edges.map(edgeKey))
    let addedNodes = 0
    let addedEdges = 0
    for (const r of rows) {
      if (r?.id && !nodeIds.has(r.id)) {
        nodes.push({ id: r.id, label: r.label || '', nodeType: r.nodeType || 'expert', properties: {} })
        nodeIds.add(r.id)
        addedNodes++
      }
      for (const fh of r?.firstHops || []) {
        if (!fh?.from || !fh?.to || fh.from === fh.to) continue
        const k = `${fh.from}|${fh.to}|${fh.edgeType}`
        if (!edgeKeys.has(k)) {
          edges.push({ source: fh.from, target: fh.to, edgeType: fh.edgeType || 'collaborates_with', weight: fh.weight || 0 })
          edgeKeys.add(k)
          addedEdges++
        }
      }
    }
    graph.value = {
      ...g,
      nodes,
      edges,
      stats: { ...(g.stats || {}), nodeCount: nodes.length, edgeCount: edges.length }
    }
    return { addedNodes, addedEdges }
  }

  /** 展开邻域并就地并入画布：以当前选中节点为唯一 seed，深度 2（读面，登录即可用） */
  async function expandSelectedNeighborhood() {
    const id = selectedId.value
    if (!id) return { addedNodes: 0, addedEdges: 0 }
    ragDraft.seeds = [id]
    ragDraft.maxDepth = 2
    const res = await expandNeighborhood()
    if (!res) return { addedNodes: 0, addedEdges: 0 }
    return mergeRagResults(res.results || [])
  }

  return {
    graph, metrics, communities, selectedId, selectedNode, neighbors, collaborators, path, team, rebuildResult, ragResults,
    pathDraft, teamDraft, ragDraft, collaboratorLimit, loading, error,
    layout, nodesById, expertOptions, domainOptions, collaboratorsTruncated, teamProblem, ragProblem,
    loadGraph, loadMetrics, loadCommunities, selectNode, changeCollaboratorLimit,
    setPathDraft, setTeamValue, findPath, rebuild, formTeam, expandNeighborhood,
    createGraphNode, updateGraphNode, deleteGraphNode,
    createGraphEdge, updateGraphEdge, deleteGraphEdge,
    editMode, dragPositions, linkSourceId, pendingEdge, nodeDraft,
    setEditMode, setNodePosition, canvasClickNode, cancelLink, resetNodeDraft,
    mergeRagResults, expandSelectedNeighborhood
  }
})
