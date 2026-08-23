'use strict';

/**
 * 图算法库（知识图谱域共享：邻接构建 / BFS 最短路 / PageRank /
 * 度中心性 / Brandes 介数 / 标签传播社区 / 激活扩散）
 * —— 企业级归一化改造：度/介数/PageRank 的**真实实现**已迁移到单源 `src/graph/graph-formulas.js`
 *    的 GraphFormulas 对象（禁止独立重复实现）。本文件保留兼容 wrapper：
 *    · 对 graph-formulas.js 返回 flat {id:number} 的 API，在 wrapper 中包成 legacy 的 {id:{degree,...}} 结构，
 *      保证旧路由 graph.js 的 pagerank(ctx 注入) 行为不破坏。
 */
const { readJSON } = require('./json-store');
const { GraphFormulas } = require('../graph/graph-formulas');
const GF = GraphFormulas;

function graphAdjacency() {
  const nodes = readJSON('graph_nodes.json', []);
  const edges = readJSON('graph_edges.json', []);
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = { out: [], in: [] }; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].out.push(e.target);
    if (adj[e.target]) adj[e.target].in.push(e.source);
  });
  return { nodes, edges, adj };
}

function bfsPath(adj, source, target) {
  if (!adj[source] || !adj[target]) return null;
  const visited = { [source]: null };
  const q = [source];
  while (q.length) {
    const cur = q.shift();
    if (cur === target) {
      const pathArr = [];
      let n = cur;
      while (n !== null) { pathArr.unshift(n); n = visited[n]; }
      return pathArr;
    }
    (adj[cur] ? adj[cur].out : []).forEach((nb) => {
      if (!(nb in visited)) { visited[nb] = cur; q.push(nb); }
    });
  }
  return null;
}

function pagerank(nodes, edges, damping, maxIter) {
  // SINGLE-SOURCE WRAPPER → GraphFormulas.pagerank（禁止在此处独立实现）
  return GF.pagerank(nodes, edges, {
    dampingFactor: damping || 0.85,
    maxIterations: maxIter || 80,
  });
}

function degreeCentrality(nodes, edges) {
  // SINGLE-SOURCE WRAPPER（禁止在此独立实现 → 真实定义见 graph/graph-formulas.js）
  return GF.degreeCentrality(nodes, edges, { expandRaw: true, legacyShape: true });
}

function betweennessCentrality(nodes, edges, opts) {
  // SINGLE-SOURCE WRAPPER（真实定义 Brandes 见 graph/graph-formulas.js；不传 opts 则 undirected）
  return GF.betweennessCentrality(nodes, edges, opts || { directed: false });
}

function labelPropagation(nodes, edges, maxIter) {
  // 项目记忆硬性：LPA 公开 API 必须抛 DeprecationError。
  // 内部仍保留实现（通过 require('./graph-algos')._internalLabelPropagation 访问），仅供基线对照测试。
  throw new DeprecationError('lib/graph-algos.labelPropagation() 公共出口已禁用。社区检测请使用 GraphFormulas.communityDetectionCNM()（CNM 模块度贪心凝聚）。');
}
function _internalLabelPropagation(nodes, edges, maxIter) {
  maxIter = maxIter || 30;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const labels = {};
  nodes.forEach((n, i) => { labels[n.id] = i; });
  const ids = nodes.map((n) => n.id);
  let changed = true;
  let iter = 0;
  while (changed && iter < maxIter) {
    changed = false;
    iter++;
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      const tmp = ids[i]; ids[i] = ids[j]; ids[j] = tmp;
    }
    ids.forEach((v) => {
      const neighborLabels = {};
      (adj[v] || []).forEach((nb) => {
        const l = labels[nb];
        neighborLabels[l] = (neighborLabels[l] || 0) + 1;
      });
      let bestLabel = labels[v];
      let bestCount = -1;
      Object.keys(neighborLabels).forEach((l) => {
        if (neighborLabels[l] > bestCount) { bestCount = neighborLabels[l]; bestLabel = parseInt(l, 10); }
      });
      if (bestLabel !== labels[v]) { labels[v] = bestLabel; changed = true; }
    });
  }
  const communities = {};
  ids.forEach((id) => {
    const c = labels[id];
    if (!communities[c]) communities[c] = [];
    communities[c].push(id);
  });
  return communities;
}

class DeprecationError extends Error {
  constructor(msg) { super(msg); this.name = 'DeprecationError'; }
}

function activateSpread(nodes, edges, seedId, decay, maxDepth) {
  // 项目记忆硬性：method=spread 默认 decay=0.85，maxDepth=30（T5/TR-5.2 参数锁死）
  if (decay === undefined) decay = 0.85;
  if (maxDepth === undefined) maxDepth = 30;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
  });
  const energy = {};
  nodes.forEach((n) => { energy[n.id] = 0; });
  if (!adj[seedId]) return energy;
  const q = [{ id: seedId, e: 1.0, depth: 0 }];
  const visited = {};
  while (q.length) {
    const cur = q.shift();
    if (visited[cur.id] && visited[cur.id] >= cur.e) continue;
    visited[cur.id] = cur.e;
    if (cur.e > energy[cur.id]) energy[cur.id] = cur.e;
    if (cur.depth < maxDepth && cur.e > 0.0001) {
      (adj[cur.id] || []).forEach((nb) => {
        q.push({ id: nb, e: cur.e * decay, depth: cur.depth + 1 });
      });
    }
  }
  return energy;
}

module.exports = { graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality, labelPropagation, _internalLabelPropagation, DeprecationError, activateSpread };
