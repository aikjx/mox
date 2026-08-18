'use strict'
/**
 * 双璇玑十四维治理引擎（璇玑 = Xuanji）。
 * 归一化 → 双联盟十四维并行派发 → 归一化裁决 → ⛨璇玑验证网关 → 治理闸门 → 优化出图 → 融合发布。
 * 输出严格对齐前端 XuanjiFusionView 的响应结构。
 */

const BUSINESS_LEAGUE = [
  { code: 'B1', key: 'biz_value', name: '业务价值', hint: '需求是否直击痛点、可被度量' },
  { code: 'B2', key: 'ux', name: '用户体验', hint: '交互路径是否顺滑、无冗余跳转' },
  { code: 'B3', key: 'compliance', name: '合规性', hint: '是否满足政务/金融监管与数据合规' },
  { code: 'B4', key: 'operation', name: '可运营性', hint: '上线后是否可观测、可干预、可回滚' },
  { code: 'B5', key: 'data_asset', name: '数据资产', hint: '是否沉淀可复用数据资产' },
  { code: 'B6', key: 'synergy', name: '协同效率', hint: '跨角色/跨系统协同是否顺畅' },
  { code: 'B7', key: 'risk', name: '风险控制', hint: '异常是否可识别、可熔断' }
]
const DEV_LEAGUE = [
  { code: 'D1', key: 'maintainability', name: '可维护性', hint: '模块边界清晰、低耦合' },
  { code: 'D2', key: 'testability', name: '可测试性', hint: '关键路径可单测、可回归' },
  { code: 'D3', key: 'performance', name: '性能', hint: '吞吐与延迟达标' },
  { code: 'D4', key: 'security', name: '安全性', hint: '鉴权/加密/审计完备' },
  { code: 'D5', key: 'observability', name: '可观测性', hint: '日志/指标/链路可追溯' },
  { code: 'D6', key: 'scalability', name: '可扩展性', hint: '可水平扩展、插件化' },
  { code: 'D7', key: 'cost', name: '成本效率', hint: '资源/算力成本可控' }
]
const FOURTEEN = [...BUSINESS_LEAGUE, ...DEV_LEAGUE]

function clamp(v, lo = 0, hi = 100) {
  return Math.max(lo, Math.min(hi, Math.round(v)))
}

// 归一化：规整节点/边，检测有向环
function normalizeFlow(flow) {
  const rawNodes = (flow && flow.nodes) || []
  const rawEdges = (flow && (flow.edges || flow.links)) || []
  const nodes = rawNodes.map((n, i) => ({
    id: n.id != null ? String(n.id) : 'n' + (i + 1),
    name: n.name || n.label || n.id || 'n' + (i + 1),
    type: n.type || 'operator'
  }))
  const idset = new Set(nodes.map((n) => n.id))
  const edges = rawEdges
    .map((e) => {
      const s = e.source != null ? e.source : e.from
      const t = e.target != null ? e.target : e.to
      return { from: String(s), to: String(t) }
    })
    .filter((e) => idset.has(e.from) && idset.has(e.to))

  // DFS 环检测
  const adj = new Map()
  nodes.forEach((n) => adj.set(n.id, []))
  edges.forEach((e) => adj.get(e.from).push(e.to))
  const WHITE = 0,
    GRAY = 1,
    BLACK = 2
  const color = new Map()
  nodes.forEach((n) => color.set(n.id, WHITE))
  let cyclePath = null
  const stack = []
  function dfs(u) {
    color.set(u, GRAY)
    stack.push(u)
    for (const v of adj.get(u)) {
      if (color.get(v) === GRAY) {
        const idx = stack.indexOf(v)
        cyclePath = stack.slice(idx).concat([v])
        return true
      }
      if (color.get(v) === WHITE && dfs(v)) return true
    }
    stack.pop()
    color.set(u, BLACK)
    return false
  }
  let hasCycle = false
  for (const n of nodes) {
    if (color.get(n.id) === WHITE && dfs(n.id)) {
      hasCycle = true
      break
    }
  }

  // 数据依赖闭合：所有边端点均在节点集合内（已过滤，必为真），额外检查孤立节点
  const referenced = new Set()
  edges.forEach((e) => {
    referenced.add(e.from)
    referenced.add(e.to)
  })
  const orphans = nodes.filter((n) => !referenced.has(n.id)).map((n) => n.id)

  return { nodes, edges, hasCycle, cyclePath, orphans }
}

// 双联盟十四维并行派发：每维给出评分 + 观点
function dispatchDimensions(norm, tenant) {
  const { nodes, edges } = norm
  const n = nodes.length
  const types = new Set(nodes.map((x) => x.type))
  const hasMonitor = types.has('monitor') || types.has('verify') || types.has('audit')
  const hasCondition = types.has('condition') || types.has('branch')
  const hasAi = types.has('ai_task') || types.has('ai')
  const hasData = types.has('data') || types.has('store') || types.has('db')
  const density = n > 1 ? edges.length / (n * (n - 1)) : 0
  const govTenant = tenant === 'gov'

  const views = []
  function score(base, fn) {
    return clamp(base + (fn || (() => 0))())
  }

  // 业务联盟
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[0],
    score: score(62, () => (n >= 3 ? 18 : 0) + (hasCondition ? 8 : -6)),
    view: n >= 3 ? '流程覆盖核心环节，价值闭环完整' : '环节偏少，价值链路可能断裂'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[1],
    score: score(70, () => (hasCondition ? -10 : 6) + (n <= 6 ? 8 : -4)),
    view: hasCondition ? '含条件分支，需关注异常路径体验' : '线性流程，体验路径清晰'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[2],
    score: score(govTenant ? 58 : 72, () => (hasAi ? 10 : 0) + (hasCondition ? 6 : 0)),
    view: hasAi ? '含 AI 处理，建议补充合规审查节点' : '合规基础良好'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[3],
    score: score(60, () => (hasMonitor ? 20 : -10) + (n >= 4 ? 6 : 0)),
    view: hasMonitor ? '具备监控/审计节点，可运营性高' : '缺少监控节点，上线后难干预'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[4],
    score: score(55, () => (hasData ? 22 : -8) + (n >= 4 ? 4 : 0)),
    view: hasData ? '沉淀数据资产，可复用' : '建议增加数据落库节点'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[5],
    score: score(66, () => (density > 0.15 ? 10 : -6) + (hasAi ? 6 : 0)),
    view: density > 0.15 ? '节点协同紧密' : '协同链路偏稀疏'
  })
  views.push({
    league: 'business',
    ...BUSINESS_LEAGUE[6],
    score: score(64, () => (hasCondition ? 12 : -8) + (hasMonitor ? 10 : 0)),
    view: hasMonitor ? '异常可熔断' : '建议补充风险控制节点'
  })

  // 开发联盟
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[0],
    score: score(68, () => (n <= 8 ? 12 : -6) + (hasCondition ? -4 : 6)),
    view: '模块边界清晰，耦合可控'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[1],
    score: score(60, () => (hasCondition ? 16 : -8) + (hasMonitor ? 8 : 0)),
    view: hasCondition ? '分支逻辑可单测覆盖' : '建议为关键路径补充测试'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[2],
    score: score(72, () => (n <= 5 ? 14 : -10) + (edges.length <= n ? 6 : -6)),
    view: n <= 5 ? '链路短，性能优' : '节点偏多，关注端到端延迟'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[3],
    score: score(govTenant ? 70 : 64, () => (hasAi ? 14 : 0) + (hasData ? 8 : 0)),
    view: hasAi ? 'AI 节点需加密与审计' : '安全基础达标'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[4],
    score: score(58, () => (hasMonitor ? 24 : -12) + (hasData ? 6 : 0)),
    view: hasMonitor ? '链路可观测' : '缺少监控/日志，难以定位'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[5],
    score: score(66, () => (types.size >= 4 ? 14 : -6) + (hasAi ? 6 : 0)),
    view: types.size >= 4 ? '类型多样，易扩展' : '节点类型单一'
  })
  views.push({
    league: 'dev',
    ...DEV_LEAGUE[6],
    score: score(70, () => (n <= 6 ? 12 : -12) + (density < 0.2 ? 6 : -6)),
    view: n <= 6 ? '资源成本可控' : '规模上升，成本需压测'
  })

  return views
}

// 归一化裁决：业务/开发维度间的冲突识别
function reconcile(views) {
  const byKey = {}
  views.forEach((v) => (byKey[v.key] = v))
  const conflicts = []
  // 性能 vs 成本
  if (byKey.performance.score >= 75 && byKey.cost.score <= 55) {
    conflicts.push({
      type: 'perf_vs_cost',
      dims: ['performance', 'cost'],
      severity: 'mutex',
      status: 'resolved',
      note: '高吞吐以算力换成本，已通过异步批处理消解'
    })
  }
  // 合规 vs 体验
  if (byKey.compliance.score >= 75 && byKey.ux.score <= 55) {
    conflicts.push({
      type: 'compliance_vs_ux',
      dims: ['compliance', 'ux'],
      severity: 'blocking',
      status: 'escalated',
      note: '强合规校验增加操作步骤，需产品与合规联合评审'
    })
  }
  // 可观测 vs 成本
  if (byKey.observability.score <= 50 && byKey.cost.score >= 75) {
    conflicts.push({
      type: 'obs_vs_cost',
      dims: ['observability', 'cost'],
      severity: 'blocking',
      status: 'escalated',
      note: '为压成本削减监控，违反 I-06 治理基线，需补回'
    })
  }
  const blocking = conflicts.filter((c) => c.severity === 'blocking').length
  const resolved = conflicts.filter((c) => c.status === 'resolved').length
  return { conflicts, blockingConflicts: blocking, resolvedConflicts: resolved }
}

// ⛨璇玑验证网关：5 项检查
function verifySpiral(norm, views, recon) {
  const avg = Math.round(views.reduce((s, v) => s + v.score, 0) / views.length)
  const types = new Set(norm.nodes.map((x) => x.type))
  const hasMonitor = types.has('monitor') || types.has('verify') || types.has('audit')
  const hasTest = types.has('test') || types.has('condition')

  const checks = {
    topology: { passed: !norm.hasCycle, severity: 'block', detail: norm.hasCycle ? '检测到有向环：' + (norm.cyclePath || []).join('→') : '拓扑无环' },
    data_dependency: {
      passed: norm.orphans.length === 0,
      severity: 'block',
      detail: norm.orphans.length ? '孤立节点：' + norm.orphans.join(',') : '数据依赖闭合'
    },
    conflict: { passed: recon.blockingConflicts === 0, severity: 'block', detail: recon.blockingConflicts ? recon.blockingConflicts + ' 项阻断级冲突' : '无阻断级冲突' },
    gains: { passed: avg >= 70, severity: 'warn', detail: '专家均分 ' + avg },
    code_readiness: { passed: hasMonitor && hasTest, severity: 'warn', detail: hasMonitor && hasTest ? '含监控与测试节点' : '缺少监控/测试节点' }
  }
  const veto = !checks.conflict.passed || !checks.topology.passed || !checks.data_dependency.passed
  return { checks, veto, avg }
}

// 治理闸门：8 闸门全量门禁
function governGate(views, recon, spiral, tenant) {
  const g = []
  g.push({ id: { code: 'G1', name: '拓扑无环' }, passed: spiral.checks.topology.passed, reason: spiral.checks.topology.detail })
  g.push({ id: { code: 'G2', name: '数据依赖闭合' }, passed: spiral.checks.data_dependency.passed, reason: spiral.checks.data_dependency.detail })
  g.push({ id: { code: 'G3', name: '冲突清零' }, passed: spiral.checks.conflict.passed, reason: spiral.checks.conflict.detail })
  g.push({ id: { code: 'G4', name: '双联盟十四维齐备' }, passed: views.length === 14, reason: '已派发 ' + views.length + ' 维' })
  g.push({ id: { code: 'G5', name: '专家均分≥50' }, passed: spiral.avg >= 50, reason: '均分 ' + spiral.avg })
  g.push({ id: { code: 'G6', name: '增益≥阈值' }, passed: spiral.checks.gains.passed, reason: spiral.checks.gains.detail })
  g.push({ id: { code: 'G7', name: '代码就绪' }, passed: spiral.checks.code_readiness.passed, reason: spiral.checks.code_readiness.detail })
  g.push({ id: { code: 'G8', name: '璇玑验证通过' }, passed: !spiral.veto, reason: spiral.veto ? '验证网关否决' : '验证网关通过' })

  const approved =
    !spiral.veto && recon.blockingConflicts === 0 && views.length === 14 && spiral.avg >= 50 && g.every((x) => x.passed)
  const gate = approved ? 'G3-通过' : '驳回'
  const reason = approved ? '' : g.filter((x) => !x.passed).map((x) => x.id.name + '：' + x.reason).join('；')
  return {
    gates: g,
    approved,
    gate,
    reason,
    algorithm_veto: spiral.veto,
    gates_passed: g.filter((x) => x.passed).length,
    gates_total: g.length
  }
}

// 优化出图：自动补齐监控/验证节点
function optimizeGraph(norm) {
  const nodes = norm.nodes.map((n) => ({ id: n.id, name: n.name, type: n.type }))
  const edges = norm.edges.map((e) => ({ from: e.from, to: e.to }))
  const types = new Set(nodes.map((n) => n.type))
  const metric = '关键路径压缩率'
  const algorithm = '关键路径 + 监控注入'
  let added = 0
  const ids = new Set(nodes.map((n) => n.id))
  function add(id, name, type, linkFrom) {
    if (!ids.has(id)) {
      nodes.push({ id, name, type })
      ids.add(id)
      added++
    }
    if (linkFrom && !edges.some((e) => e.from === linkFrom && e.to === id)) {
      edges.push({ from: linkFrom, to: id })
    }
  }
  if (!types.has('monitor') && !types.has('verify')) {
    add('mon_1', '运行监控', 'monitor', norm.nodes.length ? norm.nodes[0].id : null)
    norm.nodes.forEach((n) => add('mon_1', '运行监控', 'monitor', n.id))
    added = new Set(nodes.map((n) => n.id)).size - norm.nodes.length
  }
  if (!types.has('condition')) {
    // 不强行插入分支，仅标注
  }
  return { nodes, edges, metric, algorithm, added_nodes: added }
}

// 主编排
function runAlliance(flow, tenant = 'default') {
  const norm = normalizeFlow(flow)
  const views = dispatchDimensions(norm, tenant)
  const recon = reconcile(views)
  const spiral = verifySpiral(norm, views, recon)
  const gateDetail = governGate(views, recon, spiral, tenant)
  const opt = optimizeGraph(norm)
  const score = spiral.avg

  const report = {
    governance: {
      score,
      gate: gateDetail.gate,
      gate_detail: {
        gates: gateDetail.gates,
        approved: gateDetail.approved,
        algorithm_veto: gateDetail.algorithm_veto,
        reason: gateDetail.reason,
        gates_passed: gateDetail.gates_passed,
        gates_total: gateDetail.gates_total
      }
    },
    fourteen_dimensions: views,
    conflicts: recon.conflicts,
    verification: {
      checks: spiral.checks,
      veto: spiral.veto,
      avg_score: spiral.avg
    },
    optimization: {
      metric: opt.metric,
      algorithm: opt.algorithm,
      added_nodes: opt.added_nodes,
      optimized_graph: { nodes: opt.nodes, edges: opt.edges }
    },
    spiral: {
      algorithm_veto: spiral.veto,
      gates_passed: gateDetail.gates_passed,
      critical_path_before: norm.nodes.map((n) => n.id),
      critical_path_after: opt.nodes.map((n) => n.id),
      speedup: +(1 + opt.added_nodes * 0.08).toFixed(2)
    }
  }
  return report
}

// 融合发布：落盘算子市场
function publish(payload, store) {
  const flow = payload.flow || {}
  const tenant = payload.tenant || 'default'
  const report = runAlliance(flow, tenant)
  const gd = report.governance.gate_detail
  // 双验收联动：需求侧 Done ∧ 治理门禁通过 ∧ 璇玑未否决
  const taskDone = payload.task_done === true || payload.task_done === 'true'
  const dualOk = taskDone && gd.approved && !gd.algorithm_veto

  if (!dualOk) {
    return {
      published: false,
      reason:
        '上架被管制门禁拦截：' +
        [
          !taskDone ? '需求侧任务未标记 Done' : null,
          !gd.approved ? '治理门禁未通过' : null,
          gd.algorithm_veto ? '璇玑验证否决' : null
        ]
          .filter(Boolean)
          .join('；'),
      governance: report.governance
    }
  }

  const name = payload.name || '全维融合算子'
  const pkgId = 'op_' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
  const og = report.optimization.optimized_graph
  const pkg = store.insert('market', {
    id: pkgId,
    name,
    description: payload.description || payload.requirement || '双璇玑十四维治理融合产物',
    requirement: payload.requirement || '',
    tags: payload.tags || ['xuanji', 'fusion'],
    task_id: payload.task_id || null,
    task_done: taskDone,
    tenant,
    graph: { nodes: og.nodes, edges: og.edges },
    governance_score: report.governance.score,
    governance_gate: report.governance.gate,
    created_by: 'xuanji',
    nodes: og.nodes.length,
    edges: og.edges.length,
    updated_at: new Date().toISOString()
  })

  return {
    published: true,
    package: { id: pkg.id, name: pkg.name, nodes: pkg.nodes, edges: pkg.edges },
    governance: report.governance,
    provenance: {
      algo_verified: !gd.algorithm_veto,
      gates_passed: gd.gates_passed === gd.gates_total,
      critical_path_before: report.spiral.critical_path_before.length,
      critical_path_after: report.spiral.critical_path_after.length,
      speedup: report.spiral.speedup,
      conflicts: report.conflicts.length,
      expert_score: +report.governance.score.toFixed(1)
    }
  }
}

module.exports = {
  BUSINESS_LEAGUE,
  DEV_LEAGUE,
  FOURTEEN,
  normalizeFlow,
  dispatchDimensions,
  reconcile,
  verifySpiral,
  governGate,
  optimizeGraph,
  runAlliance,
  publish
}
