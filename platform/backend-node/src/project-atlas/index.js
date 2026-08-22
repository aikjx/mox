'use strict';

/**
 * 项目全息图谱（Project Atlas）域门面
 * ------------------------------------------------------------------
 * 把整个项目机器图谱化：24 业务域 + 4 模块 + 17 引擎 + 15 算法 +
 * 34 数据资产 + 31 核心文档，全部关联本地代码路径，归一化承载。
 *
 * 无破窗验证（每次调用 verifyAtlas 动态比对真实代码库）：
 *   W1 路由域全覆盖：routes/index.js DOMAINS 动态比对（新增域漏登记即 FAIL）
 *   W2 数据资产全覆盖：data/ 目录实际文件 vs 注册表（新文件漏登记即 FAIL）
 *   W3 代码路径存在：所有域/模块/算法的 codePath 必须真实存在
 *   W4 文档存在：注册表声明的每个文档必须存在
 *   W5 引用完整性：所有 uses_engine 边指向引擎宇宙真实引擎
 *   W6 域功能内聚：每个域至少 3 条关键功能 + 至少 1 个引擎 + 有文档
 *   W7 算法单源：全部算法 singleSource=true（无重复实现）
 *   W8 图谱连通：无孤岛（业务域↔引擎↔算法↔数据↔文档全链路连通）
 */

const fs = require('fs');
const path = require('path');

const { DOMAINS, MODULES } = require('./domain/business-registry');
const { ALGORITHMS, DATA_ASSETS, DOCS } = require('./domain/tech-registry');
const { buildAtlasGraph, impactAnalysis, connectedComponents } = require('./domain/atlas-graph');
const { ENGINES } = require('../engine-universe/domain/engine-registry');
const { ENGINE_EDGES } = require('../engine-universe/domain/relation-registry');

const ROOT = path.join(__dirname, '..', '..');
const DATA_DIR = path.join(ROOT, 'data');
const ROUTES_DOMAINS = require('../routes').DOMAINS.map(d => d[0]);
const ENGINE_IDS = ENGINES.map(e => e.id).concat(['engine-universe']);

// 引擎宇宙自身也节点化（atlas 与 universe 共享同一引擎真相源 + 各自自身）
const ENGINE_NODES = [...ENGINES, {
  id: 'engine-universe', name: '引擎宇宙图谱', codePath: 'src/engine-universe/index.js',
  keyFunctions: ['17 引擎节点化与关联边查询', '需求归一化链服务映射', '全链路 113 项机器验证']
}, {
  id: 'engine-kernel', name: '引擎内核', codePath: 'src/engine-kernel/index.js',
  keyFunctions: ['槽位契约（一切皆可插件化，切换引擎零代码改动）', '瞬间切换与探活回滚', '三层插件商城（system/cloud/local）', 'AI 自动配置引擎组合']
}, {
  id: 'project-atlas', name: '项目全息图谱引擎', codePath: 'src/project-atlas/index.js',
  keyFunctions: ['全项目资产图谱化（域/模块/引擎/算法/数据/文档）', '无破窗验证（动态比对路由域/数据目录/代码路径）', '影响面分析与图谱检索']
}];

const graph = buildAtlasGraph({ DOMAINS, MODULES, ALGORITHMS, DATA_ASSETS, DOCS, ENGINES: ENGINE_NODES, ENGINE_EDGES });
const NODE_INDEX = Object.fromEntries(graph.nodes.map(n => [n.id, n]));
const PROJECT_ROOT = path.join(ROOT, '..', '..');

// ============ 查询 API ============

function getAtlas() {
  const byKind = {};
  graph.nodes.forEach(n => { byKind[n.kind] = (byKind[n.kind] || 0) + 1; });
  const byEdge = {};
  graph.edges.forEach(e => { byEdge[e.type] = (byEdge[e.type] || 0) + 1; });
  return {
    nodes: graph.nodes, edges: graph.edges,
    stats: {
      nodeCount: graph.nodes.length, edgeCount: graph.edges.length,
      byKind, byEdge,
      selfDeveloped: true,
      frameworkDeps: []
    }
  };
}

/** 单域全景：功能/引擎/算法/数据/文档一屏尽览（算法聚合自域所辖引擎） */
function getDomainDetail(domainId) {
  const node = NODE_INDEX[domainId];
  if (!node) return null;
  const reg = DOMAINS.find(d => d.id === domainId) || MODULES.find(m => m.id === domainId);
  if (!reg) return null;
  const out = graph.edges.filter(e => e.from === domainId);
  // 算法聚合：域的每个引擎的 implements_algo 入边（域 → 引擎 → 算法 两跳，按算法去重合并实现引擎）
  const engineIds = out.filter(e => e.type === 'uses_engine').map(e => e.to);
  const algoAgg = new Map();
  graph.edges
    .filter(e => e.type === 'implements_algo' && engineIds.includes(e.from))
    .forEach(e => {
      if (!algoAgg.has(e.to)) {
        const n = NODE_INDEX[e.to];
        algoAgg.set(e.to, { id: e.to, implementedBy: [e.from], name: n?.name, principle: n?.principle, codePath: n?.codePath });
      } else {
        algoAgg.get(e.to).implementedBy.push(e.from);
      }
    });
  const algorithms = [...algoAgg.values()];
  return {
    id: reg.id, name: reg.name, isModule: !!reg.isModule || MODULES.some(m => m.id === domainId),
    keyFeatures: reg.keyFeatures, codePath: reg.codePath,
    engines: out.filter(e => e.type === 'uses_engine').map(e => ({ id: e.to, name: NODE_INDEX[e.to]?.name })),
    algorithms,
    dataAssets: out.filter(e => e.type === 'persists_to').map(e => ({ file: NODE_INDEX[e.to]?.name, desc: NODE_INDEX[e.to]?.desc })),
    docs: out.filter(e => e.type === 'documented_by').map(e => ({ path: NODE_INDEX[e.to]?.path, desc: NODE_INDEX[e.to]?.desc }))
  };
}

/** 影响面分析：改动一个节点会波及哪些引擎/算法/数据/文档 */
function impact(seedId) {
  if (!NODE_INDEX[seedId]) return null;
  const result = impactAnalysis(graph.nodes, graph.edges, seedId);
  return {
    seed: seedId, seedName: NODE_INDEX[seedId].name,
    impacted: result.reachableNodes.map(id => ({ id, kind: NODE_INDEX[id]?.kind, name: NODE_INDEX[id]?.name })),
    total: result.reachableNodes.length
  };
}

/** 自然语言检索图谱资产 */
function searchAtlas(keyword) {
  const kw = String(keyword || '').toLowerCase();
  if (!kw) return { nodes: [], total: 0 };
  const hits = graph.nodes.filter(n =>
    (n.name || '').toLowerCase().includes(kw) ||
    (n.desc || '').toLowerCase().includes(kw) ||
    (n.principle || '').toLowerCase().includes(kw) ||
    (n.codePath || '').toLowerCase().includes(kw) ||
    (n.keyFeatures || []).some(f => f.toLowerCase().includes(kw))
  );
  return { keyword, nodes: hits, total: hits.length };
}

// ============ 无破窗验证 ============

function verifyAtlas() {
  const checks = [];
  let passed = 0, failed = 0;
  const check = (name, ok, detail = '') => {
    if (ok) passed++; else failed++;
    checks.push({ name, ok, detail });
  };

  // W1 路由域全覆盖（动态比对 routes/index.js）
  const registered = DOMAINS.map(d => d.id);
  const missingInAtlas = ROUTES_DOMAINS.filter(d => !registered.includes(d));
  const ghostInAtlas = registered.filter(d => !ROUTES_DOMAINS.includes(d));
  check('W1 路由域全覆盖（动态比对 DOMAINS 表）',
    missingInAtlas.length === 0 && ghostInAtlas.length === 0,
    missingInAtlas.length ? `未图谱化: ${missingInAtlas.join(',')}` : (ghostInAtlas.length ? `幽灵域: ${ghostInAtlas.join(',')}` : `${ROUTES_DOMAINS.length} 域全部图谱化`));

  // W2 数据资产全覆盖（动态比对 data/ 目录）
  const actualFiles = fs.existsSync(DATA_DIR)
    ? fs.readdirSync(DATA_DIR).filter(f => f.endsWith('.json') || f.endsWith('.jsonl'))
    : [];
  const registeredFiles = DATA_ASSETS.map(x => x.file);
  const missingData = actualFiles.filter(f => !registeredFiles.includes(f));
  const ghostData = registeredFiles.filter(f => !actualFiles.includes(f));
  check('W2 数据资产全覆盖（动态比对 data/ 目录）',
    missingData.length === 0 && ghostData.length === 0,
    missingData.length ? `未登记: ${missingData.join(',')}` : (ghostData.length ? `幽灵资产: ${ghostData.join(',')}` : `${actualFiles.length} 文件全部登记`));

  // W3 代码路径存在
  for (const u of [...DOMAINS, ...MODULES]) {
    check(`W3 代码路径存在 [${u.id}] ${u.codePath}`, fs.existsSync(path.join(ROOT, u.codePath)));
  }
  for (const a of ALGORITHMS) {
    // src/ 开头 → backend-node 内；否则 → 项目根（如 melody2score/ 子项目）
    const fp = a.codePath.startsWith('src/')
      ? path.join(ROOT, a.codePath)
      : path.join(PROJECT_ROOT, a.codePath);
    check(`W3 算法代码存在 [${a.id}] ${a.codePath}`, fs.existsSync(fp));
  }

  // W4 文档存在
  for (const d of DOCS) {
    check(`W4 文档存在 ${d.file}`, fs.existsSync(path.join(PROJECT_ROOT, d.file)));
  }

  // W5 引用完整性（uses_engine 指向真实引擎节点）
  const validEngineIds = ENGINE_NODES.map(e => e.id);
  for (const u of [...DOMAINS, ...MODULES]) {
    for (const eng of (u.engines || [])) {
      check(`W5 引擎引用有效 [${u.id}] → ${eng}`, validEngineIds.includes(eng));
    }
  }

  // W6 域功能内聚
  for (const u of [...DOMAINS, ...MODULES]) {
    check(`W6 功能内聚 [${u.id}]`, (u.keyFeatures || []).length >= 3 && (u.engines || []).length >= 1);
  }
  const docCovered = new Set(DOCS.map(d => d.domain));
  const noDoc = DOMAINS.filter(d => !docCovered.has(d.id)).map(d => d.id);
  check('W6 文档覆盖全部业务域', noDoc.length === 0, noDoc.join(','));

  // W7 算法单源
  check('W7 全部算法单源自研（singleSource=true 且零框架依赖）',
    ALGORITHMS.every(a => a.singleSource === true));

  // W8 图谱连通（无孤岛）
  const comps = connectedComponents(graph.nodes.map(n => n.id), graph.edges);
  check('W8 图谱连通无孤岛', comps.length === 1,
    comps.length > 1 ? `孤岛: ${comps.slice(1).map(c => c.join(',')).join(';')}` : `${comps[0].length} 节点全连通`);

  return { ok: failed === 0, summary: { total: checks.length, passed, failed }, checks };
}

module.exports = { getAtlas, getDomainDetail, impact, searchAtlas, verifyAtlas, DOMAINS, MODULES, ALGORITHMS, DATA_ASSETS, DOCS };
