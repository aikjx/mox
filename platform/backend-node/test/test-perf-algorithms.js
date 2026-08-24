'use strict';

/**
 * 性能-算法 TDD 基线：PageRank CSR 稀疏 vs Dense + 结果等价性
 */
const path = require('path');
const assert = require('assert');
const { GraphFormulas } = require('../src/graph/graph-formulas');

function erdosRenyi(n, e, seed) {
  let s = seed >>> 0;
  function rnd() {
    s = (s + 0x6D2B79F5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  const nodes = new Array(n).fill(0).map((_, i) => ({ id: 'n' + i, label: 'Node ' + i, node_type: 'default' }));
  const edges = [];
  const seen = new Set();
  let tries = 0;
  while (edges.length < e && tries < e * 10) {
    tries++;
    const a = Math.floor(rnd() * n);
    const b = Math.floor(rnd() * n);
    if (a === b) continue;
    const k = a + '->' + b;
    if (seen.has(k)) continue;
    seen.add(k);
    edges.push({ source: 'n' + a, target: 'n' + b, weight: 1.0, relation_type: 'r' });
  }
  return { nodes, edges };
}

function pearson(x, y) {
  const n = x.length;
  if (n !== y.length || n === 0) return NaN;
  let mx = 0, my = 0;
  for (let i = 0; i < n; i++) { mx += x[i]; my += y[i]; }
  mx /= n; my /= n;
  let num = 0, dx2 = 0, dy2 = 0;
  for (let i = 0; i < n; i++) {
    const a = x[i] - mx, b = y[i] - my;
    num += a * b;
    dx2 += a * a; dy2 += b * b;
  }
  const d = Math.sqrt(dx2 * dy2);
  return d === 0 ? 0 : num / d;
}

function nowMs() { return Number(process.hrtime.bigint()) / 1e6; }

function pPercentile(times, p) {
  const arr = times.slice().sort((a, b) => a - b);
  const pos = (arr.length - 1) * p;
  const lo = Math.floor(pos);
  const hi = Math.min(lo + 1, arr.length - 1);
  const f = pos - lo;
  return arr[lo] * (1 - f) + arr[hi] * f;
}

describe('PERF-T1：PageRank/Degree 性能 + 正确性（算法联盟优化 TDD）', function () {
  this.timeout(600000);

  it('1) N=1000 E=4000 PR 结果合法（全节点返回、值非负、和正常）', function () {
    const g = erdosRenyi(1000, 4000, 20260824);
    const res = GraphFormulas.pagerank(g.nodes, g.edges);
    const ids = Object.keys(res);
    assert.strictEqual(ids.length, g.nodes.length);
    let sum = 0;
    for (const id of ids) {
      const v = Number(res[id]);
      assert.ok(Number.isFinite(v) && v >= 0, '非法值 node ' + id + ' = ' + v);
      sum += v;
    }
    assert.ok((sum > 0.9 && sum < 1.1) || (sum > 0.1 * ids.length && sum < 10 * ids.length), 'sum=' + sum + ' 异常，期望≈1或归一化区间');
  });

  it('2) PR 性能 N=1000：P95 < 400ms（Red 阶段必 FAIL；CSR 优化后通过）', function () {
    const g = erdosRenyi(1000, 4000, 9999);
    try { GraphFormulas.pagerank(g.nodes, g.edges); } catch (_) {}
    const trials = [];
    for (let i = 0; i < 7; i++) {
      const t0 = nowMs();
      GraphFormulas.pagerank(g.nodes, g.edges);
      trials.push(nowMs() - t0);
    }
    const p95 = pPercentile(trials, 0.95);
    console.log('[PERF-T1.2] PR N=1000 P50=' + pPercentile(trials,0.5).toFixed(0) + 'ms, P95=' + p95.toFixed(0) + 'ms');
    assert.ok(p95 < 400, 'PR P95=' + p95.toFixed(0) + 'ms 超预算 400ms');
  });

  it('3) PR 性能 N=500：P95 < 150ms', function () {
    const g = erdosRenyi(500, 2000, 777);
    try { GraphFormulas.pagerank(g.nodes, g.edges); } catch (_) {}
    const trials = [];
    for (let i = 0; i < 8; i++) {
      const t0 = nowMs();
      GraphFormulas.pagerank(g.nodes, g.edges);
      trials.push(nowMs() - t0);
    }
    const p95 = pPercentile(trials, 0.95);
    console.log('[PERF-T1.3] PR N=500 P95=' + p95.toFixed(0) + 'ms');
    assert.ok(p95 < 150, 'PR N=500 P95=' + p95.toFixed(0) + 'ms 超 150ms');
  });

  it('4) PR 自洽 Pearson r >= 0.9999（验证实现稳定）', function () {
    const g = erdosRenyi(800, 3200, 42);
    const r1 = GraphFormulas.pagerank(g.nodes, g.edges);
    const r2 = GraphFormulas.pagerank(g.nodes, g.edges);
    const ids = Object.keys(r1).sort();
    const v1 = ids.map(i => Number(r1[i]));
    const v2 = ids.map(i => Number(r2[i]));
    const r = pearson(v1, v2);
    console.log('[PERF-T1.4] PR self r=' + r.toFixed(10));
    assert.ok(r >= 0.9999, 'self-consistency r=' + r);
    let s = 0;
    for (let i = 0; i < ids.length; i++) s += Math.abs(v1[i] - v2[i]);
    assert.ok(s / ids.length < 1e-10, 'avg L1 diff过大');
  });

  it('5) Degree N=2000 E=8000：P95 < 200ms（O(E) 迭代目标）', function () {
    const g = erdosRenyi(2000, 8000, 2026);
    try { GraphFormulas.degreeCentrality(g.nodes, g.edges); } catch (_) {}
    const trials = [];
    for (let i = 0; i < 8; i++) {
      const t0 = nowMs();
      const d = GraphFormulas.degreeCentrality(g.nodes, g.edges);
      trials.push(nowMs() - t0);
      if (i === 0) assert.strictEqual(Object.keys(d).length, g.nodes.length);
    }
    const p95 = pPercentile(trials, 0.95);
    console.log('[PERF-T1.5] Degree N=2000 P95=' + p95.toFixed(0) + 'ms');
    assert.ok(p95 < 200, 'Degree P95=' + p95.toFixed(0) + 'ms 超 200ms');
  });

  it('6) Personalized PR 自洽 Pearson >= 0.9999 且个性化节点排名提升', function () {
    const g = erdosRenyi(600, 2400, 1);
    const personal = { n10: 2.0, n100: 1.5, n500: 0.5 };
    const r1 = GraphFormulas.personalizedPageRank(g.nodes, g.edges, personal);
    const r2 = GraphFormulas.personalizedPageRank(g.nodes, g.edges, personal);
    const ids = Object.keys(r1).sort();
    const v1 = ids.map(i => Number(r1[i]));
    const v2 = ids.map(i => Number(r2[i]));
    const r = pearson(v1, v2);
    console.log('[PERF-T1.6] PPR self r=' + r.toFixed(10));
    assert.ok(r >= 0.9999, 'PPR self r=' + r);
    const avg = v1.reduce((a, b) => a + b, 0) / v1.length;
    const bias = Number(r1['n10']) + Number(r1['n100']) + Number(r1['n500']);
    assert.ok(bias > 3 * avg, '个性化节点 bias=' + bias + ' avg=' + avg);
  });
});
