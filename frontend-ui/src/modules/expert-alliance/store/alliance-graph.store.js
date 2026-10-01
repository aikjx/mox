// 协作图谱状态：唯一持有图数据的前序状态。视图只读绑定、只发意图。
// 图与统计/社区是三份独立接口，各自可失败，因此逐区记账 loading/error，
// 一处失败不得连带清空已取到的部分。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import { COLLABORATOR_LIMIT_DEFAULT, DOMAIN_NODE_ID_PREFIX, GRAPH_NODE_TYPE, optimalTeamProblem } from '@/modules/expert-alliance/contract'
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

  const pathDraft = reactive({ source: '', target: '' })
  const collaboratorLimit = ref(COLLABORATOR_LIMIT_DEFAULT)
  const teamDraft = reactive({
    requiredSkills: [],
    requiredDomains: [],
    maxMembers: 5,
    minRating: 4,
    goal: ''
  })

  const loading = reactive({
    graph: false, metrics: false, communities: false, node: false, path: false, team: false, rebuild: false
  })
  const error = reactive({
    graph: '', metrics: '', communities: '', node: '', path: '', team: '', rebuild: ''
  })

  const layout = computed(() => graphLayout(graph.value?.nodes || [], graph.value?.edges || []))
  const nodesById = computed(() => {
    const map = {}
    for (const n of layout.value.nodes) map[n.id] = n
    return map
  })
  const expertOptions = computed(() =>
    layout.value.nodes
      .filter((n) => n.nodeType === GRAPH_NODE_TYPE.expert)
      .map((n) => ({ value: n.id, label: n.label || n.id }))
  )
  const domainOptions = computed(() =>
    graph.value?.nodes
      ?.filter((n) => n.nodeType === GRAPH_NODE_TYPE.domain)
      .map((n) => ({ value: n.id.slice(DOMAIN_NODE_ID_PREFIX.length), label: n.label })) || []
  )
  const selectedNode = computed(() => nodesById.value[selectedId.value] || null)
  /** 截断事实要说给界面：limit 生效后返回条数会小于 total_collaborators */
  const collaboratorsTruncated = computed(() => {
    const c = collaborators.value
    return !!c && c.collaborators.length < c.totalCollaborators
  })
  const teamProblem = computed(() => optimalTeamProblem(teamDraft))

  async function loadGraph() {
    loading.graph = true
    error.graph = ''
    try {
      graph.value = await api.graphOverview()
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

  return {
    graph, metrics, communities, selectedId, selectedNode, neighbors, collaborators, path, team, rebuildResult,
    pathDraft, teamDraft, collaboratorLimit, loading, error,
    layout, nodesById, expertOptions, domainOptions, collaboratorsTruncated, teamProblem,
    loadGraph, loadMetrics, loadCommunities, selectNode, changeCollaboratorLimit,
    setPathDraft, setTeamValue, findPath, rebuild, formTeam
  }
})
