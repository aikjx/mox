'use strict';

/**
 * 引擎宇宙域门面（AINA-STD-001 域包入口）
 * ------------------------------------------------------------------
 * 技术图谱管理所有链接的统一入口：
 *   - 17 引擎节点 + 5 需求归一化链节点 + 30+ 关联边（依赖/委托/降级/数据流/服务）
 *   - 引擎 ↔ 本地代码路径关联（codePath + CODE_ASSOCIATIONS）
 *   - 全链路验证：代码路径存在性 / 边两端节点存在性 / 需求链连通性 / 降级链收敛性
 *
 * 对外 API：
 *   getUniverse()          完整引擎宇宙图谱（节点+边+统计）
 *   getEngineDetail(id)    单引擎详情（上下游关系+服务需求+代码路径）
 *   trace(from, to)        BFS 链路追踪（引擎间/需求链任意两点）
 *   verifyFullChain()      全链路验证报告
 */

const fs = require('fs');
const path = require('path');

const { ENGINES, ENGINE_INDEX, CATEGORY_ORDER, getEngine, listEngines } = require('./domain/engine-registry');
const {
  REQUIREMENT_NODES, ENGINE_EDGES, REQUIREMENT_EDGES, SERVICE_EDGES,
  CODE_ASSOCIATIONS, ALL_EDGES
} = require('./domain/relation-registry');
const { buildAdjacency, tracePath, reachableSet, degreeStats, verifyDegradeChains, neighborsOf, connectedComponents } = require('./domain/graph-query');

const ROOT = path.join(__dirname, '..', '..');

// 引擎宇宙自身也节点化（自举：图谱管理图谱）
const UNIVERSE_NODE = {
  id: 'engine-universe',
  name: '引擎宇宙图谱',
  category: 'knowledge',
  layer: '知识层',
  codePath: 'src/engine-universe/index.js',
  keyFunctions: [
    '全系统 17 引擎节点化：身份/类别/关键功能/代码路径/能力清单唯一权威定义',
    '关联关系显式建模：depends_on/delegates_to/degrades_to/data_flows_to/serves 可直接查询',
    '全链路验证：代码路径存在性 + 需求归一化链连通性 + 降级链收敛到 chat 兜底'
  ],
  capabilities: ['universe.query', 'universe.verify']
};

const ALL_NODES = [...ENGINES, UNIVERSE_NODE, ...REQUIREMENT_NODES.map(n => ({ ...n, isRequirement: true }))];
const NODE_INDEX = Object.fromEntries(ALL_NODES.map(n => [n.id, n]));

// ---------- 图构建（惰性单次） ----------
let adjacency = null;
function getAdjacency() {
  if (!adjacency) adjacency = buildAdjacency(ALL_NODES, ALL_EDGES);
  return adjacency;
}

// ---------- 查询 API ----------

function getUniverse() {
  const adj = getAdjacency();
  const stats = {
    engineCount: ENGINES.length + 1, // 含引擎宇宙自身
    requirementNodeCount: REQUIREMENT_NODES.length,
    nodeCount: ALL_NODES.length,
    edgeCount: ALL_EDGES.length,
    edgeByType: {},
    categories: CATEGORY_ORDER.map(([id, label]) => ({
      id, label,
      engines: ENGINES.filter(e => e.category === id).map(e => e.id)
    })),
    degrees: degreeStats(adj, ALL_NODES)
  };
  ALL_EDGES.forEach(e => { stats.edgeByType[e.type] = (stats.edgeByType[e.type] || 0) + 1; });
  return {
    nodes: ALL_NODES.map(n => ({
      id: n.id, name: n.name || n.label, category: n.category,
      layer: n.layer, stage: n.stage, isRequirement: !!n.isRequirement,
      codePath: n.codePath || null,
      keyFunctions: n.keyFunctions || null,
      capabilities: n.capabilities || null
    })),
    edges: ALL_EDGES,
    stats
  };
}

function getEngineDetail(id) {
  const node = NODE_INDEX[id];
  if (!node) return null;
  const adj = getAdjacency();
  const nb = neighborsOf(adj, id);
  const codeAssoc = CODE_ASSOCIATIONS.find(c => c.engine === id);
  return {
    ...node,
    relations: nb,
    codeFiles: codeAssoc ? codeAssoc.files : (node.codePath ? [node.codePath] : []),
    servesRequirements: nb.downstream
      .filter(e => e.type === 'serves')
      .map(e => ({ requirement: e.to, label: NODE_INDEX[e.to]?.label || e.to, note: e.note }))
  };
}

function trace(from, to, edgeType = null) {
  const adj = getAdjacency();
  const filter = edgeType ? (e) => e.type === edgeType : null;
  const result = tracePath(adj, from, to, filter);
  return {
    from, to, edgeType: edgeType || 'any',
    ...result,
    pathNodes: result.path.map(e => ({ id: e.from, name: NODE_INDEX[e.from]?.name || NODE_INDEX[e.from]?.label }))
      .concat(result.path.length ? [{ id: to, name: NODE_INDEX[to]?.name || NODE_INDEX[to]?.label }] : [])
  };
}

/** 从需求节点出发的引擎服务链路：需求归一化链 → 每环服务的引擎 */
function requirementChain() {
  const adj = getAdjacency();
  return REQUIREMENT_NODES.map(n => ({
    node: { id: n.id, label: n.label, stage: n.stage },
    servedBy: (adj.get(n.id)?.in || [])
      .filter(e => e.type === 'serves')
      .map(e => ({ engine: e.from, name: NODE_INDEX[e.from]?.name, note: e.note }))
  }));
}

// ---------- 全链路验证 ----------

function verifyFullChain() {
  const checks = [];
  let passed = 0, failed = 0;
  const check = (name, ok, detail) => {
    if (ok) passed++; else failed++;
    checks.push({ name, ok, detail: detail || '' });
  };

  // V1 代码路径存在性：每个引擎声明的 codePath 必须真实存在
  for (const e of [...ENGINES, UNIVERSE_NODE]) {
    const fp = path.join(ROOT, e.codePath);
    check(`代码路径存在 [${e.id}] ${e.codePath}`, fs.existsSync(fp));
  }
  // V1b 协作文件存在性：CODE_ASSOCIATIONS 声明的每个文件必须存在
  for (const c of CODE_ASSOCIATIONS) {
    for (const f of c.files) {
      check(`协作文件存在 [${c.engine}] ${f}`, fs.existsSync(path.join(ROOT, f)));
    }
  }

  // V2 边完整性：每条边两端节点必须在注册表中定义
  for (const e of ALL_EDGES) {
    const ok = NODE_INDEX[e.from] && NODE_INDEX[e.to];
    check(`边两端节点存在 ${e.from} -[${e.type}]-> ${e.to}`, ok,
      ok ? '' : `缺失: ${!NODE_INDEX[e.from] ? e.from : e.to}`);
  }

  // V3 需求归一化链连通性：n_ingest → n_gate 沿 flows_to 可达
  const chain = tracePath(getAdjacency(), 'n_ingest', 'n_gate', e => e.type === 'flows_to');
  check('需求归一化链连通 n_ingest → n_gate（flows_to）', chain.found,
    chain.found ? `路径 ${chain.path.map(p => `${p.from}→${p.to}`).join(' | ')}` : chain.reason);

  // V3b 需求链每一环都有引擎服务（serves 覆盖完备）
  const adj = getAdjacency();
  for (const n of REQUIREMENT_NODES) {
    const served = (adj.get(n.id)?.in || []).some(e => e.type === 'serves');
    check(`需求环节有引擎服务 [${n.id} ${n.label}]`, served);
  }

  // V4 降级链收敛性：所有 degrades_to 链必须收敛到 llm-gateway（chat 兜底不变式）
  const degrade = verifyDegradeChains(ALL_NODES, ALL_EDGES, 'llm-gateway');
  check('降级链全部收敛到 llm-gateway', degrade.allConverged,
    degrade.chains.filter(c => !c.converged).map(c => `${c.from} → ${c.terminals.join(',')}`).join('; '));

  // V5 核心能力承接完备：ai-engine-core 六大能力各有承接引擎（CAPABILITY_META 一致性）
  const capabilityEngines = {
    expert: 'expert-alliance-engine', reasoning: 'ultimate-ai-engine',
    memory: 'ultimate-ai-engine', graph: 'ai-engine',
    workflow: 'ai-engine', chat: 'llm-gateway'
  };
  for (const [cap, engine] of Object.entries(capabilityEngines)) {
    check(`能力承接完备 [${cap}] → ${engine}`, !!ENGINE_INDEX[engine]);
  }

  // V6 全域连通无孤岛：整个引擎宇宙（含需求链节点）构成单一无向连通分量
  //    语义：auto-dev/optimizer/kb 等独立域能力由 API 层直接使用，不要求从编排核心可达；
  //    但所有引擎必须通过关联边链入同一张图——这就是"技术图谱管理所有链接"。
  const components = connectedComponents(ALL_NODES, ALL_EDGES);
  const isolated = components.slice(1).map(c => c.nodes.join(','));
  check('全域连通无孤岛（单一连通分量）', components.length === 1,
    isolated.length ? `孤岛: ${isolated.join('; ')}` : `${components[0].size} 节点全部连通`);

  return {
    ok: failed === 0,
    summary: { total: checks.length, passed, failed },
    checks,
    degradeChains: degrade.chains,
    requirementChain: requirementChain()
  };
}

module.exports = {
  // 查询
  getUniverse, getEngineDetail, trace, requirementChain,
  listEngines, getEngine, CATEGORY_ORDER,
  // 验证
  verifyFullChain,
  // 注册表直出（供测试与扩展）
  ENGINES, ALL_NODES, ALL_EDGES, REQUIREMENT_NODES, CODE_ASSOCIATIONS
};
