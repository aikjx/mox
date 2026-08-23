'use strict';
/**
 * H1 · 高并发治理 Benchmark（T3 AC-04）
 *   - 混合租户：NORMAL 80% / VIP 15% / ANONYMOUS 5%
 *   - 默认：QPS=200，SECONDS=60；可调 env: H1_QPS / H1_SECONDS
 *   - O2 补丁开关：env H1_ENABLE_TOKEN_BUCKET=1 启用 O2 TokenBucket；并可通过 H1_BUCKET_CFG='{"NORMAL":10,"VIP":50,"ANONYMOUS":2}' 配置
 *   - 输出：${HARNESS_DIR}/h1_{before|after}.csv
 *       ts_ms,ok_count,fail_count,rl_blocked,cb_open,p50_ms,p95_ms,p99_ms,mem_rss_kb
 *       summary,total_ok,total_fail,success_rate,rl_total_blocked,cb_open_count,p50_avg,p95_avg,p99_avg,mem_rss_kb
 */
const path = require('path');
const fs = require('fs');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
const SPECDIR = path.resolve(ROOT, '..', '..', '.trae', 'specs', '20260823-enterprise-compare-top-oss-ai-products-optimize');
const OUTDIR = process.env.H1_OUTDIR || path.join(SPECDIR, 'harness-data');
const DEFAULT_OUTFILE = process.env.H1_OUTFILE || (process.env.H1_ENABLE_TOKEN_BUCKET === '1' ? 'h1_after.csv' : 'h1_before.csv');
const OUTFILE = path.join(OUTDIR, DEFAULT_OUTFILE);

const QPS = Math.max(1, parseInt(process.env.H1_QPS || '200', 10));
const SECONDS = Math.max(2, parseInt(process.env.H1_SECONDS || '60', 10));
const TICK_MS = 1000;

// ---------------- 限流：内联滑动窗口（baseline）或 TokenBucket（after O2） ---------------
class SlidingWindowRL {
  constructor(cfg) { this.cfg = Object.assign({ window: 60000, max: 1e9 }, cfg); this.map = new Map(); }
  checkRateLimit(key) {
    const now = Date.now(); let e = this.map.get(key);
    if (!e || now > e.resetTime) { e = { count: 0, resetTime: now + this.cfg.window, blocked: false }; this.map.set(key, e); }
    e.count++;
    if (e.count > this.cfg.max) { e.blocked = true; return { allowed:false, resetMs: e.resetTime - now }; }
    return { allowed: true, remaining: this.cfg.max - e.count };
  }
}

class TokenBucket {
  constructor(capacity, tokensPerSec) {
    this.capacity = Math.max(1, capacity);
    this.tps = Math.max(0.0001, tokensPerSec);
    this.tokens = this.capacity;
    this.last = Date.now();
  }
  _refill() {
    const now = Date.now();
    const dt = (now - this.last) / 1000;
    if (dt > 0) {
      this.tokens = Math.min(this.capacity, this.tokens + dt * this.tps);
      this.last = now;
    }
  }
  tryTake(n = 1) {
    this._refill();
    if (this.tokens >= n) { this.tokens -= n; return true; }
    return false;
  }
}

class MultiTenantBucketRL {
  constructor(quotaMap) {
    // quotaMap: { TIER: qps } ; default if unset: 10 / 50 / 2 (NORMAL/VIP/ANONYMOUS)
    this.q = Object.assign({ NORMAL: 10, VIP: 50, ANONYMOUS: 2 }, quotaMap || {});
    this.buckets = new Map(); // tenantKey -> TokenBucket
  }
  checkRateLimit(key, tier = 'NORMAL') {
    const qps = this.q[tier] != null ? this.q[tier] : this.q.NORMAL;
    let b = this.buckets.get(key);
    if (!b) {
      b = new TokenBucket(qps, qps); // capacity=qps（burst≈1s），fill=qps/sec
      this.buckets.set(key, b);
    }
    const ok = b.tryTake(1);
    if (!ok) return { allowed: false, resetMs: 1000 };
    return { allowed: true, remaining: Math.floor(b.tokens) };
  }
}

let BUCKET_CFG;
try { BUCKET_CFG = process.env.H1_BUCKET_CFG ? JSON.parse(process.env.H1_BUCKET_CFG) : null; } catch (_) { BUCKET_CFG = null; }
const USE_BUCKET = process.env.H1_ENABLE_TOKEN_BUCKET === '1';

const rl = USE_BUCKET
  ? new MultiTenantBucketRL(BUCKET_CFG)
  : new SlidingWindowRL({ window: 60000, max: QPS * SECONDS * 2 }); // baseline：窗口足够大，主要测并发而非拦截

// 对外暴露，便于单元测试（或 O2 补丁测试时直接 new 外部传入）
function _getRateLimiter() { return rl; }

// ---------------- Mock LLM Route ----------------
const MOCK_PROVIDERS = [
  { id: 'mock-fast',   priority: 100, latencyMs: 80,  errorRate: 0.001 },
  { id: 'mock-stable', priority: 50,  latencyMs: 180, errorRate: 0.0001 },
  { id: 'local-1',     priority: 10,  latencyMs: 20,  errorRate: 0 },
];
async function routeChat() {
  const start = Date.now();
  const sorted = MOCK_PROVIDERS.slice().sort((a,b)=>b.priority-a.priority);
  for (const p of sorted) {
    const ok = Math.random() >= p.errorRate;
    const jitter = (Math.random() - 0.5) * 0.2 * p.latencyMs;
    const delay = Math.max(5, p.latencyMs + jitter);
    await new Promise(r => setTimeout(r, delay));
    if (ok) return { ok: true, latency_ms: Date.now() - start, provider_id: p.id };
  }
  return { ok: false, latency_ms: Date.now() - start, error: 'all providers failed' };
}

function pickTenant() {
  const r = Math.random();
  if (r < 0.80) return { id: 't-normal-1', tier: 'NORMAL', apikey: 'u-normal' };
  if (r < 0.95) return { id: 't-vip-1',   tier: 'VIP',    apikey: 'u-vip' };
  return                { id: 'anon',      tier: 'ANONYMOUS',apikey: null };
}

async function runOneRequest() {
  const tenant = pickTenant();
  const key = tenant.apikey ? `api:${tenant.apikey}` : `ip:127.0.0.1`;
  const rlRes = USE_BUCKET ? rl.checkRateLimit(key, tenant.tier) : rl.checkRateLimit(key);
  if (!rlRes.allowed) return { rl_blocked: 1, cb_open: 0, latency_ms: 1, ok: 0, fail: 0 };
  const res = await routeChat();
  return { rl_blocked: 0, cb_open: 0, latency_ms: res.latency_ms, ok: res.ok ? 1 : 0, fail: res.ok ? 0 : 1 };
}

function percentile(sortedArr, p) {
  if (!sortedArr.length) return 0;
  const i = Math.min(sortedArr.length - 1, Math.floor(p / 100 * sortedArr.length));
  return sortedArr[i];
}

async function main() {
  fs.mkdirSync(OUTDIR, { recursive: true });
  const lines = [['ts_ms','ok_count','fail_count','rl_blocked','cb_open','p50_ms','p95_ms','p99_ms','mem_rss_kb'].join(',')];
  console.log(`[H1] start QPS=${QPS} SECONDS=${SECONDS} mode=${USE_BUCKET ? 'TokenBucket' : 'SlidingWindow'} OUTFILE=${OUTFILE}`);

  const totalIntervalMs = SECONDS * 1000;
  const intervalPerReq = 1000 / QPS;
  const totalReqs = QPS * SECONDS;

  let perSec = { ok: 0, fail: 0, rl_blocked: 0, cb_open: 0, lats: [] };
  let totals = { ok: 0, fail: 0, rl_blocked: 0, cb_open: 0, lats: [] };

  const startTs = Date.now();
  let nextTickEnd = startTs + TICK_MS;
  const scheduled = new Array(totalReqs);
  for (let i = 0; i < totalReqs; i++) scheduled[i] = startTs + i * intervalPerReq;

  let inflight = 0, idx = 0, stopped = false;

  function pump() {
    if (stopped) return;
    const now = Date.now();
    while (idx < scheduled.length && scheduled[idx] <= now && inflight < 512) {
      idx++; inflight++;
      runOneRequest()
        .then(r => { inflight--; perSec.ok += r.ok; perSec.fail += r.fail; perSec.rl_blocked += r.rl_blocked; perSec.cb_open += r.cb_open; perSec.lats.push(r.latency_ms); })
        .catch(() => { inflight--; perSec.fail++; });
    }
    if (now >= nextTickEnd) {
      const sortedLats = perSec.lats.slice().sort((a,b)=>a-b);
      lines.push([
        nextTickEnd, perSec.ok, perSec.fail, perSec.rl_blocked, perSec.cb_open,
        percentile(sortedLats, 50), percentile(sortedLats, 95), percentile(sortedLats, 99),
        Math.round(process.memoryUsage().rss / 1024),
      ].join(','));
      totals.ok += perSec.ok; totals.fail += perSec.fail; totals.rl_blocked += perSec.rl_blocked;
      totals.cb_open += perSec.cb_open; totals.lats = totals.lats.concat(perSec.lats);
      perSec = { ok:0, fail:0, rl_blocked:0, cb_open:0, lats:[] };
      nextTickEnd += TICK_MS;
    }
    if ((now - startTs) > totalIntervalMs + 5000 && inflight === 0 && idx >= scheduled.length) {
      stop(); return;
    }
    setImmediate(pump);
  }

  function stop() {
    if (stopped) return;
    stopped = true;
    const allSorted = totals.lats.slice().sort((a,b)=>a-b);
    const total = totals.ok + totals.fail;
    const success_rate = total === 0 ? 0 : totals.ok / total;
    lines.push([
      'summary', totals.ok, totals.fail, success_rate.toFixed(6),
      totals.rl_blocked, totals.cb_open,
      percentile(allSorted, 50), percentile(allSorted, 95), percentile(allSorted, 99),
      Math.round(process.memoryUsage().rss / 1024),
    ].join(','));
    fs.writeFileSync(OUTFILE, lines.join('\n') + '\n', 'utf8');
    console.log(`[H1] total=${total} ok=${totals.ok} fail=${totals.fail} SR=${(success_rate*100).toFixed(3)}% blocked=${totals.rl_blocked} p99=${percentile(allSorted,99)}ms`);
    console.log(`[H1] CSV (${lines.length} lines) -> ${OUTFILE}`);
    // TR-3.1 + TR-3.2
    assert.ok(lines.length - 2 >= SECONDS, `tick rows too few: ${lines.length - 2} vs ${SECONDS}`); // header + summary in between 算 2
    assert.ok(totals.ok > 0, 'no successful requests');
    assert.ok(total === 0 || totals.fail / total < 0.01, `non-rl fail too high: ${totals.fail}/${total}`);
    process.exit(0);
  }

  setImmediate(pump);
}

if (require.main === module) {
  main().catch(e => { console.error('[H1] FATAL', e); process.exit(1); });
} else {
  module.exports = {
    main,
    routeChat,
    TokenBucket,
    MultiTenantBucketRL,
    SlidingWindowRL,
    _getRateLimiter,
  };
}
