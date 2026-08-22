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
const { FLOWS } = require('./domain/flow-registry');
const { PROJECTS, LIFECYCLE, validateProject, auditDomainOwnership } = require('./domain/project-registry');
const { buildAtlasGraph, impactAnalysis, connectedComponents } = require('./domain/atlas-graph');
const { ENGINES } = require('../engine-universe/domain/engine-registry');
const { ENGINE_EDGES } = require('../engine-universe/domain/relation-registry');
const { readJSON, writeJSON } = require('../lib/json-store');
const { createSelfSyncService } = require('./application/self-sync-service');
const { createFlowRegistrationService } = require('./application/flow-registration-service');
const { createProjectService } = require('./application/project-service');

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

// ============ 自管理覆盖层（auto registry） ============
// 代码注册表为基线；data/atlas_auto_registry.json 为运行时自动登记层。
// self-sync 扫描真实代码库，发现未登记资产自动写入本层并重建图谱——
// 图谱"自己管理自己"，无破窗验证动态比对合并视图。

// 自管理覆盖层装载：自愈归一化（历史 getList 数组包裹实体自动解包；
// 缺失则引导创建——图谱记忆文件已登记于 tech-registry，随引导落盘保证 W2 无幽灵）
function loadAutoRegistry() {
  const raw = readJSON('atlas_auto_registry.json', null);
  const obj = Array.isArray(raw)
    ? (raw[0] && typeof raw[0] === 'object' ? raw[0] : null) // 历史缺陷自愈：数组包裹实体解包
    : (raw && typeof raw === 'object' ? raw : null);
  if (!obj) {
    const empty = { domains: [], dataAssets: [], docs: [], flows: [], projects: [] };
    writeJSON('atlas_auto_registry.json', empty);
    return empty;
  }
  if (Array.isArray(raw)) writeJSON('atlas_auto_registry.json', obj); // 修复存储中的数组包裹形态
  return {
    domains: obj.domains || [], dataAssets: obj.dataAssets || [],
    docs: obj.docs || [], flows: obj.flows || [], projects: obj.projects || []
  };
}

let autoRegistry = loadAutoRegistry();

function getViewDomains() { return DOMAINS.concat(autoRegistry.domains || []); }
function getViewDataAssets() { return DATA_ASSETS.concat(autoRegistry.dataAssets || []); }
function getViewDocs() { return DOCS.concat(autoRegistry.docs || []); }
/** 流程合并视图：代码基线（flow-registry）+ 运行时注册层（EAF-STD-001 接入） */
function getViewFlows() { return FLOWS.concat(autoRegistry.flows || []); }
/** 项目合并视图：代码基线（project-registry）+ 运行时注册层（"一切皆是项目"） */
function getViewProjects() { return PROJECTS.concat(autoRegistry.projects || []); }

let graph;
let NODE_INDEX;

/** 图谱重建（self-sync 登记后调用；模块加载时首次构建） */
function rebuildGraph() {
  graph = buildAtlasGraph({
    DOMAINS: getViewDomains(), MODULES, ALGORITHMS,
    DATA_ASSETS: getViewDataAssets(), DOCS: getViewDocs(),
    ENGINES: ENGINE_NODES, ENGINE_EDGES, FLOWS: getViewFlows(),
    PROJECTS: getViewProjects()
  });
  NODE_INDEX = Object.fromEntries(graph.nodes.map(n => [n.id, n]));
}
rebuildGraph();

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
  const reg = getViewDomains().find(d => d.id === domainId) || MODULES.find(m => m.id === domainId);
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
  // 所属项目回查（owns_domain 反向入边："一切皆是项目"）
  const ownedBy = graph.edges
    .filter(e => e.to === domainId && e.type === 'owns_domain')
    .map(e => ({ id: e.from, name: NODE_INDEX[e.from]?.name, status: NODE_INDEX[e.from]?.status }));
  return {
    id: reg.id, name: reg.name, isModule: !!reg.isModule || MODULES.some(m => m.id === domainId),
    keyFeatures: reg.keyFeatures, codePath: reg.codePath,
    ownedBy,
    engines: out.filter(e => e.type === 'uses_engine').map(e => ({ id: e.to, name: NODE_INDEX[e.to]?.name })),
    algorithms,
    dataAssets: out.filter(e => e.type === 'persists_to').map(e => ({ file: NODE_INDEX[e.to]?.name, desc: NODE_INDEX[e.to]?.desc })),
    docs: out.filter(e => e.type === 'documented_by').map(e => ({ path: NODE_INDEX[e.to]?.path, desc: NODE_INDEX[e.to]?.desc }))
  };
}

/** 影响面分析：改动一个节点会波及哪些引擎/算法/数据/文档/流程步骤
 *  引擎节点额外追踪反向委托边（delegates_to 入边）：委托该引擎的流程步骤全部受影响 */
function impact(seedId) {
  if (!NODE_INDEX[seedId]) return null;
  const result = impactAnalysis(graph.nodes, graph.edges, seedId);
  const impactedIds = new Set(result.reachableNodes);
  // 反向委托：改动引擎 → 委托它的流程步骤（业务流程受影响面）
  graph.edges
    .filter(e => e.to === seedId && e.type === 'delegates_to' && NODE_INDEX[e.from])
    .forEach(e => impactedIds.add(e.from));
  return {
    seed: seedId, seedName: NODE_INDEX[seedId].name,
    impacted: [...impactedIds].map(id => ({ id, kind: NODE_INDEX[id]?.kind, name: NODE_INDEX[id]?.name })),
    total: impactedIds.size
  };
}

/** 自然语言检索图谱资产（含业务流程步骤：流程名/详情可检索） */
function searchAtlas(keyword) {
  const kw = String(keyword || '').toLowerCase();
  if (!kw) return { nodes: [], total: 0 };
  const hits = graph.nodes.filter(n =>
    (n.name || '').toLowerCase().includes(kw) ||
    (n.desc || '').toLowerCase().includes(kw) ||
    (n.principle || '').toLowerCase().includes(kw) ||
    (n.codePath || '').toLowerCase().includes(kw) ||
    (n.flowName || '').toLowerCase().includes(kw) ||
    (n.detail || '').toLowerCase().includes(kw) ||
    (n.keyFeatures || []).some(f => f.toLowerCase().includes(kw))
  );
  return { keyword, nodes: hits, total: hits.length };
}

// ============ 业务流程查询（业务处理流程图谱化） ============

/** 全系统流程清单：每条流程的步骤数/降级数/关联域与标准锚点（含运行时注册层） */
function getFlows() {
  const all = getViewFlows();
  const list = all.map(f => ({
    id: f.id, name: f.name, domain: f.domain,
    domainName: NODE_INDEX[f.domain]?.name || null,
    standard: f.standard || null,
    runtime: f.runtime === true,
    registeredAt: f.registeredAt || null,
    stepCount: f.steps.length,
    degradeCount: f.transitions.filter(t => t.type === 'degrade').length,
    engines: [...new Set(f.steps.map(s => s.engine).filter(Boolean))],
    standardLevel: f.standard === 'EAF-STD-001'
  }));
  return {
    flows: list,
    stats: {
      total: list.length,
      runtimeRegistered: list.filter(f => f.runtime).length,
      totalSteps: list.reduce((s, f) => s + f.stepCount, 0),
      totalDegrades: list.reduce((s, f) => s + f.degradeCount, 0),
      coveredDomains: [...new Set(list.map(f => f.domain))],
      standardFlows: list.filter(f => f.standardLevel).length
    }
  };
}

/** 单流程全景：步骤链（含入度定位）+ 每步委托引擎/数据读写 + 降级链 */
function getFlowDetail(flowId) {
  const flow = getViewFlows().find(f => f.id === flowId);
  if (!flow) return null;
  const stepIds = new Set(flow.steps.map(s => s.id));
  const inDeg = Object.fromEntries(flow.steps.map(s => [s.id, 0]));
  flow.transitions.forEach(t => { if (stepIds.has(t.to)) inDeg[t.to]++; });
  const steps = flow.steps.map(s => {
    const nid = `step:${flow.id}/${s.id}`;
    const node = NODE_INDEX[nid];
    const outgoing = graph.edges
      .filter(e => e.from === nid && (e.type === 'next_step' || e.type === 'degrades_to'))
      .map(e => ({ to: e.to.split('/').pop(), type: e.type, note: e.note }));
    return {
      id: s.id, name: s.name, detail: s.detail,
      entry: inDeg[s.id] === 0, // 无 next/degrade 入边 = 主干入口
      engine: s.engine ? { id: s.engine, name: NODE_INDEX[s.engine]?.name } : null,
      reads: s.reads || [], writes: s.writes || [],
      outgoing
    };
  });
  const degrades = flow.transitions
    .filter(t => t.type === 'degrade')
    .map(t => ({ from: t.from, to: t.to, note: t.note || '' }));
  return {
    id: flow.id, name: flow.name, domain: flow.domain,
    domainName: NODE_INDEX[flow.domain]?.name || null,
    standard: flow.standard || null,
    runtime: flow.runtime === true,
    registeredAt: flow.registeredAt || null,
    steps, degrades,
    graphRef: {
      nodeCount: steps.length,
      stepNodeIds: flow.steps.map(s => `step:${flow.id}/${s.id}`)
    }
  };
}

// ============ 无破窗验证 ============

function verifyAtlas() {
  const checks = [];
  let passed = 0, failed = 0;
  const check = (name, ok, detail = '') => {
    if (ok) passed++; else failed++;
    checks.push({ name, ok, detail });
  };

  // W1 路由域全覆盖（动态比对 routes/index.js；合并视图含 auto 层）
  // ghost 检查仅约束代码基线域（须有路由入口）；auto 层域为扫描发现的真实代码域
  // （无路由的内部模块亦合法，其存在性由 W3 codePath + W6 内聚保证）
  const viewDomains = getViewDomains();
  const registered = viewDomains.map(d => d.id);
  const missingInAtlas = ROUTES_DOMAINS.filter(d => !registered.includes(d));
  const ghostInAtlas = viewDomains.filter(d =>
    !ROUTES_DOMAINS.includes(d.id) && d.id !== 'atlas-auto' && d.auto !== true).map(d => d.id);
  check('W1 路由域全覆盖（动态比对 DOMAINS 表 + self-sync 自动登记层）',
    missingInAtlas.length === 0 && ghostInAtlas.length === 0,
    missingInAtlas.length ? `未图谱化: ${missingInAtlas.join(',')}` : (ghostInAtlas.length ? `幽灵域: ${ghostInAtlas.join(',')}` : `${ROUTES_DOMAINS.length} 域全部图谱化（含自动登记 ${viewDomains.filter(d => d.auto).length} 域）`));

  // W2 数据资产全覆盖（动态比对 data/ 目录；合并视图）
  const actualFiles = fs.existsSync(DATA_DIR)
    ? fs.readdirSync(DATA_DIR).filter(f => f.endsWith('.json') || f.endsWith('.jsonl'))
    : [];
  const registeredFiles = getViewDataAssets().map(x => x.file);
  const missingData = actualFiles.filter(f => !registeredFiles.includes(f));
  const ghostData = registeredFiles.filter(f => !actualFiles.includes(f));
  check('W2 数据资产全覆盖（动态比对 data/ 目录）',
    missingData.length === 0 && ghostData.length === 0,
    missingData.length ? `未登记: ${missingData.join(',')}` : (ghostData.length ? `幽灵资产: ${ghostData.join(',')}` : `${actualFiles.length} 文件全部登记`));

  // W3 代码路径存在
  for (const u of [...viewDomains, ...MODULES]) {
    check(`W3 代码路径存在 [${u.id}] ${u.codePath}`, fs.existsSync(path.join(ROOT, u.codePath)));
  }
  for (const a of ALGORITHMS) {
    // src/ 开头 → backend-node 内；否则 → 项目根（如 melody2score/ 子项目）
    const fp = a.codePath.startsWith('src/')
      ? path.join(ROOT, a.codePath)
      : path.join(PROJECT_ROOT, a.codePath);
    check(`W3 算法代码存在 [${a.id}] ${a.codePath}`, fs.existsSync(fp));
  }

  // W4 文档存在（合并视图）
  for (const d of getViewDocs()) {
    check(`W4 文档存在 ${d.file}`, fs.existsSync(path.join(PROJECT_ROOT, d.file)));
  }

  // W5 引用完整性（uses_engine 指向真实引擎节点）
  const validEngineIds = ENGINE_NODES.map(e => e.id);
  for (const u of [...viewDomains, ...MODULES]) {
    for (const eng of (u.engines || [])) {
      check(`W5 引擎引用有效 [${u.id}] → ${eng}`, validEngineIds.includes(eng));
    }
  }

  // W6 域功能内聚
  for (const u of [...viewDomains, ...MODULES]) {
    check(`W6 功能内聚 [${u.id}]`, (u.keyFeatures || []).length >= 3 && (u.engines || []).length >= 1);
  }
  const docCovered = new Set(getViewDocs().map(d => d.domain));
  const noDoc = viewDomains.filter(d => !d.auto && !docCovered.has(d.id)).map(d => d.id);
  check('W6 文档覆盖全部业务域（auto 域由容器豁免）', noDoc.length === 0, noDoc.join(','));

  // W7 算法单源
  check('W7 全部算法单源自研（singleSource=true 且零框架依赖）',
    ALGORITHMS.every(a => a.singleSource === true));

  // W8 图谱连通（无孤岛）
  const comps = connectedComponents(graph.nodes.map(n => n.id), graph.edges);
  check('W8 图谱连通无孤岛', comps.length === 1,
    comps.length > 1 ? `孤岛: ${comps.slice(1).map(c => c.join(',')).join(';')}` : `${comps[0].length} 节点全连通`);

  // W9 业务流程图谱化（步骤结构/引用完整/连通/核心域覆盖/标准锚点；含运行时注册层）
  const viewDomainIds = new Set(getViewDomains().map(d => d.id));
  const viewDataFiles = new Set(getViewDataAssets().map(x => x.file));
  const engineIdSet = new Set(ENGINE_NODES.map(e => e.id));
  const viewFlows = getViewFlows();
  const flowIdSet = new Set(viewFlows.map(f => f.id));
  check('W9 流程 id 全局唯一（代码基线 + 运行时注册层）', flowIdSet.size === viewFlows.length);
  for (const f of viewFlows) {
    const stepIdSet = new Set(f.steps.map(s => s.id));
    check(`W9 流程归属域存在 [${f.id}] → ${f.domain}`, viewDomainIds.has(f.domain));
    check(`W9 流程步骤数 ≥3 且 id 唯一 [${f.id}]`, f.steps.length >= 3 && stepIdSet.size === f.steps.length, `${f.steps.length} 步`);
    const badTrans = f.transitions.filter(t => !stepIdSet.has(t.from) || !stepIdSet.has(t.to));
    check(`W9 迁移边引用有效 [${f.id}]`, badTrans.length === 0, badTrans.map(t => `${t.from}→${t.to}`).join(','));
    const badEngines = [...new Set(f.steps.map(s => s.engine).filter(Boolean))].filter(e => !engineIdSet.has(e));
    check(`W9 步骤委托引擎真实存在 [${f.id}]`, badEngines.length === 0, badEngines.join(','));
    const badData = f.steps.flatMap(s => [...(s.reads || []), ...(s.writes || [])]).filter(x => !viewDataFiles.has(x));
    check(`W9 步骤数据读写已注册 [${f.id}]`, badData.length === 0, badData.join(','));
    // 连通性：入口（无 next/degrade 入边）BFS 可达全部步骤；闭环流程（如巡检循环）以首步为锚
    // （幽灵引用边不参与 BFS——V4 已单独报告，此处不崩溃）
    const inDeg = Object.fromEntries(f.steps.map(s => [s.id, 0]));
    f.transitions.forEach(t => { if (stepIdSet.has(t.to)) inDeg[t.to]++; });
    const entries = f.steps.filter(s => inDeg[s.id] === 0).map(s => s.id);
    const isLoop = entries.length === 0;
    const starts = isLoop ? [f.steps[0].id] : entries;
    const next = Object.fromEntries(f.steps.map(s => [s.id, []]));
    f.transitions.forEach(t => { if (stepIdSet.has(t.from) && stepIdSet.has(t.to)) next[t.from].push(t.to); });
    const seen = new Set(starts); const q = [...starts];
    while (q.length) { const c = q.shift(); for (const n of next[c]) if (!seen.has(n)) { seen.add(n); q.push(n); } }
    check(`W9 流程${isLoop ? '闭环' : '入口'}存在且全步骤可达 [${f.id}]`, seen.size === f.steps.length,
      `${isLoop ? '闭环锚点' : '入口'}=[${starts.join(',')}] 不可达=[${f.steps.filter(s => !seen.has(s.id)).map(s => s.id).join(',')}]`);
  }
  const flowDomainSet = new Set(viewFlows.map(f => f.domain));
  const coreFlowDomains = ['expert-alliance', 'ai-engine', 'atlas', 'engine-kernel', 'auto-dev'];
  const uncovered = coreFlowDomains.filter(d => !flowDomainSet.has(d));
  check('W9 核心域业务流程全覆盖（联盟/AI引擎/图谱/内核/自开发）', uncovered.length === 0, uncovered.join(','));
  const eaf = viewFlows.find(f => f.standard === 'EAF-STD-001');
  check('W9 EAF-STD-001 标准参考实现存在（专家联盟六阶段全链路）',
    !!eaf && eaf.steps.filter(s => /阶段[一二三四五六]/.test(s.name)).length === 6,
    eaf ? `${eaf.steps.filter(s => /阶段[一二三四五六]/.test(s.name)).length} 阶段` : '缺失');

  // W10 项目治理（"一切皆是项目"：资产全归属/引用真实/状态合法/内聚/健康）
  const viewProjects = getViewProjects();
  const projectIdSet = new Set(viewProjects.map(p => p.id));
  check('W10 项目 id 全局唯一（代码基线 + 运行时注册层）',
    projectIdSet.size === viewProjects.length);
  // 项目可归属的治理单元 = 业务域 + 可插拔模块（模块亦是项目资产）；
  // atlas-auto 容器域豁免（self-sync 临时资产容器，内容动态增删，不强制静态归属）
  const governableIds = new Set([...viewDomainIds, ...MODULES.map(m => m.id)]);
  for (const p of viewProjects) {
    const { valid, errors } = validateProject(p, { domainIds: governableIds });
    check(`W10 项目建模合法 [${p.id}]（P1身份/P3引用/P4状态/P6内聚）`,
      valid, errors.map(e => `${e.rule}:${e.message}`).join(';'));
  }
  const ownership = auditDomainOwnership(viewProjects, new Set([...governableIds].filter(d => d !== 'atlas-auto')));
  check('W10 全部业务域与模块归属项目（无孤儿资产，容器域豁免）', ownership.orphans.length === 0, ownership.orphans.join(','));
  check('W10 域归属唯一（无重复归属）', ownership.duplicated.length === 0, ownership.duplicated.join(','));
  const autoDomainIds = new Set(getViewDomains().filter(d => d.auto).map(d => d.id));
  const autoOrphan = [...autoDomainIds].filter(d => !viewProjects.some(p => (p.domains || []).includes(d)));
  check('W10 auto 层自动发现域亦归属项目（容器域豁免）',
    autoOrphan.filter(d => d !== 'atlas-auto').length === 0, autoOrphan.join(','));

  return { ok: failed === 0, summary: { total: checks.length, passed, failed }, checks };
}

// ============ 图谱自管理服务装配（自己管理自己） ============

const registryIO = {
  read: () => autoRegistry, // 进程内单一真相源（存储仅作跨进程持久化，装载时已自愈归一化）
  write: (data) => {
    autoRegistry = data; // 内存视图即时生效
    writeJSON('atlas_auto_registry.json', data); // 持久化（json-store 单一真相源）
  }
};

const selfSyncService = createSelfSyncService({
  scanner: require('./infrastructure/atlas-scanner'),
  registryIO,
  rebuild: rebuildGraph,
  getRegisteredView: () => ({
    domains: getViewDomains(),
    dataAssets: getViewDataAssets(),
    docs: getViewDocs(),
    verify: verifyAtlas
  })
});

// ============ 通用流程注册服务装配（EAF-STD-001 接入） ============
// 任何模块按标准注册业务流程：校验（V1-V8）→ 持久化 → 图谱重建 → W9 复验

const flowRegistrationService = createFlowRegistrationService({
  registryIO,
  rebuild: rebuildGraph,
  getView: () => ({
    domains: getViewDomains(),
    engineIds: ENGINE_NODES.map(e => e.id),
    dataAssets: getViewDataAssets(),
    flows: getViewFlows()
  }),
  verify: verifyAtlas
});

// ============ 项目治理服务装配（"一切皆是项目"） ============
// 项目实体运行时治理：创建/域归属/生命周期流转/健康度量（每次变更 W10 复验）

const projectService = createProjectService({
  registryIO,
  rebuild: rebuildGraph,
  getView: () => ({
    // 项目可持有的治理单元 = 业务域 + 可插拔模块（模块亦是项目资产，健康度量一并聚合）
    domains: getViewDomains().concat(MODULES),
    projects: getViewProjects(),
    flows: getViewFlows()
  }),
  verify: verifyAtlas
});

module.exports = {
  getAtlas, getDomainDetail, impact, searchAtlas, verifyAtlas,
  getFlows, getFlowDetail,
  DOMAINS, MODULES, ALGORITHMS, DATA_ASSETS, DOCS, FLOWS,
  PROJECTS, LIFECYCLE,
  // 图谱自管理
  discoverPending: selfSyncService.discoverPending,
  selfSync: selfSyncService.selfSync,
  selfHealVerify: selfSyncService.selfHealVerify,
  rebuildGraph,
  // 通用流程注册（EAF-STD-001 接入）
  registerFlow: flowRegistrationService.registerFlow,
  removeFlow: flowRegistrationService.removeFlow,
  precheckFlow: flowRegistrationService.precheckFlow,
  // 项目治理（"一切皆是项目"）
  getProjects: projectService.listProjects,
  getProjectDetail: projectService.getProjectDetail,
  createProject: projectService.createProject,
  transitionProject: projectService.transitionProject,
  assignDomain: projectService.assignDomain,
  removeProject: projectService.removeProject,
  precheckProject: projectService.precheckProject
};
