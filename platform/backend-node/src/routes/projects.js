'use strict';

/**
 * 路由域：项目中心（全维项目化归类）
 *
 * 设计定位：系统内一切可管理对象（平台模块、MCP 工具、插件、智能体、技能、
 * 循环体、自动化、工作流、专家、图谱、任务、算子、管线、知识库、大模型、
 * 服务、商城方案……）均可挂载到"项目"维度统一归档。
 *
 * /projects            项目 CRUD
 * /projects/types      项目类别 + 资源类型注册表（全维归类单一真相源）
 * /projects/catalog    全维资源目录（实时聚合各业务域）
 * /projects/stats      项目化统计总览
 * /projects/:id/resources  项目-资源绑定管理
 */

// ===== 全维类型注册表（单一真相源）=====

// 项目类别：项目本身的形态（平台系统 / MCP / 插件 / APP / PC / Skills / Loop / Graph Agents / 自动化…）
const PROJECT_CATEGORIES = [
  { key: 'platform', label: '平台系统', color: '#4f46e5', icon: 'Odometer' },
  { key: 'mcp', label: 'MCP 服务', color: '#8b5cf6', icon: 'Link' },
  { key: 'plugin', label: '插件', color: '#a855f7', icon: 'Connection' },
  { key: 'app', label: 'APP', color: '#ec4899', icon: 'Iphone' },
  { key: 'pc', label: 'PC 客户端', color: '#0ea5e9', icon: 'Monitor' },
  { key: 'skill', label: 'Skills 技能包', color: '#10b981', icon: 'MagicStick' },
  { key: 'loop', label: 'Loop 循环体', color: '#f59e0b', icon: 'Refresh' },
  { key: 'graph_agent', label: 'Graph Agents', color: '#06b6d4', icon: 'Share' },
  { key: 'automation', label: '自动化', color: '#f97316', icon: 'Lightning' },
  { key: 'workflow', label: '工作流', color: '#d97706', icon: 'Operation' },
  { key: 'custom', label: '自定义', color: '#64748b', icon: 'Folder' }
];

// 资源类型：可被项目归档的全维资源清单（label + 前端跳转路由）
const RESOURCE_TYPES = [
  { key: 'module', label: '平台模块', route: '/docs' },
  { key: 'mcp_tool', label: 'MCP 工具', route: '/mcp' },
  { key: 'plugin', label: 'AI 插件', route: '/plugins' },
  { key: 'agent', label: '图谱智能体', route: '/botCenter' },
  { key: 'skill', label: '技能', route: '/expert-center' },
  { key: 'loop', label: '循环体', route: '/automation' },
  { key: 'automation', label: '自动化流程', route: '/automation' },
  { key: 'workflow', label: '工作流', route: '/workflow' },
  { key: 'flow', label: '流程图', route: '/xuanji-fusion' },
  { key: 'expert', label: '专家', route: '/expert-center' },
  { key: 'graph_node', label: '图谱节点', route: '/graph' },
  { key: 'task', label: '任务', route: '/tasks' },
  { key: 'operator', label: '算子', route: '/operators' },
  { key: 'pipeline', label: '管线', route: '/expert-orchestrator' },
  { key: 'kb_doc', label: '知识库文档', route: '/knowledge-base' },
  { key: 'llm', label: '大模型', route: '/llm-config' },
  { key: 'service', label: '服务', route: '/monitor' },
  { key: 'market', label: '商城方案', route: '/market' }
];

module.exports = function registerProjectsRoutes(ctx) {
  const { uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg, serviceManager } = ctx;

  const TYPE_MAP = new Map(RESOURCE_TYPES.map((t) => [t.key, t]));
  const CATEGORY_MAP = new Map(PROJECT_CATEGORIES.map((c) => [c.key, c]));

  // ===== 全维资源目录采集（实时聚合各业务域存储）=====
  // 统一输出：{ type, id, name, desc, status, meta }
  function buildCatalog() {
    const out = [];

    // 1. 平台模块：28 个业务域（来自路由装配清单）
    try {
      const { DOMAINS } = require('./index');
      for (const [file, name] of DOMAINS) {
        out.push({ type: 'module', id: file, name, desc: `业务域 ${file}`, status: 'online', meta: {} });
      }
    } catch { /* 装配清单不可用时跳过 */ }

    // 2. MCP 工具：专家联盟 MCP 编排器对外暴露的标准工具
    try {
      const { TOOLS } = require('../mcp');
      for (const t of TOOLS) {
        out.push({ type: 'mcp_tool', id: t.name, name: t.name, desc: t.description || '', status: 'online', meta: {} });
      }
    } catch { /* MCP 不可用时跳过 */ }

    // 3~N. 各 JSON 存储域
    const pick = (type, file, map) => {
      let arr = [];
      try { arr = readJSON(file, []); } catch { return; }
      if (!Array.isArray(arr)) return;
      for (const item of arr) {
        const r = map(item);
        if (r) out.push({ type, ...r, meta: r.meta || {} });
      }
    };

    pick('plugin', 'plugins.json', (x) => ({ id: x.id, name: x.name, desc: x.desc || '', status: x.status || 'active' }));
    pick('agent', 'registered_agents.json', (x) => ({ id: x.id, name: x.name, desc: x.role || '', status: x.status || 'active', meta: { capabilities: x.capabilities } }));
    pick('skill', 'learned_skills.json', (x) => ({ id: x.id, name: x.name || x.key, desc: x.desc || x.question_brief || '', status: 'active' }));
    pick('skill', 'alliance_learned_skills.json', (x) => ({ id: x.id, name: x.name || x.key, desc: `意图:${x.intent || '-'} · 置信:${x.confidence ?? '-'}`, status: 'active', meta: { intent: x.intent, confidence: x.confidence } }));
    pick('automation', 'automation.json', (x) => ({ id: x.id, name: x.name, desc: (x.requirement || '').slice(0, 80), status: x.status || 'idle', meta: { has_flow: !!x.flow } }));
    // 循环体：自动化中处于运行态的持续执行单元
    pick('loop', 'automation.json', (x) => (x.status === 'running'
      ? { id: x.id, name: x.name, desc: `循环执行中 · ${(x.requirement || '').slice(0, 60)}`, status: 'running', meta: { has_flow: !!x.flow } }
      : null));
    pick('workflow', 'workflows.json', (x) => ({ id: x.id, name: x.name, desc: x.desc || '', status: x.status || 'draft', meta: { steps: (x.steps || []).length } }));
    pick('flow', 'flows.json', (x) => ({ id: x.id, name: x.name, desc: x.desc || '', status: x.valid ? 'valid' : 'invalid', meta: { nodes: (x.nodes || []).length, edges: (x.edges || []).length } }));
    pick('expert', 'experts.json', (x) => ({ id: x.id, name: x.name, desc: x.description || '', status: x.status || 'active', meta: { type: x.type } }));
    pick('graph_node', 'graph_nodes.json', (x) => ({ id: x.id, name: x.name || x.label || x.id, desc: x.type || '', status: 'active', meta: { node_type: x.type } }));
    pick('task', 'tasks.json', (x) => ({ id: x.id, name: x.title || x.id, desc: (x.description || '').slice(0, 80), status: x.status || 'todo', meta: { priority: x.priority } }));
    pick('operator', 'operators.json', (x) => ({ id: x.id, name: x.name, desc: x.desc || '', status: x.status || 'active', meta: { category: x.category } }));
    pick('pipeline', 'registered_pipelines.json', (x) => ({ id: x.id, name: x.name, desc: x.description || '', status: x.status || 'active', meta: { stages: (x.stages || []).length } }));
    pick('kb_doc', 'kb_documents.json', (x) => ({ id: x.id, name: x.title || x.name || x.id, desc: (x.summary || x.content || '').slice(0, 80), status: x.status || 'active', meta: { category: x.category } }));
    pick('llm', 'llm_config.json', (x) => ({ id: x.id, name: x.name, desc: `${x.provider || ''} · ${x.model || ''}`, status: x.enabled ? 'active' : 'disabled', meta: { provider: x.provider, model: x.model } }));
    pick('market', 'market.json', (x) => ({ id: x.id, name: x.name, desc: (x.description || '').slice(0, 80), status: 'published', meta: { downloads: x.downloads } }));

    // 服务：服务管理器运行态（processes 为 Map: id → {pid}，定义见 service-manager）
    try {
      const running = new Set((serviceManager && serviceManager.processes && serviceManager.processes.keys()) || []);
      const defs = [
        { id: 'api', name: 'API 网关服务', desc: '主 API 网关，端口 3002' },
        { id: 'frontend', name: '前端静态服务', desc: '前端静态托管，端口 3000' }
      ];
      for (const s of defs) {
        out.push({ type: 'service', id: s.id, name: s.name, desc: s.desc, status: running.has(s.id) ? 'running' : 'stopped', meta: {} });
      }
    } catch { /* 服务管理器不可用时跳过 */ }

    return out;
  }

  // ===== 项目 CRUD =====

  reg('get', '/projects', (req, res) => {
    const projects = readJSON('projects.json', []);
    const enriched = projects.map((p) => ({
      ...p,
      resource_count: (p.resources || []).length,
      resource_types: [...new Set((p.resources || []).map((r) => r.resource_type))]
    }));
    ok(res, enriched);
  });

  reg('get', '/projects/types', (req, res) => {
    ok(res, { categories: PROJECT_CATEGORIES, resource_types: RESOURCE_TYPES });
  });

  reg('get', '/projects/catalog', (req, res) => {
    const items = buildCatalog();
    const grouped = {};
    for (const it of items) {
      (grouped[it.type] = grouped[it.type] || []).push(it);
    }
    // 附带每个类型的中文名与路由，供前端直接渲染
    const groups = Object.entries(grouped).map(([type, list]) => ({
      type,
      label: (TYPE_MAP.get(type) || {}).label || type,
      route: (TYPE_MAP.get(type) || {}).route || '',
      count: list.length,
      items: list
    }));
    groups.sort((a, b) => b.count - a.count);
    ok(res, { total: items.length, groups });
  });

  reg('get', '/projects/stats', (req, res) => {
    const projects = readJSON('projects.json', []);
    const catalog = buildCatalog();
    const byCategory = {};
    for (const p of projects) byCategory[p.category] = (byCategory[p.category] || 0) + 1;
    const catalogByType = {};
    for (const it of catalog) catalogByType[it.type] = (catalogByType[it.type] || 0) + 1;
    const bound = projects.reduce((n, p) => n + (p.resources || []).length, 0);
    ok(res, {
      total: projects.length,
      active: projects.filter((p) => (p.status || 'active') === 'active').length,
      archived: projects.filter((p) => p.status === 'archived').length,
      by_category: byCategory,
      catalog_total: catalog.length,
      catalog_by_type: catalogByType,
      bound_resources: bound,
      categories: PROJECT_CATEGORIES,
      resource_types: RESOURCE_TYPES
    });
  });

  reg('post', '/projects', async (req, res) => {
    const body = await readBody(req);
    if (!body.name || !String(body.name).trim()) return fail(res, 400, '项目名称不能为空');
    const category = CATEGORY_MAP.has(body.category) ? body.category : 'custom';
    const projects = readJSON('projects.json', []);
    const project = {
      id: uid('proj'),
      name: String(body.name).trim(),
      description: body.description || '',
      category,
      tags: Array.isArray(body.tags) ? body.tags : [],
      status: body.status || 'active',
      owner: body.owner || '',
      color: body.color || (CATEGORY_MAP.get(category) || {}).color || '#64748b',
      resources: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    projects.unshift(project);
    writeJSON('projects.json', projects);
    appendLog({ type: 'project', msg: 'create', project_id: project.id, name: project.name });
    ok(res, project);
  });

  reg('get', '/projects/:id', (req, res, params) => {
    const projects = readJSON('projects.json', []);
    const project = projects.find((p) => p.id === params.id);
    if (!project) return fail(res, 404, '项目不存在');
    // 附带资源实时状态快照（目录中同 id 资源的最新状态）
    const catalog = buildCatalog();
    const live = new Map(catalog.map((c) => [`${c.type}:${c.id}`, c]));
    const resources = (project.resources || []).map((r) => {
      const cur = live.get(`${r.resource_type}:${r.resource_id}`);
      return { ...r, live_status: cur ? cur.status : 'missing', live_desc: cur ? cur.desc : '' };
    });
    ok(res, { ...project, resources, resource_count: resources.length });
  });

  reg('put', '/projects/:id', async (req, res, params) => {
    const body = await readBody(req);
    const projects = readJSON('projects.json', []);
    const idx = projects.findIndex((p) => p.id === params.id);
    if (idx < 0) return fail(res, 404, '项目不存在');
    const { id, resources, ...updatable } = body;
    if (body.category && !CATEGORY_MAP.has(body.category)) delete updatable.category;
    projects[idx] = { ...projects[idx], ...updatable, id: params.id, updated_at: new Date().toISOString() };
    writeJSON('projects.json', projects);
    ok(res, projects[idx]);
  });

  reg('delete', '/projects/:id', (req, res, params) => {
    const projects = readJSON('projects.json', []);
    const idx = projects.findIndex((p) => p.id === params.id);
    if (idx < 0) return fail(res, 404, '项目不存在');
    const [removed] = projects.splice(idx, 1);
    writeJSON('projects.json', projects);
    appendLog({ type: 'project', msg: 'delete', project_id: params.id, name: removed.name });
    ok(res, { deleted: true, id: params.id });
  });

  // ===== 项目-资源绑定 =====

  reg('post', '/projects/:id/resources', async (req, res, params) => {
    const body = await readBody(req);
    const items = Array.isArray(body.items) ? body.items : [body];
    const projects = readJSON('projects.json', []);
    const idx = projects.findIndex((p) => p.id === params.id);
    if (idx < 0) return fail(res, 404, '项目不存在');
    if (!items.length) return fail(res, 400, '缺少资源条目');
    const exist = new Set((projects[idx].resources || []).map((r) => `${r.resource_type}:${r.resource_id}`));
    let added = 0;
    for (const it of items) {
      if (!it || !it.type || !it.id) continue;
      if (!TYPE_MAP.has(it.type)) continue;
      if (exist.has(`${it.type}:${it.id}`)) continue;
      (projects[idx].resources = projects[idx].resources || []).push({
        rid: uid('res'),
        resource_type: it.type,
        resource_id: String(it.id),
        resource_name: it.name || String(it.id),
        note: it.note || '',
        added_at: new Date().toISOString()
      });
      added++;
    }
    projects[idx].updated_at = new Date().toISOString();
    writeJSON('projects.json', projects);
    appendLog({ type: 'project', msg: 'bind_resources', project_id: params.id, added });
    ok(res, { added, total: projects[idx].resources.length });
  });

  reg('delete', '/projects/:id/resources/:rid', (req, res, params) => {
    const projects = readJSON('projects.json', []);
    const idx = projects.findIndex((p) => p.id === params.id);
    if (idx < 0) return fail(res, 404, '项目不存在');
    const before = (projects[idx].resources || []).length;
    projects[idx].resources = (projects[idx].resources || []).filter((r) => r.rid !== params.rid);
    if (projects[idx].resources.length === before) return fail(res, 404, '资源绑定不存在');
    projects[idx].updated_at = new Date().toISOString();
    writeJSON('projects.json', projects);
    ok(res, { deleted: true, rid: params.rid });
  });

  reg('put', '/projects/:id/resources/:rid', async (req, res, params) => {
    const body = await readBody(req);
    const projects = readJSON('projects.json', []);
    const idx = projects.findIndex((p) => p.id === params.id);
    if (idx < 0) return fail(res, 404, '项目不存在');
    const r = (projects[idx].resources || []).find((x) => x.rid === params.rid);
    if (!r) return fail(res, 404, '资源绑定不存在');
    if (typeof body.note === 'string') r.note = body.note;
    projects[idx].updated_at = new Date().toISOString();
    writeJSON('projects.json', projects);
    ok(res, r);
  });

  // ===== 反查：某资源被哪些项目归档 =====

  reg('get', '/projects/by-resource', (req, res) => {
    // 框架仅注入路径参数，query 需自行解析（与其他域一致）
    const q = new URL(req.url, 'http://localhost').searchParams;
    const type = q.get('type');
    const id = q.get('id');
    if (!type || !id) return fail(res, 400, '缺少 type / id 参数');
    const projects = readJSON('projects.json', []);
    const hits = [];
    for (const p of projects) {
      for (const r of p.resources || []) {
        if (r.resource_type === type && r.resource_id === String(id)) {
          hits.push({ project_id: p.id, project_name: p.name, category: p.category, rid: r.rid, note: r.note });
        }
      }
    }
    ok(res, hits);
  });
};
