'use strict';

/**
 * O7 补丁：图谱执行 P99 上报聚合器（Graph Metrics Reporter）
 *
 *   对照矩阵（T10）维度 14「SLO/SLA 可观测」+ 维度 6「图谱算法正确性」差距：
 *   Dify/LangGraph/Flowise/AutoGen 仅在 HTTP 入口暴露 prom / 无图谱级指标。
 *   璇玑补齐：
 *     1. 每个图谱调用（算法/联盟/扇出/RAG/Wasm）写入一条 GraphPerfSample
 *     2. 聚合窗口：1m/5m/15m/total（与 O4 SloTracker 对齐）
 *     3. 分类域：category ∈ { algo, alliance, fanout, rag_chunk, wasm_plugin, slo_metric }
 *     4. 输出：p50/p95/p99 + success_rate + error_count + rps 可直接喂给 O8 仪表盘
 *     5. 与 O4 SloTracker 解耦：O7 只负责图谱域，O4 做全局 SLO。
 */

const WINDOWS_MS = {
  '1m':   60 * 1000,
  '5m':   5 * 60 * 1000,
  '15m':  15 * 60 * 1000,
  total:  Number.MAX_SAFE_INTEGER,
};
const WINDOW_ORDER = ['1m', '5m', '15m', 'total'];
const VALID_CATEGORIES = new Set([
  'algo', 'alliance', 'fanout', 'rag_chunk', 'wasm_plugin', 'slo_metric', 'other',
]);

class GraphP99Reporter {
  constructor({ maxSamples = 20000 } = {}) {
    this._samples = [];
    this._max = Math.max(100, Number(maxSamples) || 20000) | 0;
    this._cats = new Set();
  }

  /** 上报一条样本；非法 category 归一到 'other'。
   *  sample: { category, key, latency_ms, ok, error?, nodes?, edges?, extra? }
   */
  record(sample) {
    const s = {
      ts: Date.now(),
      category: VALID_CATEGORIES.has(sample.category) ? sample.category : 'other',
      key: String(sample.key || 'global').slice(0, 128),
      latency_ms: Math.max(0, Number(sample.latency_ms) || 0),
      ok: sample.ok !== false,
      error: sample.error ? String(sample.error).slice(0, 512) : null,
      nodes: sample.nodes == null ? null : Math.max(0, sample.nodes | 0),
      edges: sample.edges == null ? null : Math.max(0, sample.edges | 0),
      extra: sample.extra || null,
    };
    this._cats.add(s.category);
    this._samples.push(s);
    if (this._samples.length > this._max) {
      this._samples.splice(0, this._samples.length - this._max);
    }
    return s;
  }

  reset() {
    this._samples.length = 0;
    this._cats.clear();
  }

  /** 汇总快照：总览 + 分 category + 分 key（可选 topKeysOnly: N）
   *  opts: { categories?, key?, topKeysOnly? }
   */
  snapshot(opts = {}) {
    const now = Date.now();
    const catF = Array.isArray(opts.categories) && opts.categories.length
      ? new Set(opts.categories)
      : null;
    const keyF = opts.key ? String(opts.key) : null;
    const topK = typeof opts.topKeysOnly === 'number' ? Math.max(0, opts.topKeysOnly | 0) : 0;

    const filtered = this._samples.filter(s =>
      (!catF || catF.has(s.category)) &&
      (!keyF || s.key === keyF)
    );

    const byWindow = {};
    for (const w of WINDOW_ORDER) byWindow[w] = [];
    for (const s of filtered) {
      const age = now - s.ts;
      for (const w of WINDOW_ORDER) {
        if (age <= WINDOWS_MS[w]) byWindow[w].push(s);
      }
    }

    const windows = {};
    for (const w of WINDOW_ORDER) {
      windows[w] = summarize(byWindow[w]);
    }

    // 分 category
    const per_category = {};
    for (const s of filtered) {
      if (!per_category[s.category]) per_category[s.category] = [];
      per_category[s.category].push(s);
    }
    for (const c of Object.keys(per_category)) {
      per_category[c] = summarize(per_category[c]);
    }

    // 分 key（取 topK：按调用量倒序）
    let per_key = {};
    const byKey = new Map();
    for (const s of filtered) byKey.set(s.key, (byKey.get(s.key) || 0) + 1);
    let keys = [...byKey.keys()];
    if (topK > 0) {
      keys.sort((a, b) => byKey.get(b) - byKey.get(a)).splice(topK);
    }
    const keySet = new Set(keys);
    for (const s of filtered) {
      if (!keySet.has(s.key)) continue;
      if (!per_key[s.key]) per_key[s.key] = [];
      per_key[s.key].push(s);
    }
    for (const k of Object.keys(per_key)) {
      per_key[k] = summarize(per_key[k]);
    }

    const overall = summarize(filtered);
    return {
      schema_version: 'graph-p99-v1',
      generated_at: new Date(now).toISOString(),
      sample_count: filtered.length,
      ring_capacity: this._max,
      overall,
      windows,
      per_category,
      per_key,
      categories_seen: [...this._cats].sort(),
    };
  }
}

function summarize(samples) {
  const n = samples.length;
  if (n === 0) {
    return {
      count: 0,
      success_rate: null,
      error_count: 0,
      rps: 0,
      latency_ms: { p50: null, p95: null, p99: null, min: null, max: null, avg: null },
      graph_nodes_total: 0,
      graph_edges_total: 0,
    };
  }
  let ok = 0, nodes = 0, edges = 0;
  const lats = new Array(n);
  for (let i = 0; i < n; i++) {
    const s = samples[i];
    lats[i] = s.latency_ms;
    if (s.ok) ok++;
    if (typeof s.nodes === 'number') nodes += s.nodes;
    if (typeof s.edges === 'number') edges += s.edges;
  }
  lats.sort((a, b) => a - b);
  // rps：用样本时间跨度（若不足 2 samples 用 max_ts-min_ts；否则按 1s 恒 1 样本 → rps = count/durationS 或 count）
  let spanMs = 0;
  for (let i = 1; i < samples.length; i++) {
    const d = samples[i].ts - samples[i - 1].ts;
    if (d > 0) spanMs += d;
  }
  const span = Math.max(1, spanMs) / 1000;
  const rps = n / span;
  return {
    count: n,
    success_rate: n > 0 ? ok / n : null,
    error_count: n - ok,
    rps: Math.round(rps * 100) / 100,
    latency_ms: {
      p50: quantile(lats, 0.50),
      p95: quantile(lats, 0.95),
      p99: quantile(lats, 0.99),
      min: lats[0],
      max: lats[n - 1],
      avg: lats.reduce((a, b) => a + b, 0) / n,
    },
    graph_nodes_total: nodes,
    graph_edges_total: edges,
  };
}

function quantile(sorted, q) {
  const n = sorted.length;
  if (n === 0) return null;
  if (n === 1) return sorted[0];
  const pos = (n - 1) * q;
  const lo = Math.floor(pos), hi = Math.min(lo + 1, n - 1);
  const f = pos - lo;
  return Math.round(sorted[lo] * (1 - f) + sorted[hi] * f);
}

module.exports = { GraphP99Reporter, WINDOW_ORDER, VALID_CATEGORIES, summarize };
