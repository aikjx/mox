'use strict'
/**
 * 全部 API 路由处理器。前端约定所有接口挂在 /api 下。
 * ctx 由 server.js 注入：{ store, xuanji, graph, seed, sendJSON, sendError, sendText, route, logs, startTime }
 */
function pick(obj, keys) {
  const r = {}
  keys.forEach((k) => (r[k] = obj[k]))
  return r
}

module.exports = function registerRoutes(ctx) {
  const { store, xuanji, graph, sendJSON, sendError, sendText, route, logs } = ctx

  // ============ 系统 ============
  route('GET', '/api/health', (h) =>
    sendJSON(h.res, 200, {
      status: 'ok',
      uptime: Math.floor((Date.now() - ctx.startTime) / 1000),
      version: '1.0.0',
      ts: new Date().toISOString(),
      modules: ['xuanji', 'graph', 'market', 'mcp', 'automation']
    })
  )

  route('GET', '/api/status', (h) =>
    sendJSON(h.res, 200, {
      status: 'running',
      backend: 'ous-backend (zero-dep node)',
      collections: ['operators', 'graph_nodes', 'graph_edges', 'market', 'plugins', 'workflows', 'flows', 'resources'],
      counts: {
        operators: store.all('operators').length,
        graph_nodes: store.all('graph_nodes').length,
        graph_edges: store.all('graph_edges').length,
        market: store.all('market').length,
        plugins: store.all('plugins').length
      }
    })
  )

  route('GET', '/api/status/full', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, {
      status: 'running',
      uptime: Math.floor((Date.now() - ctx.startTime) / 1000),
      graph: { nodes: nodes.length, edges: edges.length },
      operators: store.all('operators').length,
      market: store.all('market').length,
      workflows: store.all('workflows').length,
      flows: store.all('flows').length,
      resources: store.all('resources').length,
      xuanji: { dimensions: 14, leagues: 2 },
      mcp: { tools: 5 }
    })
  })

  route('GET', '/api/logs', (h) => {
    const n = Math.min(200, parseInt(h.query.n || '50', 10))
    sendJSON(h.res, 200, { logs: logs.slice(-n) })
  })

  route('GET', '/api/plugins', (h) => sendJSON(h.res, 200, { items: store.all('plugins') }))

  route('GET', '/api/docs', (h) =>
    sendText(
      h.res,
      200,
      '# OUS Backend API\n\nZero-dependency Node.js service. All endpoints under /api.\nSee frontend/src/api/index.js for the full contract.'
    )
  )

  // ============ 算子 ============
  route('GET', '/api/operators', (h) => sendJSON(h.res, 200, { items: store.all('operators') }))

  route('POST', '/api/operators/register', (h) => {
    const b = h.body || {}
    if (!b.name) return sendError(h.res, 400, 'name 必填')
    const op = store.insert('operators', {
      name: b.name,
      type: b.type || 'algorithm',
      category: b.category || 'general',
      desc: b.desc || '',
      version: b.version || '1.0.0',
      status: b.status || 'active',
      tags: b.tags || []
    })
    sendJSON(h.res, 201, op)
  })

  route('POST', '/api/execute', (h) => {
    const b = h.body || {}
    logs.push(`[execute] ${(b.operator || b.workflow || 'unknown')} @ ${new Date().toISOString()}`)
    sendJSON(h.res, 200, {
      executed: true,
      operator: b.operator || b.workflow || null,
      inputs: b.inputs || {},
      result: { status: 'ok', elapsed_ms: Math.floor(Math.random() * 120) + 8 },
      ts: new Date().toISOString()
    })
  })

  // ============ 知识图谱 ============
  route('GET', '/api/graph', (h) =>
    sendJSON(h.res, 200, { nodes: store.all('graph_nodes'), edges: store.all('graph_edges') })
  )

  route('GET', '/api/graph/stats', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    const n = nodes.length
    const density = n > 1 ? +(edges.length / (n * (n - 1))).toFixed(4) : 0
    const comm = graph.communities(nodes, edges)
    sendJSON(h.res, 200, {
      nodes: n,
      edges: edges.length,
      density,
      communities: comm.length,
      types: [...new Set(nodes.map((x) => x.type))].length
    })
  })

  route('GET', '/api/graph/centrality', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, {
      degree_centrality: graph.degreeCentrality(nodes, edges),
      betweenness_centrality: graph.betweennessCentrality(nodes, edges)
    })
  })

  route('GET', '/api/graph/communities', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, graph.communities(nodes, edges))
  })

  route('GET', '/api/graph/pagerank', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, graph.pagerank(nodes, edges))
  })

  route('GET', '/api/graph/neighbors/:id', (h) => {
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, graph.neighbors(nodes, edges, decodeURIComponent(h.params.id)))
  })

  route('GET', '/api/graph/path', (h) => {
    const { source, target } = h.query
    if (!source || !target) return sendError(h.res, 400, 'source 与 target 必填')
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    sendJSON(h.res, 200, graph.shortestPath(nodes, edges, source, target))
  })

  route('POST', '/api/graph/recommend', (h) => {
    const b = h.body || {}
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    const ctxNodes = b.context_nodes || b.nodes || []
    sendJSON(h.res, 200, graph.recommend(nodes, edges, ctxNodes, b.limit || 8))
  })

  route('POST', '/api/graph/node', (h) => {
    const b = h.body || {}
    if (!b.id && !b.label) return sendError(h.res, 400, 'id 或 label 必填')
    const node = store.insert('graph_nodes', {
      id: b.id || 'n_' + Date.now().toString(36),
      label: b.label || b.id || 'node',
      type: b.type || 'operator',
      node_type: b.type || 'operator',
      color: b.color || '#5B8FF9',
      size: b.size || 8
    })
    sendJSON(h.res, 201, node)
  })

  route('POST', '/api/graph/edge', (h) => {
    const b = h.body || {}
    if (!b.source && !b.from) return sendError(h.res, 400, 'source/from 必填')
    if (!b.target && !b.to) return sendError(h.res, 400, 'target/to 必填')
    const edge = store.insert('graph_edges', {
      source: b.source || b.from,
      target: b.target || b.to,
      weight: b.weight != null ? b.weight : 1
    })
    sendJSON(h.res, 201, edge)
  })

  route('POST', '/api/graph/activate', (h) => {
    const b = h.body || {}
    const nodes = store.all('graph_nodes')
    const edges = store.all('graph_edges')
    const arr = graph.activate(nodes, edges, b.start_nodes || [], b.iterations || 10)
    const map = {}
    arr.forEach((x) => (map[x.id] = x.value))
    sendJSON(h.res, 200, map)
  })

  route('GET', '/api/graph/search', (h) => {
    const q = (h.query.q || '').toLowerCase()
    const limit = Math.min(50, parseInt(h.query.limit || '20', 10))
    const nodes = store.all('graph_nodes')
    const sessions = store.all('dialogue_sessions')
    const graph_nodes = nodes
      .filter((n) => (n.label || '').toLowerCase().includes(q) || (n.id || '').toLowerCase().includes(q))
      .slice(0, limit)
      .map((n) => ({ id: n.id, title: n.label, snippet: '类型=' + n.type }))
    const dialogues = sessions
      .filter((s) => (s.title || '').toLowerCase().includes(q))
      .slice(0, limit)
      .map((s) => ({ id: s.id, snippet: s.title }))
    sendJSON(h.res, 200, { dialogues, graph_nodes, q })
  })

  route('POST', '/api/graph/auto-sync/toggle', (h) => {
    const b = h.body || {}
    const cur = store.get('settings', 'auto_sync') || { id: 'auto_sync' }
    const upd = store.update('settings', 'auto_sync', { enabled: !!b.enabled, updated_at: new Date().toISOString() })
    sendJSON(h.res, 200, { enabled: upd ? upd.enabled : !!b.enabled })
  })

  route('GET', '/api/graph/auto-sync/status', (h) => {
    const s = store.get('settings', 'auto_sync') || { id: 'auto_sync', enabled: false }
    sendJSON(h.res, 200, { enabled: s.enabled })
  })

  route('GET', '/api/dialogue/sessions', (h) => sendJSON(h.res, 200, { items: store.all('dialogue_sessions') }))

  route('GET', '/api/graph/export', (h) => sendJSON(h.res, 200, graph.exportBundle(store)))

  route('POST', '/api/graph/import', (h) => {
    const b = h.body || {}
    const r = graph.mergeBundle(store, b)
    sendJSON(h.res, 200, r)
  })

  // ============ AI 对话 ============
  function localReply(text) {
    const t = (text || '').toLowerCase()
    if (t.includes('图谱') || t.includes('graph')) return '知识图谱模块支持 PageRank、社区发现、最短路径与激活传播分析，可在「知识图谱」页直接操作。'
    if (t.includes('璇玑') || t.includes('治理') || t.includes('xuanji')) return '双璇玑十四维治理：业务7维 + 开发7维并行派发 → 归一化裁决 → ⛨璇玑验证网关 → 治理闸门 → 优化出图。可在「全维融合」页粘贴蓝图归一化。'
    if (t.includes('算子') || t.includes('operator')) return '算子中心已注册 PageRank、社区发现、激活传播、归一化 IR、双璇玑治理等算子，可在「算子中心」查看或注册新算子。'
    if (t.includes('市场') || t.includes('market')) return '算子商城支持上传、克隆、导出 DSL，融合发布产物可直接上架。'
    return '我是 OUS 智能助手。可帮助你进行全维治理、图谱分析、算子管理与自动化编排。请描述你的需求。'
  }

  route('POST', '/api/ai/chat', (h) => {
    const b = h.body || {}
    const session = b.session || 'sess_' + Date.now().toString(36)
    const reply = localReply(b.message)
    let sess = store.get('dialogue_sessions', session)
    if (!sess) {
      sess = store.insert('dialogue_sessions', { id: session, title: (b.message || '').slice(0, 20), messages: [] })
    }
    const messages = sess.messages || []
    messages.push({ role: 'user', content: b.message, ts: new Date().toISOString() })
    messages.push({ role: 'assistant', content: reply, ts: new Date().toISOString() })
    store.update('dialogue_sessions', session, { messages })
    sendJSON(h.res, 200, { session, reply, messages: messages.slice(-20) })
  })

  route('GET', '/api/ai/chat/history/:session', (h) => {
    const sess = store.get('dialogue_sessions', decodeURIComponent(h.params.session))
    sendJSON(h.res, 200, { session: h.params.session, messages: sess ? sess.messages || [] : [] })
  })

  route('POST', '/api/ai/analyze-algorithm', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, {
      algorithm: b.algorithm || b.name || 'unknown',
      complexity: 'O(V+E)',
      recommendation: '可在图谱上以归一化 IR 表达后交由双璇玑治理',
      ts: new Date().toISOString()
    })
  })

  route('GET', '/api/ai/algorithm-types', (h) =>
    sendJSON(h.res, 200, {
      items: [
        { key: 'graph', name: '图算法', list: ['pagerank', 'community', 'shortest_path', 'activation'] },
        { key: 'fusion', name: '融合治理', list: ['xuanji', 'normalize'] },
        { key: 'nlp', name: '自然语言', list: ['caomei_compile', 'refine'] }
      ]
    })
  )

  route('POST', '/api/analyze/spiral', (h) => {
    const b = h.body || {}
    const flow = b.flow || { nodes: b.nodes, edges: b.edges }
    if (!flow.nodes) return sendError(h.res, 400, 'flow.nodes 必填')
    const report = xuanji.runAlliance(flow, b.tenant || 'default')
    sendJSON(h.res, 200, { spiral: report.spiral, verification: report.verification, governance: report.governance })
  })

  route('GET', '/api/ai/resources', (h) => sendJSON(h.res, 200, { items: store.all('resources') }))
  route('GET', '/api/ai/resources/health', (h) => {
    const rs = store.all('resources')
    const avg = rs.length ? Math.round(rs.reduce((s, r) => s + (r.used || 0), 0) / rs.length) : 0
    sendJSON(h.res, 200, { healthy: avg < 85, utilization: avg, items: rs.length })
  })

  // ============ AI 插件 ============
  route('GET', '/api/ai/plugins', (h) => sendJSON(h.res, 200, { items: store.all('plugins') }))
  route('POST', '/api/ai/plugins/register', (h) => {
    const b = h.body || {}
    if (!b.name) return sendError(h.res, 400, 'name 必填')
    const p = store.insert('plugins', { name: b.name, type: b.type || 'plugin', desc: b.desc || '', status: 'active', endpoints: b.endpoints || 1 })
    sendJSON(h.res, 201, p)
  })
  route('POST', '/api/ai/plugins/send-message', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { delivered: true, plugin: b.plugin, message: b.message, ts: new Date().toISOString() })
  })
  route('GET', '/api/ai/plugins/topology', (h) => {
    const plugins = store.all('plugins')
    const nodes = plugins.map((p) => ({ id: p.id, label: p.name, type: 'plugin' }))
    const edges = plugins.slice(1).map((p, i) => ({ source: plugins[0].id, target: p.id, weight: 1 }))
    sendJSON(h.res, 200, { nodes, edges })
  })

  // ============ 工作流 ============
  route('GET', '/api/ai/workflows/templates', (h) =>
    sendJSON(h.res, 200, {
      items: [
        { id: 't_demand', name: '需求驱动闭环', steps: ['采集', '归一化', '治理', '发布'] },
        { id: 't_graph', name: '图谱分析', steps: ['导入', 'PageRank', '社区发现'] }
      ]
    })
  )
  route('GET', '/api/ai/workflows', (h) => sendJSON(h.res, 200, { items: store.all('workflows') }))
  route('POST', '/api/ai/workflows/save', (h) => {
    const b = h.body || {}
    const wf = b.id ? store.update('workflows', b.id, b) : store.insert('workflows', b)
    sendJSON(h.res, b.id ? 200 : 201, wf)
  })
  route('POST', '/api/ai/workflows/execute', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { executed: true, workflow: b.id || b.name, status: 'ok', ts: new Date().toISOString() })
  })
  route('GET', '/api/ai/workflows/instances', (h) =>
    sendJSON(h.res, 200, { items: [{ id: 'inst_1', workflow: 'wf_demo', status: 'completed', started_at: new Date().toISOString() }] })
  )

  // ============ 流程图 FlowGraph IR ============
  route('GET', '/api/ai/flows', (h) => sendJSON(h.res, 200, { items: store.all('flows') }))
  route('POST', '/api/ai/flows', (h) => {
    const b = h.body || {}
    if (!b.name) return sendError(h.res, 400, 'name 必填')
    const f = store.insert('flows', b)
    sendJSON(h.res, 201, f)
  })
  route('GET', '/api/ai/flows/:id', (h) => {
    const f = store.get('flows', decodeURIComponent(h.params.id))
    if (!f) return sendError(h.res, 404, 'flow 不存在')
    sendJSON(h.res, 200, f)
  })
  route('DELETE', '/api/ai/flows/:id', (h) => {
    const ok = store.remove('flows', decodeURIComponent(h.params.id))
    sendJSON(h.res, ok ? 200 : 404, { deleted: ok })
  })
  route('POST', '/api/ai/flows/validate', (h) => {
    const b = h.body || {}
    const nodes = b.nodes || []
    const edges = b.edges || []
    const ids = new Set(nodes.map((n) => String(n.id != null ? n.id : n)))
    const errors = []
    const warnings = []
    if (!nodes.length) errors.push('缺少节点')
    const dup = {}
    nodes.forEach((n) => {
      const k = String(n.id != null ? n.id : n)
      dup[k] = (dup[k] || 0) + 1
    })
    Object.keys(dup).forEach((k) => dup[k] > 1 && errors.push('重复节点 id: ' + k))
    edges.forEach((e) => {
      const s = e.source != null ? e.source : e.from
      const t = e.target != null ? e.target : e.to
      if (!ids.has(String(s))) errors.push('边起点缺失: ' + s)
      if (!ids.has(String(t))) errors.push('边终点缺失: ' + t)
    })
    if (!nodes.some((n) => (n.type || '') === 'monitor') && nodes.length > 3) warnings.push('建议补充监控节点')
    sendJSON(h.res, 200, { valid: errors.length === 0, errors, warnings, nodes: nodes.length, edges: edges.length })
  })
  route('POST', '/api/ai/flows/execute', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { executed: true, flow: b.id || b.name, status: 'ok', ts: new Date().toISOString() })
  })
  route('GET', '/api/ai/flows/node-types', (h) =>
    sendJSON(h.res, 200, {
      items: ['operator', 'ai_task', 'condition', 'data', 'monitor', 'system', 'plugin', 'resource', 'workflow', 'fusion', 'league']
    })
  )

  // ============ LLM 配置 ============
  route('GET', '/api/ai/llm/config', (h) => {
    const c = store.get('llm_config', 'llm_default') || { id: 'llm_default', provider: 'local', enabled: false }
    sendJSON(h.res, 200, c)
  })
  route('POST', '/api/ai/llm/config', (h) => {
    const b = h.body || {}
    const c = store.update('llm_config', 'llm_default', Object.assign({}, b, { id: 'llm_default', updated_at: new Date().toISOString() }))
    sendJSON(h.res, 200, c)
  })
  route('POST', '/api/ai/llm/test', (h) => sendJSON(h.res, 200, { ok: true, message: '本地推理引擎可达（无需外部 LLM）' }))

  // ============ 浏览器自动化 ============
  route('GET', '/api/ai/browser/templates', (h) =>
    sendJSON(h.res, 200, {
      items: [
        { id: 'bt_search', name: '搜索并摘录', steps: ['打开', '输入关键词', '提取结果'] },
        { id: 'bt_form', name: '表单填写', steps: ['定位字段', '填值', '提交'] }
      ]
    })
  )
  route('GET', '/api/ai/browser/sessions', (h) => sendJSON(h.res, 200, { items: [] }))
  route('GET', '/api/ai/browser/sessions/:id', (h) => sendJSON(h.res, 200, { id: h.params.id, status: 'closed' }))
  route('DELETE', '/api/ai/browser/sessions/:id', (h) => sendJSON(h.res, 200, { closed: true, id: h.params.id }))
  route('POST', '/api/ai/browser/execute-task', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { task: b.task, status: 'simulated', steps_run: (b.steps || 3), ts: new Date().toISOString() })
  })
  route('POST', '/api/ai/browser/execute-steps', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { steps: b.steps || [], status: 'simulated' })
  })
  route('POST', '/api/ai/browser/execute-action', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { action: b.action, status: 'simulated' })
  })
  route('POST', '/api/ai/browser/natural', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, { interpreted: b.text, plan: ['导航', '操作', '校验'], status: 'simulated' })
  })

  // ============ 算子商城 ============
  route('GET', '/api/market', (h) => {
    let items = store.all('market')
    const q = h.query.q
    const tag = h.query.tag
    if (q) items = items.filter((m) => (m.name || '').toLowerCase().includes(q.toLowerCase()))
    if (tag) items = items.filter((m) => (m.tags || []).includes(tag))
    sendJSON(h.res, 200, { items, total: items.length })
  })
  route('GET', '/api/market/random', (h) => {
    const items = store.all('market')
    const pick = items.length ? items[Math.floor(Math.random() * items.length)] : null
    sendJSON(h.res, 200, pick)
  })
  route('GET', '/api/market/:id', (h) => {
    const m = store.get('market', decodeURIComponent(h.params.id))
    if (!m) return sendError(h.res, 404, '算子不存在')
    sendJSON(h.res, 200, m)
  })
  route('POST', '/api/market/upload', (h) => {
    const b = h.body || {}
    if (!b.name) return sendError(h.res, 400, 'name 必填')
    const m = store.insert('market', {
      name: b.name,
      description: b.description || '',
      requirement: b.requirement || '',
      tags: b.tags || [],
      tenant: b.tenant || 'default',
      graph: b.graph || { nodes: [], edges: [] },
      governance_score: b.governance_score || null,
      governance_gate: b.governance_gate || null,
      nodes: (b.graph && b.graph.nodes ? b.graph.nodes.length : 0),
      edges: (b.graph && b.graph.edges ? b.graph.edges.length : 0),
      downloads: 0,
      author: b.author || 'user'
    })
    sendJSON(h.res, 201, m)
  })
  route('POST', '/api/market/:id', (h) => {
    const b = h.body || {}
    const m = store.update('market', decodeURIComponent(h.params.id), b)
    if (!m) return sendError(h.res, 404, '算子不存在')
    sendJSON(h.res, 200, m)
  })
  route('DELETE', '/api/market/:id', (h) => {
    const ok = store.remove('market', decodeURIComponent(h.params.id))
    sendJSON(h.res, ok ? 200 : 404, { deleted: ok })
  })
  route('POST', '/api/market/:id/clone', (h) => {
    const src = store.get('market', decodeURIComponent(h.params.id))
    if (!src) return sendError(h.res, 404, '算子不存在')
    const clone = store.insert('market', Object.assign({}, src, { id: undefined, name: src.name + ' (副本)', downloads: 0 }))
    sendJSON(h.res, 201, clone)
  })
  route('GET', '/api/market/:id/export', (h) => {
    const m = store.get('market', decodeURIComponent(h.params.id))
    if (!m) return sendError(h.res, 404, '算子不存在')
    sendJSON(h.res, 200, {
      kind: 'FlowDefinition',
      version: '1.0',
      id: m.id,
      name: m.name,
      description: m.description,
      tags: m.tags || [],
      graph: m.graph || { nodes: [], edges: [] }
    })
  })

  // ============ Caomei 需求编译器 ============
  function compileNL(text) {
    const t = (text || '').toLowerCase()
    const nodes = []
    const edges = []
    let i = 1
    const add = (name, type) => {
      const id = 'n' + i++
      nodes.push({ id, name, type })
      return id
    }
    const ingest = add('需求采集', 'operator')
    const norm = add('归一化 IR', 'operator')
    edges.push({ from: ingest, to: norm })
    if (t.includes('合规') || t.includes('审查') || t.includes('compliance')) {
      const c = add('AI 合规审查', 'ai_task')
      edges.push({ from: norm, to: c })
      var last = c
    } else {
      var last = norm
    }
    if (t.includes('分流') || t.includes('条件') || t.includes('branch')) {
      const cond = add('条件分流', 'condition')
      edges.push({ from: last, to: cond })
      last = cond
    }
    const arc = add('归档', 'operator')
    edges.push({ from: last, to: arc })
    return { nodes, edges, summary: '已基于自然语言生成 ' + nodes.length + ' 节点流程蓝图' }
  }
  route('POST', '/api/caomei/compile', (h) => {
    const b = h.body || {}
    const out = compileNL(b.text || b.requirement || '')
    sendJSON(h.res, 200, Object.assign({ text: b.text || b.requirement || '' }, out))
  })
  route('POST', '/api/caomei/refine', (h) => {
    const b = h.body || {}
    const base = b.blueprint || compileNL(b.text || '')
    // 精化：补监控节点
    const nodes = base.nodes ? base.nodes.slice() : []
    const edges = base.edges ? base.edges.slice() : []
    const monId = 'n_mon_' + Date.now().toString(36)
    nodes.push({ id: monId, name: '运行监控', type: 'monitor' })
    nodes.forEach((n) => edges.push({ from: n.id, to: monId }))
    sendJSON(h.res, 200, { refined: true, nodes, edges, summary: '已注入监控节点，蓝图更健壮' })
  })
  route('GET', '/api/caomei/templates', (h) => sendJSON(h.res, 200, { items: store.all('caomei_templates') }))

  // ============ MCP 兼容层 (JSON-RPC 2.0) ============
  const MCP_TOOLS = [
    { name: 'xuanji_optimize', description: '对流程蓝图做双璇玑十四维治理，返回治理报告', inputSchema: { type: 'object', properties: { flow: { type: 'object' }, tenant: { type: 'string' } }, required: ['flow'] } },
    { name: 'graph_pagerank', description: '计算图谱 PageRank 中心性', inputSchema: { type: 'object', properties: {}, required: [] } },
    { name: 'graph_communities', description: '图谱社区发现', inputSchema: { type: 'object', properties: {}, required: [] } },
    { name: 'market_list', description: '列出算子商城', inputSchema: { type: 'object', properties: { tag: { type: 'string' } }, required: [] } },
    { name: 'operator_register', description: '注册一个新算子', inputSchema: { type: 'object', properties: { name: { type: 'string' }, type: { type: 'string' } }, required: ['name'] } }
  ]
  function mcpCall(name, args) {
    const a = args || {}
    if (name === 'xuanji_optimize') {
      const report = xuanji.runAlliance(a.flow || { nodes: [], edges: [] }, a.tenant || 'default')
      return { content: [{ type: 'text', text: JSON.stringify({ score: report.governance.score, gate: report.governance.gate }) }] }
    }
    if (name === 'graph_pagerank') {
      const pr = graph.pagerank(store.all('graph_nodes'), store.all('graph_edges'))
      return { content: [{ type: 'text', text: JSON.stringify(pr) }] }
    }
    if (name === 'graph_communities') {
      const c = graph.communities(store.all('graph_nodes'), store.all('graph_edges'))
      return { content: [{ type: 'text', text: JSON.stringify(c.map((x) => ({ id: x.id, nodes: x.nodes.length }))) }] }
    }
    if (name === 'market_list') {
      const items = store.all('market').filter((m) => !a.tag || (m.tags || []).includes(a.tag))
      return { content: [{ type: 'text', text: JSON.stringify(items.map((m) => ({ id: m.id, name: m.name }))) }] }
    }
    if (name === 'operator_register') {
      const op = store.insert('operators', { name: a.name, type: a.type || 'algorithm', category: 'mcp', status: 'active', tags: [] })
      return { content: [{ type: 'text', text: JSON.stringify(op) }] }
    }
    return { isError: true, content: [{ type: 'text', text: '未知工具: ' + name }] }
  }
  route('POST', '/api/mcp', (h) => {
    const b = h.body || {}
    const id = b.id != null ? b.id : 1
    if (b.method === 'tools/list') {
      return sendJSON(h.res, 200, { jsonrpc: '2.0', id, result: { tools: MCP_TOOLS } })
    }
    if (b.method === 'tools/call') {
      const name = b.params && b.params.name
      const args = (b.params && b.params.arguments) || {}
      if (!MCP_TOOLS.find((t) => t.name === name)) {
        return sendJSON(h.res, 200, { jsonrpc: '2.0', id, error: { code: -32601, message: '方法不存在: ' + name } })
      }
      try {
        const result = mcpCall(name, args)
        return sendJSON(h.res, 200, { jsonrpc: '2.0', id, result })
      } catch (e) {
        return sendJSON(h.res, 200, { jsonrpc: '2.0', id, error: { code: -32000, message: e.message } })
      }
    }
    return sendJSON(h.res, 200, { jsonrpc: '2.0', id, error: { code: -32601, message: '不支持的方法: ' + b.method } })
  })

  // ============ AI 自动化中枢 ============
  route('GET', '/api/automation', (h) => sendJSON(h.res, 200, { items: store.all('automation') }))
  route('POST', '/api/automation/chat', (h) => {
    const b = h.body || {}
    sendJSON(h.res, 200, {
      session: b.session || 'auto_' + Date.now().toString(36),
      reply: '已收到自动化需求。建议流程：归一化 → 双璇玑十四维治理 → 治理闸门 → 融合发布。可在「全维融合」页执行归一化，或在下方「运行」触发端到端闭环。',
      next: 'run'
    })
  })
  route('POST', '/api/automation/:id/refine', (h) => {
    const b = h.body || {}
    const a = store.get('automation', decodeURIComponent(h.params.id))
    if (!a) return sendError(h.res, 404, 'automation 不存在')
    const upd = store.update('automation', a.id, { requirement: b.requirement || a.requirement, status: 'refined' })
    sendJSON(h.res, 200, upd)
  })
  route('POST', '/api/automation/:id/run', (h) => {
    const a = store.get('automation', decodeURIComponent(h.params.id))
    if (!a) return sendError(h.res, 404, 'automation 不存在')
    const flow = (h.body && h.body.flow) || a.flow || { nodes: [], edges: [] }
    const report = xuanji.runAlliance(flow, (h.body && h.body.tenant) || 'default')
    const approved = report.governance.gate_detail.approved
    store.update('automation', a.id, { status: approved ? 'passed' : 'blocked', last_report: report.governance.gate })
    sendJSON(h.res, 200, {
      id: a.id,
      status: approved ? 'passed' : 'blocked',
      governance: report.governance,
      optimization: report.optimization,
      message: approved ? '端到端闭环通过治理闸门' : '被治理闸门拦截：' + report.governance.gate_detail.reason
    })
  })
  route('GET', '/api/automation/:id/permissions', (h) => {
    const a = store.get('automation', decodeURIComponent(h.params.id))
    if (!a) return sendError(h.res, 404, 'automation 不存在')
    sendJSON(h.res, 200, { id: a.id, permissions: a.permissions || { read: true, write: true, deploy: false } })
  })
  route('PUT', '/api/automation/:id', (h) => {
    const b = h.body || {}
    const a = store.get('automation', decodeURIComponent(h.params.id))
    if (!a) return sendError(h.res, 404, 'automation 不存在')
    sendJSON(h.res, 200, store.update('automation', a.id, b))
  })

  // ============ 璇玑全维治理 ============
  route('GET', '/api/xuanji/health', (h) =>
    sendJSON(h.res, 200, {
      status: 'ok',
      dimensions: 14,
      leagues: ['business', 'dev'],
      gates: 8,
      verify_gateway: '⛨璇玑',
      ts: new Date().toISOString()
    })
  )
  route('POST', '/api/xuanji/optimize', (h) => {
    const b = h.body || {}
    if (!b.flow || !b.flow.nodes) return sendError(h.res, 400, 'flow.nodes 必填')
    const report = xuanji.runAlliance(b.flow, b.tenant || 'default')
    sendJSON(h.res, 200, report)
  })
  route('POST', '/api/xuanji/publish', (h) => {
    const b = h.body || {}
    if (!b.flow || !b.flow.nodes) return sendError(h.res, 400, 'flow.nodes 必填')
    const r = xuanji.publish(b, store)
    sendJSON(h.res, r.published ? 200 : 409, r)
  })
}
