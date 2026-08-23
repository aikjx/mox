'use strict';

/**
 * 项目注册表与项目治理规则（domain 层 · 纯函数零 IO）
 * ------------------------------------------------------------------
 * "一切皆是项目"的实体承载：项目 = 聚合业务域的顶层治理单元。
 * 每个项目是一个独立小项目（愿景/生命周期/资产聚合/健康度量），
 * 域/模块/引擎/算法/数据/文档通过域归属间接挂载到项目。
 *
 * 生命周期状态机（单向流转，archived 为终态）：
 *   planning → building → delivered → maintaining → archived
 *
 * 治理不变式（W10 检查族）：
 *   P1 项目身份：id 格式合法（proj- 前缀）、name 必填
 *   P2 域归属唯一：每个业务域恰好归属一个项目（无孤儿、无重复归属）
 *   P3 引用真实：项目声明的域必须存在于图谱
 *   P4 状态合法：status 属于状态机合法集
 *   P5 流转合法：生命周期只能沿状态机正向流转（不可逆）
 *   P6 项目内聚：每个项目 ≥2 个域（单域不成项目，直接挂图谱）
 */

const PROJECT_ID_RE = /^proj-[a-z][a-z0-9-]*$/;

/** 生命周期状态机：合法状态 + 合法流转边 */
const LIFECYCLE = {
  states: ['planning', 'building', 'delivered', 'maintaining', 'archived'],
  transitions: [
    { from: 'planning', to: 'building' },
    { from: 'building', to: 'delivered' },
    { from: 'delivered', to: 'maintaining' },
    { from: 'maintaining', to: 'archived' },
    { from: 'delivered', to: 'archived' }
  ]
};

/**
 * 基线项目注册表（代码层不可变基线；运行时项目进 auto 覆盖层 projects 键）
 * 每条登记：身份 / 愿景 / 生命周期状态 / 归属域清单。
 */
const PROJECTS = [
  {
    id: 'proj-xuanji-core', name: '璇玑核心平台', status: 'maintaining',
    vision: '系统底座：运行状态、服务管理、安全审计、模块装载与存储基础设施 + Rust 璇玑系统 / PrimiFlow 双核心',
    domains: ['system', 'services', 'internal', 'security', 'modules-admin', 'mod-storage', 'studio',
      'domain-rust-xuanji-system', 'domain-rust-primiflow-core', 'domain-rust-primiflow-fusion',
      'rust::xuanji-system', 'rust::primiflow-core', 'rust::primiflow-fusion', 'rust::xuanji-common-meta']
  },
  {
    id: 'proj-knowledge', name: '知识图谱与知识库', status: 'maintaining',
    vision: '图谱节点/边/算法分析与结构化知识管理，知识资产单源承载（含 Rust KG Hub）',
    domains: ['graph', 'mod-graph', 'kb', 'domain-rust-kg-hub', 'rust::kg-hub']
  },
  {
    id: 'proj-ai-dialogue', name: 'AI 对话协作', status: 'delivered',
    vision: '多会话对话、联网搜索增强与多引擎编排协作（含 Rust AI Agent）',
    domains: ['chat', 'web-search', 'orchestration', 'domain-rust-ai-agent', 'rust::ai-agent']
  },
  {
    id: 'proj-expert-alliance', name: '专家联盟', status: 'maintaining',
    vision: '多专家协同咨询：意图识别、最优组队、辩论综合、质量门禁全链路（EAF-STD-001），MCP 协议标准对外暴露 + Rust 璇玑专家',
    domains: ['expert-alliance', 'expert-graph', 'mcp', 'domain-rust-xuanji-expert', 'rust::xuanji-expert']
  },
  {
    id: 'proj-ai-engine', name: 'AI 引擎编排', status: 'maintaining',
    vision: 'AI 引擎统一编排核心：意图路由、能力矩阵、多引擎集成与终极推理 + Rust 图算法',
    domains: ['ai-engine', 'ai-integrated', 'ai-ultimate', 'ai-enhanced', 'integration',
      'domain-rust-graph-algorithms', 'rust::graph-algorithms']
  },
  {
    id: 'proj-ai-platform', name: 'AI 平台生态', status: 'delivered',
    vision: '工作流/算子/资源池、智能体市场与任务自动化生态',
    domains: ['ai-platform', 'browser-market', 'tasks', 'auto-tasks', 'mod-task']
  },
  {
    id: 'proj-xuanji-platform', name: '璇玑平台运行时', status: 'maintaining',
    vision: '璇玑平台网关运行时（Cordis 插件内核 + HITL 审批 + 路由治理）——Rust 单项目承载',
    domains: ['atlas', 'domain-rust-runtime', 'rust::runtime']
  },
  {
    id: 'proj-auto-dev', name: '自动开发引擎', status: 'building',
    vision: '自开发闭环：LLM 生成架构图谱、确定性渲染、无穷维度寻优与制品管理 + Rust 4 模块桥接',
    domains: ['auto-dev', 'artifacts', 'optimizer', 'mod-melody2score',
      'domain-rust-hermes-flow-bridge', 'domain-rust-business-catalog', 'domain-rust-template-market', 'domain-rust-operator-wasm',
      'mod-rust-hermes-flow-bridge', 'mod-rust-business-catalog', 'mod-rust-template-market', 'mod-rust-operator-wasm',
      'rust::hermes-flow-bridge', 'rust::business-catalog', 'rust::template-market', 'rust::operator-wasm']
  },
  {
    id: 'proj-graph-infra', name: '图谱基础设施', status: 'building',
    vision: '全机器图谱化底座：引擎宇宙、引擎内核插槽位体系 + Rust 算子核心/优化器/FlowAI 三件套（项目中心/全息图谱属平台运行时）',
    domains: ['engine-universe', 'engine-kernel', 'projects',
      'domain-rust-operator-core', 'domain-rust-optimizer', 'domain-rust-flow-ai',
      'rust::operator-core', 'rust::optimizer', 'rust::flow-ai']
  }
];

/** 校验流转合法性：from → to 是否为状态机合法边 */
function canTransition(from, to) {
  return LIFECYCLE.transitions.some(t => t.from === from && t.to === to);
}

/**
 * 校验项目定义（注册前 + W10 复验共用）。
 * @param {object} project {id,name,vision?,status,domains[]}
 * @param {object} ctx {domainIds:Set, projectIds:Set, allowOverwrite?:boolean}
 * @returns {{valid:boolean, errors:[{rule,message}]}}
 */
function validateProject(project, ctx) {
  const errors = [];
  const err = (rule, message) => errors.push({ rule, message });
  const { domainIds, projectIds, allowOverwrite = false } = ctx || {};

  if (!project || typeof project !== 'object') {
    return { valid: false, errors: [{ rule: 'P1', message: '项目必须为对象' }] };
  }

  // P1 项目身份
  if (!project.id || typeof project.id !== 'string') err('P1', '项目 id 为必填字符串');
  else if (!PROJECT_ID_RE.test(project.id)) err('P1', `项目 id 须匹配 ${PROJECT_ID_RE}: ${project.id}`);
  else if (projectIds && projectIds.has(project.id) && !allowOverwrite) err('P1', `项目 id 已存在: ${project.id}`);
  if (!project.name || typeof project.name !== 'string') err('P1', '项目 name 为必填字符串');

  // P4 状态合法
  if (!LIFECYCLE.states.includes(project.status)) {
    err('P4', `项目状态非法（合法集 ${LIFECYCLE.states.join('|')}）: ${project.status}`);
  }

  // P3 引用真实 + P6 项目内聚
  const domains = Array.isArray(project.domains) ? project.domains : null;
  if (!domains) err('P3', '项目 domains 为必填数组');
  else {
    if (domains.length < 2) err('P6', `项目域数不足（${domains.length} < 2，单域不成项目）`);
    const seen = new Set();
    domains.forEach(d => {
      if (typeof d !== 'string') err('P3', `域引用须为字符串: ${JSON.stringify(d)}`);
      else {
        if (domainIds && !domainIds.has(d)) err('P3', `项目引用的域不存在于图谱: ${d}`);
        if (seen.has(d)) err('P3', `项目内域引用重复: ${d}`);
        seen.add(d);
      }
    });
  }

  return { valid: errors.length === 0, errors };
}

/**
 * 域归属唯一性审计（P2）：全部域必须恰好归属一个项目。
 * @param {Array} projects 项目清单（合并视图）
 * @param {Set<string>} domainIds 图谱全部业务域 id
 * @returns {{orphans:string[], duplicated:string[]}}
 */
function auditDomainOwnership(projects, domainIds) {
  const owner = new Map(); // domainId -> [projectId]
  for (const p of projects) {
    for (const d of (p.domains || [])) {
      if (!owner.has(d)) owner.set(d, []);
      owner.get(d).push(p.id);
    }
  }
  const orphans = [...domainIds].filter(d => !owner.has(d));
  const duplicated = [...owner.entries()].filter(([, ps]) => ps.length > 1).map(([d, ps]) => `${d}@${ps.join(',')}`);
  return { orphans, duplicated };
}

/**
 * 项目健康度量：聚合归属域的资产规模与验证状态。
 * @param {object} project 项目实体
 * @param {object} view 合并视图 {domains, engineIds, dataAssets, docs, flows}
 * @param {object} verifyState W1-W9 验证结果（可选）
 */
function projectHealth(project, view, verifyState) {
  const domainById = new Map(view.domains.map(d => [d.id, d]));
  const owned = (project.domains || []).map(id => domainById.get(id)).filter(Boolean);
  const engines = [...new Set(owned.flatMap(d => d.engines || []))];
  const dataFiles = [...new Set(owned.flatMap(d => d.dataAssets || []))];
  const docFiles = [...new Set(owned.flatMap(d => d.docs || []))];
  const flows = view.flows.filter(f => (project.domains || []).includes(f.domain));
  const algoByDomain = owned.length; // 算法经引擎关联，域数作代理维度

  return {
    projectId: project.id,
    domainCount: owned.length,
    engineCount: engines.length,
    dataCount: dataFiles.length,
    docCount: docFiles.length,
    flowCount: flows.length,
    degradeCount: flows.reduce((s, f) => s + (f.transitions || []).filter(t => t.type === 'degrade').length, 0),
    verification: verifyState ? { ok: verifyState.ok, total: verifyState.summary.total, failed: verifyState.summary.failed } : null,
    score: computeScore({ owned, engines, dataFiles, docFiles, flows, verifyState })
  };
}

/** 健康分（0-100）：资产覆盖 60 分 + 验证全绿 40 分 */
function computeScore({ owned, engines, dataFiles, docFiles, flows, verifyState }) {
  let score = 0;
  if (owned.length >= 2) score += 15;
  if (engines.length >= 2) score += 15;
  if (dataFiles.length >= 1) score += 10;
  if (docFiles.length >= 1) score += 10;
  if (flows.length >= 1) score += 10;
  if (verifyState && verifyState.ok) score += 40;
  return score;
}

module.exports = {
  PROJECTS, LIFECYCLE, PROJECT_ID_RE,
  canTransition, validateProject, auditDomainOwnership, projectHealth
};
