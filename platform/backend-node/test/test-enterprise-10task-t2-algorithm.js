'use strict';
/**
 * T2: Graph Formulas 算法性能与稳定性 (performance & stability).
 *
 * 7 算法 × 30 runs: P95 ms budget
 *   degree ≤ 30 / betweenness ≤ 1500 / harmonic ≤ 1000 / cnm ≤ 800
 *   pagerank ≤ 400 / raw_expand ≤ 200 / conservation ≤ 60
 *
 * Edge cases:
 *   - Empty graph (0 nodes, 0 edges)
 *   - Single node with self-loop
 *   - 10000-node sparse graph
 * Stability:
 *   - 10 runs on same deterministic input, Σ abs diff across 10 runs ≤ 1e-6
 */
const assert = require('assert');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Use isolated DATA_DIR + memory storage so json-store/storage import is safe
// (this test does NOT touch json-store, but graph-formulas transitively requires
//  chunk-backend which tries to read DATA_DIR; we point it to tmp to avoid writes
//  to production data directory).
const TMP = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t2-algo-'));
const DATA_DIR = path.join(TMP, 'data');
fs.mkdirSync(DATA_DIR, { recursive: true });
process.env.DB_PROVIDER = 'memory';
process.env.DATA_DIR_OVERRIDE = DATA_DIR;
// Force DATA_DIR inside config via rewrite: easiest — mock env before require.
process.env.DATA_DIR = DATA_DIR;
const configMod = require.resolve('../src/config');
const storageMod = require.resolve('../src/storage');
delete require.cache[configMod];
delete require.cache[storageMod];
const { config } = require(configMod);
config.storage.provider = 'memory';
config.storage.providers.sqlite.path = path.join(DATA_DIR, 'ous.db');

const { GraphFormulas, expandRawEdges } = require('../src/graph/graph-formulas');

/**
 * Conservation (flow balance): C = Σ_v |inDeg(v) - outDeg(v)| / max(1, 2|E|)
 * For directed RAW edges: inDeg + outDeg balance. Undirected edges expand to bidirectional,
 * giving C = 0 trivially (which is the expected "flow conserved" property).
 */
function conservation(nodes, edges) {
  const inDeg = new Map();
  const outDeg = new Map();
  const ids = (nodes || []).map(n => n.id);
  ids.forEach(id => { inDeg.set(id, 0); outDeg.set(id, 0); });
  const E = Array.isArray(edges) ? edges : [];
  for (const e of E) {
    if (!e || e.source == null || e.target == null) continue;
    if (outDeg.has(e.source)) outDeg.set(e.source, outDeg.get(e.source) + 1);
    if (inDeg.has(e.target)) inDeg.set(e.target, inDeg.get(e.target) + 1);
  }
  let sum = 0;
  for (const id of ids) {
    sum += Math.abs((inDeg.get(id) || 0) - (outDeg.get(id) || 0));
  }
  const denom = Math.max(1, 2 * E.length);
  return { value: sum / denom, sum, edges: E.length, nodes: ids.length };
}

function buildSyntheticGraph(n, m) {
  const nodes = [];
  for (let i = 0; i < n; i++) nodes.push({ id: 'n' + i });
  const edges = [];
  let seed = 42;
  function rand() {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 0xffffffff;
  }
  for (let i = 0; i < m; i++) {
    const a = Math.floor(rand() * n);
    const b = Math.floor(rand() * n);
    edges.push({
      id: 'e' + i,
      source: 'n' + a,
      target: 'n' + b,
      weight: 1,
    });
  }
  return { nodes, edges };
}

function p95(arr) {
  if (!arr || !arr.length) return 0;
  const s = arr.slice().sort((a, b) => a - b);
  const idx = Math.min(s.length - 1, Math.ceil(0.95 * s.length) - 1);
  return s[idx];
}

describe('T2 算法性能与稳定性', function () {
  after(function () {
    try { fs.rmSync(TMP, { recursive: true, force: true, maxRetries: 3 }); } catch {}
  });

  const { nodes: N500, edges: E4000 } = buildSyntheticGraph(500, 4000);
  const RUNS = 30;

  function measureP95(fn, runs) {
    const samples = [];
    let firstResult = null;
    for (let i = 0; i < runs; i++) {
      const t0 = Date.now();
      const r = fn();
      const t1 = Date.now();
      samples.push(t1 - t0);
      if (firstResult === null) firstResult = r;
    }
    return { p95: p95(samples), min: Math.min(...samples), max: Math.max(...samples), firstResult };
  }

  it('[degree] 500 nodes / 4000 edges P95 ≤ 30ms (30 runs)', function () {
    this.timeout(10000);
    const res = measureP95(() => GraphFormulas.degreeCentrality(N500, E4000), RUNS);
    console.log(`    [degree] P95=${res.p95}ms min=${res.min} max=${res.max}`);
    assert.ok(res.p95 <= 30, `degree P95 ${res.p95}ms exceeds budget 30ms`);
  });

  it('[betweenness] 500 nodes / 4000 edges P95 ≤ 1500ms (30 runs)', function () {
    this.timeout(60000);
    const res = measureP95(() => GraphFormulas.betweennessCentrality(N500, E4000, { directed: false }), RUNS);
    console.log(`    [betweenness] P95=${res.p95}ms min=${res.min} max=${res.max}`);
    assert.ok(res.p95 <= 1500, `betweenness P95 ${res.p95}ms exceeds budget 1500ms`);
  });

  it('[harmonic/closeness] 500 nodes / 4000 edges P95 ≤ 1000ms (30 runs)', function () {
    this.timeout(60000);
    const res = measureP95(() => GraphFormulas.closenessCentrality(N500, E4000, { directed: false }), RUNS);
    console.log(`    [harmonic] P95=${res.p95}ms min=${res.min} max=${res.max}`);
    assert.ok(res.p95 <= 1000, `harmonic P95 ${res.p95}ms exceeds budget 1000ms`);
  });

  it('[cnm community] 500 nodes / 4000 edges P95 ≤ 800ms (30 runs)', function () {
    this.timeout(60000);
    const res = measureP95(() => GraphFormulas.communityDetectionCNM(N500, E4000), RUNS);
    console.log(`    [cnm] P95=${res.p95}ms min=${res.min} max=${res.max} communities=${(res.firstResult || {}).communities?.length}`);
    assert.ok(res.p95 <= 800, `cnm P95 ${res.p95}ms exceeds budget 800ms`);
  });

  it('[pagerank] 500 nodes / 4000 edges P95 ≤ 400ms (30 runs)', function () {
    this.timeout(30000);
    const res = measureP95(() => GraphFormulas.pagerank(N500, E4000), RUNS);
    console.log(`    [pagerank] P95=${res.p95}ms min=${res.min} max=${res.max}`);
    assert.ok(res.p95 <= 400, `pagerank P95 ${res.p95}ms exceeds budget 400ms`);
  });

  it('[raw_expand] 4000 edges expand to bidirectional P95 ≤ 200ms (30 runs)', function () {
    this.timeout(5000);
    const res = measureP95(() => expandRawEdges(E4000, { directed: false }), RUNS);
    const expanded = res.firstResult;
    assert.ok(Array.isArray(expanded), 'expandRawEdges returns array');
    assert.strictEqual(expanded.length, 2 * E4000.length, 'undirected RAW doubles');
    console.log(`    [raw_expand] P95=${res.p95}ms min=${res.min} max=${res.max} len=${expanded.length}`);
    assert.ok(res.p95 <= 200, `raw_expand P95 ${res.p95}ms exceeds budget 200ms`);
  });

  it('[conservation] 500 nodes / 4000 edges P95 ≤ 60ms (30 runs)', function () {
    this.timeout(5000);
    const res = measureP95(() => conservation(N500, E4000), RUNS);
    console.log(`    [conservation] P95=${res.p95}ms min=${res.min} max=${res.max} value=${(res.firstResult || {}).value}`);
    assert.ok(res.p95 <= 60, `conservation P95 ${res.p95}ms exceeds budget 60ms`);
    assert.strictEqual(typeof (res.firstResult || {}).value, 'number');
  });

  // ---------- edge cases ----------
  describe('T2 edge cases', function () {
    it('Empty graph (0/0): degree/betweenness/harmonic/cnm/pagerank return empty-stable shapes', function () {
      const deg = GraphFormulas.degreeCentrality([], []);
      const btw = GraphFormulas.betweennessCentrality([], [], { directed: false });
      const clo = GraphFormulas.closenessCentrality([], [], { directed: false });
      const cnm = GraphFormulas.communityDetectionCNM([], []);
      const pr = GraphFormulas.pagerank([], []);
      assert.deepStrictEqual(deg, {}, 'empty degree = {}');
      assert.deepStrictEqual(btw, {}, 'empty betweenness = {}');
      assert.deepStrictEqual(clo, {}, 'empty closeness = {}');
      assert.strictEqual(cnm.communities.length, 0);
      assert.strictEqual(cnm.nodeCommunity && Object.keys(cnm.nodeCommunity).length, 0);
      assert.deepStrictEqual(pr, {}, 'empty pagerank = {}');
      const c0 = conservation([], []);
      assert.strictEqual(c0.value, 0);
      assert.strictEqual(expandRawEdges([], { directed: false }).length, 0);
    });

    it('Single node with self-loop returns sensible numbers', function () {
      const n = [{ id: 's' }];
      const e = [{ source: 's', target: 's', weight: 1 }];
      const deg = GraphFormulas.degreeCentrality(n, e);
      assert.ok('s' in deg);
      // betweenness/closeness require n>=3 / n>1: fallback to 0 is fine
      const btw = GraphFormulas.betweennessCentrality(n, e, { directed: false });
      assert.ok(btw.s === 0 || btw.s === undefined || !Number.isNaN(btw.s));
      const clo = GraphFormulas.closenessCentrality(n, e, { directed: false });
      assert.ok(!Number.isNaN(clo.s));
      const cnm = GraphFormulas.communityDetectionCNM(n, e);
      assert.strictEqual(cnm.communities.length, 1);
      const pr = GraphFormulas.pagerank(n, e);
      assert.ok(typeof pr.s === 'number' && !Number.isNaN(pr.s));
      const cons = conservation(n, e); // inDeg[s]=1, outDeg[s]=1 => C=0
      assert.strictEqual(cons.value, 0, 'self-loop: conservation must be 0');
    });

    it('Sparse 10000 nodes / ~5000 edges: every call returns without error + output length matches', function () {
      this.timeout(240000);
      const { nodes, edges } = buildSyntheticGraph(10000, 5000);
      const deg = GraphFormulas.degreeCentrality(nodes, edges);
      assert.strictEqual(Object.keys(deg).length, nodes.length, 'degree: key count matches nodes');
      const cnm = GraphFormulas.communityDetectionCNM(nodes, edges);
      assert.ok(cnm.communities.length >= 1, 'cnm: at least 1 community');
      const pr = GraphFormulas.pagerank(nodes, edges);
      assert.strictEqual(Object.keys(pr).length, nodes.length, 'pagerank: key count matches nodes');
      const expanded = expandRawEdges(edges, { directed: false });
      assert.strictEqual(expanded.length, 2 * edges.length);
      const cons = conservation(nodes, edges);
      assert.strictEqual(typeof cons.value, 'number');
    });
  });

  // ---------- numerical stability ----------
  describe('T2 numerical stability (10 runs Σ abs diff ≤ 1e-6)', function () {
    it('deterministic tiny graph: degree stable', function () {
      const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }];
      const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }, { source: 'c', target: 'd' }];
      const runs = [];
      for (let i = 0; i < 10; i++) runs.push(GraphFormulas.degreeCentrality(nodes, edges));
      const ids = nodes.map(n => n.id);
      const diffs = new Array(ids.length).fill(0);
      const base = runs[0];
      for (let r = 1; r < runs.length; r++) {
        for (let k = 0; k < ids.length; k++) {
          diffs[k] += Math.abs(Number(runs[r][ids[k]] || 0) - Number(base[ids[k]] || 0));
        }
      }
      const sumAbs = diffs.reduce((a, b) => a + b, 0);
      console.log(`    [degree-stability] Σ|Δ| = ${sumAbs.toExponential(2)}`);
      assert.ok(sumAbs <= 1e-6, `degree stability: Σ|Δ| = ${sumAbs} > 1e-6`);
    });

    it('deterministic tiny graph: pagerank stable', function () {
      const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }];
      const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }, { source: 'c', target: 'd' }, { source: 'd', target: 'a' }];
      const runs = [];
      for (let i = 0; i < 10; i++) runs.push(GraphFormulas.pagerank(nodes, edges));
      const ids = nodes.map(n => n.id);
      let sumAbs = 0;
      const base = runs[0];
      for (let r = 1; r < runs.length; r++) {
        for (const id of ids) {
          sumAbs += Math.abs(Number(runs[r][id] || 0) - Number(base[id] || 0));
        }
      }
      console.log(`    [pagerank-stability] Σ|Δ| = ${sumAbs.toExponential(2)}`);
      assert.ok(sumAbs <= 1e-6, `pagerank stability Σ|Δ| = ${sumAbs} > 1e-6`);
    });

    it('CNM modularity stable on deterministic input', function () {
      // Use CNM modularity output which is a scalar; nodeCommunity might be order-dependent;
      // we compare modularity scalar across runs.
      const nodes = [];
      for (let i = 0; i < 40; i++) nodes.push({ id: 'n' + i });
      const edges = [];
      // Two cliques + one bridge to ensure CNM has a deterministic seed merge order
      for (let i = 0; i < 20; i++) for (let j = i + 1; j < 20; j++) edges.push({ source: 'n' + i, target: 'n' + j });
      for (let i = 20; i < 40; i++) for (let j = i + 1; j < 40; j++) edges.push({ source: 'n' + i, target: 'n' + j });
      edges.push({ source: 'n0', target: 'n39' });
      const mods = [];
      for (let i = 0; i < 10; i++) mods.push(GraphFormulas.communityDetectionCNM(nodes, edges).modularity);
      let sumAbs = 0;
      const base = mods[0];
      for (let r = 1; r < mods.length; r++) sumAbs += Math.abs(mods[r] - base);
      console.log(`    [cnm-modularity-stability] Σ|Δ| = ${sumAbs.toExponential(2)}`);
      assert.ok(sumAbs <= 1e-6, `CNM modularity Σ|Δ| = ${sumAbs} > 1e-6`);
    });
  });
});
