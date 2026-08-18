'use strict'
/**
 * 真实图算法实现（无第三方依赖）。
 * 统一约定：node = { id, label?, type?, ... }，edge = { source, target, weight? }。
 * 前端部分接口使用 { from, to }，这里自动兼容。
 */

function normEdge(e) {
  const s = e.source != null ? e.source : e.from
  const t = e.target != null ? e.target : e.to
  return { source: s, target: t, weight: e.weight != null ? e.weight : 1 }
}

function build(nodes, edges) {
  const ns = nodes || []
  const es = (edges || []).map(normEdge)
  const idx = new Map()
  ns.forEach((n, i) => idx.set(n.id, i))
  const adj = new Map() // id -> [{to, w}]
  ns.forEach((n) => adj.set(n.id, []))
  es.forEach((e) => {
    if (idx.has(e.source) && idx.has(e.target)) {
      adj.get(e.source).push({ to: e.target, w: e.weight })
      adj.get(e.target).push({ to: e.source, w: e.weight }) // 默认无向
    }
  })
  return { ns, es, idx, adj }
}

// PageRank（带阻尼 0.85，处理悬挂节点）
function pagerank(nodes, edges, opts = {}) {
  const { ns, es, idx } = build(nodes, edges)
  const n = ns.length
  if (n === 0) return {}
  const damping = opts.damping != null ? opts.damping : 0.85
  const iters = opts.iterations || 60
  const out = new Array(n).fill(0).map(() => [])
  const inL = new Array(n).fill(0).map(() => [])
  es.forEach((e) => {
    const s = idx.get(e.source)
    const t = idx.get(e.target)
    out[s].push(t)
    inL[t].push(s)
  })
  let pr = new Array(n).fill(1 / n)
  for (let it = 0; it < iters; it++) {
    const next = new Array(n).fill((1 - damping) / n)
    for (let i = 0; i < n; i++) {
      if (out[i].length === 0) {
        for (let j = 0; j < n; j++) next[j] += (damping * pr[i]) / n
      } else {
        const share = (damping * pr[i]) / out[i].length
        out[i].forEach((t) => (next[t] += share))
      }
    }
    pr = next
  }
  const res = {}
  ns.forEach((nd, i) => (res[nd.id] = +pr[i].toFixed(6)))
  return res
}

// 度中心性（标准化到 0..1）
function degreeCentrality(nodes, edges) {
  const { ns, adj } = build(nodes, edges)
  const deg = new Map()
  ns.forEach((n) => (deg.set(n.id, adj.get(n.id).length)))
  const max = Math.max(1, ...Array.from(deg.values()))
  const res = {}
  deg.forEach((v, k) => (res[k] = +(v / max).toFixed(4)))
  return res
}

// 中介中心性（Brandes 近似，节点规模适中时使用）
function betweennessCentrality(nodes, edges) {
  const { ns, adj, idx } = build(nodes, edges)
  const n = ns.length
  const cb = new Array(n).fill(0)
  for (let s = 0; s < n; s++) {
    const stack = []
    const pred = Array.from({ length: n }, () => [])
    const sigma = new Array(n).fill(0)
    const dist = new Array(n).fill(-1)
    sigma[s] = 1
    dist[s] = 0
    const q = [s]
    while (q.length) {
      const v = q.shift()
      stack.push(v)
      for (const { to: w } of adj.get(ns[v].id)) {
        const wi = idx.get(w)
        if (dist[wi] < 0) {
          dist[wi] = dist[v] + 1
          q.push(wi)
        }
        if (dist[wi] === dist[v] + 1) {
          sigma[wi] += sigma[v]
          pred[wi].push(v)
        }
      }
    }
    const delta = new Array(n).fill(0)
    while (stack.length) {
      const w = stack.pop()
      pred[w].forEach((v) => {
        delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w])
      })
      if (w !== s) cb[w] += delta[w]
    }
  }
  const max = Math.max(1e-9, ...cb)
  const res = {}
  ns.forEach((nd, i) => (res[nd.id] = +(cb[i] / max).toFixed(4)))
  return res
}

// 标签传播社区发现
function communities(nodes, edges, opts = {}) {
  const { ns, adj, idx } = build(nodes, edges)
  const n = ns.length
  if (n === 0) return []
  const label = new Array(n).fill(0).map((_, i) => i)
  const order = [...Array(n).keys()]
  for (let it = 0; it < 25; it++) {
    let changed = false
    for (let k = n - 1; k > 0; k--) {
      const j = Math.floor(Math.random() * (k + 1))
      ;[order[k], order[j]] = [order[j], order[k]]
    }
    for (const i of order) {
      const nb = adj.get(ns[i].id)
      if (nb.length === 0) continue
      const count = new Map()
      nb.forEach(({ to }) => {
        const li = label[idx.get(to)]
        count.set(li, (count.get(li) || 0) + 1)
      })
      let best = label[i]
      let bestC = -1
      count.forEach((c, l) => {
        if (c > bestC) {
          bestC = c
          best = l
        }
      })
      if (best !== label[i]) {
        label[i] = best
        changed = true
      }
    }
    if (!changed) break
  }
  const groups = new Map()
  label.forEach((l, i) => {
    if (!groups.has(l)) groups.set(l, [])
    groups.get(l).push(ns[i].id)
  })
  const result = []
  let gi = 0
  groups.forEach((members) => {
    gi++
    // 社区内部边数 / 理论最大边数 = 密度
    const set = new Set(members)
    let internal = 0
    ;(edges || []).forEach((e) => {
      const ne = normEdge(e)
      if (set.has(ne.source) && set.has(ne.target)) internal++
    })
    const density = members.length > 1 ? +(internal / (members.length * (members.length - 1) / 2)).toFixed(3) : 0
    result.push({ id: 'C' + gi, nodes: members, density })
  })
  return result
}

// BFS 最短路径（无向，加权代价）
function shortestPath(nodes, edges, source, target) {
  const { ns, adj } = build(nodes, edges)
  if (!adj.has(source) || !adj.has(target)) return { path: [], length: 0, total_weight: 0, found: false }
  const prev = new Map()
  const cost = new Map()
  const q = [source]
  prev.set(source, null)
  cost.set(source, 0)
  while (q.length) {
    const cur = q.shift()
    if (cur === target) break
    for (const { to, w } of adj.get(cur)) {
      if (!prev.has(to)) {
        prev.set(to, cur)
        cost.set(to, cost.get(cur) + w)
        q.push(to)
      }
    }
  }
  if (!prev.has(target)) return { path: [], length: 0, total_weight: 0, found: false }
  const path = []
  let c = target
  while (c !== null) {
    path.unshift(c)
    c = prev.get(c)
  }
  return { path, length: path.length - 1, total_weight: +cost.get(target).toFixed(3), found: true }
}

// 激活传播：从种子节点沿边扩散能量（衰减）
function activate(nodes, edges, startNodes, iterations = 10, decay = 0.85) {
  const { ns, adj } = build(nodes, edges)
  const act = new Map()
  ns.forEach((n) => act.set(n.id, 0))
  ;(startNodes || []).forEach((s) => {
    if (act.has(s)) act.set(s, 1)
  })
  for (let i = 0; i < iterations; i++) {
    const next = new Map()
    act.forEach((v, k) => next.set(k, v))
    act.forEach((v, k) => {
      if (v > 0) {
        for (const { to, w } of adj.get(k)) {
          const gain = v * decay * (w && w > 0 ? Math.min(1, 1 / w) : 1)
          if (gain > next.get(to)) next.set(to, +gain.toFixed(6))
        }
      }
    })
    act.forEach((v, k) => act.set(k, next.get(k)))
  }
  const res = []
  act.forEach((v, k) => {
    if (v > 0) res.push({ id: k, value: +v.toFixed(4) })
  })
  res.sort((a, b) => b.value - a.value)
  return res
}

// 节点邻居：返回 [id, weight] 二维数组（适配前端 GraphView）
function neighbors(nodes, edges, id) {
  const { adj } = build(nodes, edges)
  const list = adj.get(id) || []
  return list.map(({ to, w }) => [to, w])
}

// 共同邻居推荐（Jaccard 相似度）
function recommend(nodes, edges, contextNodes, topN = 8) {
  const { ns, adj } = build(nodes, edges)
  const ctx = (contextNodes || []).filter((x) => adj.has(x))
  if (!ctx.length) return []
  const ctxSets = ctx.map((x) => new Set(adj.get(x).map((a) => a.to)))
  const scoreMap = new Map()
  const commonMap = new Map()
  ns.forEach((n) => {
    if (ctx.includes(n.id)) return
    const nb = adj.get(n.id).map((a) => a.to)
    let inter = 0
    const commons = []
    ctxSets.forEach((s, si) => {
      const a = ctxSets[si]
      nb.forEach((x) => {
        if (a.has(x)) {
          inter++
          if (!commons.includes(x)) commons.push(x)
        }
      })
    })
    const union = new Set([...nb, ...ctxSets[0] ? [] : []]).size || 1
    const score = ctxSets[0] ? inter / Math.max(1, nb.length + Array.from(ctxSets[0]).length - inter) : 0
    if (inter > 0) {
      scoreMap.set(n.id, +score.toFixed(4))
      commonMap.set(n.id, commons)
    }
  })
  const res = []
  scoreMap.forEach((score, id) => {
    const node = ns.find((x) => x.id === id)
    res.push({ id, node_id: id, label: node ? node.label : id, score, common: commonMap.get(id) })
  })
  res.sort((a, b) => b.score - a.score)
  return res.slice(0, topN)
}

// 导出/导入迁移包
function exportBundle(store) {
  return {
    version: '1.0',
    exported_at: new Date().toISOString(),
    graph: {
      nodes: store.all('graph_nodes'),
      edges: store.all('graph_edges')
    },
    dialogue_sessions: store.all('dialogue_sessions')
  }
}

function mergeBundle(store, bundle) {
  if (bundle && bundle.graph) {
    ;(bundle.graph.nodes || []).forEach((n) => store.upsert('graph_nodes', n))
    ;(bundle.graph.edges || []).forEach((e) => store.upsert('graph_edges', e))
  }
  if (bundle && bundle.dialogue_sessions) {
    bundle.dialogue_sessions.forEach((d) => store.upsert('dialogue_sessions', d))
  }
  return { ok: true, graph_nodes: store.all('graph_nodes').length, graph_edges: store.all('graph_edges').length }
}

module.exports = {
  pagerank,
  degreeCentrality,
  betweennessCentrality,
  communities,
  shortestPath,
  activate,
  neighbors,
  recommend,
  exportBundle,
  mergeBundle,
  normEdge,
  build
}
