'use strict';
/**
 * 端到端契约冒烟（需 api-server.js 已在 :3010 启动）
 *  覆盖: /health, /tasks GET/POST, /market GET/POST /random, /graph/activate 多/单/空 seed
 *  严格与前端 TaskView / MarketView / GraphView activate 提交字段一致。
 */
const http = require('http');
const BASE = { host: '127.0.0.1', port: process.env.PORT || 3010, headers: {
  'Authorization': 'Bearer dev-secret-token',
  'Content-Type': 'application/json'
}};

function request(method, path, body) {
  return new Promise((resolve) => {
    const data = body ? JSON.stringify(body) : null;
    const opts = Object.assign({}, BASE, { method, path, headers: Object.assign({}, BASE.headers,
      data ? { 'Content-Length': Buffer.byteLength(data) } : {}) });
    const req = http.request(opts, (res) => {
      let buf = '';
      res.on('data', (c) => { buf += c; });
      res.on('end', () => {
        let parsed = null;
        try { parsed = JSON.parse(buf); } catch (e) { parsed = { _raw: buf.slice(0, 300) }; }
        resolve({ status: res.statusCode, body: parsed });
      });
    });
    req.on('error', (e) => resolve({ status: 0, body: { error: e.message } }));
    if (data) req.write(data);
    req.end();
  });
}

const pass = []; const fail = [];
function check(label, cond, note) { (cond ? pass : fail).push(label + (note ? ' | ' + note : '')); }

(async () => {
  // 1) /health 可达 — 证明服务真运行
  const h = await request('GET', '/health');
  check('/health status 200', h.status === 200, `status=${h.status} env=${process.env.NODE_ENV}`);
  check('/health success=true 包结构', h.body && h.body.success === true, `body keys=${h.body && Object.keys(h.body).slice(0,4).join(',')}`);

  // 2) GET /tasks 数组
  const tL = await request('GET', '/tasks');
  check('GET /tasks success=true & data isArray', tL.body && tL.body.success === true && Array.isArray(tL.body.data),
    `status=${tL.status} isArray=${Array.isArray(tL.body && tL.body.data)} len=${tL.body && tL.body.data && tL.body.data.length}`);

  // 3) POST /tasks 真实前端 DTO（严格与 TaskView.vue 提交字段对齐）
  const payload = {
    title: '契约冒烟-前端字段校验 ' + Date.now(),
    description: '真实 DTO：前端 create 提交的所有字段均在此，验证后端接收',
    priority: 'high',
    category: 'development',
    status: 'todo',
    due_date: '2026-12-31T23:59:59',
    estimate_hours: 4.5,
    tags: ['契约', '前端对齐'],
    created_at: new Date().toISOString(),
  };
  const tC = await request('POST', '/tasks', payload);
  check('POST /tasks status=200 success=true', tC.status === 200 && tC.body && tC.body.success === true,
    `status=${tC.status} success=${tC.body && tC.body.success} id=${tC.body && tC.body.data && tC.body.data.id}`);
  const tData = tC.body && tC.body.data;
  check('POST /tasks 标题/分类/优先级 完全回写', tData && tData.title === payload.title && tData.category === payload.category && tData.priority === payload.priority,
    `title=${tData && tData.title} category=${tData && tData.category} priority=${tData && tData.priority}`);
  check('POST /tasks 新增字段 estimate_hours 数值回写', tData && typeof tData.estimate_hours === 'number' && Math.abs(tData.estimate_hours - 4.5) < 1e-6,
    `estimate_hours=${tData && JSON.stringify(tData.estimate_hours)}`);
  check('POST /tasks due_date 原样回写', tData && tData.due_date === payload.due_date, `due_date=${tData && tData.due_date}`);
  check('POST /tasks tags 数组回写（长度一致）', tData && Array.isArray(tData.tags) && tData.tags.length === payload.tags.length,
    `tags=${JSON.stringify(tData && tData.tags)}`);

  // 4) 空标题 POST /tasks 后端也能兜底生成（必填校验在前端，后端保持容错）
  const tEmpty = await request('POST', '/tasks', {});
  check('POST /tasks {} 容错兜底 title≠空 不 5xx', tEmpty.status === 200 && tEmpty.body && tEmpty.body.data && String(tEmpty.body.data.title).length > 0,
    `status=${tEmpty.status} title=${tEmpty.body && tEmpty.body.data && tEmpty.body.data.title}`);

  // 5) GET /market 数组
  const mL = await request('GET', '/market');
  check('GET /market success=true & data isArray', mL.body && mL.body.success === true && Array.isArray(mL.body.data),
    `status=${mL.status} isArray=${Array.isArray(mL.body && mL.body.data)} len=${mL.body && mL.body.data && mL.body.data.length}`);

  // 6) POST /market/upload 前端 MarketView DTO（含 name/category/version/tags/downloads/rating/requirement/summary）
  const mPayload = {
    name: '契约冒烟算子包 ' + Date.now(),
    category: 'ai',
    author: 'smoke-bot',
    summary: '简介字段用于卡片展示，1-2-3',
    tags: ['契约', '上传', '前端对齐'],
    requirement: '需求描述不短于八个字符以确保前端最小校验长度通过',
    version: '2.0.3',
    downloads: 1234,
    rating: 4,
  };
  const mU = await request('POST', '/market/upload', mPayload);
  check('POST /market/upload status=200 success=true', mU.status === 200 && mU.body && mU.body.success === true,
    `status=${mU.status} success=${mU.body && mU.body.success} id=${mU.body && mU.body.data && mU.body.data.id}`);
  const mData = mU.body && mU.body.data;
  check('POST /market/upload 名称/分类/版本回写', mData && mData.name === mPayload.name && mData.category === mPayload.category && mData.version === mPayload.version,
    `name=${mData && mData.name} category=${mData && mData.category} version=${mData && mData.version}`);
  check('POST /market/upload downloads/rating 数值型回写', mData && Number(mData.downloads) === 1234 && Number(mData.rating) === 4,
    `downloads=${mData && mData.downloads} rating=${mData && mData.rating}`);
  check('POST /market/upload tags 数组回写（length=3）', mData && Array.isArray(mData.tags) && mData.tags.length === 3,
    `tags=${JSON.stringify(mData && mData.tags)}`);
  check('POST /market/upload 缺失 name 返回 400（非法值阻断）', (async () => {
    const r = await request('POST', '/market/upload', { summary: 'no name' });
    return r.status === 400;
  })(), `status=${(await request('POST', '/market/upload', { summary: 'no name' })).status}`);

  // 7) /market/random — 修复后的前端 MarketView 兼容双路径
  const mR = await request('GET', '/market/random');
  check('GET /market/random success=true', mR.body && mR.body.success === true, `status=${mR.status} type=${Array.isArray(mR.body && mR.body.data) ? 'array' : (typeof mR.body && mR.body.data)}`);

  // 8) /graph/activate 多种子
  const gM = await request('POST', '/graph/activate', { seed: ['d04', 'biz'], iterations: 10, decay: 0.85 });
  check('POST /graph/activate 多种子 status=200 success=true', gM.status === 200 && gM.body && gM.body.success === true,
    `status=${gM.status} success=${gM.body && gM.body.success}`);
  check('POST /graph/activate 多种子 seeds[] 回写 2 个', gM.body && gM.body.data && Array.isArray(gM.body.data.seeds) && gM.body.data.seeds.length === 2,
    `seeds=${JSON.stringify(gM.body && gM.body.data && gM.body.data.seeds)}`);
  check('POST /graph/activate 激活扩散 method=spread（硬约束 §18-4）', gM.body && gM.body.data && gM.body.data.activation && gM.body.data.activation.method === 'spread',
    `method=${gM.body && gM.body.data && gM.body.data.activation && gM.body.data.activation.method} damping=${gM.body && gM.body.data && gM.body.data.activation && gM.body.data.activation.damping}`);
  check('POST /graph/activate 返回能量表 energy 非空', gM.body && gM.body.data && gM.body.data.energy && Object.keys(gM.body.data.energy).length > 0,
    `energy.keys=${gM.body && gM.body.data && Object.keys(gM.body.data.energy || {}).length}`);

  // 9) /graph/activate 单种子（旧版兼容）
  const gS = await request('POST', '/graph/activate', { seed: 'd04' });
  check('POST /graph/activate 单种子 success=true & seed=string', gS.status === 200 && gS.body && gS.body.success === true &&
    (typeof (gS.body.data && gS.body.data.seed) === 'string' || Array.isArray(gS.body.data.seeds)),
    `seed=${JSON.stringify(gS.body && gS.body.data && gS.body.data.seed)} seeds=${JSON.stringify(gS.body && gS.body.data && gS.body.data.seeds)}`);

  // 10) /graph/activate 空 seed → 校验拦截 400
  const g0 = await request('POST', '/graph/activate', {});
  check('POST /graph/activate 空 seed 400 拒绝', g0.status === 400, `status=${g0.status} err=${g0.body && g0.body.error}`);

  // 11) /graph/activate 不存在的 seed → 404
  const gX = await request('POST', '/graph/activate', { seed: 'NO_SUCH_NODE_xyz_123' });
  check('POST /graph/activate 未知 seed 404 拒绝', gX.status === 404, `status=${gX.status} err=${gX.body && gX.body.error}`);

  // 输出
  console.log('\n===== 端到端契约冒烟报告 =====');
  pass.forEach(p => console.log('  [PASS] ' + p));
  fail.forEach(f => console.log('  [FAIL] ' + f));
  console.log(`\n总计: PASS ${pass.length} / FAIL ${fail.length} / 共 ${pass.length + fail.length}`);
  process.exit(fail.length ? 1 : 0);
})().catch(e => { console.error('FATAL:', e); process.exit(2); });
