'use strict'
/**
 * 引擎单元测试：graph.js（真实图算法）+ xuanji.js（双璇玑十四维治理）。
 * 全部使用内置 assert，无第三方依赖。
 */
const { test, section, assert, assertEqual, assertDeep, assertIncludes, assertRange, SUITE } = require('./_harness')
const graph = require('../src/graph')
const xuanji = require('../src/xuanji')

// ---------------- 图算法 ----------------
module.exports = async function runUnit() {
  section('Graph: PageRank')
  await test('星形图中心节点 PageRank 最高', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }, { id: 'e' }]
    const edges = [
      { source: 'b', target: 'a' },
      { source: 'c', target: 'a' },
      { source: 'd', target: 'a' },
      { source: 'e', target: 'a' }
    ]
    const pr = graph.pagerank(nodes, edges)
    assertRange(pr.a, 0, 1)
    assert.ok(pr.a > pr.b && pr.a > pr.c, '中心 a 应高于叶子')
    const sum = Object.values(pr).reduce((s, v) => s + v, 0)
    assertRange(sum, 0.95, 1.05, 'PageRank 应近似归一')
  })

  await test('空图 PageRank 返回空对象', () => {
    assertEqual(Object.keys(graph.pagerank([], [])).length, 0)
  })

  section('Graph: 度中心性')
  await test('星形中心度=1，叶子<1', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    const edges = [{ source: 'b', target: 'a' }, { source: 'c', target: 'a' }]
    const dc = graph.degreeCentrality(nodes, edges)
    assertEqual(dc.a, 1)
    assertRange(dc.b, 0, 1)
    assert.ok(dc.a > dc.b, '中心度最高')
  })

  section('Graph: 中介中心性 (Brandes)')
  await test('路径 b 在 a-b-c 中中介最高', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }]
    const bc = graph.betweennessCentrality(nodes, edges)
    assert.ok(bc.b > bc.a && bc.b > bc.c, 'b 是必经节点')
  })

  section('Graph: 社区发现 (标签传播)')
  await test('三角+孤立点 被分为 >=2 社区且无重叠', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'x' }]
    const edges = [
      { source: 'a', target: 'b' },
      { source: 'b', target: 'c' },
      { source: 'c', target: 'a' }
    ]
    const comm = graph.communities(nodes, edges)
    assert.ok(comm.length >= 2, '至少 2 个社区')
    const flat = comm.reduce((acc, c) => acc.concat(c.nodes), [])
    assertEqual(flat.length, 4, '所有节点都被分配')
    assertEqual(new Set(flat).size, 4, '无重叠')
    comm.forEach((c) => assert.ok(c.id && c.nodes.length > 0))
  })
  await test('空图社区返回空', () => {
    assertEqual(graph.communities([], []).length, 0)
  })

  section('Graph: 最短路径 (BFS)')
  await test('a->c 经过 b，长度 2', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }]
    const r = graph.shortestPath(nodes, edges, 'a', 'c')
    assert.ok(r.found)
    assertDeep(r.path, ['a', 'b', 'c'])
    assertEqual(r.length, 2)
  })
  await test('不连通节点 found=false', () => {
    const nodes = [{ id: 'a' }, { id: 'z' }]
    const edges = []
    const r = graph.shortestPath(nodes, edges, 'a', 'z')
    assert.ok(!r.found)
    assertEqual(r.path.length, 0)
  })

  section('Graph: 激活传播')
  await test('从 a 激活，值沿边衰减', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }]
    const act = graph.activate(nodes, edges, ['a'], 5, 0.85)
    assert.ok(act.length >= 1)
    assertEqual(act[0].id, 'a')
    assertEqual(act[0].value, 1)
    const b = act.find((x) => x.id === 'b')
    const c = act.find((x) => x.id === 'c')
    assert.ok(b && c, 'b、c 应被激活')
    assert.ok(b.value > c.value, '衰减：b 应强于 c')
  })

  section('Graph: 邻居 / normEdge / 推荐')
  await test('星形中心返回 4 个邻居', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }]
    const edges = [
      { source: 'b', target: 'a' },
      { source: 'c', target: 'a' },
      { source: 'd', target: 'a' }
    ]
    const nb = graph.neighbors(nodes, edges, 'a')
    assertEqual(nb.length, 3)
  })
  await test('normEdge 兼容 {from,to} 与 {source,target,weight}', () => {
    assertDeep(graph.normEdge({ from: 'x', to: 'y' }), { source: 'x', target: 'y', weight: 1 })
    assertDeep(graph.normEdge({ source: 'x', target: 'y', weight: 2 }), { source: 'x', target: 'y', weight: 2 })
  })
  await test('共同邻居推荐：从叶子 b 推回其余叶子', () => {
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }]
    const edges = [
      { source: 'b', target: 'a' },
      { source: 'c', target: 'a' },
      { source: 'd', target: 'a' }
    ]
    const rec = graph.recommend(nodes, edges, ['b'], 8)
    // 仅 c、d 与 b 共享邻居 a（a 自身不共享），故推荐 2 个
    assert.ok(rec.length === 2, '应推荐 c、d 共 2 个，实际 ' + rec.length)
    rec.forEach((r) => {
      assert.ok(['c', 'd'].includes(r.id), '推荐项应为 c 或 d')
      assertRange(r.score, 0, 1)
    })
  })

  // ---------------- 双璇玑治理引擎 ----------------
  section('Xuanji: 归一化')
  await test('检测有向环 a->b->c->a', () => {
    const norm = xuanji.normalizeFlow({
      nodes: [{ id: 'a' }, { id: 'b' }, { id: 'c' }],
      edges: [{ from: 'a', to: 'b' }, { from: 'b', to: 'c' }, { from: 'c', to: 'a' }]
    })
    assert.ok(norm.hasCycle, '应检测到环')
    assert.ok(norm.cyclePath && norm.cyclePath.length >= 3)
  })
  await test('孤立节点被识别', () => {
    const norm = xuanji.normalizeFlow({
      nodes: [{ id: 'a' }, { id: 'b' }],
      edges: [{ from: 'a', to: 'a' }]
    })
    assertIncludes(norm.orphans, 'b')
  })
  await test('兼容 source/target 边写法', () => {
    const norm = xuanji.normalizeFlow({
      nodes: [{ id: 'a' }, { id: 'b' }],
      edges: [{ source: 'a', target: 'b' }]
    })
    assertEqual(norm.edges.length, 1)
    assertEqual(norm.edges[0].from, 'a')
  })

  section('Xuanji: 双联盟十四维派发')
  await test('dispatch 返回恰好 14 维 (业务7+开发7)', () => {
    const norm = xuanji.normalizeFlow({
      nodes: [{ id: 'a', type: 'operator' }, { id: 'b', type: 'monitor' }],
      edges: [{ from: 'a', to: 'b' }]
    })
    const views = xuanji.dispatchDimensions(norm, 'default')
    assertEqual(views.length, 14)
    const biz = views.filter((v) => v.league === 'business').length
    const dev = views.filter((v) => v.league === 'dev').length
    assertEqual(biz, 7)
    assertEqual(dev, 7)
    views.forEach((v) => {
      assertRange(v.score, 0, 100)
      assert.ok(v.code && v.key && v.name && v.view)
    })
  })

  section('Xuanji: 归一化裁决 (冲突识别)')
  await test('合规高 + 体验低 -> 阻断级冲突', () => {
    const views = [
      { key: 'compliance', score: 80 },
      { key: 'ux', score: 50 },
      { key: 'performance', score: 60 },
      { key: 'cost', score: 60 },
      { key: 'observability', score: 60 }
    ]
    const recon = xuanji.reconcile(views)
    assert.ok(recon.blockingConflicts >= 1, '应有阻断级冲突')
    assert.ok(recon.conflicts.some((c) => c.severity === 'blocking' && c.status === 'escalated'))
  })
  await test('性能高 + 成本低 -> 互斥冲突(已消解)', () => {
    const views = [
      { key: 'performance', score: 80 },
      { key: 'cost', score: 50 },
      { key: 'compliance', score: 60 },
      { key: 'ux', score: 60 },
      { key: 'observability', score: 60 }
    ]
    const recon = xuanji.reconcile(views)
    assert.ok(recon.conflicts.some((c) => c.type === 'perf_vs_cost' && c.status === 'resolved'))
  })

  section('Xuanji: ⛨璇玑验证网关')
  await test('有环或阻断冲突 -> veto=true', () => {
    const norm = { hasCycle: true, orphans: [], cyclePath: ['a', 'b', 'a'], nodes: [] }
    const views = xuanji.dispatchDimensions(xuanji.normalizeFlow({ nodes: [{ id: 'a' }], edges: [] }), 'default')
    const recon = { blockingConflicts: 1, resolvedConflicts: 0 }
    const spiral = xuanji.verifySpiral(norm, views, recon)
    assert.ok(spiral.veto === true)
    assert.ok(spiral.checks.topology.passed === false)
    assert.ok(spiral.checks.conflict.passed === false)
  })
  await test('均分计算正确', () => {
    const norm = xuanji.normalizeFlow({ nodes: [{ id: 'a', type: 'operator' }], edges: [] })
    const views = xuanji.dispatchDimensions(norm, 'default')
    const recon = { blockingConflicts: 0 }
    const spiral = xuanji.verifySpiral(norm, views, recon)
    const avg = Math.round(views.reduce((s, v) => s + v.score, 0) / 14)
    assertEqual(spiral.avg, avg)
  })

  section('Xuanji: 治理闸门')
  // 一个公认良好的流程（含 monitor + condition，节点适中、无环）
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
  await test('良好流程 -> 闸门 G3-通过 (8/8)', () => {
    const norm = xuanji.normalizeFlow(goodFlow)
    const views = xuanji.dispatchDimensions(norm, 'default')
    const recon = xuanji.reconcile(views)
    const spiral = xuanji.verifySpiral(norm, views, recon)
    const gd = xuanji.governGate(views, recon, spiral, 'default')
    assert.ok(gd.approved, '应被批准；原因=' + gd.reason)
    assertEqual(gd.gate, 'G3-通过')
    assertEqual(gd.gates_passed, 8)
    assertEqual(gd.gates_total, 8)
  })
  await test('有环流程 -> 驳回', () => {
    const cycleFlow = {
      nodes: [{ id: 'a', type: 'operator' }, { id: 'b', type: 'monitor' }],
      edges: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }]
    }
    const norm = xuanji.normalizeFlow(cycleFlow)
    const views = xuanji.dispatchDimensions(norm, 'default')
    const recon = xuanji.reconcile(views)
    const spiral = xuanji.verifySpiral(norm, views, recon)
    const gd = xuanji.governGate(views, recon, spiral, 'default')
    assert.ok(!gd.approved)
    assertEqual(gd.gate, '驳回')
    assert.ok(gd.reason && gd.reason.length > 0)
  })

  section('Xuanji: runAlliance 主编排')
  await test('返回完整报告结构', () => {
    const rep = xuanji.runAlliance(goodFlow, 'default')
    assert.ok(rep.governance && rep.governance.score >= 0)
    assertEqual(rep.fourteen_dimensions.length, 14)
    assert.ok(Array.isArray(rep.conflicts))
    assert.ok(rep.verification && rep.verification.checks)
    assert.ok(rep.optimization && rep.optimization.optimized_graph.nodes.length > 0)
    assert.ok(rep.spiral && typeof rep.spiral.speedup === 'number')
  })
  await test('优化出图自动注入监控节点(若缺失)', () => {
    const noMon = {
      nodes: [{ id: 'a', type: 'operator' }, { id: 'b', type: 'operator' }],
      edges: [{ from: 'a', to: 'b' }]
    }
    const norm = xuanji.normalizeFlow(noMon)
    const opt = xuanji.optimizeGraph(norm)
    assert.ok(opt.nodes.some((n) => n.type === 'monitor'), '应注入 monitor')
  })

  section('Xuanji: 融合发布 (双验收)')
  await test('task_done=true + 良好流程 -> 上架成功', () => {
    const store = new (require('../src/store').Store)()
    const r = xuanji.publish({ flow: goodFlow, name: '测试融合算子', task_done: true, requirement: '专家联盟全维分析' }, store)
    assert.ok(r.published === true, '应发布成功；reason=' + (r.reason || ''))
    assert.ok(r.package && r.package.id)
    assert.ok(r.provenance && r.provenance.algo_verified === true)
  })
  await test('task_done=false -> 被管制门禁拦截(双验收联动)', () => {
    const store = new (require('../src/store').Store)()
    const r = xuanji.publish({ flow: goodFlow, name: '未完成任务', task_done: false }, store)
    assert.ok(r.published === false)
    assert.ok(r.reason && r.reason.includes('任务未标记 Done'))
  })
  await test('有环流程 + task_done=true -> 璇玑否决拦截', () => {
    const store = new (require('../src/store').Store)()
    const cycleFlow = {
      nodes: [{ id: 'a', type: 'operator' }, { id: 'b', type: 'monitor' }],
      edges: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }]
    }
    const r = xuanji.publish({ flow: cycleFlow, name: '环流程', task_done: true }, store)
    assert.ok(r.published === false)
    assert.ok(r.reason && r.reason.includes('璇玑验证否决'))
  })
}
