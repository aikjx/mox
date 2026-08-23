'use strict';

/**
 * O4 补丁：企业级 SLO 四窗口追踪器
 *
 *   对照矩阵（T10）维度 14「SLO/SLA 可观测」差距：Dify/LangGraph/Flowise/AutoGen 仅
 *   暴露基础 Prometheus 指标，无结构化 JSON 端点。璇玑补齐：
 *     1. 四窗口滑窗：1m / 5m / 15m / total（对齐 Dify 企业版 SLO 看板语义）
 *     2. 分位数：p50 / p95 / p99 精确估计（小样本精确排序，避免近似估计器方差）
 *     3. 命名域：每租户/每模块独立 SLO 集，按 key 聚合，可横向扩展 O8 仪表盘
 *     4. 统一契约：SystemSloSnapshot，与 GET /system/slo 返回形状完全一致
 *
 *   用法：
 *     const tracker = new SloTracker();
 *     tracker.record('chat', 120, true);            // latency_ms + ok
 *     tracker.record('chat', 5000, false, 'T-001'); // 按租户维度
 *     tracker.snapshot();                            // → SystemSloSnapshot
 */

const WINDOWS_MS = {
  '1m':   60 * 1000,
  '5m':   5 * 60 * 1000,
  '15m':  15 * 60 * 1000,
  total:  Number.MAX_SAFE_INTEGER,
};
const WINDOW_ORDER = ['1m', '5m', '15m', 'total'];

class SloTracker {
  constructor({ maxRingSize = 50_000 } = {}) {
    /** @type {{ts:number, key:string, latency_ms:number, ok:boolean, tenant?:string}[]} */
    this._ring = [];
    this._maxRing = Math.max(100, Number(maxRingSize) || 0) | 0;
    this._domains = new Set();
  }

  /** 每一笔事件进来：写入环形缓冲；超过上限丢弃最旧样本（total 也会掉尾，符合 bounded memory） */
  record(key, latencyMs, ok = true, tenant) {
    const item = {
      ts: Date.now(),
      key: String(key),
      latency_ms: Math.max(0, Number(latencyMs) || 0),
      ok: !!ok,
    };
    if (tenant != null) item.tenant = String(tenant);
    this._domains.add(item.key);
    this._ring.push(item);
    if (this._ring.length > this._maxRing) {
      // drop oldest head
      const drop = this._ring.length - this._maxRing;
      this._ring.splice(0, drop);
    }
  }

  /** 可选：重置追踪（用于测试基线） */
  reset() {
    this._ring.length = 0;
    this._domains.clear();
  }

  /** 域名列表，用于 snapshot 遍历时稳定顺序 */
  listDomains() {
    return [...this._domains].sort();
  }

  /**
   * 生成 SystemSloSnapshot（O4 /system/slo 返回的主体）
   *   opts:
   *     domains?: string[]     只取指定域名；缺省 = 全部
   *     tenant?: string        只取指定租户事件；缺省 = 全租户聚合
   *     objectiveP99Ms?: int   SLO 目标 P99（毫秒），用于 status 判断。默认 1000ms。
   *     objectiveSuccess?:num  SLO 目标成功率，默认 0.99 (99%)。
   */
  snapshot(opts = {}) {
    const now = Date.now();
    const domainsFilter = Array.isArray(opts.domains) && opts.domains.length > 0
      ? new Set(opts.domains)
      : null;
    const tenantFilter = opts.tenant ? String(opts.tenant) : null;
    const objP99 = typeof opts.objectiveP99Ms === 'number' && opts.objectiveP99Ms > 0 ? opts.objectiveP99Ms : 1000;
    const objSucc = typeof opts.objectiveSuccess === 'number' ? opts.objectiveSuccess : 0.99;

    // 1) 筛选：按域名 + 租户
    const filtered = [];
    for (const ev of this._ring) {
      if (domainsFilter && !domainsFilter.has(ev.key)) continue;
      if (tenantFilter && ev.tenant !== tenantFilter) continue;
      filtered.push(ev);
    }

    // 2) 按窗口切分
    const byWindow = {};
    for (const w of WINDOW_ORDER) byWindow[w] = [];
    for (const ev of filtered) {
      const age = now - ev.ts;
      for (const w of WINDOW_ORDER) {
        if (age <= WINDOWS_MS[w]) byWindow[w].push(ev);
      }
    }

    const windows = {};
    let overallStatus = 'ok';
    for (const w of WINDOW_ORDER) {
      const s = summarize(byWindow[w]);
      s.status = evaluateStatus(s, objP99, objSucc);
      if (s.status === 'violated') overallStatus = 'violated';
      else if (s.status === 'warning' && overallStatus === 'ok') overallStatus = 'warning';
      windows[w] = s;
    }

    return {
      schema_version: 'system-slo-v1',
      generated_at: new Date(now).toISOString(),
      sample_count: filtered.length,
      ring_capacity: this._maxRing,
      objective: { p99_ms: objP99, success_rate: objSucc },
      filters: {
        domains: domainsFilter ? [...domainsFilter].sort() : null,
        tenant: tenantFilter || null,
      },
      status: overallStatus,
      windows,
      per_domain: this._perDomainSummary(filtered, objP99, objSucc),
    };
  }

  _perDomainSummary(filtered, objP99, objSucc) {
    const groups = new Map();
    for (const ev of filtered) {
      if (!groups.has(ev.key)) groups.set(ev.key, []);
      groups.get(ev.key).push(ev);
    }
    const out = {};
    for (const k of [...groups.keys()].sort()) {
      const s = summarize(groups.get(k));
      s.status = evaluateStatus(s, objP99, objSucc);
      out[k] = s;
    }
    return out;
  }
}

function summarize(samples) {
  const n = samples.length;
  if (n === 0) {
    return {
      count: 0,
      success_count: 0,
      success_rate: null,
      error_count: 0,
      latency_avg_ms: null,
      latency_p50_ms: null,
      latency_p95_ms: null,
      latency_p99_ms: null,
      latency_min_ms: null,
      latency_max_ms: null,
      status: 'no_data',
    };
  }
  let okCount = 0;
  const lats = new Array(n);
  for (let i = 0; i < n; i++) {
    const s = samples[i];
    lats[i] = s.latency_ms;
    if (s.ok) okCount++;
  }
  lats.sort((a, b) => a - b);
  const errCount = n - okCount;
  return {
    count: n,
    success_count: okCount,
    success_rate: n > 0 ? okCount / n : null,
    error_count: errCount,
    // 小样本精确排序（精确分位数，避免 HdrHistogram 依赖）
    latency_avg_ms: lats.reduce((a, b) => a + b, 0) / n,
    latency_p50_ms: quantile(lats, 0.50),
    latency_p95_ms: quantile(lats, 0.95),
    latency_p99_ms: quantile(lats, 0.99),
    latency_min_ms: lats[0],
    latency_max_ms: lats[n - 1],
    status: 'ok',
  };
}

/** 线性插值分位数（已排序数组） */
function quantile(sorted, q) {
  const n = sorted.length;
  if (n === 0) return null;
  if (n === 1) return sorted[0];
  const pos = (n - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.min(lo + 1, n - 1);
  const frac = pos - lo;
  return sorted[lo] * (1 - frac) + sorted[hi] * frac;
}

function evaluateStatus(summary, objP99Ms, objSucc) {
  if (summary.count === 0) return 'no_data';
  const p99 = summary.latency_p99_ms;
  const sr = summary.success_rate;
  // violated：任一硬目标失败
  if (p99 != null && p99 > objP99Ms) return 'violated';
  if (sr != null && sr < objSucc) return 'violated';
  // warning：距目标 ≤10% buffer
  const p99Buf = objP99Ms * 0.90;
  const srBuf = objSucc + (1 - objSucc) * 0.10;
  if ((p99 != null && p99 > p99Buf) || (sr != null && sr < srBuf)) return 'warning';
  return 'ok';
}

module.exports = { SloTracker, WINDOWS_MS, WINDOW_ORDER, summarize, quantile, evaluateStatus };
