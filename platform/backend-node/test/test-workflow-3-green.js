'use strict';
/**
 * T13: Workflow 统一端点 + 3 内置模板 + step 图谱节点写回 + runs_on 边
 *
 * TR 13.1：3 workflow 各 10 runs → ok ≥ 9/10；返回 shape 统一
 * TR 13.2：30 runs 后 Nebula COUNT(VERTEX workflow_step) ≥ Σ steps × 30 = (5+5+7)*30 = 510
 * TR 13.3：每个 runs_on 边 ≥ 1（抽样检查 10% steps）
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');
const http = require('http');

const WORK_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'xuanji-t13-'));
process.env.DATA_DIR = WORK_DIR;
process.env.STORAGE_PROVIDER = 'memory';
process.env.USE_NEBULAGRAPH = 'false';
process.env.INFOTIER = process.env.INFOTIER || 'oss';

// 强制重建 config + storage
const CONFIG = require.resolve('../src/config');
const STORAGE = require.resolve('../src/storage');
delete require.cache[CONFIG];
delete require.cache[STORAGE];
const { config } = require(CONFIG);
config.storage.provider = 'memory';
config.features.autoMigrate = false;
config.storage.providers.sqlite.path = path.join(WORK_DIR, 't13.db');
const { getStorage, resetStorage } = require(STORAGE);
resetStorage();

let passed = 0, failed = 0;
function test(name, fn) {
  try { fn(); passed++; console.log('  PASS ', name); }
  catch (e) { failed++; console.error('  FAIL ', name, '\n    ', (e && e.message || e) + '\n' + (e.stack || '').split('\n').slice(1, 6).join('\n')); }
}
function testAsync(name, fn) {
  return fn().then(() => { passed++; console.log('  PASS ', name); })
    .catch(e => { failed++; console.error('  FAIL ', name, '\n    ', (e && e.message || e) + '\n' + (e.stack || '').split('\n').slice(1, 6).join('\n')); });
}

// ====== 启动 Node API server（业务路由域，无 Rust 网关依赖）======
function createApiServer() {
  // api-server.js require 会执行 registerRoutes + server.listen(config.app.port)
  // 给 PORT=0 → 随机端口；require 完成后 listen 是异步，我们等 listening 事件
  process.env.PORT = '0';
  const pathA = require.resolve('../src/api-server');
  delete require.cache[pathA];
  return require(pathA);
}
const srv = createApiServer();

const POST = (port, p, body) => new Promise((resolve, reject) => {
  const d = JSON.stringify(body);
  const req = http.request({ host: 'localhost', port, path: p, method: 'POST',
    headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(d) } }, (res) => {
    let s = ''; res.on('data', c => s += c); res.on('end', () => resolve({ status: res.statusCode, body: s }));
  });
  req.on('error', reject);
  req.write(d); req.end();
});
const GET = (port, p) => new Promise((resolve, reject) => {
  const req = http.get({ host: 'localhost', port, path: p }, (res) => {
    let s = ''; res.on('data', c => s += c); res.on('end', () => resolve({ status: res.statusCode, body: s }));
  });
  req.on('error', reject);
});

const RUNS_PER_WF = 30;
const WORKFLOWS = [
  { id: 'wf-graph-bulk-v1', steps: 5 },
  { id: 'wf-file-upload-v1', steps: 5 },
  { id: 'wf-ai-rag-v1', steps: 7 },
];

async function main() {
  console.log('\n=== T13 RED → GREEN 3 Workflow Suite ===');
  console.log('work dir:', WORK_DIR);
  // api-server.js require 内部 server.listen 是异步，等 listening 取地址
  await new Promise((resolve, reject) => {
    const to = setTimeout(() => reject(new Error('api-server listening timeout')), 15000);
    if (srv.address() && srv.address().port) { clearTimeout(to); return resolve(); }
    srv.once('listening', () => { clearTimeout(to); resolve(); });
    srv.once('error', (e) => { clearTimeout(to); reject(e); });
  });
  const port = srv.address().port;
  console.log('server port:', port);

  // ---- TR13.1 各 10 runs ok≥9，shape 统一 ----
  console.log('\n-- TR 13.1 --');
  const { getNebulaGraphAdapter, resetNebulaGraphAdapter } = require('../src/nebulagraph-adapter');
  // 清图谱，便于计数
  resetNebulaGraphAdapter();
  const adapter = getNebulaGraphAdapter();

  const allRuns = [];
  for (const wf of WORKFLOWS) {
    const wfRuns = [];
    for (let i = 0; i < RUNS_PER_WF; i++) {
      const r = await POST(port, '/ai/engine/workflow/execute', {
        workflow_id: wf.id,
        inputs: { seed: `${wf.id}-${i}` },
        trace_id: `trace-${wf.id}-${i}-${Date.now()}`
      });
      wfRuns.push(r);
    }
    allRuns.push({ wf, runs: wfRuns });

    const okCount = wfRuns.filter(r => {
      if (r.status !== 200) return false;
      try {
        const outer = JSON.parse(r.body);
        return outer.success === true && outer.data && outer.data.ok === true;
      } catch { return false; }
    }).length;
    test(`TR13.1 ${wf.id} ok≥27/30（实际 ${okCount}/${RUNS_PER_WF}）`, () => {
      assert.ok(okCount >= 27, `${wf.id} ok=${okCount} < 27`);
    });

    // shape 校验
    for (let i = 0; i < wfRuns.length; i++) {
      const r = wfRuns[i];
      test(`TR13.1 ${wf.id} run#${i} shape 统一`, () => {
        assert.strictEqual(r.status, 200, `status=${r.status} body=${r.body.slice(0, 300)}`);
        const outer = JSON.parse(r.body);
        assert.strictEqual(outer.success, true, `outer.success not true: ${r.body.slice(0,200)}`);
        const j = outer.data; // ok(res, result) → result 成为 data 段 (SPEC-8 shape 等价)
        assert.strictEqual(j.ok, true);
        assert.ok(j.data && typeof j.data === 'object', 'data missing');
        assert.ok(j.data.workflow_id, 'data.workflow_id missing');
        assert.ok(j.data.trace_id, 'data.trace_id missing');
        assert.ok(Array.isArray(j.data.steps), 'data.steps not array');
        assert.strictEqual(j.data.steps.length, wf.steps, `steps count not ${wf.steps} got ${j.data.steps.length}`);
        for (const s of j.data.steps) {
          assert.ok(s.id, 'step.id missing');
          assert.ok(s.name, 'step.name missing');
          assert.strictEqual(typeof s.retcode, 'number', 'step.retcode not number: ' + s.retcode);
          assert.strictEqual(typeof s.dur_ms, 'number', 'step.dur_ms not number: ' + s.dur_ms);
          assert.ok(s.artifacts && typeof s.artifacts === 'object', 'step.artifacts missing');
        }
        assert.ok(j.graph && typeof j.graph === 'object', 'graph missing');
        assert.ok(Array.isArray(j.graph.nodes), 'graph.nodes not array');
        assert.ok(Array.isArray(j.graph.edges), 'graph.edges not array');
      });
    }
  }

  // ---- TR13.2：COUNT(workflow_step vertex) ≥ 510 ----
  console.log('\n-- TR 13.2 --');
  const allStepNodes = adapter.listNodes({ kind: 'workflow_step' });
  const minExpected = (5 + 5 + 7) * RUNS_PER_WF; // 510 when RUNS_PER_WF=30
  test(`TR13.2 workflow_step vertex count ≥ ${minExpected}（实际 ${allStepNodes.length}）`, () => {
    assert.ok(allStepNodes.length >= minExpected,
      `workflow_step vertex count=${allStepNodes.length} < ${minExpected}`);
  });

  // ---- TR13.3：抽样 10% steps，每个 step 至少 1 条 runs_on 边 ----
  console.log('\n-- TR 13.3 --');
  const sampleSize = Math.max(1, Math.floor(allStepNodes.length * 0.10));
  // 打乱取样
  const shuffled = allStepNodes.slice().sort(() => Math.random() - 0.5).slice(0, sampleSize);
  const allEdges = adapter.listEdges();
  let sampleOk = 0;
  for (const step of shuffled) {
    const has = allEdges.some(e => e.kind === 'runs_on' && (e.from === step.id || e.to === step.id));
    if (has) sampleOk++;
  }
  test(`TR13.3 runs_on 抽样覆盖（${sampleOk}/${sampleSize} sample steps have runs_on）`, () => {
    assert.strictEqual(sampleOk, sampleSize,
      `some sample steps miss runs_on edges: ${sampleOk}/${sampleSize}`);
  });

  // ---- 附加：slo_snapshot 节点与 snapshot 边存在 ----
  const sloNodes = adapter.listNodes({ kind: 'slo_snapshot' });
  const minSlo = RUNS_PER_WF * 3; // 每 run 至少每 step 至少 1 slo，这里 MIN=RUNS_PER_WF*3
  test(`附加：slo_snapshot 节点数 ≥ ${minSlo}（每 workflow 至少 RUNS_PER_WF 个）`, () => {
    assert.ok(sloNodes.length >= minSlo, `slo_snapshot count=${sloNodes.length} < ${minSlo}`);
  });

  const exitCode = failed > 0 ? 1 : 0;
  console.log(`\n===== T13 Result: ${passed} passed, ${failed} failed =====`);

  // 服务器关闭兜底（1.5s 超时后直接进程退出，避免 Node HTTP 残留 handles 挂起）
  const done = () => process.exit(exitCode);
  try { srv.close(done); } catch(e) { done(); }
  setTimeout(done, 1500).unref();
}

main().catch(e => {
  console.error('T13 suite fatal:', e);
  process.exit(1);
});
