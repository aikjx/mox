'use strict';

/**
 * O4 企业级 SLO 四窗口追踪器（v4）
 *
 * v4 关键改进：
 *   (a) summarizeFromSortedIndex 对 4 窗口合为 1 趟 idx 遍历（原 4 趟）；
 *       同时 per-domain 不再预先 push lats，改为聚合数值后按需做域内排序。
 *   (b) 维持头指针环形缓冲 O(1) record。
 *   (c) 预期 snapshot 100 次预算 50k 样本 ≤ 2000ms。
 */

const WINDOWS_MS = {
  '1m':   60 * 1000,
  '5m':   5 * 60 * 1000,
  '15m':  15 * 60 * 1000,
  '1h':   60 * 60 * 1000,
};
const WINDOW_ORDER = ['1m', '5m', '15m', '1h'];
const WINDOW_VALUES = WINDOW_ORDER.map(w => WINDOWS_MS[w]);
const W = WINDOW_ORDER.length; // 4

const USE_LEGACY = process.env.SLO_LEGACY_RING === '1';

class SloTracker {
  constructor({ maxRingSize = 50_000 } = {}) {
    this._maxRing = Math.max(100, Number(maxRingSize) || 0) | 0;
    this._domains = new Set();
    if (USE_LEGACY) {
      this._ring = [];
    } else {
      this._buf = new Array(this._maxRing);
      this._write = 0;
      this._count = 0;
    }
  }

  record(key, latencyMs, ok = true, tenant) {
    const item = {
      ts: Date.now(),
      key: String(key),
      latency_ms: Math.max(0, Number(latencyMs) || 0),
      ok: !!ok,
    };
    if (tenant != null) item.tenant = String(tenant);
    this._domains.add(item.key);
    if (USE_LEGACY) {
      this._ring.push(item);
      if (this._ring.length > this._maxRing) {
        const drop = this._ring.length - this._maxRing;
        this._ring.splice(0, drop);
      }
    } else {
      this._buf[this._write] = item;
      this._write = (this._write + 1) % this._maxRing;
      if (this._count < this._maxRing) this._count++;
    }
  }

  reset() {
    if (USE_LEGACY) this._ring.length = 0;
    else {
      for (let i = 0; i < this._maxRing; i++) this._buf[i] = undefined;
      this._write = 0;
      this._count = 0;
    }
    this._domains.clear();
  }

  listDomains() {
    return [...this._domains].sort();
  }

  _iterate(fn) {
    if (USE_LEGACY) {
      for (let i = 0; i < this._ring.length; i++) fn(this._ring[i]);
      return;
    }
    const cap = this._maxRing;
    const count = this._count;
    let start = (this._write - count + cap) % cap;
    for (let i = 0; i < count; i++) {
      const idx = (start + i) % cap;
      fn(this._buf[idx]);
    }
  }

  snapshot(opts = {}) {
    const now = Date.now();
    const domainsFilter = Array.isArray(opts.domains) && opts.domains.length > 0
      ? new Set(opts.domains)
      : null;
    const tenantFilter = opts.tenant ? String(opts.tenant) : null;
    const objP99 = typeof opts.objectiveP99Ms === 'number' && opts.objectiveP99Ms > 0 ? opts.objectiveP99Ms : 1000;
    const objSucc = typeof opts.objectiveSuccess === 'number' ? opts.objectiveSuccess : 0.99;

    // 单趟遍历：(1) 过滤域/租户；(2) 标记 mask（哪 4 窗口）；(3) per-domain 累积。
    const approxN = this._countApprox();
    const lats = new Array(approxN);
    const masks = new Uint8Array(approxN);
    const oks = new Uint8Array(approxN);
    const keys = new Array(approxN);  // 只在需要 per-domain 时保留引用

    const wCount = [0, 0, 0, 0];
    const domainAgg = new Map();

    let trueN = 0;
    this._iterate(ev => {
      if (domainsFilter && !domainsFilter.has(ev.key)) return;
      if (tenantFilter && ev.tenant !== tenantFilter) return;
      const age = now - ev.ts;
      let mask = 0;
      for (let w = 0; w < W; w++) {
        if (age <= WINDOW_VALUES[w]) { mask |= (1 << w); wCount[w]++; }
      }
      lats[trueN] = ev.latency_ms;
      masks[trueN] = mask;
      oks[trueN] = ev.ok ? 1 : 0;
      keys[trueN] = ev.key;
      // per-domain：仅聚合全局（非窗口）的 count/ok/sum/min/max 和 lat 数组（后续 sort 一次）
      if (!domainAgg.has(ev.key)) {
        domainAgg.set(ev.key, { count: 0, ok: 0, sum: 0, min: Infinity, max: -Infinity, lats: [] });
      }
      const agg = domainAgg.get(ev.key);
      const v = ev.latency_ms;
      agg.count++; if (ev.ok) agg.ok++; agg.sum += v; if (v < agg.min) agg.min = v; if (v > agg.max) agg.max = v;
      // 不存所有 lat（占 O(N) 内存重复）→ 改用索引重映射：存 index of entry → lat 数组
      agg._idxPtr = (agg._idxPtr || 0) + 1;  // 占位（稍后用 global idx 重写 per-domain）
      trueN++;
    });
    if (trueN < lats.length) { lats.length = trueN; keys.length = trueN; }

    // 全局索引排序（按 lat 升序），只排序 1 次
    const idx = new Array(trueN);
    for (let k = 0; k < trueN; k++) idx[k] = k;
    idx.sort((a, b) => lats[a] - lats[b]);

    // 1 趟 idx 遍历：同时填充 4 窗口各自的 collected lat（保持升序）
    const wCollected = [
      wCount[0] > 0 ? new Array(wCount[0]) : null,
      wCount[1] > 0 ? new Array(wCount[1]) : null,
      wCount[2] > 0 ? new Array(wCount[2]) : null,
      wCount[3] > 0 ? new Array(wCount[3]) : null,
    ];
    const wPtr = [0, 0, 0, 0];
    const wOk = [0, 0, 0, 0];
    const wSum = [0, 0, 0, 0];
    const wMin = [Infinity, Infinity, Infinity, Infinity];
    const wMax = [-Infinity, -Infinity, -Infinity, -Infinity];

    for (let p = 0; p < trueN; p++) {
      const k = idx[p];
      const v = lats[k];
      const ok = oks[k];
      const m = masks[k];
      for (let w = 0; w < W; w++) {
        if ((m & (1 << w)) === 0) continue;
        const wp = wPtr[w];
        wCollected[w][wp] = v;
        wPtr[w] = wp + 1;
        wSum[w] += v;
        if (v < wMin[w]) wMin[w] = v;
        if (v > wMax[w]) wMax[w] = v;
        if (ok) wOk[w]++;
      }
    }

    const windows = {};
    let overallStatus = 'ok';
    for (let w = 0; w < W; w++) {
      const s = wCount[w] === 0
        ? emptySummary()
        : buildFromAscArray(wCollected[w], wPtr[w], wOk[w], wSum[w], wMin[w], wMax[w]);
      s.status = evaluateStatus(s, objP99, objSucc);
      if (s.status === 'violated') overallStatus = 'violated';
      else if (s.status === 'warning' && overallStatus === 'ok') overallStatus = 'warning';
      windows[WINDOW_ORDER[w]] = s;
    }

    // per-domain：基于 idx 排序后的顺序，每域用 ptr 递增填充（不再 per-domain sort）
    // 准备每域结果数组（升序 lat）
    const domLatArrays = new Map();
    const domMeta = new Map();
    for (const [k, agg] of domainAgg) {
      domLatArrays.set(k, new Array(agg.count));
      domMeta.set(k, { ptr: 0, ok: 0, sum: 0, min: Infinity, max: -Infinity, count: agg.count });
    }
    for (let p = 0; p < trueN; p++) {
      const k = idx[p];
      const dom = keys[k];
      // 域可能被 filter 排除（domainAgg 只含可见域）
      const arr = domLatArrays.get(dom);
      if (!arr) continue;
      const meta = domMeta.get(dom);
      const v = lats[k];
      arr[meta.ptr] = v;
      meta.ptr++;
      meta.sum += v;
      if (v < meta.min) meta.min = v;
      if (v > meta.max) meta.max = v;
      if (oks[k]) meta.ok++;
    }
    const perDomain = {};
    for (const name of [...domainAgg.keys()].sort()) {
      const meta = domMeta.get(name);
      const arr = domLatArrays.get(name);
      perDomain[name] = buildFromAscArray(arr, meta.count, meta.ok, meta.sum, meta.min, meta.max);
      perDomain[name].status = evaluateStatus(perDomain[name], objP99, objSucc);
    }

    return {
      schema_version: 'system-slo-v1',
      generated_at: new Date(now).toISOString(),
      sample_count: trueN,
      ring_capacity: this._maxRing,
      objective: { p99_ms: objP99, success_rate: objSucc },
      filters: {
        domains: domainsFilter ? [...domainsFilter].sort() : null,
        tenant: tenantFilter || null,
      },
      status: overallStatus,
      windows,
      per_domain: perDomain,
    };
  }

  _countApprox() {
    if (USE_LEGACY) return this._ring.length;
    return this._count;
  }
}

function emptySummary() {
  return {
    count: 0, success_count: 0, success_rate: null, error_count: 0,
    latency_avg_ms: null, latency_p50_ms: null, latency_p95_ms: null, latency_p99_ms: null,
    latency_min_ms: null, latency_max_ms: null, status: 'no_data',
  };
}

function quantileAlreadySorted(sorted, n, q) {
  if (n === 0) return null;
  if (n === 1) return sorted[0];
  const pos = (n - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.min(lo + 1, n - 1);
  const frac = pos - lo;
  return sorted[lo] * (1 - frac) + sorted[hi] * frac;
}

function buildFromAscArray(ascLats, n, okCount, sum, min, max) {
  if (n === 0) return emptySummary();
  return {
    count: n,
    success_count: okCount,
    success_rate: okCount / n,
    error_count: n - okCount,
    latency_avg_ms: sum / n,
    latency_p50_ms: quantileAlreadySorted(ascLats, n, 0.50),
    latency_p95_ms: quantileAlreadySorted(ascLats, n, 0.95),
    latency_p99_ms: quantileAlreadySorted(ascLats, n, 0.99),
    latency_min_ms: min,
    latency_max_ms: max,
    status: 'ok',
  };
}

// 对外保持兼容的辅助 API
function quantile(sorted, q) { return quantileAlreadySorted(sorted, sorted.length, q); }

function summarize(samples) {
  const n = samples.length;
  if (n === 0) return emptySummary();
  const lats = new Array(n);
  let okCount = 0, sum = 0, min = Infinity, max = -Infinity;
  for (let i = 0; i < n; i++) {
    const s = samples[i];
    const v = s.latency_ms;
    lats[i] = v;
    if (s.ok) okCount++;
    sum += v; if (v < min) min = v; if (v > max) max = v;
  }
  lats.sort((a, b) => a - b);
  return buildFromAscArray(lats, n, okCount, sum, min, max);
}

function evaluateStatus(summary, objP99Ms, objSucc) {
  if (summary.count === 0) return 'no_data';
  const p99 = summary.latency_p99_ms;
  const sr = summary.success_rate;
  if (p99 != null && p99 > objP99Ms) return 'violated';
  if (sr != null && sr < objSucc) return 'violated';
  const p99Buf = objP99Ms * 0.90;
  const srBuf = objSucc + (1 - objSucc) * 0.10;
  if ((p99 != null && p99 > p99Buf) || (sr != null && sr < srBuf)) return 'warning';
  return 'ok';
}

module.exports = { SloTracker, WINDOWS_MS, WINDOW_ORDER, summarize, quantile, evaluateStatus };
