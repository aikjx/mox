'use strict'
/**
 * HTTP 集成测试：启动真实 createServer 实例（随机端口），经 fetch 全量验证 API 与静态托管。
 * 依赖 global fetch（Node 18+）。鉴权令牌 dev-secret-token。
 */
const { test, section, assert, assertEqual, assertIncludes, SUITE } = require('./_harness')
const { createServer } = require('../src/server')

const TOKEN = 'dev-secret-token'
const goodFlow = {
  nodes: [
    { id: 'n1', name: '采集需求', type: 'operator' },
    { id: 'n2', name: '归一化IR', type: 'operator' },
    { id: 'n3', name: 'AI合规审查', type: 'ai_task' },
    { id: 'n4', name: '条件分流', type: 'condition' },
    { id: 'n5', name: '运行监控', type: 'monitor' },
    { id: 'n6', name: '数据落库', type: 'data' },
    { id: 'n7', name: '归档', type: 'operator' }
  ],
  edges: [
    { from: 'n1', to: 'n2' },
    { from: 'n2', to: 'n3' },
    { from: 'n3', to: 'n4' },
    { from: 'n4', to: 'n5' },
    { from: 'n4', to: 'n6' },
    { from: 'n6', to: 'n7' },
    { from: 'n5', to: 'n7' }
  ]
}

module.exports = async function runIntegration() {
  const server = createServer()
  await new Promise((r) => server.listen(0, r))
  const port = server.address().port
  const base = `http://127.0.0.1:${port}`
  console.log('  [起服] http://127.0.0.1:' + port)

  async function call(method, path, body, authed = true) {
    const opts = { method, headers: {} }
    if (authed) opts.headers['Authorization'] = 'Bearer ' + TOKEN
    if (body !== undefined) {
      opts.headers['Content-Type'] = 'application/json'
      opts.body = JSON.stringify(body)
    }
    const res = await fetch(base + path, opts)
    let data = null
    try {
      data = await res.json()
    } catch (e) {
      data = null
    }
    return { status: res.status, data }
  }

  section('System: 健康与状态')
  await test('GET /api/health 公开 200', async () => {
    const r = await call('GET', '/api/health', undefined, false)
    assertEqual(r.status, 200)
    assertEqual(r.data.status, 'ok')
    assert.ok(r.data.modules.includes('xuanji'))
  })
  await test('GET /api/status/full 200 含图计数', async () => {
    const r = await call('GET', '/api/status/full', undefined, false)
    assertEqual(r.status, 200)
    assert.ok(typeof r.data.graph.nodes === 'number')
  })

  section('Graph: 全量算法端点')
  await test('GET /api/graph 返回节点数组', async () => {
    const r = await call('GET', '/api/graph')
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data.nodes) && r.data.nodes.length > 0)
  })
  await test('GET /api/graph/pagerank 返回对象', async () => {
    const r = await call('GET', '/api/graph/pagerank')
    assertEqual(r.status, 200)
    assert.ok(typeof r.data === 'object')
  })
  await test('GET /api/graph/communities 返回数组', async () => {
    const r = await call('GET', '/api/graph/communities')
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data))
  })
  await test('GET /api/graph/stats 含 density', async () => {
    const r = await call('GET', '/api/graph/stats')
    assertEqual(r.status, 200)
    assert.ok('density' in r.data)
  })
  await test('GET /api/graph/centrality 含两种中心性', async () => {
    const r = await call('GET', '/api/graph/centrality')
    assertEqual(r.status, 200)
    assert.ok(r.data.degree_centrality && r.data.betweenness_centrality)
  })
  await test('GET /api/graph/neighbors/:id', async () => {
    const r = await call('GET', '/api/graph/neighbors/d04')
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data))
  })
  await test('GET /api/graph/path?source=d04&target=ea', async () => {
    const r = await call('GET', '/api/graph/path?source=d04&target=ea')
    assertEqual(r.status, 200)
    assert.ok('found' in r.data)
  })
  await test('POST /api/graph/activate 返回激活图', async () => {
    const r = await call('POST', '/api/graph/activate', { start_nodes: ['d04'], iterations: 8 })
    assertEqual(r.status, 200)
    assert.ok(typeof r.data === 'object')
  })
  await test('POST /api/graph/recommend', async () => {
    const r = await call('POST', '/api/graph/recommend', { context_nodes: ['d04', 'ea'] })
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data))
  })
  await test('POST /api/graph/node + edge 落库', async () => {
    const n = await call('POST', '/api/graph/node', { id: 'unit_node', label: '单测节点', type: 'operator' })
    assertEqual(n.status, 201)
    const e = await call('POST', '/api/graph/edge', { source: 'unit_node', target: 'd04' })
    assertEqual(e.status, 201)
  })
  await test('GET /api/graph/export 含 version', async () => {
    const r = await call('GET', '/api/graph/export')
    assertEqual(r.status, 200)
    assertEqual(r.data.version, '1.0')
  })

  section('Xuanji: 双璇玑治理')
  await test('POST /api/xuanji/optimize -> G3-通过', async () => {
    const r = await call('POST', '/api/xuanji/optimize', { flow: goodFlow })
    assertEqual(r.status, 200)
    assertEqual(r.data.governance.gate, 'G3-通过')
    assertEqual(r.data.fourteen_dimensions.length, 14)
    assert.ok(r.data.governance.gate_detail.approved)
  })
  await test('POST /api/xuanji/publish task_done -> published:true', async () => {
    const r = await call('POST', '/api/xuanji/publish', { flow: goodFlow, name: '集成测试算子', task_done: true })
    assertEqual(r.status, 200)
    assert.ok(r.data.published === true)
  })
  await test('POST /api/xuanji/publish 无 task_done -> 409 published:false', async () => {
    const r = await call('POST', '/api/xuanji/publish', { flow: goodFlow, name: '未完成任务' })
    assertEqual(r.status, 409)
    assert.ok(r.data.published === false)
  })
  await test('POST /api/analyze/spiral 返回 verification', async () => {
    const r = await call('POST', '/api/analyze/spiral', { flow: goodFlow })
    assertEqual(r.status, 200)
    assert.ok(r.data.verification && r.data.verification.checks)
  })
  await test('GET /api/xuanji/health 200', async () => {
    const r = await call('GET', '/api/xuanji/health')
    assertEqual(r.status, 200)
    assertEqual(r.data.dimensions, 14)
  })

  section('MCP: JSON-RPC 2.0 兼容层')
  await test('tools/list 返回 5 个工具', async () => {
    const r = await call('POST', '/api/mcp', { jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
    assertEqual(r.status, 200)
    assertEqual(r.data.jsonrpc, '2.0')
    assertEqual(r.data.result.tools.length, 5)
  })
  await test('tools/call graph_pagerank', async () => {
    const r = await call('POST', '/api/mcp', {
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/call',
      params: { name: 'graph_pagerank', arguments: {} }
    })
    assertEqual(r.status, 200)
    assert.ok(r.data.result && r.data.result.content)
  })
  await test('未知方法返回 error', async () => {
    const r = await call('POST', '/api/mcp', { jsonrpc: '2.0', id: 3, method: 'bogus', params: {} })
    assertEqual(r.status, 200)
    assert.ok(r.data.error)
  })

  section('Market / Operators / Automation')
  await test('GET /api/market 返回列表', async () => {
    const r = await call('GET', '/api/market')
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data.items))
  })
  await test('POST /api/market/upload 201', async () => {
    const r = await call('POST', '/api/market/upload', { name: '新算子X', tags: ['test'], graph: { nodes: [{ id: 'a', type: 'operator' }], edges: [] } })
    assertEqual(r.status, 201)
    assert.ok(r.data.id)
  })
  await test('POST /api/operators/register 201', async () => {
    const r = await call('POST', '/api/operators/register', { name: '算子Z', type: 'algorithm' })
    assertEqual(r.status, 201)
    assertEqual(r.data.name, '算子Z')
  })
  await test('POST /api/operators/register 缺 name -> 400', async () => {
    const r = await call('POST', '/api/operators/register', {})
    assertEqual(r.status, 400)
  })
  await test('POST /api/automation/auto_1/run 200', async () => {
    const r = await call('POST', '/api/automation/auto_1/run', {})
    assertEqual(r.status, 200)
    assert.ok(r.data.status === 'passed' || r.data.status === 'blocked')
  })

  section('AI: 对话 / 流程 / Caomei / 插件')
  await test('POST /api/ai/chat 返回 session+reply', async () => {
    const r = await call('POST', '/api/ai/chat', { message: '璇玑治理怎么用' })
    assertEqual(r.status, 200)
    assert.ok(r.data.session && r.data.reply)
  })
  await test('POST /api/ai/flows/validate 校验合法流程', async () => {
    const r = await call('POST', '/api/ai/flows/validate', { nodes: [{ id: 'a', type: 'operator' }, { id: 'b', type: 'monitor' }], edges: [{ from: 'a', to: 'b' }] })
    assertEqual(r.status, 200)
    assert.ok(typeof r.data.valid === 'boolean')
  })
  await test('POST /api/caomei/compile 自然语言生成蓝图', async () => {
    const r = await call('POST', '/api/caomei/compile', { text: '生成合规审查流程并归档' })
    assertEqual(r.status, 200)
    assert.ok(r.data.nodes && r.data.nodes.length > 0)
  })
  await test('GET /api/ai/plugins 返回插件', async () => {
    const r = await call('GET', '/api/ai/plugins')
    assertEqual(r.status, 200)
    assert.ok(Array.isArray(r.data.items))
  })
  await test('GET /api/ai/algorithm-types 200', async () => {
    const r = await call('GET', '/api/ai/algorithm-types')
    assertEqual(r.status, 200)
    assert.ok(r.data.items.length > 0)
  })

  section('Security & 404')
  await test('受保护接口无令牌 -> 401', async () => {
    const r = await call('GET', '/api/graph', undefined, false)
    assertEqual(r.status, 401)
  })
  await test('未知 API -> 404', async () => {
    const r = await call('GET', '/api/does/not/exist')
    assertEqual(r.status, 404)
  })

  section('Static: 前端 SPA 托管')
  await test('GET / 返回 index.html', async () => {
    const res = await fetch(base + '/', { headers: { Authorization: 'Bearer ' + TOKEN } })
    const text = await res.text()
    assertEqual(res.status, 200)
    assert.ok(text.includes('<div id="app"'), '应含 Vue 挂载点')
  })
  await test('GET /api (裸前缀) 不误判为静态', async () => {
    const r = await call('GET', '/api')
    assertEqual(r.status, 404)
  })

  await new Promise((r) => server.close(r))
  console.log('  [闭服] 端口释放')
}
