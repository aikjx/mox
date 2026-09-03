// 知识图谱 API
import http from './http'

export const getGraph = () => http.get('/graph')
export const getGraphStats = () => http.get('/graph/stats')
export const getCentrality = () => http.get('/graph/centrality')
export const getCommunities = () => http.get('/graph/communities')
export const getPagerank = () => http.get('/graph/pagerank')
export const getNeighbors = (id) => http.get(`/graph/neighbors/${encodeURIComponent(id)}`)
export const getShortestPath = (source, target) =>
  http.get('/graph/path', { params: { source, target } })
export const recommendNodes = (payload) => http.post('/graph/recommend', payload)
export const addGraphNode = (payload) => http.post('/graph/node', payload)
export const addGraphEdge = (payload) => http.post('/graph/edge', payload)
// 激活传播：从种子节点沿边扩散激活能量，返回各节点激活值
export const propagateActivation = (seedNodes, iterations = 10) =>
  http.post('/graph/activate', { start_nodes: seedNodes, iterations })

// ===== 对话自动→知识图谱 自动整理 =====
// 统一搜索：对话内容 + 知识图谱节点
export const graphSearch = (q, limit = 20) =>
  http.get('/graph/search', { params: { q, limit } })
// 切换全自动同步开关
export const toggleAutoSync = (enabled) =>
  http.post('/graph/auto-sync/toggle', { enabled })
// 查询全自动同步状态
export const getAutoSyncStatus = () => http.get('/graph/auto-sync/status')
// 列出对话会话
export const getDialogueSessions = () => http.get('/dialogue/sessions')
/** @deprecated 请使用 getDialogueSessions */
export const listDialogueSessions = getDialogueSessions
// 导出：对话 + 知识图谱 打包为单文件迁移包（返回 JSON 文本）
export const graphExport = () => http.get('/graph/export')
// 导入：从迁移包恢复对话 + 知识图谱（幂等合并）
export const graphImport = (bundle) => http.post('/graph/import', bundle)

// AI 图谱增强
export const aiGraphInsights = (payload) => http.post('/graph/ai-insights', payload)

// ===== 聚合图谱：系统架构 + 所有业务实体 =====
// 并行调用所有业务 API，将业务实体映射为图谱节点，创建关联边，与基础系统架构图谱合并
export const getAggregatedGraph = async () => {
  // 1. 获取基础系统架构图谱
  const base = await getGraph()
  const nodeMap = new Map()
  const edgeSet = new Set()
  const nodes = []
  const edges = []

  // 去重辅助：添加节点（按 id 去重）
  const addNode = (n) => {
    if (!n || !n.id) return
    if (nodeMap.has(n.id)) return
    nodeMap.set(n.id, n)
    nodes.push(n)
  }

  // 去重辅助：添加边（按 source-target-relation 去重）
  const addEdge = (source, target, relation, weight = 0.6) => {
    if (!source || !target) return
    const key = `${source}->${target}:${relation}`
    if (edgeSet.has(key)) return
    edgeSet.add(key)
    edges.push({
      id: `e-${edges.length + 1}-${source}-${target}`,
      source,
      target,
      relation,
      weight
    })
  }

  // 2. 先加入基础系统架构节点和边
  ;(base.nodes || []).forEach(addNode)
  ;(base.edges || []).forEach((e) => {
    const s = e.source || e.from
    const t = e.target || e.to
    if (s && t) addEdge(s, t, e.relation || e.type || 'related', e.weight || 0.5)
  })

  // 3. 并行调用所有业务 API（单个失败不影响整体）
  //    标记 silent：这些是图谱的可选增强数据，失败时仅降级（不渲染对应节点），
  //    不触发全局报错 toast——由 Promise.allSettled 统一吞掉
  const results = await Promise.allSettled([
    http.get('/projects', { silent: true }),
    http.get('/experts', { silent: true }),
    http.get('/operators', { silent: true }),
    http.get('/tasks', { silent: true }),
    http.get('/kb/documents', { silent: true }),
    http.get('/ai/workflows', { silent: true }),
    http.get('/automation', { silent: true })
  ])

  const unwrap = (r) => (r.status === 'fulfilled' ? r.value : [])
  const projects = unwrap(results[0]) || []
  const experts = unwrap(results[1]) || []
  const operators = unwrap(results[2]) || []
  const tasks = unwrap(results[3]) || []
  const documents = unwrap(results[4]) || []
  const workflows = unwrap(results[5]) || []
  // automations 目前为空，预留扩展

  // 4. 映射业务实体为图谱节点
  const projectNodes = projects.map((p) => ({
    id: 'proj-' + p.id,
    name: p.name,
    node_type: 'project',
    category: p.type || 'platform',
    description: p.description,
    status: p.status
  }))

  const expertNodes = experts.map((e) => ({
    id: 'exp-' + e.id,
    name: e.name,
    node_type: 'expert',
    category: e.role,
    description: (e.capabilities || []).join(', '),
    status: e.status
  }))

  const operatorNodes = operators.map((o) => ({
    id: 'op-' + o.id,
    name: o.name,
    node_type: 'operator',
    category: o.category,
    status: o.status
  }))

  const taskNodes = (tasks || []).map((t) => ({
    id: 'task-' + t.id,
    name: t.title || t.name || ('任务' + t.id),
    node_type: 'task',
    category: t.priority || 'normal',
    status: t.status
  }))

  // 需求文档（doc_type=requirement）→ 需求节点；其他文档 → 文档节点（需求数据源=云盘知识库 kb-store）
  const requirementNodes = []
  const docNodes = []
  ;(documents || []).forEach((d) => {
    const isReq = d.doc_type === 'requirement' || String(d.category || '').includes('requirement')
    const base = {
      name: d.title || d.name || ((isReq ? '需求' : '文档') + String(d.id).slice(-6)),
      category: d.category || 'knowledge',
      project_id: d.project_id
    }
    if (isReq) requirementNodes.push({ id: 'req-' + d.id, ...base, node_type: 'requirement' })
    else docNodes.push({ id: 'doc-' + d.id, ...base, node_type: 'doc' })
  })

  const workflowNodes = (workflows || []).map((w) => ({
    id: 'wf-' + w.id,
    name: w.name || w.title || ('工作流' + w.id),
    node_type: 'workflow',
    category: w.category || 'automation',
    status: w.status
  }))

  // 加入所有业务节点
  projectNodes.forEach(addNode)
  expertNodes.forEach(addNode)
  operatorNodes.forEach(addNode)
  taskNodes.forEach(addNode)
  requirementNodes.forEach(addNode)
  docNodes.forEach(addNode)
  workflowNodes.forEach(addNode)

  // 5. 创建业务实体 → 系统组件的边
  projectNodes.forEach((p) => {
    addEdge(p.id, 'op-engine', 'uses', 0.8)
    addEdge(p.id, 'kg-engine', 'queries', 0.7)
    addEdge(p.id, 'kb-store', 'stores', 0.6)
    addEdge(p.id, 'flow-engine', 'runs', 0.7)
  })

  expertNodes.forEach((e) => {
    addEdge(e.id, 'expert-alliance', 'belongs_to', 0.9)
  })

  operatorNodes.forEach((o) => {
    addEdge(o.id, 'op-engine', 'executed_by', 0.8)
  })

  docNodes.forEach((d) => {
    addEdge(d.id, 'kb-store', 'stored_in', 0.7)
  })

  // 需求节点：源于云盘知识库 → kb-store，并关联所属项目（has_requirement）
  requirementNodes.forEach((r) => {
    addEdge(r.id, 'kb-store', 'stored_in', 0.7)
    const pid = r.project_id
    if (pid && nodeMap.has('proj-' + pid)) {
      addEdge('proj-' + pid, r.id, 'has_requirement', 0.8)
    }
  })

  workflowNodes.forEach((w) => {
    addEdge(w.id, 'flow-engine', 'orchestrated_by', 0.8)
  })

  // 6. 创建业务实体间关联边

  // 6a. project → expert（按项目类型映射专家）
  //   platform → exp-arch
  //   government → exp-arch + exp-algo
  const expertById = new Map(experts.map((e) => [e.id, e]))
  projectNodes.forEach((p) => {
    const rawProject = projects.find((rp) => 'proj-' + rp.id === p.id)
    const ptype = rawProject ? rawProject.type : 'platform'
    let targetExpertIds = []
    if (ptype === 'government') {
      targetExpertIds = ['exp-arch', 'exp-algo']
    } else {
      targetExpertIds = ['exp-arch']
    }
    targetExpertIds.forEach((eid) => {
      if (expertById.has(eid)) {
        addEdge(p.id, 'exp-' + eid, 'has_expert', 0.7)
      }
    })
  })

  // 6b. project → operator（按项目类型映射算子，默认关联所有算子）
  projectNodes.forEach((p) => {
    operatorNodes.forEach((o) => {
      addEdge(p.id, o.id, 'uses_operator', 0.6)
    })
  })

  // 6c. project → document（当 doc.project_id === project.id 时）
  docNodes.forEach((d) => {
    if (d.project_id) {
      const projId = 'proj-' + d.project_id
      if (nodeMap.has(projId)) {
        addEdge(projId, d.id, 'has_document', 0.7)
      }
    }
  })

  // 6d. expert → operator（按 expert.capabilities 与 operator.category 匹配）
  //   架构专家(architect) → 所有算子
  //   算法专家(algo) → nlp 类算子
  //   其他 → 按 capabilities 交集匹配
  expertNodes.forEach((e) => {
    const rawExpert = experts.find((re) => 'exp-' + re.id === e.id)
    if (!rawExpert) return
    const role = rawExpert.role || ''
    const caps = (rawExpert.capabilities || []).map((c) => String(c).toLowerCase())
    operatorNodes.forEach((o) => {
      const rawOp = operators.find((ro) => 'op-' + ro.id === o.id)
      const opCat = rawOp ? String(rawOp.category || '').toLowerCase() : ''
      let match = false
      if (role === 'architect') {
        match = true
      } else if (role === 'algo') {
        match = opCat === 'nlp' || caps.includes(opCat)
      } else {
        match = caps.includes(opCat) || caps.some((c) => opCat.includes(c))
      }
      if (match) {
        addEdge(e.id, o.id, 'develops', 0.65)
      }
    })
  })

  return { nodes, edges }
}
