'use strict';

/**
 * D4-SEC 安全：OUS_API_TOKEN 鉴权
 * TR:
 *   1) 设置 OUS_API_TOKEN 后：
 *      - 无 token 的 POST/PUT/DELETE 敏感操作应返回 401
 *      - 非法 token 的敏感操作应返回 401/403
 *      - 合法 token（Authorization Bearer / X-Token header / ?token= query / Cookie: x-token）任一方式均可通过
 *   2) GET 即使有 OUS_API_TOKEN 也不需要鉴权（公共读；SLO/health/列表是公开查询）
 *   3) 未设置 OUS_API_TOKEN 时：写操作无需鉴权（开发模式兼容）
 */
const fs = require('fs');
const path = require('path');
const http = require('http');
const { spawn } = require('child_process');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
let HOST = null;
let PORT = 0;
let serverProc = null;

function request(method, p, body, headers) {
  return new Promise((resolve) => {
    const u = new URL(p.startsWith('http') ? p : (HOST + p));
    const payload = body ? JSON.stringify(body) : null;
    const h = Object.assign({}, headers || {}, payload ? { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(payload) } : {});
    const req = http.request({ host: u.hostname, port: u.port, method, path: u.pathname + u.search, headers: h }, (res) => {
      let text = '';
      res.setEncoding('utf8');
      res.on('data', c => text += c);
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, text }));
    });
    req.on('error', (e) => resolve({ status: 0, text: String(e && e.message || e) }));
    if (payload) req.write(payload);
    req.end();
  });
}

function probe(p) {
  // 真实 HTTP GET /health 探测（比 socket 绑定方式更可靠：兼容 IPv6/dual-stack）
  return new Promise(r => {
    const timer = setTimeout(() => r(false), 1200);
    const req = http.request({ host: '127.0.0.1', port: p, method: 'GET', path: '/health' }, (res) => {
      clearTimeout(timer);
      r(res.statusCode > 0);
    });
    req.on('error', () => { clearTimeout(timer); r(false); });
    req.end();
  });
}

describe('D4-SEC：OUS_API_TOKEN 鉴权（分发层 + 敏感写接口）', function () {
  this.timeout(90000);

  before(async function () {
    // 固定 token，独占随机端口
    const TEST_TOKEN = 'ous-sec-token-test-xyz-42';
    process.env.OUS_API_TOKEN = TEST_TOKEN;
    PORT = 3510 + Math.floor(Math.random() * 50);
    serverProc = spawn(process.execPath, ['src/api-server.js'], {
      cwd: ROOT,
      env: Object.assign({}, process.env, { PORT: String(PORT), OUS_API_TOKEN: TEST_TOKEN, NODE_ENV: 'd4-sec-smoke' }),
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const logp = path.join(ROOT, 'outputs', 'd4-sec-server.log');
    try { fs.mkdirSync(path.dirname(logp), { recursive: true }); } catch(_){}
    serverProc.stdout.pipe(fs.createWriteStream(logp, { flags: 'a' }));
    serverProc.stderr.pipe(fs.createWriteStream(logp, { flags: 'a' }));
    const deadline = Date.now() + 45000;
    let up = false;
    while (Date.now() < deadline) {
      if (await probe(PORT)) { up = true; break; }
      await new Promise(r => setTimeout(r, 400));
    }
    if (!up) {
      const last = (() => { try { return fs.readFileSync(logp, 'utf8').split(/\r?\n/).slice(-30).join('\n'); } catch(_){return '';} })();
      throw new Error('D4 SEC api-server(' + PORT + ') 启动失败；last logs:\n' + last);
    }
    HOST = 'http://127.0.0.1:' + PORT;
  });

  after(function (done) {
    delete process.env.OUS_API_TOKEN;
    let finished = false;
    const end = (err) => { if (finished) return; finished = true; done(err); };
    if (serverProc && !serverProc.killed) {
      serverProc.on('close', () => end());
      serverProc.on('exit', () => end());
      try { serverProc.kill('SIGTERM'); } catch (e) { end(e); return; }
      setTimeout(() => { try { if (!serverProc.killed) serverProc.kill('SIGKILL'); } catch(_){} end(); }, 4000);
    } else end();
  });

  const TOKEN = 'ous-sec-token-test-xyz-42';

  it('1) 未带 token 时敏感写 POST /artifacts 返回 401（Gated: OFF 时返回 2xx/4xx 业务码，ON 时必须 401）', async function () {
    const r = await request('POST', '/artifacts', { kind: 'html', name: 'd4sec-test', content: '<h1>D4</h1>' });
    // 需要鉴权失败。401 是期望；若为 403 也视为鉴权失败（权限语义，已拒绝）。
    assert.ok(r.status === 401 || r.status === 403, '敏感操作无 token 应被拒绝；期望 401/403，实际 status=' + r.status + ' body=' + r.text.slice(0, 250));
    try {
      const j = JSON.parse(r.text);
      assert.ok(j && (j.error || j.success === false), '拒绝响应结构应包含 error/success=false；body=' + r.text.slice(0, 200));
    } catch (_) {}
  });

  it('2) 非法 token（Bearer 随机字符串）POST /system/logs/append 也被拒绝', async function () {
    const r = await request('POST', '/system/logs/append',
      { level: 'info', message: 'd4sec-should-block' },
      { 'Authorization': 'Bearer wrong-token-123' }
    );
    assert.ok(r.status === 401 || r.status === 403, '非法 token 应拒绝；status=' + r.status);
  });

  it('3) 合法 token（Authorization: Bearer）POST /system/logs/append 放行并返回 2xx', async function () {
    const r = await request('POST', '/system/logs/append',
      { level: 'info', message: 'd4sec-bearer-ok-' + Date.now() },
      { 'Authorization': 'Bearer ' + TOKEN }
    );
    assert.ok(r.status < 300, '合法 Bearer token 应通过；status=' + r.status + ' body=' + r.text.slice(0, 200));
  });

  it('4) 合法 token（X-Token header）POST /system/logs/append 放行', async function () {
    const r = await request('POST', '/system/logs/append',
      { level: 'info', message: 'd4sec-xheader-ok-' + Date.now() },
      { 'X-Token': TOKEN }
    );
    assert.ok(r.status < 300, '合法 X-Token 应通过；status=' + r.status);
  });

  it('5) 合法 token（?token= query）POST /system/logs/append 放行', async function () {
    const r = await request('POST', '/system/logs/append?token=' + encodeURIComponent(TOKEN),
      { level: 'info', message: 'd4sec-query-ok-' + Date.now() }
    );
    assert.ok(r.status < 300, '合法 ?token= 应通过；status=' + r.status);
  });

  it('6) GET 类接口即使 OUS_API_TOKEN 开启也无需鉴权（/system/health, /system/slo, /artifacts, /kb/list 全部返回 2xx）', async function () {
    const paths = ['/system/health', '/system/slo', '/artifacts', '/kb/list'];
    for (const p of paths) {
      const r = await request('GET', p);
      assert.ok(r.status < 300, `GET ${p} 应免鉴权放行；status=${r.status} body=${r.text.slice(0, 150)}`);
    }
  });

  it('7) 敏感写接口清单：已知敏感写路径的统一行为一致性（无 token → 401/403；有 token → 2xx 或业务级错误，但不能是 401）', async function () {
    const sensitivePaths = [
      { method: 'POST', path: '/artifacts', payload: { kind: 'html', name: 'd4-sec-c2', content: 'x' } },
      { method: 'POST', path: '/operators/register', payload: { name: 'd4op', code: 'fn(a){a}' } },
      { method: 'POST', path: '/kb/documents', payload: { title: 'd4 kb', content: 'kb content' } },
      { method: 'PUT', path: '/kb/documents/999', payload: { content: 'update d4' } },
      { method: 'DELETE', path: '/kb/documents/999', payload: {} },
      { method: 'POST', path: '/kb/documents/999/delete', payload: {} },
    ];
    for (const req0 of sensitivePaths) {
      // 无 token
      const noT = await request(req0.method, req0.path, req0.payload);
      assert.ok(noT.status === 401 || noT.status === 403, `[NO TOKEN] ${req0.method} ${req0.path} 应返回 401/403；实际=${noT.status}`);
      // 有 token（允许业务错误，如不存在=404，但不能是鉴权错误）
      const wT = await request(req0.method, req0.path + (req0.path.includes('?') ? '&' : '?') + 'token=' + encodeURIComponent(TOKEN), req0.payload);
      assert.notStrictEqual(wT.status, 401, `[WITH TOKEN] ${req0.method} ${req0.path} 不应为 401`);
      assert.notStrictEqual(wT.status, 403, `[WITH TOKEN] ${req0.method} ${req0.path} 不应为 403`);
    }
  });
});
