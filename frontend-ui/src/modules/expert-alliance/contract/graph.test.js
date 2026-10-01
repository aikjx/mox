// 协作图谱契约门禁：contract/graph.js、model/normalize.js、model/layout.js 三处前端单源
// 必须与网关 experts_graph.rs / experts_common.rs 的 handler 逐字对齐。
// 后端改路由、加字段、动钳位而没同步前端时，本文件先红。
import { describe, it, expect } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'

import {
  COLLABORATOR_LIMITS, COLLABORATOR_LIMIT_DEFAULT, DOMAIN_NODE_ID_PREFIX,
  GRAPH_EDGE_TYPES, GRAPH_NODE_TYPE,
  OPTIMAL_TEAM_BACKEND_RULES, OPTIMAL_TEAM_DEFAULTS, OPTIMAL_TEAM_FIELDS, OPTIMAL_TEAM_ROLES,
  collaboratorQuery, coverageText, edgeTypeMeta, graphNodeLabel, graphNodeShortId,
  isDomainNode, isLostGraphLabel, nodeTypeMeta,
  optimalTeamBody, optimalTeamProblem, optimalTeamRoleLabel
} from './graph.js'
import { ENDPOINTS } from './endpoints.js'
import {
  normCollaborators, normCommunities, normGraph, normGraphPath, normGraphRebuild, normGraphStats,
  normNeighbors, normOptimalTeam
} from '../model/normalize.js'
import { GRAPH_VIEWPORT, graphDegrees, graphLayout, nodeRadius, pathChainText } from '../model/layout.js'

function findRepoRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    if (existsSync(path.join(dir, 'docs/API-REGISTRY.md')) && existsSync(path.join(dir, 'platform'))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const ROOT = findRepoRoot(process.cwd())
const src = (rel) => readFileSync(path.join(ROOT, rel), 'utf8').replace(/\r\n/g, '\n')

const GRAPH_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_graph.rs')
const COMMON_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs')
const ACTUATOR_RS = src('platform/gateway/mox-platform-gateway-svc/src/actuator.rs')
const REGISTRY_MD = src('docs/API-REGISTRY.md')
const NORMALIZE_JS = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')

/** 从 openIdx 的 { 起取平衡块（跳过字符串字面量里的括号） */
function balanced(text, openIdx) {
  let depth = 0
  let inStr = false
  let esc = false
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i]
    if (esc) { esc = false; continue }
    if (inStr) {
      if (c === '\\') esc = true
      else if (c === '"') inStr = false
      continue
    }
    if (c === '"') { inStr = true; continue }
    if (c === '{' || c === '[' || c === '(') depth++
    else if (c === '}' || c === ']' || c === ')') {
      depth--
      if (depth === 0) return text.slice(openIdx, i + 1)
    }
  }
  throw new Error('块未闭合，解析器该修了')
}

/** 取某个 Rust 函数的整体（含签名后的花括号体） */
function rustFn(header, text = GRAPH_RS) {
  const at = text.indexOf(header)
  expect(at, `Rust 源里找不到 ${header}`).toBeGreaterThan(-1)
  return balanced(text, text.indexOf('{', at))
}

const jsonStarts = (body) => {
  const starts = []
  let at = body.indexOf('json!({')
  while (at >= 0) {
    starts.push(body.indexOf('{', at))
    at = body.indexOf('json!({', at + 7)
  }
  expect(starts.length, '该 handler 里没有 json! 字面量').toBeGreaterThan(0)
  return starts
}

/** 收集函数里每个 json! 对象的顶层键（并集）：内联对象的键同样要求前端读到 */
function jsonFaces(body) {
  const top = new Set()
  for (const open of jsonStarts(body)) {
    const obj = balanced(body, open)
    let depth = 0
    let inStr = false
    let esc = false
    let cursor = 0
    let pending = null
    for (let i = 0; i < obj.length; i++) {
      const c = obj[i]
      if (esc) { esc = false; continue }
      if (inStr) {
        if (c === '\\') esc = true
        else if (c === '"') { inStr = false; pending = obj.slice(cursor, i) }
        continue
      }
      if (c === '"') { inStr = true; cursor = i + 1; continue }
      if (c === '{' || c === '[' || c === '(') depth++
      else if (c === '}' || c === ']' || c === ')') depth--
      else if (c === ':' && depth === 1 && pending !== null) {
        top.add(pending)
        pending = null
      }
    }
  }
  expect(top.size, '一个顶层键都没解析出来').toBeGreaterThan(0)
  return top
}

/** 图谱由注册表派生，节点/边属性（含 shared_domains）都在这个 builder 里写入 */
const BUILDER = rustFn('pub fn build_graph_from_registry', COMMON_RS)

/** 前端归一化函数体 */
function normFn(name) {
  const at = NORMALIZE_JS.indexOf(`export function ${name}(`)
  expect(at, `normalize.js 里找不到 ${name}`).toBeGreaterThan(-1)
  return balanced(NORMALIZE_JS, NORMALIZE_JS.indexOf('{', at))
}

const readsKey = (fnSrc, key) => new RegExp(`\\.${key}(?:\\?\\.|\\b)`).test(fnSrc)
// JS 方法名一律 camelCase，所以带下划线的读取必定是想读某个 wire 键
const snakeReads = (fnSrc) =>
  [...new Set([...fnSrc.matchAll(/\.([a-z][a-z0-9]*(?:_[a-z0-9]+)+)\b/g)].map((m) => m[1]))]

/** 双向对齐：后端给的键前端必须都读，前端读的 wire 键后端必须真给 */
function assertFace(handlerHeader, normName) {
  const body = rustFn(handlerHeader)
  const face = jsonFaces(body)
  const fnSrc = normFn(normName)
  for (const key of face) {
    expect(readsKey(fnSrc, key), `${normName} 没读后端返回的 ${key}（能力被吞掉）`).toBe(true)
  }
  // properties 里的键由 build_graph_from_registry 写入 handler，故两者都算"后端真给"
  for (const key of snakeReads(fnSrc)) {
    expect(`${body}\n${BUILDER}`, `${normName} 读了 ${key}，handler 与图谱构建都不产出它`).toContain(`"${key}"`)
  }
  return face
}

/** 结构体字段：逐行解析，属性行归到紧随其后的字段上（跨字段正则会张冠李戴） */
function structFields(structBody) {
  const out = []
  let pending = []
  for (const line of structBody.split('\n')) {
    const attr = line.match(/^\s*(#\[.+\])\s*$/)
    if (attr) { pending.push(attr[1]); continue }
    const field = line.match(/^\s*(\w+)\s*:\s*(.+?),\s*$/)
    if (field) {
      out.push({ name: field[1], type: field[2].trim(), attrs: pending })
      pending = []
    }
  }
  return out
}

const graphEndpoints = () => Object.entries(ENDPOINTS).filter(([, ep]) => ep.path.startsWith('/api/expert-graph'))

describe('协作图谱端点面', () => {
  it('契约里的图谱端点与 build_experts_graph_router 注册的路由等集，方法逐字一致', () => {
    const router = rustFn('pub fn build_experts_graph_router')
    // 每条 .route("PATH", CHAIN) 独占一行；CHAIN 形如 get(h) / post(h) / put(h).delete(h)
    const routed = new Map()
    for (const line of router.split('\n')) {
      const m = line.match(/\.route\("([^"]+)",\s*(.+)\)\s*$/)
      if (!m) continue
      const path = m[1]
      const methods = [...m[2].matchAll(/\b(get|post|put|delete|patch)\((\w+)\)/g)]
        .map((mm) => ({ method: mm[1].toUpperCase(), handler: mm[2] }))
      routed.set(path, methods)
    }
    const contracted = graphEndpoints()
    // 契约里每条 (method, path) 都能在 router 里找到，反之亦然
    const contractedKeys = contracted.map(([, ep]) => `${ep.method} ${ep.path}`).sort()
    const routedKeys = [...routed.entries()]
      .flatMap(([p, ms]) => ms.map((x) => `${x.method} ${p}`))
      .sort()
    expect(contractedKeys).toEqual(routedKeys)
    for (const [name, ep] of contracted) {
      expect(ep.nesting, `${name} 信封不是 flat`).toBe('flat')
    }
  })

  it('registry 行与 actuator 登记、API-REGISTRY.md 三处同源', () => {
    for (const [name, ep] of graphEndpoints()) {
      const escaped = ep.path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      expect(ACTUATOR_RS, `actuator 未登记 ${name}`).toMatch(new RegExp(`r\\("${ep.registry}", "${ep.method}", "${escaped}"`))
      expect(REGISTRY_MD, `API-REGISTRY.md 缺 ${ep.registry}`).toContain(`\`${ep.registry}\``)
      expect(REGISTRY_MD).toContain(ep.path)
    }
  })

  it('/api/expert-graph/overview 这个不存在的旧路径不得复活', () => {
    for (const [, ep] of graphEndpoints()) expect(ep.path).not.toBe('/api/expert-graph/overview')
  })
})

describe('图的真实构成', () => {
  const builder = BUILDER

  it('节点类型只有契约里那两种，多一种少一种都是漂移', () => {
    const emitted = [...new Set([...builder.matchAll(/node_type:\s*"(\w+)"/g)].map((m) => m[1]))]
    expect(emitted.sort()).toEqual(Object.values(GRAPH_NODE_TYPE).sort())
    const getGraph = rustFn('async fn get_graph(')
    expect(getGraph).toMatch(/node_type == "expert"/)
    expect(getGraph).toMatch(/node_type == "domain"/)
    for (const t of Object.values(GRAPH_NODE_TYPE)) {
      expect(nodeTypeMeta(t).label).not.toBe(t)
      expect(nodeTypeMeta('nope').label).toBe('nope')
    }
  })

  it('边类型只有契约里那两种，建边阈值 0.1 必须写进提示', () => {
    const emitted = [...new Set([...builder.matchAll(/edge_type:\s*"(\w+)"/g)].map((m) => m[1]))]
    expect(emitted.sort()).toEqual(GRAPH_EDGE_TYPES.map((e) => e.value).sort())
    expect(builder).toMatch(/if similarity > 0\.1/)
    for (const e of GRAPH_EDGE_TYPES) {
      expect(e.label.length, `${e.value} 缺中文标签`).toBeGreaterThan(1)
      expect(e.weightHint.length, `${e.value} 缺权重说明`).toBeGreaterThan(3)
      expect(builder).toContain(`"${e.value}"`)
    }
    expect(edgeTypeMeta('collaborates_with').weightHint).toContain('0.1')
    expect(edgeTypeMeta('has_domain').weightHint).toContain('1.0')
    expect(builder).toMatch(/weight: 1\.0/)
  })

  it('领域节点 id 前缀与 builder 的 format! 一致', () => {
    expect(builder).toContain(`format!("${DOMAIN_NODE_ID_PREFIX}{}`)
  })

  it('节点属性只有 builder 塞的那几项，界面不得声称图里带技能清单', () => {
    const keys = [...new Set([...builder.matchAll(/p\.insert\("(\w+)"\.into\(\)/g)].map((m) => m[1]))]
    expect(keys.sort()).toEqual(['avg_rating', 'domains', 'shared_domains', 'similarity', 'status', 'title'])
    const nodeSrc = normFn('normGraphNode')
    for (const k of ['title', 'domains', 'avg_rating', 'status']) {
      expect(readsKey(nodeSrc, k), `normGraphNode 丢了节点属性 ${k}`).toBe(true)
    }
    expect(nodeSrc).not.toMatch(/\.skills/)
    expect(normGraph({ nodes: [{ id: 'e1', label: '甲', node_type: 'expert', properties: { avg_rating: 4.5 } }] })
      .nodes[0].avgRating).toBe(4.5)
    expect(normGraph({ nodes: [{ id: 'e1', node_type: 'expert' }] }).nodes[0].avgRating).toBe(null)
  })
})

describe('最优团队请求面', () => {
  const BODY_STRUCT = rustFn('struct OptimalTeamBody')
  const HANDLER = rustFn('async fn post_optimal_team(')
  const FINDER = rustFn('pub fn find_optimal_team(')
  const rustFields = structFields(BODY_STRUCT)

  it('契约字段表与 OptimalTeamBody 等集', () => {
    expect(rustFields.length).toBe(6)
    expect(OPTIMAL_TEAM_FIELDS.map((f) => f.key).sort()).toEqual(rustFields.map((f) => f.name).sort())
    for (const f of OPTIMAL_TEAM_FIELDS) {
      expect(f.label.length, `${f.key} 缺标签`).toBeGreaterThan(1)
      expect(['tags', 'number', 'text', 'unknown']).toContain(f.kind)
    }
  })

  it('mounted 与否等于 handler 是否真的读该字段', () => {
    for (const f of OPTIMAL_TEAM_FIELDS) {
      const read = new RegExp(`body\\.${f.key}\\b`).test(HANDLER)
      expect(f.mounted, `${f.key} 后端${read ? '会' : '不会'}读，前端标记错了`).toBe(read)
    }
    // constraints 是最容易踩的一个：结构体收了，post_optimal_team 从不读，所以界面里没有它
    expect(HANDLER).not.toMatch(/body\.constraints/)
    expect(OPTIMAL_TEAM_FIELDS.find((f) => f.key === 'constraints').mounted).toBe(false)
  })

  it('body 全字段可选：任何组合都不会 422，故空 body 是合法请求', () => {
    for (const f of rustFields) {
      const optional = /^(Vec<|Option<)/.test(f.type) || f.attrs.some((a) => /serde\(default\)/.test(a))
      expect(optional, `${f.name}: ${f.type} 若必填，缺省请求会 422，界面文案要改`).toBe(true)
    }
  })

  it('两个缺省值取 handler 的 unwrap_or 字面量，0 兜底成 5 这条要说出来', () => {
    expect(Number(HANDLER.match(/max_members\.unwrap_or\((\d+)\)/)?.[1])).toBe(OPTIMAL_TEAM_DEFAULTS.maxMembers)
    expect(Number(HANDLER.match(/min_rating\.unwrap_or\(([0-9.]+)\)/)?.[1])).toBe(OPTIMAL_TEAM_DEFAULTS.minRating)
    expect(FINDER).toMatch(/if max_members == 0 \{ 5 \}/)
    expect(OPTIMAL_TEAM_BACKEND_RULES.maxMembers).toContain('无上限')
    expect(OPTIMAL_TEAM_BACKEND_RULES.availability).toContain('0.6')
    // 后端不校验区间，界面必须显式声明而不是假装受控
    expect(HANDLER).not.toMatch(/min_rating\s*[<>]/)
    expect(OPTIMAL_TEAM_BACKEND_RULES.minRating).toContain('不校验')
    for (const f of OPTIMAL_TEAM_FIELDS) {
      if (f.kind !== 'number') continue
      expect(f.min, `${f.key} 缺下界`).toBeDefined()
      expect(f.max, `${f.key} 缺上界`).toBeGreaterThan(f.min)
    }
  })

  it('goal 只在两类需求都为空时才被读，同时填时前端不发 goal', () => {
    expect(HANDLER).toMatch(/body\.required_skills\.is_empty\(\)\s*&&\s*body\.required_domains\.is_empty\(\)/)
    const body = optimalTeamBody({ requiredSkills: ['rust'], goal: '造一个 rust 团队' })
    expect(body).not.toHaveProperty('goal')
    expect(body.required_skills).toEqual(['rust'])
    expect(optimalTeamBody({ goal: '  造一个数据团队  ' }).goal).toBe('造一个数据团队')
  })

  it('等于后端缺省值的输入不进请求体，改动过的才发', () => {
    expect(optimalTeamBody({ requiredDomains: ['data'], maxMembers: 5, minRating: 4 })).toEqual({ required_domains: ['data'] })
    expect(optimalTeamBody({ requiredDomains: ['data'], maxMembers: 8, minRating: 4.5 })).toEqual({
      required_domains: ['data'], max_members: 8, min_rating: 4.5
    })
    expect(optimalTeamBody({ requiredDomains: ['data'], maxMembers: 0 })).not.toHaveProperty('max_members')
    expect(optimalTeamBody({ requiredDomains: ['data'] })).not.toHaveProperty('constraints')
    expect(optimalTeamBody({ requiredDomains: ['data', ''], maxMembers: 7.4 })).toEqual({
      required_domains: ['data'], max_members: 7
    })
  })

  it('提交前的三条拦截：越界后端不管，所以前端必须管', () => {
    expect(optimalTeamProblem({})).toContain('至少')
    expect(optimalTeamProblem({ maxMembers: 0, requiredDomains: ['data'] })).toContain('至少为 1')
    expect(optimalTeamProblem({ maxMembers: 21, requiredDomains: ['data'] })).toContain('不超过')
    expect(optimalTeamProblem({ minRating: 6, requiredDomains: ['data'] })).toContain('0–5')
    expect(optimalTeamProblem({ requiredDomains: ['data'], maxMembers: 5, minRating: 4 })).toBe('')
  })

  it('成员 role 标签与 handler 三个分支等集，未知值原样透传', () => {
    const roleBlock = FINDER.slice(FINDER.indexOf('let role = '), FINDER.indexOf('team_details.push'))
    const roles = [...new Set([...roleBlock.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]))]
    expect(roles.length).toBe(3)
    expect(Object.keys(OPTIMAL_TEAM_ROLES).sort()).toEqual(roles.sort())
    for (const r of roles) expect(optimalTeamRoleLabel(r)).not.toBe(r)
    expect(optimalTeamRoleLabel('future_role')).toBe('future_role')
    expect(optimalTeamRoleLabel('')).toBe('未标注')
  })

  it('覆盖率文案如实带上未覆盖项，需求为空时不谎称 100%', () => {
    const team = normOptimalTeam({
      coverage: { required_total: 3, covered_count: 2, coverage_ratio: 0.6667, missing_domains: ['legal'] }
    })
    expect(coverageText(team.coverage)).toContain('2/3')
    expect(coverageText(team.coverage)).toContain('legal')
    expect(coverageText({ requiredTotal: 0 })).toContain('未给出需求项')
  })

  it('六个出参 handler 的顶层键全被各自的归一化读到', () => {
    const pairs = [
      ['pub fn compute_graph_stats(', 'normGraphStats'],
      ['async fn get_neighbors(', 'normNeighbors'],
      ['async fn get_collaborators(', 'normCollaborators'],
      ['async fn get_path(', 'normGraphPath'],
      ['async fn get_communities(', 'normCommunities'],
      ['pub fn find_optimal_team(', 'normOptimalTeam']
    ]
    for (const [header, normName] of pairs) expect(assertFace(header, normName).size).toBeGreaterThan(2)
  })
})

describe('图谱其余出参', () => {
  it('GET /api/expert-graph 顶层面精确，version 是数值而不是字符串', () => {
    const top = assertFace('async fn get_graph(', 'normGraph')
    expect([...top].sort()).toEqual(['built_at', 'edges', 'nodes', 'stats', 'version'])
    const statsKeys = [...new Set([...rustFn('async fn get_graph(').matchAll(/"(\w+)":/g)].map((m) => m[1]))]
    const fnSrc = normFn('normGraph')
    for (const key of statsKeys.filter((k) => !top.has(k))) {
      expect(readsKey(fnSrc, key), `normGraph 没读 stats.${key}`).toBe(true)
    }
    const graph = normGraph({ version: 7, stats: { node_count: '3' } })
    expect(graph.version).toBe(7)
    expect(graph.stats.nodeCount).toBe(3)
    expect(normGraph(null).nodes).toEqual([])
    expect(normGraph(null).version).toBe(0)
  })

  it('统计与社区在缺字段时归零而不是抛错', () => {
    const s = normGraphStats({ top_centrality_experts: [{ id: 'e1', name: '甲', degree: 3, degree_centrality: 0.5, betweenness: 2 }] })
    expect(s.avgClusteringCoefficient).toBe(0)
    expect(s.topCentralityExperts[0].degreeCentrality).toBe(0.5)
    expect(normGraphStats(null).totalNodes).toBe(0)
    const c = normCommunities({
      communities: [{ community_id: 'community-1', member_labels: ['甲'], internal_edges: 2, external_edges: 1, modularity_contribution: 0.1 }],
      modularity: 0.3, algorithm: 'label_propagation', iterations: 4, converged: true
    })
    expect(c.communities[0].memberLabels).toEqual(['甲'])
    expect(c.converged).toBe(true)
    expect(normCommunities({ converged: 'false' }).converged).toBe(false)
    expect(normNeighbors(null).neighbors).toEqual([])
    expect(normNeighbors({ neighbors: [{ id: 'd', direction: 'in', properties: { shared_domains: ['ai'] } }] })
      .neighbors[0].sharedDomains).toEqual(['ai'])
  })

  it('协作者 limit 缺省 10，非正整数不发；截断事实由 total 暴露', () => {
    const handler = rustFn('async fn get_collaborators(')
    expect(handler.match(/unwrap_or\((\d+)\)/)?.[1]).toBe(String(COLLABORATOR_LIMIT_DEFAULT))
    for (const n of COLLABORATOR_LIMITS) {
      expect(Number.isInteger(n), `${n} 必须是整数`).toBe(true)
      expect(n).toBeGreaterThan(0)
    }
    expect(COLLABORATOR_LIMITS).toContain(COLLABORATOR_LIMIT_DEFAULT)
    expect(collaboratorQuery(10)).toBeUndefined()
    expect(collaboratorQuery(0)).toBeUndefined()
    expect(collaboratorQuery('abc')).toBeUndefined()
    expect(collaboratorQuery(20)).toEqual({ limit: 20 })
    const col = normCollaborators({ collaborators: [{ collaboration_rank: 1 }], total_collaborators: 42 })
    expect(col.collaborators[0].rank).toBe(1)
    expect(col.collaborators.length).toBeLessThan(col.totalCollaborators)
  })

  it('路径不可达仍是 200 + found:false，归一化不得把缺省当命中', () => {
    const handler = rustFn('async fn get_path(')
    expect(handler).toMatch(/"found": true/)
    expect(handler).toMatch(/"found": false/)
    expect(normGraphPath({ found: false, path: [] }).found).toBe(false)
    expect(normGraphPath({}).found).toBe(false)
    expect(normGraphPath({ path: [{ node_id: 'a', label: '甲' }], found: true }).path[0].nodeId).toBe('a')
  })

  it('重建是写操作：版本号自增与耗时都要呈现', () => {
    const handler = rustFn('async fn post_rebuild(')
    expect(handler).toMatch(/version: previous_version \+ 1/)
    expect(jsonFaces(handler)).toContain('duration_ms')
    const r = normGraphRebuild({ previous_version: 3, new_version: 4, duration_ms: 12, rebuilt: true })
    expect([r.previousVersion, r.newVersion, r.rebuilt]).toEqual([3, 4, true])
  })
})

describe('布局确定性', () => {
  const nodes = [
    { id: 'domain-ai', label: 'ai', nodeType: 'domain' },
    { id: 'domain-data', label: 'data', nodeType: 'domain' },
    { id: 'e1', label: '甲', nodeType: 'expert' },
    { id: 'e2', label: '乙', nodeType: 'expert' },
    { id: 'e3', label: '丙', nodeType: 'expert' }
  ]
  const edges = [
    { source: 'e1', target: 'domain-ai', edgeType: 'has_domain', weight: 1 },
    { source: 'e2', target: 'domain-data', edgeType: 'has_domain', weight: 1 },
    { source: 'e1', target: 'e2', edgeType: 'collaborates_with', weight: 0.5 }
  ]

  it('同一份图两次布局坐标逐点相同，且没有任何 NaN', () => {
    const a = graphLayout(nodes, edges)
    const b = graphLayout(structuredClone(nodes), structuredClone(edges))
    expect(a.positions).toEqual(b.positions)
    for (const n of a.nodes) {
      expect(Number.isFinite(n.x) && Number.isFinite(n.y), `${n.id} 坐标非数`).toBe(true)
      expect(Number.isFinite(n.radius) && n.radius > 0).toBe(true)
    }
    expect(a.edges).toBe(edges)
  })

  it('每个节点都拿到坐标，孤点落外环', () => {
    const layout = graphLayout(nodes, edges)
    expect(layout.nodes.map((n) => n.id).sort()).toEqual(nodes.map((n) => n.id).sort())
    const orphan = layout.nodes.find((n) => n.id === 'e3')
    const dist = Math.hypot(orphan.x - GRAPH_VIEWPORT.width / 2, orphan.y - GRAPH_VIEWPORT.height / 2)
    expect(Math.round(dist)).toBe(Math.round(layout.ringRadius[1]))
    const clustered = layout.nodes.find((n) => n.id === 'e1')
    expect(Math.round(Math.hypot(clustered.x - layout.positions['domain-ai'].x, clustered.y - layout.positions['domain-ai'].y)))
      .toBeLessThanOrEqual(Math.round(layout.ringRadius[0] * 0.62))
  })

  it('度数与半径单调，viewBox 与画布同源', () => {
    expect(graphDegrees(edges).e1).toBe(2)
    expect(graphDegrees([])).toEqual({})
    expect(nodeRadius(0)).toBeLessThan(nodeRadius(3))
    expect(nodeRadius(999)).toBe(nodeRadius(18))
    expect(GRAPH_VIEWPORT.width).toBeGreaterThan(GRAPH_VIEWPORT.height)
  })

  it('后端塞进没有 id 的条目时不画它，否则 SVG 圆心是 undefined', () => {
    expect(graphLayout([], []).nodes).toEqual([])
    const junk = graphLayout([{ nodeType: 'expert' }, nodes[0], nodes[2]], edges)
    expect(junk.nodes.map((n) => n.id)).toEqual(['domain-ai', 'e1'])
    for (const n of junk.nodes) expect(Number.isFinite(n.x) && Number.isFinite(n.y)).toBe(true)
  })

  it('不可达路径给出明确文案而不是空串', () => {
    expect(pathChainText([], false)).toBe('两节点间不存在连通路径')
    expect(pathChainText([{ label: '甲' }, { nodeId: 'e2' }], true)).toBe('甲 → e2')
  })

  it('链路里丢码的名字显形，但"压根没给名字"仍照裸 id 走', () => {
    // 两种缺名不是一回事：label 有值但全是问号 = 写入侧丢了编码，要显形；
    // label 缺失只是调用方只拿到 id，不该被改标成"未命名节点"。
    expect(pathChainText([
      { nodeId: 'e1', label: '甲' },
      { nodeId: 'exp-af875a6f60d943e6964fcef4db0ab73f', label: '???????' }
    ], true)).toBe('甲 → 未命名节点 af875a6f')
    expect(pathChainText([{ nodeId: 'e1' }, { nodeId: 'e2', label: '  ' }], true))
      .toBe('e1 → 未命名节点 e2')
  })
})

// 真机探针（2026-09-27）：GET /api/expert-graph 的 45 个节点里，两个专家的 label 原样就是
// '???????' 与 '?????????'，而同一网关用 UTF-8 请求体写入的中文名可以字节级读回，
// 其余 13 个专家的中文名也正常。所以问号是**写入侧**丢的字符，不是字体渲染问题。
// 界面既不能把一串问号当真名印出来，也不能因此把两个节点混成同一个"未命名"。
describe('编码丢失的节点标签显形，不当真名用', () => {
  const LOST_EXPERTS = [
    { id: 'exp-af875a6f60d943e6964fcef4db0ab73f', label: '?????????' },
    { id: 'exp-cbf168cdeded4127bb812dca4d2b2936', label: '???????' }
  ]

  it('正对照：好名字原样用，绝不套上"未命名"外壳', () => {
    expect(graphNodeLabel({ id: 'exp-01', label: '张三' })).toBe('张三')
    expect(isLostGraphLabel('张三')).toBe(false)
    // 中文名里带问号是语义（疑问句），不是编码丢失，不许误判
    expect(isLostGraphLabel('这个能行？')).toBe(false)
    expect(graphNodeLabel({ id: 'domain-ai', label: 'ai' })).toBe('ai')
  })

  it('实测的丢码节点被改标为 id 短码，一串问号不外泄', () => {
    for (const n of LOST_EXPERTS) {
      expect(isLostGraphLabel(n.label), `${n.id} 应判为编码丢失`).toBe(true)
      expect(graphNodeLabel(n)).toBe(`未命名节点 ${graphNodeShortId(n.id)}`)
      expect(graphNodeLabel(n)).not.toContain('?')
    }
  })

  it('两个丢码节点仍可互相区分，短码截断会撞车所以要把边界说出来', () => {
    const [a, b] = LOST_EXPERTS.map(graphNodeLabel)
    expect(a).not.toBe(b)
    // 短码取前 8 位：同一前缀的两个 id 必然撞车，这是**已知代价**而不是缺陷——
    // 撞车时图上有两个同名节点，但信息卡与提问仍按 id 寻址，不会串到别人身上。
    const samePrefix = { id: 'exp-af875a6f00000000000000000000ffff', label: '?????????' }
    expect(graphNodeShortId('exp-af875a6f60d943e6964fcef4db0ab73f')).toBe('af875a6f')
    expect(graphNodeShortId(samePrefix.id)).toBe('af875a6f')
    expect(graphNodeShortId('domain-data-engineering')).toBe('data-eng')
    expect(graphNodeLabel(samePrefix)).toBe(graphNodeLabel(LOST_EXPERTS[0]))
    // 真实的两枚丢码节点不撞车
    expect(new Set(LOST_EXPERTS.map(graphNodeLabel)).size).toBe(2)
  })

  it('残缺输入不许印出 undefined 或空串', () => {
    for (const junk of [null, undefined, {}, { id: 'exp-' }, { id: 'exp-', label: '???' }, { label: '   ' }]) {
      const text = graphNodeLabel(junk)
      expect(typeof text).toBe('string')
      expect(text.length, JSON.stringify(junk)).toBeGreaterThan(0)
      expect(text).not.toContain('undefined')
    }
    expect(graphNodeLabel({ id: 'exp-x', label: '' })).toBe('未命名节点 x')
    expect(graphNodeShortId(null)).toBe('')
  })
})
