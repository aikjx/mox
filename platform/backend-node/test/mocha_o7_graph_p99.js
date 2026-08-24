'use strict';
/**
 * mocha 单元测试：T12 O7 Graph P99 Reporter（T12 TR-10~TR-17）
 */
const assert = require('assert');
const { GraphP99Reporter, WINDOW_ORDER, VALID_CATEGORIES, summarize } = require('../src/graph-p99-reporter');

describe('[T12-AC10] GraphP99Reporter 基本 API + 空态', function () {
  it('new/snapshot/reset 无异常，空态 count=0', () => {
    const r = new GraphP99Reporter();
    const s = r.snapshot();
    assert.strictEqual(s.schema_version, 'graph-p99-v1');
    assert.strictEqual(s.sample_count, 0);
    assert.strictEqual(s.overall.count, 0);
    for (const w of WINDOW_ORDER) assert.strictEqual(s.windows[w].count, 0);
    r.reset();
    assert.strictEqual(r.snapshot().sample_count, 0);
  });
  it('VALID_CATEGORIES 含所有契约域', () => {
    for (const c of ['algo', 'alliance', 'fanout', 'rag_chunk', 'wasm_plugin', 'slo_metric', 'other']) {
      assert.ok(VALID_CATEGORIES.has(c), `缺: ${c}`);
    }
  });
  it('非法 category 归一为 other', () => {
    const r = new GraphP99Reporter();
    r.record({ category: 'not_exist', key: 'k', latency_ms: 10, ok: true });
    const snap = r.snapshot();
    assert.strictEqual(snap.overall.count, 1);
    assert.ok(snap.per_category.other, 'per_category 必须有 other');
  });
});

describe('[T12-AC11] p50/p95/p99 单调且精确', function () {
  it('100 条线性递增 latency 1~100ms：p50≈50, p95≈95, p99≈99', () => {
    const r = new GraphP99Reporter();
    for (let i = 1; i <= 100; i++) r.record({ category: 'algo', key: 'dc', latency_ms: i, ok: true });
    const { p50, p95, p99 } = r.snapshot().overall.latency_ms;
    assert.ok(Math.abs(p50 - 50) <= 1, `p50≈50 实际 ${p50}`);
    assert.ok(Math.abs(p95 - 95) <= 1, `p95≈95 实际 ${p95}`);
    assert.ok(Math.abs(p99 - 99) <= 1, `p99≈99 实际 ${p99}`);
  });
});

describe('[T12-AC12] success_rate / error_count / rps 正确', function () {
  it('100 条中 3 条失败 → success_rate=0.97 error_count=3', () => {
    const r = new GraphP99Reporter();
    for (let i = 0; i < 100; i++) r.record({ category: 'alliance', key: 'e2e', latency_ms: 50, ok: i !== 10 && i !== 30 && i !== 60 });
    const s = r.snapshot().overall;
    assert.strictEqual(s.success_rate, 0.97);
    assert.strictEqual(s.error_count, 3);
    assert.strictEqual(typeof s.rps, 'number');
  });
});

describe('[T12-AC13] per_category 分域正确', function () {
  it('algo/wasm/rag_chunk 三类计数正确', () => {
    const r = new GraphP99Reporter();
    for (let i = 0; i < 10; i++) r.record({ category: 'algo',       key: 'pr', latency_ms: i * 3, ok: true });
    for (let i = 0; i < 5;  i++) r.record({ category: 'wasm_plugin',key: 'mul2', latency_ms: 1, ok: true });
    for (let i = 0; i < 8;  i++) r.record({ category: 'rag_chunk',  key: 'doc1', latency_ms: 2, ok: true });
    const s = r.snapshot();
    assert.strictEqual(s.per_category.algo.count,       10);
    assert.strictEqual(s.per_category.wasm_plugin.count, 5);
    assert.strictEqual(s.per_category.rag_chunk.count,   8);
  });
});

describe('[T12-AC14] per_key + topKeysOnly=N 正确', function () {
  it('5 个 key 各 10 条，topKeysOnly=2 只保留调用量相同 top 2（按 count 排序取前 2）', () => {
    const r = new GraphP99Reporter();
    const keys = ['k1', 'k2', 'k3', 'k4', 'k5'];
    // k1:20, k2:15, k3:10, k4:5, k5:2
    const counts = [20, 15, 10, 5, 2];
    for (let i = 0; i < keys.length; i++) for (let j = 0; j < counts[i]; j++) {
      r.record({ category: 'algo', key: keys[i], latency_ms: 1, ok: true });
    }
    const all = r.snapshot();
    assert.strictEqual(Object.keys(all.per_key).length, 5, '默认所有 key');
    const top2 = r.snapshot({ topKeysOnly: 2 });
    const k2 = Object.keys(top2.per_key);
    assert.strictEqual(k2.length, 2, `top 2 keys：实际 ${k2.join(',')}`);
    // 前 2 应包含 k1（20）和 k2（15）
    assert.ok(k2.includes('k1') && k2.includes('k2'), `top2 应 = k1+k2，实际 ${k2.join(',')}`);
    assert.strictEqual(top2.per_key.k1.count, 20);
    assert.strictEqual(top2.per_key.k2.count, 15);
  });
});

describe('[T12-AC15] category/key 过滤', function () {
  it('categories=[rag_chunk] 只聚该类', () => {
    const r = new GraphP99Reporter();
    for (let i = 0; i < 7; i++) r.record({ category: 'rag_chunk', key: 'a', latency_ms: 1, ok: true });
    for (let i = 0; i < 4; i++) r.record({ category: 'fanout',    key: 'b', latency_ms: 1, ok: true });
    const s = r.snapshot({ categories: ['rag_chunk'] });
    assert.strictEqual(s.sample_count, 7);
  });
  it('key filter 精确匹配', () => {
    const r = new GraphP99Reporter();
    r.record({ category: 'algo', key: 'pagerank', latency_ms: 1, ok: true });
    r.record({ category: 'algo', key: 'cnm', latency_ms: 1, ok: true });
    r.record({ category: 'algo', key: 'pagerank', latency_ms: 1, ok: false });
    const s = r.snapshot({ key: 'pagerank' });
    assert.strictEqual(s.sample_count, 2);
  });
});

describe('[T12-AC16] bounded memory ring', function () {
  it('maxSamples=200，10000 records → sample_count=200', () => {
    const r = new GraphP99Reporter({ maxSamples: 200 });
    for (let i = 0; i < 10000; i++) r.record({ category: 'algo', key: 'x', latency_ms: i % 100, ok: true });
    const s = r.snapshot();
    assert.strictEqual(s.sample_count, 200);
    assert.strictEqual(s.ring_capacity, 200);
  });
});

describe('[T12-AC17] summarize graph nodes/edges 聚合', function () {
  it('3 条 samples 分别 nodes=10, edges=20 × 3 → nodes_total=30, edges_total=60', () => {
    const arr = [];
    for (let i = 0; i < 3; i++) arr.push({ ts: Date.now(), category: 'algo', key: 'g', latency_ms: 10, ok: true, nodes: 10, edges: 20 });
    const s = summarize(arr);
    assert.strictEqual(s.graph_nodes_total, 30);
    assert.strictEqual(s.graph_edges_total, 60);
  });
});
