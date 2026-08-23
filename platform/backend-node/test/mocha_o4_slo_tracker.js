'use strict';
/**
 * mocha 单元测试：T10 O4 SloTracker + SystemSlo 契约（T10 TR-1~TR-9）
 */
const assert = require('assert');
const { SloTracker, quantile, evaluateStatus, summarize, WINDOW_ORDER } = require('../src/slo-tracker');

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

describe('[T10-AC1] SloTracker 基本 API + 空态快照', function () {
  it('new / reset / listDomains 工作正常', () => {
    const t = new SloTracker();
    assert.deepStrictEqual(t.listDomains(), []);
    const s = t.snapshot();
    assert.strictEqual(s.schema_version, 'system-slo-v1', 'schema v1');
    assert.strictEqual(s.sample_count, 0);
    for (const w of WINDOW_ORDER) {
      assert.ok(w in s.windows, `窗口 ${w} 存在`);
      assert.strictEqual(s.windows[w].count, 0, `${w} count=0`);
      assert.strictEqual(s.windows[w].status, 'no_data', `${w} status=no_data`);
    }
    assert.strictEqual(s.status, 'ok'); // 默认 status 在全 no_data 时仍为 ok
  });
  it('record 一批后 listDomains 去重排序', () => {
    const t = new SloTracker();
    t.record('chat', 100, true);
    t.record('llm', 50, true);
    t.record('chat', 200, false);
    assert.deepStrictEqual(t.listDomains(), ['chat', 'llm']);
  });
});

describe('[T10-AC2] 分位数 + 精确排序', function () {
  it('quantile(n=1) 返回自身', () => {
    assert.strictEqual(quantile([5], 0.99), 5);
  });
  it('quantile(n=5, p99) 线性插值：接近最大值', () => {
    // [10,20,30,40,50] pos = 4*0.99=3.96，lo=3 hi=4 frac=0.96
    const v = quantile([10, 20, 30, 40, 50], 0.99);
    const exp = 40 * (1 - 0.96) + 50 * 0.96; // 49.6
    assert.ok(Math.abs(v - exp) < 1e-9, `p99 exp ${exp} got ${v}`);
  });
  it('summarize: 10 样本 p50/p95/p99 非空且递增', () => {
    const samples = [];
    for (let i = 1; i <= 10; i++) samples.push({ ts: 0, key: 'k', latency_ms: i * 10, ok: true });
    const s = summarize(samples);
    assert.strictEqual(s.count, 10);
    assert.strictEqual(s.error_count, 0);
    assert.strictEqual(s.success_rate, 1.0);
    for (const k of ['latency_p50_ms', 'latency_p95_ms', 'latency_p99_ms']) {
      assert.strictEqual(typeof s[k], 'number', `${k} typeof number`);
    }
    assert.ok(s.latency_p50_ms <= s.latency_p95_ms, 'p50 <= p95');
    assert.ok(s.latency_p95_ms <= s.latency_p99_ms, 'p95 <= p99');
    assert.strictEqual(s.latency_min_ms, 10);
    assert.strictEqual(s.latency_max_ms, 100);
  });
});

describe('[T10-AC3] evaluateStatus 三态分类（ok / warning / violated）', function () {
  it('完美样本=ok', () => {
    const s = summarize(Array.from({ length: 20 }, (_, i) => ({ ts: 0, key: 'k', latency_ms: 10 + i, ok: true })));
    assert.strictEqual(evaluateStatus(s, 1000, 0.99), 'ok');
  });
  it('p99 刚好超过上限 = violated', () => {
    const s = summarize(Array.from({ length: 100 }, () => ({ ts: 0, key: 'k', latency_ms: 2000, ok: true })));
    assert.strictEqual(evaluateStatus(s, 1000, 0.99), 'violated');
  });
  it('成功率低于目标 = violated', () => {
    const arr = [];
    for (let i = 0; i < 100; i++) arr.push({ ts: 0, key: 'k', latency_ms: 50, ok: i < 90 });
    const s = summarize(arr); // 0.9 success < 0.99
    assert.strictEqual(evaluateStatus(s, 1000, 0.99), 'violated');
  });
  it('距目标 ≤10% buffer = warning（p99 接近上限）', () => {
    const arr = Array.from({ length: 50 }, () => ({ ts: 0, key: 'k', latency_ms: 950, ok: true }));
    const s = summarize(arr);
    // objP99=1000; buffer=900.  950>900 且 <=1000 → warning
    assert.strictEqual(evaluateStatus(s, 1000, 0.99), 'warning');
  });
  it('无样本 = no_data', () => {
    assert.strictEqual(evaluateStatus(summarize([]), 1000, 0.99), 'no_data');
  });
});

describe('[T10-AC4] SloTracker.snapshot 四窗口 + 整体 status', function () {
  it('全局 200 个成功样本，4 窗口 count 非增（1m ≤ 5m ≤ 15m ≤ total）', () => {
    const t = new SloTracker();
    for (let i = 0; i < 200; i++) t.record('chat', 50 + i, true);
    const s = t.snapshot();
    const counts = WINDOW_ORDER.map(w => s.windows[w].count);
    // 时间顺序一致下，1m/5m/15m/total 数量应相同（都在 1m 窗口内），或非增
    for (let i = 1; i < counts.length; i++) {
      assert.ok(counts[i] >= counts[i - 1], `${WINDOW_ORDER[i]}(${counts[i]}) >= ${WINDOW_ORDER[i-1]}(${counts[i-1]})`);
    }
    assert.strictEqual(s.status, 'ok');
    assert.strictEqual(s.windows.total.count, 200);
  });

  it('违反目标时 overall status = violated', () => {
    const t = new SloTracker();
    for (let i = 0; i < 50; i++) t.record('llm', 5000, true); // 远高于默认 1000ms P99 目标
    const s = t.snapshot();
    assert.strictEqual(s.status, 'violated', 'P99 超上限 → violated');
    assert.ok(s.per_domain.llm, 'per_domain 含 llm');
  });
});

describe('[T10-AC5] 域名/租户过滤', function () {
  it('domains=[A] 只聚 A 域样本', () => {
    const t = new SloTracker();
    for (let i = 0; i < 10; i++) t.record('A', 10, true);
    for (let i = 0; i < 7; i++) t.record('B', 10, true);
    const s = t.snapshot({ domains: ['A'] });
    assert.strictEqual(s.sample_count, 10, '只看 A 域');
    assert.strictEqual(s.filters.domains.join(','), 'A');
    assert.strictEqual('B' in s.per_domain, false, 'per_domain 不应含 B');
  });
  it('tenant 过滤，跨域正确', () => {
    const t = new SloTracker();
    for (let i = 0; i < 5; i++) t.record('chat', 10, true, 'T-1');
    for (let i = 0; i < 3; i++) t.record('chat', 10, true, 'T-2');
    for (let i = 0; i < 2; i++) t.record('llm',  10, true, 'T-1');
    const s = t.snapshot({ tenant: 'T-1' });
    assert.strictEqual(s.sample_count, 7);
    assert.strictEqual(s.filters.tenant, 'T-1');
  });
});

describe('[T10-AC6] 环形缓冲 bounded memory（maxRingSize）', function () {
  it('超过 maxRingSize 后旧样本被头截；count 稳定', () => {
    const t = new SloTracker({ maxRingSize: 100 });
    for (let i = 0; i < 1000; i++) t.record('x', 1, true);
    const s = t.snapshot();
    assert.strictEqual(s.sample_count, 100, 'maxRing 限制容量');
    assert.strictEqual(s.ring_capacity, 100);
  });
});

describe('[T10-AC7] 自定义 objective 覆盖默认', function () {
  it('目标更严格时，原来 ok 的结果变成 violated', () => {
    const t = new SloTracker();
    for (let i = 0; i < 20; i++) t.record('api', 800, true);
    // 默认 P99=1000，800 < 1000 → ok
    assert.strictEqual(t.snapshot().status, 'ok');
    // 收紧到 P99 目标=500ms → 800>500 → violated
    assert.strictEqual(t.snapshot({ objectiveP99Ms: 500 }).status, 'violated');
  });
  it('目标成功率收紧 → warning/violated', () => {
    const t = new SloTracker();
    // 100 样本中 2 个失败 = 0.98 成功率
    for (let i = 0; i < 98; i++) t.record('api', 10, true);
    for (let i = 0; i < 2; i++)  t.record('api', 10, false);
    // 默认 0.99 → 0.98 < 0.99 violated
    const s = t.snapshot();
    assert.strictEqual(s.status, 'violated');
    // 放宽到 0.95 → 0.98 距 0.95 较近，应该是 ok/warning 边界（ok）
    const s2 = t.snapshot({ objectiveSuccess: 0.95 });
    assert.ok(s2.status === 'ok' || s2.status === 'warning', `放松后 status=${s2.status} 不应 violated`);
  });
});

describe('[T10-AC8] per_domain 分域 SLO', function () {
  it('A=成功，B=全部失败：per_domain 状态分离', () => {
    const t = new SloTracker();
    for (let i = 0; i < 20; i++) t.record('A', 50, true);
    for (let i = 0; i < 20; i++) t.record('B', 50, false);
    const s = t.snapshot();
    assert.strictEqual(s.per_domain.A.status, 'ok');
    assert.strictEqual(s.per_domain.B.success_rate, 0);
    assert.strictEqual(s.per_domain.B.status, 'violated');
  });
});

describe('[T10-AC9] 窗口老化（sleep 验证 1m / total 差异）', async function () {
  it('老样本（> 1m）只落在 total，不再落在 1m 窗口', async () => {
    const t = new SloTracker();
    // 模拟 2 条"已老化 90 秒"的事件：把 ts 往前改（通过直接写入内部 ring，绕过 Date.now()）
    const _private = t;
    const past = Date.now() - 90 * 1000;
    for (let i = 0; i < 3; i++) {
      _private._ring.push({ ts: past, key: 'old', latency_ms: 100, ok: true });
      _private._domains.add('old');
    }
    for (let i = 0; i < 5; i++) t.record('new', 50, true); // 实时
    const s = t.snapshot();
    // 1m 窗口只应看到 new 的 5 条
    assert.strictEqual(s.windows['1m'].count, 5, '1m 只看最近 1m 事件');
    // total 看到 3+5=8 条
    assert.strictEqual(s.windows.total.count, 8, 'total 全部');
  }).timeout(5000);
});
