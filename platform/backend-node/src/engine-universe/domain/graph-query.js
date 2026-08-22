'use strict';

/**
 * 引擎宇宙图查询算法（domain 层 · 纯算法 · 零 IO）
 * ------------------------------------------------------------------
 * 邻接构建 / BFS 链路追踪 / 可达性 / 度统计 / 邻居查询 / 降级链终点校验。
 */

function buildAdjacency(nodes, edges) {
  const adj = new Map();
  nodes.forEach(n => adj.set(n.id, { out: [], in: [] }));
  edges.forEach(e => {
    if (adj.has(e.from)) adj.get(e.from).out.push(e);
    if (adj.has(e.to)) adj.get(e.to).in.push(e);
  });
  return adj;
}

/** BFS 最短链路追踪：返回 { found, path: [{from, to, type, note}] } */
function tracePath(adj, from, to, edgeFilter = null) {
  if (!adj.has(from) || !adj.has(to)) return { found: false, path: [], reason: `节点不存在: ${!adj.has(from) ? from : to}` };
  if (from === to) return { found: true, path: [], reason: '起点即终点' };
  const visited = new Set([from]);
  const prev = new Map(); // nodeId -> 到达该节点用的边
  const queue = [from];
  while (queue.length) {
    const cur = queue.shift();
    const neighbors = (adj.get(cur) || { out: [] }).out;
    for (const e of neighbors) {
      if (edgeFilter && !edgeFilter(e)) continue;
      if (visited.has(e.to)) continue;
      visited.add(e.to);
      prev.set(e.to, e);
      if (e.to === to) {
        const path = [];
        let node = to;
        while (node !== from) {
          const edge = prev.get(node);
          path.unshift({ from: edge.from, to: edge.to, type: edge.type, note: edge.note || '' });
          node = edge.from;
        }
        return { found: true, path };
      }
      queue.push(e.to);
    }
  }
  return { found: false, path: [], reason: '不可达' };
}

/** 可达性：from 出发沿指定边类型可达的节点集合 */
function reachableSet(adj, from, edgeFilter = null) {
  const seen = new Set();
  if (!adj.has(from)) return seen;
  const queue = [from];
  seen.add(from);
  while (queue.length) {
    const cur = queue.shift();
    for (const e of (adj.get(cur) || { out: [] }).out) {
      if (edgeFilter && !edgeFilter(e)) continue;
      if (!seen.has(e.to)) { seen.add(e.to); queue.push(e.to); }
    }
  }
  seen.delete(from);
  return seen;
}

/** 度统计：每个节点的入度/出度（按边类型细分） */
function degreeStats(adj, nodes) {
  const stats = {};
  nodes.forEach(n => {
    const a = adj.get(n.id) || { out: [], in: [] };
    const outBy = {}, inBy = {};
    a.out.forEach(e => { outBy[e.type] = (outBy[e.type] || 0) + 1; });
    a.in.forEach(e => { inBy[e.type] = (inBy[e.type] || 0) + 1; });
    stats[n.id] = { outDegree: a.out.length, inDegree: a.in.length, outBy, inBy };
  });
  return stats;
}

/** 降级链终点校验：所有 degrades_to 链最终必须收敛到指定终点（兜底不变式） */
function verifyDegradeChains(nodes, edges, terminalId) {
  const adj = buildAdjacency(nodes, edges.filter(e => e.type === 'degrades_to'));
  const results = [];
  for (const n of nodes) {
    if (!adj.get(n.id).out.length) continue;
    const reach = reachableSet(adj, n.id);
    const converged = reach.has(terminalId) || n.id === terminalId;
    results.push({ from: n.id, converged, terminals: [...reach] });
  }
  return {
    allConverged: results.every(r => r.converged),
    chains: results
  };
}

/** 邻居查询：指定节点的上下游关系（含边类型分组） */
function neighborsOf(adj, nodeId) {
  const a = adj.get(nodeId);
  if (!a) return null;
  return {
    upstream: a.in.map(e => ({ from: e.from, type: e.type, note: e.note || '' })),
    downstream: a.out.map(e => ({ to: e.to, type: e.type, note: e.note || '' }))
  };
}

/** 无向连通分量：返回 [{ nodes: [...], size }]——全域连通性（无孤岛）检查的基础 */
function connectedComponents(nodes, edges) {
  const undirected = new Map();
  nodes.forEach(n => undirected.set(n.id, new Set()));
  edges.forEach(e => {
    if (undirected.has(e.from) && undirected.has(e.to)) {
      undirected.get(e.from).add(e.to);
      undirected.get(e.to).add(e.from);
    }
  });
  const seen = new Set();
  const components = [];
  for (const n of nodes) {
    if (seen.has(n.id)) continue;
    const comp = [];
    const queue = [n.id];
    seen.add(n.id);
    while (queue.length) {
      const cur = queue.shift();
      comp.push(cur);
      for (const nb of (undirected.get(cur) || new Set())) {
        if (!seen.has(nb)) { seen.add(nb); queue.push(nb); }
      }
    }
    components.push({ nodes: comp.sort(), size: comp.length });
  }
  return components.sort((a, b) => b.size - a.size);
}

module.exports = { buildAdjacency, tracePath, reachableSet, degreeStats, verifyDegradeChains, neighborsOf, connectedComponents };
