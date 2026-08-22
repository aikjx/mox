'use strict';

/**
 * 项目全息图谱 · 图构建与查询算法（domain 层 · 纯算法 · 零 IO）
 * ------------------------------------------------------------------
 * 节点类型：domain（业务域）/ module（模块）/ engine（引擎·引用宇宙）/
 *          algorithm（算法）/ data（数据资产）/ doc（文档）
 * 边类型：
 *   implements      域实现业务功能（域 → 域自身功能已内聚，用于域↔模块关系）
 *   uses_engine     域使用引擎（domain/module → engine）
 *   implements_algo 引擎实现算法（engine → algorithm）
 *   consumes        算法消费引擎（algorithm.consumers 反向声明）
 *   persists_to     域持久化到数据资产（domain/module → data）
 *   documented_by   域被文档记录（domain/module → doc）
 */

function buildAtlasGraph({ DOMAINS, MODULES, ALGORITHMS, DATA_ASSETS, DOCS, ENGINES, ENGINE_EDGES }) {
  const nodes = [];
  const edges = [];
  const nodeIds = new Set();
  const addNode = (n) => { nodes.push(n); nodeIds.add(n.id); };
  const addEdge = (from, to, type, note = '') => edges.push({ from, to, type, note });

  // 业务域节点
  DOMAINS.forEach(d => addNode({
    id: d.id, kind: 'domain', name: d.name,
    keyFeatures: d.keyFeatures, codePath: d.codePath, isModule: false
  }));
  // 模块节点
  MODULES.forEach(m => addNode({
    id: m.id, kind: 'module', name: m.name,
    keyFeatures: m.keyFeatures, codePath: m.codePath, isModule: true
  }));
  // 引擎节点（引用引擎宇宙注册表——atlas 与 universe 共享同一引擎真相源）
  ENGINES.forEach(e => addNode({
    id: e.id, kind: 'engine', name: e.name,
    keyFunctions: e.keyFunctions, codePath: e.codePath
  }));
  // 算法节点
  ALGORITHMS.forEach(a => addNode({
    id: a.id, kind: 'algorithm', name: a.name, principle: a.principle,
    codePath: a.codePath, singleSource: a.singleSource, category: a.category
  }));
  // 数据资产节点
  DATA_ASSETS.forEach(x => addNode({
    id: `data:${x.file}`, kind: 'data', name: x.file, desc: x.desc
  }));
  // 文档节点
  DOCS.forEach(x => addNode({
    id: `doc:${x.file}`, kind: 'doc', name: x.file.split('/').pop(), path: x.file, desc: x.desc
  }));

  // 域/模块 → 引擎（uses_engine，按节点存在性过滤）
  [...DOMAINS, ...MODULES].forEach(u => {
    (u.engines || []).forEach(eng => {
      if (nodeIds.has(eng)) addEdge(u.id, eng, 'uses_engine');
    });
  });
  // 引擎/模块 → 算法（implements_algo，由算法 consumers 反推）
  ALGORITHMS.forEach(a => {
    (a.consumers || []).forEach(c => {
      if (nodeIds.has(c)) addEdge(c, a.id, 'implements_algo');
    });
  });
  // 域/模块 → 数据资产（persists_to）
  [...DOMAINS, ...MODULES].forEach(u => {
    (u.dataAssets || []).forEach(f => addEdge(u.id, `data:${f}`, 'persists_to'));
  });
  // 域 → 文档（documented_by）
  DOCS.forEach(x => addEdge(x.domain, `doc:${x.file}`, 'documented_by'));

  // 引擎间关联边（注入引擎宇宙的 depends_on/delegates_to/degrades_to/data_flows_to）
  // 保证 域→引擎→引擎→算法 跨子图全连通（如 expert-alliance 域 → expert-alliance 引擎 → llm-gateway）
  (ENGINE_EDGES || []).forEach(e => {
    if (nodeIds.has(e.from) && nodeIds.has(e.to)) {
      addEdge(e.from, e.to, e.type, e.note || '');
    }
  });

  return { nodes, edges };
}

/** 多跳链路追踪：domain → engine → algorithm / domain → data 的完整影响面 */
function impactAnalysis(nodes, edges, seedId) {
  const adj = new Map();
  nodes.forEach(n => adj.set(n.id, []));
  edges.forEach(e => { if (adj.has(e.from)) adj.get(e.from).push(e); });
  const visited = new Set([seedId]);
  const queue = [seedId];
  const reachable = [];
  while (queue.length) {
    const cur = queue.shift();
    for (const e of (adj.get(cur) || [])) {
      if (!visited.has(e.to)) { visited.add(e.to); queue.push(e.to); reachable.push(e); }
    }
  }
  return { seed: seedId, reachableNodes: [...visited].filter(x => x !== seedId), edges: reachable };
}

/** 无向连通分量（无孤岛检查） */
function connectedComponents(nodeIds, edges) {
  const u = new Map(nodeIds.map(id => [id, new Set()]));
  edges.forEach(e => {
    if (u.has(e.from) && u.has(e.to)) { u.get(e.from).add(e.to); u.get(e.to).add(e.from); }
  });
  const seen = new Set(); const comps = [];
  for (const id of nodeIds) {
    if (seen.has(id)) continue;
    const comp = []; const q = [id]; seen.add(id);
    while (q.length) {
      const c = q.shift(); comp.push(c);
      for (const nb of (u.get(c) || new Set())) if (!seen.has(nb)) { seen.add(nb); q.push(nb); }
    }
    comps.push(comp.sort());
  }
  return comps.sort((a, b) => b.length - a.length);
}

module.exports = { buildAtlasGraph, impactAnalysis, connectedComponents };
