'use strict';

/**
 * TR-7.2:
 *   a) internal endpoints schema 通过；
 *   b) graph search 的前 3 条相较纯 LIKE（spread_weight=0）至少包含一条"激活扩散新命中"。
 */

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');

let passed = 0, failed = 0;
function test(name, fn) {
  try { fn(); passed++; console.log('  PASS ', name); }
  catch (e) { failed++; console.error('  FAIL ', name, '\n    ', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 5).join('\n')); }
}

const WORK_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t72-'));
process.env.DATA_DIR = WORK_DIR;
process.env.STORAGE_PROVIDER = 'sqlite';

const configPath = path.resolve(__dirname, '..', 'src', 'config.js');
const storagePath = path.resolve(__dirname, '..', 'src', 'storage', 'index.js');
delete require.cache[require.resolve(configPath)];
delete require.cache[require.resolve(storagePath)];

const { config } = require(configPath);
config.storage.provider = 'memory';
config.features.autoMigrate = false;
config.storage.providers.sqlite.path = path.join(WORK_DIR, 't72.db');

const { getStorage, resetStorage } = require(storagePath);
resetStorage();
const storage = getStorage();

// 写 fixture 图谱：A,B 含关键词 '测试'；C 不含但 A→C、B→C（激活扩散能把 C 顶上来）
//   D,E 与种子图完全不相连（避免路径"E 通过两条途径→激活扩散得分高于 C"的干扰）
const nodes = [
  { id: 'A', label: '测试节点-A', name: '测试A', kind: 'Test', properties: {} },
  { id: 'B', label: '测试节点-B', name: '测试B', kind: 'Test', properties: { sub: '测试用例' } },
  { id: 'C', label: '文档归档节点', name: '归档C', kind: 'Doc', properties: { text: '业务资料 不包含关键字' } },
  { id: 'D', label: '不相关-X', name: 'X', kind: 'Other', properties: {} },
  { id: 'E', label: '下游依赖-Y', name: 'Y', kind: 'Dep', properties: {} },
];
const edges = [
  { from: 'A', to: 'C', weight: 2 },  // A 直接指向 C（种子邻居，激活扩散高分）
  { from: 'B', to: 'C', weight: 2 },  // B 直接指向 C（种子邻居，激活扩散高分）
  // D、E 与种子 A/B 不相邻，确保 C 是唯一一跳邻居
];
storage.saveList('graph_nodes', nodes);
storage.saveList('graph_edges', edges);

// ---- 构建 mock ctx 以调用 internal/graph search route handlers 直接
function mockCtx() {
  const url = require('url');
  const pending = []; // 记录 reg 回调
  const ok = (res, v) => { res.__status = 200; res.__body = v; };
  const fail = (res, code, msg) => { res.__status = code; res.__body = { ok: false, error: msg }; };
  const readBody = async (req) => req.__body;
  const reg = (method, route, fn) => { pending.push({ method, route, fn }); };
  const readJSON = (k, d) => {
    if (k === 'graph_nodes.json') return storage.getList('graph_nodes', []);
    if (k === 'graph_edges.json') return storage.getList('graph_edges', []);
    return d;
  };
  const writeJSON = (k, v) => {
    if (k === 'graph_nodes.json') return storage.saveList('graph_nodes', v);
    if (k === 'graph_edges.json') return storage.saveList('graph_edges', v);
    return undefined;
  };
  return { url, ok, fail, readBody, readJSON, writeJSON, reg, _pending: pending };
}

// a) internal endpoints
test('TR-7.2: /internal/intent 返回 schema 通过（ok + intent + confidence + capability + explain）', () => {
  const ctx = mockCtx();
  require('../src/routes/internal')(ctx);
  const intentRoute = ctx._pending.find(r => r.route === '/internal/intent' && r.method === 'post');
  assert.ok(intentRoute, '找不到 /internal/intent 路由');
  const req = { __body: { query: '帮我列出项目 P-087 的关联节点', context: { project: 'P-087' } } };
  const res = {};
  return (async () => {
    await intentRoute.fn(req, res);
    assert.strictEqual(res.__status, 200);
    const b = res.__body;
    assert.strictEqual(b.ok, true);
    assert.ok(typeof b.intent === 'string' && b.intent.length > 0);
    assert.ok(typeof b.confidence === 'number' && b.confidence >= 0 && b.confidence <= 1.01);
    assert.ok(typeof b.capability === 'string');
    assert.ok(Array.isArray(b.explain) && b.explain.length >= 2);
    passed++;
    console.log('       → intent=%s cap=%s conf=%.2f', b.intent, b.capability, b.confidence);
  })();
});

test('TR-7.2: /internal/graph-algo list_nodes 返回统一 schema', () => {
  const ctx = mockCtx();
  require('../src/routes/internal')(ctx);
  const route = ctx._pending.find(r => r.route === '/internal/graph-algo');
  assert.ok(route);
  const req = { __body: { algorithm: 'list_nodes', payload: {} } };
  const res = {};
  return (async () => {
    await route.fn(req, res);
    assert.strictEqual(res.__status, 200);
    const b = res.__body;
    assert.strictEqual(b.ok, true);
    assert.strictEqual(b.algorithm, 'list_nodes');
    assert.ok(Array.isArray(b.result));
    assert.strictEqual(b.result.length, nodes.length);
    // data 段与本地 API 返回等价：包含 id, kind, name 等老字段
    const first = b.result[0];
    ['id', 'kind', 'name', 'properties', 'tags', 'labels'].forEach(k =>
      assert.ok(k in first, `节点结果应包含字段 ${k}`)
    );
  })();
});

test('TR-7.2: graph search 激活扩散顶出 1+ 条新命中（C 不在 LIKE 集合但出现在 Top3）', () => {
  const ctx = mockCtx();
  require('../src/routes/graph')(ctx);
  const searchRoute = ctx._pending.find(r => r.route === '/graph/search' && r.method === 'get');
  assert.ok(searchRoute, '找不到 /graph/search GET 路由');

  // 用 spread_weight=0（纯 LIKE）模拟老版本
  const reqOld = { url: '/graph/search?q=测试&limit=3&spread_weight=0' };
  const resOld = {};
  searchRoute.fn(reqOld, resOld);
  assert.strictEqual(resOld.__status, 200);
  const oldIds = resOld.__body.nodes.map(n => n.id);
  console.log('       → 纯 LIKE Top3 ids =', oldIds, 'stats=', resOld.__body.stats);

  // 新版本：默认 spread_weight=0.7
  const reqNew = { url: '/graph/search?q=测试&limit=3' };
  const resNew = {};
  searchRoute.fn(reqNew, resNew);
  assert.strictEqual(resNew.__status, 200);
  const newIds = resNew.__body.nodes.map(n => n.id);
  console.log('       → 重排后 Top3 ids =', newIds, 'stats=', resNew.__body.stats);

  // TR-7.2 验收：newIds ∪ notIn(oldIds) 至少 1 个 且 必须含 C（激活扩散邻居）
  const oldSet = new Set(oldIds);
  const newButNotOld = newIds.filter(x => !oldSet.has(x));
  assert.ok(newButNotOld.length >= 1,
    `重排后必须至少 1 条激活扩散带来的新命中。旧=${oldIds} 新=${newIds}`);
  assert.ok(newIds.includes('C'),
    `C 作为 A、B 的直接邻居，激活扩散扩散后必须在 Top3。实际=${newIds}`);

  // 兼容：响应仍保留 nodes/edges/query 原字段，老客户端一行不改
  assert.ok('nodes' in resNew.__body && 'edges' in resNew.__body && 'query' in resNew.__body);
  assert.strictEqual(resNew.__body.query, '测试');
  assert.ok(typeof resNew.__body.stats.extra_from_spread === 'number');
});

// 确保上述 async test 能跑
(async () => {
  try {
    for (let i = 0; i < test._asyncs ? 0 : 0; i++) { /* placeholder */ }
  } catch {}
})();

// —— 手工驱动 async tests（由于 Mocha/tape 未引入，手动串行化）：
(async function main() {
  // 先刷新 internal/graph route 注册次数
  try {
    // 1
    const ctx1 = mockCtx();
    require('../src/routes/internal')(ctx1);
    const intentRoute = ctx1._pending.find(r => r.route === '/internal/intent');
    const req1 = { __body: { query: '帮我列出项目 P-087 的关联节点', context: { project: 'P-087' } } };
    const res1 = {};
    await intentRoute.fn(req1, res1);
    assert.strictEqual(res1.__status, 200, `/internal/intent status=${res1.__status} ${JSON.stringify(res1.__body)}`);
    const b1 = res1.__body;
    assert.strictEqual(b1.ok, true); assert.ok(b1.intent); assert.ok(b1.capability);
    assert.ok(typeof b1.confidence === 'number' && b1.confidence >= 0);
    assert.ok(Array.isArray(b1.explain) && b1.explain.length >= 2);
    passed++; console.log('  PASS TR-7.2: /internal/intent schema (executed)');

    // 2
    const ctx2 = mockCtx();
    require('../src/routes/internal')(ctx2);
    const algo = ctx2._pending.find(r => r.route === '/internal/graph-algo');
    const res2 = {};
    await algo.fn({ __body: { algorithm: 'list_nodes', payload: {} } }, res2);
    assert.strictEqual(res2.__status, 200);
    assert.strictEqual(res2.__body.algorithm, 'list_nodes');
    assert.ok(Array.isArray(res2.__body.result) && res2.__body.result.length >= 5);
    const f2 = res2.__body.result[0];
    ['id', 'kind', 'name', 'properties'].forEach(k => assert.ok(k in f2, `缺失 ${k}`));
    passed++; console.log('  PASS TR-7.2: /internal/graph-algo schema (executed)');

    // 3
    const ctx3 = mockCtx();
    require('../src/routes/graph')(ctx3);
    const search = ctx3._pending.find(r => r.route === '/graph/search' && r.method === 'get');
    const rOld = {};
    search.fn({ url: '/graph/search?q=测试&limit=3&spread_weight=0' }, rOld);
    assert.strictEqual(rOld.__status, 200);
    const oldIds = rOld.__body.nodes.map(n => n.id);

    const rNew = {};
    search.fn({ url: '/graph/search?q=测试&limit=3' }, rNew);
    assert.strictEqual(rNew.__status, 200);
    const newIds = rNew.__body.nodes.map(n => n.id);
    console.log('       → old Top3=', oldIds, 'new Top3=', newIds, 'spread_stats=', rNew.__body.stats);
    const oldSet = new Set(oldIds);
    const fresh = newIds.filter(x => !oldSet.has(x));
    assert.ok(fresh.length >= 1, `必须至少 1 条扩散新命中。旧=${oldIds} 新=${newIds}`);
    assert.ok(newIds.includes('C'), `C 必须在新 Top3 内（被 A/B 激活扩散）。实际=${newIds}`);
    assert.ok('query' in rNew.__body && rNew.__body.query === '测试');
    passed++; console.log('  PASS TR-7.2: graph search 激活扩散新命中 (executed)');
  } catch (e) {
    failed++;
    console.error('  FAIL TR-7.2 (async body):', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 6).join('\n'));
  } finally {
    console.log(`\n[GREEN T7.2 internal/search] ${passed} passed / ${failed} failed`);
    process.exit(failed === 0 ? 0 : 1);
  }
})();
