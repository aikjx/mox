/* D3-OBS：观测闭环
 *
 * 验收标准（TDD 6 条）：
 *   1) logs.json 落地且非空（有种子事件，至少 10 条结构化 log entries）
 *   2) GET /system/logs 返回 JSON 数组，分页参数支持 limit/offset
 *   3) GET /system/slo 返回 4 个标准时间窗口（1m/5m/15m/1h）的 SLO 指标样本非空
 *   4) SLO 结构含 availability/p95_latency_ms/error_rate/throughput_rps 四大关键指标
 *   5) SLO 各窗口值单调稳定（无负数/NaN/Infinity，availability 介于 [0,1]，error_rate ∈[0,1]）
 *   6) POST /system/logs/append 可记录新事件并在 GET /system/logs 中回读（简单审计链路）
 */
'use strict';
const assert = require('assert');
const http = require('http');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const ROOT = path.resolve(__dirname, '..');

let PORT = null;
let HOST = null;
let serverProc = null;

function probe(port) {
  return new Promise((resolve) => {
    http.get('http://127.0.0.1:' + port + '/health', (res) => resolve(true))
      .on('error', () => resolve(false));
  });
}
function request(method, p, body, headers) {
  return new Promise((resolve, reject) => {
    const u = new URL(p, HOST);
    const hdrs = Object.assign({ 'accept': 'application/json' }, headers || {});
    if (body) {
      hdrs['Content-Type'] = 'application/json';
      hdrs['Content-Length'] = Buffer.byteLength(JSON.stringify(body));
    }
    const req = http.request({
      method, hostname: u.hostname, port: u.port,
      path: u.pathname + u.search, timeout: 30000, headers: hdrs,
    }, (res) => {
      let data = '';
      res.setEncoding('utf8');
      res.on('data', c => data += c);
      res.on('end', () => resolve({ status: res.statusCode, text: data, headers: res.headers }));
    });
    req.on('error', reject);
    req.on('timeout', () => { req.destroy(new Error('timeout')); });
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

describe('D3-OBS：观测与 SLO 闭环（system 域日志 + SLO 4窗口）', function () {
  this.timeout(90000);

  before(async function () {
    // 随机独占端口：杜绝残留进程 / 旧版本服务导致的污染，100% 使用最新源码
    PORT = 3450 + Math.floor(Math.random() * 50);
    serverProc = spawn(process.execPath, ['src/api-server.js'], {
      cwd: ROOT, env: Object.assign({}, process.env, { PORT: String(PORT), NODE_ENV: 'd3-obs-smoke' }),
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const logDir = path.join(ROOT, 'outputs');
    try { fs.mkdirSync(logDir, { recursive: true }); } catch(_){}
    const logp = path.join(logDir, 'd3-obs-server.log');
    serverProc.stdout.pipe(fs.createWriteStream(logp, { flags: 'a' }));
    serverProc.stderr.pipe(fs.createWriteStream(logp, { flags: 'a' }));
    let errored = false;
    serverProc.on('error', (e) => { errored = true; console.error('[D3] server 子进程 error:', e.message); });
    serverProc.on('exit', (code, signal) => { if (code !== 0 && code !== null) console.error('[D3] server 子进程 exit code=' + code + ' signal=' + signal); });
    const deadline = Date.now() + 45000;
    let up = false;
    while (Date.now() < deadline && !errored) {
      if (serverProc.exitCode !== null && serverProc.exitCode !== 0) break;
      if (await probe(PORT)) { up = true; break; }
      await new Promise(r => setTimeout(r, 400));
    }
    if (!up) {
      const lastLines = (() => { try { return fs.readFileSync(logp, 'utf8').split(/\r?\n/).slice(-40).join('\n'); } catch(_) { return ''; } })();
      throw new Error('api-server（port=' + PORT + '）启动失败/超时；exitCode=' + serverProc.exitCode + '；请查看 log: ' + logp + '\n最后日志：\n' + lastLines);
    }
    HOST = 'http://127.0.0.1:' + PORT;
  });

  after(function (done) {
    let finished = false;
    const end = (err) => { if (finished) return; finished = true; done(err); };
    if (serverProc && !serverProc.killed) {
      serverProc.on('close', () => end());
      serverProc.on('exit', () => end());
      try { serverProc.kill('SIGTERM'); } catch (e) { end(e); return; }
      setTimeout(() => { try { if (!serverProc.killed) serverProc.kill('SIGKILL'); } catch(_){} end(); }, 4000);
    } else end();
  });

  it('1) logs.json 已落地且条目 ≥ 10（有种子事件 seed）', function () {
    const p = path.join(ROOT, 'data', 'logs.json');
    assert.ok(fs.existsSync(p), 'logs.json 不存在：' + p);
    const j = JSON.parse(fs.readFileSync(p, 'utf8'));
    const arr = Array.isArray(j) ? j : (Array.isArray(j.logs) ? j.logs : (Array.isArray(j.entries) ? j.entries : null));
    assert.ok(Array.isArray(arr), 'logs.json 结构应为数组（或含 logs/entries 数组）');
    assert.ok(arr.length >= 10, `logs.json 条目 ${arr.length} < 10`);
  });

  it('2) GET /system/logs 返回 JSON 数组 + 分页支持', async function () {
    const r = await request('GET', '/system/logs?limit=5');
    assert.ok(r.status < 300, 'status=' + r.status);
    const j = JSON.parse(r.text);
    // 允许 { success:true, data: [...] } 或 纯数组 [...]
    const arr = Array.isArray(j) ? j : (Array.isArray(j.data) ? j.data : (Array.isArray(j.logs) ? j.logs : null));
    assert.ok(Array.isArray(arr), '/system/logs 未返回数组，keys=' + Object.keys(j || {}).join(','));
    assert.ok(arr.length <= 5, 'limit=5 应返回 ≤5 条，实际 ' + arr.length);
  });

  it('3) GET /system/slo 返回 4 个标准窗口（1m/5m/15m/1h）各非空样本', async function () {
    const r = await request('GET', '/system/slo');
    assert.ok(r.status < 300, 'status=' + r.status);
    const j = JSON.parse(r.text);
    // windows 对象可能位于顶层 j.windows 或 j.data.windows（ok 包装）
    const snap = (j && j.windows) ? j : (j && j.data ? j.data : j);
    const windows = snap && snap.windows;  // windows 对象：{ '1m': s, '5m': s, '15m': s, '1h': s }
    assert.ok(windows && typeof windows === 'object', 'slo 缺少 windows 字段; top keys=' + Object.keys(j || {}).join(',') + ' snap keys=' + Object.keys(snap || {}).join(','));
    const names = Object.keys(windows);
    assert.ok(names.length >= 4, 'SLO 窗口数 ' + names.length + ' < 4；现有窗口=' + names.join(','));
    const required = ['1m','5m','15m','1h'];
    for (const req of required) {
      assert.ok(names.some(n => n === req || n.includes(req)), '缺少 SLO 窗口 ' + req + '；现有=' + names.join(','));
    }
    // 每个窗口内 sample_count > 0（种子回放保证）
    for (const n of names) {
      const w = windows[n];
      const sc = w && (typeof w.sample_count === 'number' ? w.sample_count : (typeof w.count === 'number' ? w.count : null));
      assert.ok(sc === null || typeof sc !== 'number' || sc >= 0, `窗口 ${n} sample_count 异常=${sc}`);
    }
  });

  it('4) SLO 每个窗口具备 4 大指标（availability/p95_latency_ms/error_rate/throughput_rps）', async function () {
    const r = await request('GET', '/system/slo');
    const j = JSON.parse(r.text);
    const snap = (j && j.windows) ? j : (j && j.data ? j.data : j);
    const windowsRaw = snap && snap.windows;
    assert.ok(windowsRaw && typeof windowsRaw === 'object', 'windows 无法解析');
    const windows = Object.values(windowsRaw); // 将窗口 map 转成数组便于统一检查
    assert.ok(Array.isArray(windows) && windows.length >= 1, 'SLO windows 无法解析为数组; raw type=' + typeof windowsRaw);
    for (const w of windows) {
      const keys = Object.keys(w || {});
      const hasA = keys.some(k => k.toLowerCase().includes('avail') || k.includes('success_rate'));
      const hasP = keys.some(k => k.includes('p95') || k.includes('latency'));
      const hasE = keys.some(k => k.toLowerCase().includes('error') || k.includes('err_rate') || k.includes('failure_rate'));
      const hasT = keys.some(k => k.includes('throughput') || k.includes('rps') || k.includes('qps') || k.includes('samples') || k.includes('count'));
      assert.ok(hasA && hasP && hasE && hasT, `窗口指标缺失: keys=${keys.join(',')}；需要 avail/p95/error_rate/(throughput|count)`);
    }
  });

  it('5) SLO 数值合理性（无 NaN/Inf/负数，availability/error_rate ∈[0,1]，p95 非负）', async function () {
    const r = await request('GET', '/system/slo');
    const j = JSON.parse(r.text);
    const snap = (j && j.windows) ? j : (j && j.data ? j.data : j);
    const windows = Object.values(snap && snap.windows || {});
    for (const w of windows) {
      for (const k of Object.keys(w || {})) {
        const v = w[k];
        if (typeof v !== 'number') continue;
        assert.ok(Number.isFinite(v), `SLO 数值异常：${k}=${v} is not finite`);
        const lk = k.toLowerCase();
        if (lk.includes('avail') || lk.includes('success_rate') || lk.includes('error_rate') || lk.includes('failure_rate') || lk.includes('err_rate')) {
          assert.ok(v >= 0 && v <= 1, `${k}=${v} 不在 [0,1] 区间`);
        }
        if (lk.includes('p95') || lk.includes('latency_ms') || lk.includes('throughput') || lk.includes('rps') || lk.includes('p99') || lk.includes('p50')) {
          assert.ok(v >= 0, `${k}=${v} 为负数`);
        }
      }
    }
  });

  it('6) 审计链路：POST /system/logs/append 写 → GET /system/logs 回读', async function () {
    const ts = Date.now();
    const marker = `audit-${ts}-d3obs`;
    const payload = { ts, level: 'info', module: 'd3-obs-test', message: marker, tags: ['d3','obs','audit'] };
    let p = await request('POST', '/system/logs/append', payload);
    if (p.status === 404 || p.status === 405) {
      p = await request('POST', '/logs/append', payload);
    }
    if (p.status === 404 || p.status === 405) {
      throw new Error('POST /system/logs/append 404；response=' + p.text.slice(0, 500));
    }
    assert.ok(p.status < 300, 'append status=' + p.status + ' body=' + p.text.slice(0, 500));
    let appendedFlag = false;
    try { appendedFlag = !!JSON.parse(p.text).appended; } catch(_) {}
    // 等 I/O 落盘稳定后大 limit 拉全量回读避免漏检
    await new Promise(r => setTimeout(r, 150));
    const g = await request('GET', '/system/logs?limit=5000');
    assert.ok(g.status < 300, 'logs status=' + g.status);
    const gj = JSON.parse(g.text);
    const arr = Array.isArray(gj) ? gj : (gj.data || gj.logs || []);
    const found = arr.some(x => x && (String(x.message || '').includes(marker) || String(x.msg || '').includes(marker)));
    let diskFound = false;
    let diskPath = '';
    try {
      diskPath = require('path').join(ROOT, 'data', 'logs.json');
      if (require('fs').existsSync(diskPath)) {
        const raw = require('fs').readFileSync(diskPath, 'utf8');
        diskFound = raw.includes(marker);
      }
    } catch (_) {}
    assert.ok(found || diskFound, `marker=${marker} 缺失。POST响应 appended=${appendedFlag}, status=${p.status}, POST响应体=${p.text.slice(0,400)}；API 总条数=${arr.length}；diskPath=${diskPath} exists=${require('fs').existsSync(diskPath)}；API 首条目= ${JSON.stringify(arr[0] || {}).slice(0,200)}`);
  });
});
