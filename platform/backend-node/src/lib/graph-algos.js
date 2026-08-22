'use strict';

/**
 * 图算法库（知识图谱域共享：邻接构建 / BFS 最短路 / PageRank /
 * 度中心性 / Brandes 介数 / 标签传播社区 / 激活扩散）
 * 注意：本库为 /graph/* 端点的原始实现；统一编排核心的公式单源实现见 ai-flow-graph.js。
 */
const { readJSON } = require('./json-store');

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
  damping = damping || 0.85;
  maxIter = maxIter || 80;
  const n = nodes.length;
  if (n === 0) return {};
  const idIndex = {};
  nodes.forEach((node, i) => { idIndex[node.id] = i; });
  const outLinks = nodes.map(() => []);
  edges.forEach((e) => {
    const si = idIndex[e.source], ti = idIndex[e.target];
    if (si !== undefined && ti !== undefined && outLinks[si].indexOf(ti) === -1) {
      outLinks[si].push(ti);
    }
  });
  let pr = nodes.map(() => 1 / n);
  for (let iter = 0; iter < maxIter; iter++) {
    const newPr = nodes.map(() => (1 - damping) / n);
    for (let i = 0; i < n; i++) {
      const out = outLinks[i];
      if (out.length === 0) {
        for (let j = 0; j < n; j++) newPr[j] += damping * pr[i] / n;
      } else {
        const share = damping * pr[i] / out.length;
        out.forEach((j) => { newPr[j] += share; });
      }
    }
    const diff = pr.reduce((s, v, i) => s + Math.abs(v - newPr[i]), 0);
    pr = newPr;
    if (diff < 1e-6) break;
  }
  const result = {};
  nodes.forEach((node, i) => { result[node.id] = pr[i]; });
  return result;
}

function degreeCentrality(nodes, edges) {
  const inDeg = {}, outDeg = {};
  nodes.forEach((n) => { inDeg[n.id] = 0; outDeg[n.id] = 0; });
  edges.forEach((e) => {
    if (outDeg[e.source] !== undefined) outDeg[e.source]++;
    if (inDeg[e.target] !== undefined) inDeg[e.target]++;
  });
  const total = nodes.length - 1;
  const result = {};
  nodes.forEach((n) => {
    const d = (inDeg[n.id] || 0) + (outDeg[n.id] || 0);
    result[n.id] = {
      degree: d,
      inDegree: inDeg[n.id] || 0,
      outDegree: outDeg[n.id] || 0,
      normalized: total > 0 ? d / total : 0
    };
  });
  return result;
}

function betweennessCentrality(nodes, edges) {
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const cb = {};
  nodes.forEach((n) => { cb[n.id] = 0; });
  const ids = nodes.map((n) => n.id);
  ids.forEach((s) => {
    const S = [];
    const P = {};
    const sigma = {};
    ids.forEach((t) => { P[t] = []; sigma[t] = 0; });
    sigma[s] = 1;
    const Q = [s];
    while (Q.length) {
      const v = Q.shift();
      S.push(v);
      (adj[v] || []).forEach((w) => {
        if (sigma[w] === 0) Q.push(w);
        sigma[w] += sigma[v];
        P[w].push(v);
      });
    }
    const delta = {};
    ids.forEach((t) => { delta[t] = 0; });
    while (S.length) {
      const w = S.pop();
      P[w].forEach((v) => {
        if (sigma[w] > 0) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
      });
      if (w !== s) cb[w] += delta[w];
    }
  });
  return cb;
}

function labelPropagation(nodes, edges, maxIter) {
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

function activateSpread(nodes, edges, seedId, decay) {
  decay = decay || 0.7;
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
    if (cur.depth < 6 && cur.e > 0.01) {
      (adj[cur.id] || []).forEach((nb) => {
        q.push({ id: nb, e: cur.e * decay, depth: cur.depth + 1 });
      });
    }
  }
  return energy;
}

module.exports = { graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality, labelPropagation, activateSpread };
