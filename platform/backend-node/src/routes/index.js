'use strict';

/**
 * 路由域装配清单（配置前置）
 *
 * 新增业务域三步：
 *   1. 本目录新建 <domain>.js，导出 register<Domain>Routes(ctx)，内部 const {...} = ctx 解构依赖
 *   2. 在下方 DOMAINS 登记一行（顺序即注册顺序，保持与既有路由优先级一致）
 *   3. 重启服务，路由自动生效
 *
 * ctx 注入清单见 api-server.js 的 registerRoutes()。
 */
const DOMAINS = [
  ['system', '系统与状态', require('./system')],
  ['studio', '璇玑工作台', require('./studio')],
  ['graph', '知识图谱', require('./graph')],
  ['chat', 'AI 对话', require('./chat')],
  ['web-search', '联网搜索', require('./web-search')],
  ['artifacts', '本地制品', require('./artifacts')],
  ['optimizer', '无穷维度优化', require('./optimizer')],
  ['ai-platform', 'AI 平台资源', require('./ai-platform')],
  ['browser-market', '浏览器与市场', require('./browser-market')],
  ['integration', '集成通道', require('./integration')],
  ['expert-alliance', '专家联盟', require('./expert-alliance')],
  ['expert-alliance-v3', '专家联盟 v3（7服务架构）', require('./expert-alliance-v3')],
  ['expert-graph', '专家图谱', require('./expert-graph')],
  ['mcp', 'MCP 协议服务', require('./mcp')],
  ['orchestration', '编排协作', require('./orchestration')],
  ['ai-enhanced', '16 模块 AI 增强', require('./ai-enhanced')],
  ['tasks', '任务管理', require('./tasks')],
  ['kb', '知识库', require('./kb')],
  ['engine-universe', '引擎宇宙图谱', require('./engine-universe')],
  ['engine-kernel', '引擎内核', require('./engine-kernel')],
  ['atlas', '项目全息图谱', require('./atlas')],
  ['auto-tasks', '自动任务', require('./auto-tasks')],
  ['modules-admin', '模块与存储管理', require('./modules-admin')],
  ['security', '安全审计', require('./security')],
  ['ai-engine', 'AI 引擎核心', require('./ai-engine')],
  ['ai-integrated', '智能集成引擎', require('./ai-integrated')],
  ['ai-ultimate', '终极 AI 引擎', require('./ai-ultimate')],
  ['auto-dev', '自动开发引擎', require('./auto-dev')],
  ['services', '服务管理', require('./services')],
  ['projects', '项目中心', require('./projects')],
  ['internal', '内部端点（sidecar 调用）', require('./internal')],
];

// -------- D1-ARCH 一致性补齐：business-registry 中所有 domain-rust-* / mod-* 治理域必须对应 DOMAINS 注册 --------
// 否则会出现"注册了但路由无法到达"的治理孤点，违反 AIS 三向一致性（Registry ↔ Routes ↔ Projects）。
(function appendGovernanceVirtualDomains() {
  // 加载 business-registry 拿到"治理图谱全量实体"：业务主域 + Rust 治理域 + 引擎模块
  // 每个实体在 DOMAINS 装配列表中必须对应一个路由注册项（即便瘦路由仅返回元数据）—— 0 孤点要求。
  let registry = null;
  try { registry = require('../project-atlas/domain/business-registry'); }
  catch (_) { registry = null; }
  let ids = [];
  const metaById = new Map();
  if (registry && typeof registry.getAllEntityIds === 'function') {
    ids = registry.getAllEntityIds();
    for (const e of registry.getAllEntities()) {
      if (e && e.id) metaById.set(e.id, e);
    }
  } else {
    // fallback: 静态 list（与 business-registry 保持同步：15 domain-rust-* + 8 mod-* = 23）
    ids = [
      'domain-rust-operator-core','domain-rust-operator-wasm','domain-rust-graph-algorithms',
      'domain-rust-optimizer','domain-rust-flow-ai','domain-rust-mox-expert',
      'domain-rust-hermes-flow-bridge','domain-rust-business-catalog','domain-rust-ai-agent',
      'domain-rust-template-market','domain-rust-runtime','domain-rust-mox-system',
      'domain-rust-primiflow-core','domain-rust-primiflow-fusion','domain-rust-kg-hub',
      'mod-graph','mod-task','mod-storage','mod-melody2score',
      'mod-rust-operator-wasm','mod-rust-hermes-flow-bridge','mod-rust-business-catalog',
      'mod-rust-template-market',
    ];
  }
  const already = new Set(DOMAINS.map(d => d[0]));
  for (const id of ids) {
    if (already.has(id)) continue;
    const kind = id.startsWith('domain-rust-') ? 'rust-crate' : (id.startsWith('mod-') ? 'engine-module' : 'virtual');
    const name = metaById.get(id)?.name || (kind === 'rust-crate' ? `Rust Crate/${id.slice('domain-rust-'.length)}` : `Module/${id}`);
    // 生成瘦路由：GET /<id> → 返回域元数据（kind、name、owner、contracts）；这保证 A 中每个治理域在 B 中都有 REST 入口，而非 404
    const syntheticRegister = function (ctx) {
      const { reg, ok, readJSON } = ctx;
      reg('get', '/' + id, (req, res) => {
        const meta = metaById.get(id) || { id, name, kind };
        const endpoints = [];
        endpoints.push({ method: 'GET', path: '/' + id, desc: `${name} · 域元数据契约` });
        endpoints.push({ method: 'GET', path: '/' + id + '/health', desc: `${name} · 健康探针` });
        if (kind === 'rust-crate') {
          endpoints.push({ method: 'GET', path: '/' + id + '/crate', desc: 'Cargo.toml / crate 元数据（由 internal 域代理）' });
        } else if (kind === 'engine-module') {
          endpoints.push({ method: 'GET', path: '/' + id + '/module', desc: '模块描述 / 安装状态 / 路由数' });
        }
        ok(res, {
          id, name, kind,
          domainOwner: meta?.domain_owner || meta?.ownerName || '璇玑 AIS 平台',
          scope: meta?.scope || null,
          keyFeatures: meta?.keyFeatures || [],
          engines: meta?.engines || null,
          endpoints,
          registeredAt: new Date().toISOString(),
        });
      });
      reg('get', '/' + id + '/health', (req, res) => {
        ok(res, { status: 'ok', domain: id, kind, uptime: process.uptime() | 0 });
      });
      if (kind === 'rust-crate') {
        reg('get', '/' + id + '/crate', (req, res) => {
          const crate = id.slice('domain-rust-'.length);
          // Cargo.toml 位置：platform/services/<crate>/Cargo.toml 或 platform/gateway/<crate>/Cargo.toml
          const candidates = [
            require('path').resolve(__dirname, '..', '..', '..', 'services', crate, 'Cargo.toml'),
            require('path').resolve(__dirname, '..', '..', '..', 'gateway', crate, 'Cargo.toml'),
          ];
          const found = candidates.find(p => { try { return require('fs').existsSync(p); } catch (_) { return false; } });
          if (!found) return ok(res, { id, crate, manifest: null, note: 'Cargo.toml not found on disk (纯治理元域)' });
          const raw = require('fs').readFileSync(found, 'utf8');
          ok(res, { id, crate, manifestPath: found, manifest: raw.slice(0, 4000), trimmed: raw.length > 4000 });
        });
      }
      if (kind === 'engine-module') {
        reg('get', '/' + id + '/module', (req, res) => {
          const modName = id.startsWith('mod-rust-') ? id.slice('mod-rust-'.length) : id.slice('mod-'.length);
          const installed = (ctx.modules && typeof ctx.modules.installedList === 'function')
            ? ctx.modules.installedList() : [];
          ok(res, {
            id, moduleName: modName,
            installed: installed.includes(modName),
            routes: (ctx.modules && typeof ctx.modules.routeCount === 'function')
              ? ctx.modules.routeCount(modName) : 0,
            description: metaById.get(id)?.desc || `${name} 引擎模块`,
          });
        });
      }
    };
    DOMAINS.push([id, name, syntheticRegister]);
  }
})();

function registerAllRoutes(ctx) {
  for (const [file, name, register] of DOMAINS) {
    try {
      register(ctx);
    } catch (e) {
      console.error(`[routes] 域 ${name}(${file}) 注册失败:`, e);
      throw e;
    }
  }
  console.log(`[routes] ${DOMAINS.length} 个业务域装配完成（= business-registry 三向一致性）`);
}

module.exports = { DOMAINS, registerAllRoutes };
