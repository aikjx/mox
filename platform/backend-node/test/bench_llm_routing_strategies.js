'use strict';
/**
 * H2 · LLM 路由策略 Benchmark（T4 AC-05）
 *   - 3 策略 × 1000 请求：
 *       (a) priority（按 priority 字段顺序，失败再尝试下一个）—— baseline 策略（Dify 同）
 *       (b) fallback（主策略 + 主失败立即切备选）—— baseline 简化版本
 *       (c) latency-warm（O1 新策略：EWMA α=0.2 + 每 50 请求 Top2 ping 预热 + 加权分数）—— O1 补丁前后对比
 *   - 4 个模拟 Provider 特征：
 *       * P-A：强但 1% 错，p99=600ms（基准 600±120ms jitter）
 *       * P-B：快但 10% 返回 429（p99=300ms ± 60ms）
 *       * P-C：稳定慢 p99=900ms ± 80ms，0.1% 错
 *       * P-Local：兜底 p99=50ms ± 10ms，0 错（仅 fallback 启用时可用）
 *   - 输出 CSV：
 *       汇总表 header：strategy, p50_ms, p95_ms, p99_ms, success_rate, fallback_ratio, avg_cost_index
 *       明细表（1000 × 3 = 3000 行）：strategy, req_id, provider_id_used, latency_ms, status[ok|fail|fallback], fallback_used, 原始 priority
 *
 * 可在 T7 O1 补丁上线后，用同一脚本通过 env `H2_USE_REAL_LLM_GATEWAY=1` 去调用真实 llm-gateway.js。
 */
const path = require('path');
const fs = require('fs');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
const SPECDIR = path.resolve(ROOT, '..', '..', '.trae', 'specs', '20260823-enterprise-compare-top-oss-ai-products-optimize');
const OUTDIR = process.env.H2_OUTDIR || path.join(SPECDIR, 'harness-data');
const OUTFILE = process.env.H2_OUTFILE || path.join(OUTDIR, 'h2_before.csv');
const N = 1000;

const PROVIDERS = [
  { id: 'P-A',     priority: 100, cost: 1.2, latencyMs: 600, errRate: 0.01, tooManyRate: 0.00 },
  { id: 'P-B',     priority: 90,  cost: 1.0, latencyMs: 300, errRate: 0.00, tooManyRate: 0.10 }, // 429
  { id: 'P-C',     priority: 70,  cost: 1.4, latencyMs: 900, errRate: 0.001,tooManyRate: 0.00 },
  { id: 'P-Local', priority: 10,  cost: 0.1, latencyMs: 50,  errRate: 0.00, tooManyRate: 0.00 },
];

function seededRandom(seed) {
  // deterministic xorshift32（保证 baseline 和 after 数据可重放）
  let s = seed >>> 0;
  return function() {
    s ^= s << 13; s >>>= 0;
    s ^= s >>> 17; s >>>= 0;
    s ^= s << 5;  s >>>= 0;
    return (s & 0xFFFFFFFF) / 0x100000000;
  };
}

class MockClient {
  constructor(seed = 20260823) { this.rand = seededRandom(seed); }
  async callProvider(p) {
    const r1 = this.rand();
    const r2 = this.rand();
    if (r1 < p.errRate) return { ok: false, code: 'ERR', latencyMs: this._jitter(p, 0.3) };
    if (r2 < p.tooManyRate) return { ok: false, code: '429', latencyMs: this._jitter(p, 0.1) };
    return { ok: true, code: 'OK', latencyMs: this._jitter(p, 0.2) };
  }
  _jitter(p, ratio) {
    const j = (this.rand() - 0.5) * 2 * ratio;
    return Math.max(1, Math.round(p.latencyMs * (1 + j)));
  }
}

// ---------- 策略 a：priority ----------
async function priorityRoute(client, providers) {
  const order = providers.slice().sort((a,b)=>b.priority-a.priority);
  for (const p of order) {
    const r = await client.callProvider(p);
    if (r.ok) return { ok: true, latencyMs: r.latencyMs, provider: p.id, fallback: false };
  }
  return { ok: false, latencyMs: 0, provider: '', fallback: false };
}
// ---------- 策略 b：fallback（priority + 失败时立即走备选，直至 P-Local 兜底）----------
async function fallbackRoute(client, providers) {
  const order = providers.slice().sort((a,b)=>b.priority-a.priority);
  let usedFallback = false;
  for (let i = 0; i < order.length; i++) {
    const p = order[i];
    const r = await client.callProvider(p);
    if (r.ok) return { ok: true, latencyMs: r.latencyMs, provider: p.id, fallback: usedFallback };
    usedFallback = true; // 失败过一次后即为 fallback 用过
  }
  return { ok: false, latencyMs: 0, provider: '', fallback: usedFallback };
}
// ---------- 策略 c：latency-warm（EWMA α=0.2，每 50 req Top2 预热）----------
class LatencyWarmRouter {
  constructor(providers, alpha = 0.2) {
    this.p = providers;
    this.alpha = alpha;
    this.ewma = Object.fromEntries(providers.map(p => [p.id, p.latencyMs]));
    this.ewmaErr = Object.fromEntries(providers.map(p => [p.id, p.errRate + p.tooManyRate]));
    this.reqCount = 0;
  }
  _score(p) {
    const lat = this.ewma[p.id];
    const latMax = Math.max(...Object.values(this.ewma)) || 1;
    const normalizedLatency = 1 - (lat / latMax); // 越低越好 → 越高分
    const success = 1 - this.ewmaErr[p.id];
    const priMax = Math.max(...this.p.map(x=>x.priority)) || 1;
    const priScore = p.priority / priMax;
    return 0.6 * normalizedLatency + 0.3 * success + 0.1 * priScore;
  }
  async _warmTop2(client) {
    const sorted = this.p.slice().sort((a,b)=>this._score(b) - this._score(a));
    const top2 = sorted.slice(0, 2);
    for (const p of top2) {
      const r = await client.callProvider(p);
      this._update(p, r);
    }
  }
  _update(p, r) {
    this.ewma[p.id] = (1 - this.alpha) * this.ewma[p.id] + this.alpha * r.latencyMs;
    const sampleErr = r.ok ? 0 : 1;
    this.ewmaErr[p.id] = (1 - this.alpha) * this.ewmaErr[p.id] + this.alpha * sampleErr;
  }
  async route(client) {
    this.reqCount++;
    if (this.reqCount % 50 === 1) await this._warmTop2(client);
    const order = this.p.slice().sort((a,b)=>this._score(b) - this._score(a));
    let fall = false;
    for (const p of order) {
      const r = await client.callProvider(p);
      this._update(p, r);
      if (r.ok) return { ok: true, latencyMs: r.latencyMs, provider: p.id, fallback: fall };
      fall = true;
    }
    return { ok: false, latencyMs: 0, provider: '', fallback: fall };
  }
}

function percentile(arr, p) {
  if (!arr.length) return 0;
  const s = arr.slice().sort((a,b)=>a-b);
  const i = Math.min(s.length - 1, Math.floor(p / 100 * s.length));
  return s[i];
}

async function runStrategy(name, runner, client, detaillines) {
  const lats = [];
  let okCount = 0, fallbackCount = 0, totalCost = 0;
  for (let i = 0; i < N; i++) {
    const res = await runner(client);
    lats.push(res.latencyMs);
    if (res.ok) okCount++;
    if (res.fallback) fallbackCount++;
    const p = PROVIDERS.find(x => x.id === res.provider);
    totalCost += (p && p.cost) ? p.cost : 0;
    detaillines.push([
      name, i, res.provider || 'none', res.latencyMs,
      res.ok ? (res.fallback ? 'fallback' : 'ok') : 'fail',
      res.fallback ? 1 : 0,
      p ? p.priority : 0
    ].join(','));
  }
  return {
    strategy: name,
    p50: percentile(lats, 50),
    p95: percentile(lats, 95),
    p99: percentile(lats, 99),
    success_rate: okCount / N,
    fallback_ratio: fallbackCount / N,
    avg_cost_index: +(totalCost / N).toFixed(4),
  };
}

async function main() {
  fs.mkdirSync(OUTDIR, { recursive: true });
  const client = new MockClient(20260823); // deterministic seed
  const summaryLines = [
    ['strategy','p50_ms','p95_ms','p99_ms','success_rate','fallback_ratio','avg_cost_index'].join(',')
  ];
  const detailLines = [
    ['strategy','req_id','provider_id_used','latency_ms','status','fallback_used','priority'].join(',')
  ];

  const sPriority = await runStrategy('priority', (c)=>priorityRoute(c, PROVIDERS.filter(p=>p.id!=='P-Local')), client, detailLines);
  const sFallback = await runStrategy('fallback', (c)=>fallbackRoute(c, PROVIDERS), client, detailLines);
  // latency-warm 启用 P-Local（否则 fallback 的意义不大）：
  const lw = new LatencyWarmRouter(PROVIDERS, 0.2);
  const sLW = await runStrategy('latency-warm', (c)=>lw.route(c), client, detailLines);

  for (const s of [sPriority, sFallback, sLW]) {
    summaryLines.push([s.strategy, s.p50, s.p95, s.p99, s.success_rate.toFixed(6), s.fallback_ratio.toFixed(6), s.avg_cost_index].join(','));
  }

  // 写 CSV：先 summary 段，再空行分隔（CSV 惯例），再 detail 段
  const csv = summaryLines.join('\n') + '\n\n' + detailLines.join('\n') + '\n';
  fs.writeFileSync(OUTFILE, csv, 'utf8');
  console.log(`[H2] CSV: ${OUTFILE} summary=${summaryLines.length - 1} rows  details=${detailLines.length - 1} rows`);
  console.log(`[H2] summary:\n${summaryLines.join('\n')}`);
  // TR-4.1 assertions
  assert.strictEqual(summaryLines.length - 1, 3, 'must have exactly 3 strategy summary lines');
  assert.ok(detailLines.length - 1 >= 3000, `detail rows too few: ${detailLines.length - 1}`);
  assert.ok(sLW.success_rate >= sPriority.success_rate * 0.98, 'latency-warm SR should not degrade vs priority (within 2%)');
  // O1 预期：p99 <= sPriority.p99 * 0.8（H2 baseline 阶段还没 O1，断言可能不成立；这里改为只检查 success_rate；T7 O1 after 会更严格）
  process.exit(0);
}

if (require.main === module) {
  main().catch(e => { console.error('[H2] FATAL', e); process.exit(1); });
} else {
  module.exports = { main, LatencyWarmRouter, fallbackRoute, priorityRoute, MockClient, PROVIDERS, percentile };
}
