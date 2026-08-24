'use strict';

/**
 * PERF-T2 SloTracker 性能 TDD（算法联盟环形缓冲优化基线）
 * TR:
 *   1) 100k 次 record 总耗时 P95 预算：单条平均 ≤ 5μs（总体 ≤ 500ms）。旧 splice 实现通常 4-6s，Red FAIL。
 *   2) 100 次 snapshot（容量 50k 样本满）总耗时 ≤ 2000ms（P95）。
 *   3) 头指针环形缓冲写满一圈后，写入仍 O(1)（1M 次滚动后单条平均 ≤ 5μs，验证无 splice 退化）。
 *   4) 读回数据正确性：record 的条目都能 snapshot 读到（FIFO/最新语义正确）。
 */
const assert = require('assert');
const { SloTracker } = require('../src/slo-tracker');

function nowMs() { return Number(process.hrtime.bigint()) / 1e6; }

describe('PERF-T2：SloTracker 环形缓冲性能（替换 Array.splice 退化）', function () {
  this.timeout(600000);

  it('1) 100k record 总耗时 ≤ 500ms（O(1) append，无 splice 左移退化）', function () {
    const s = new SloTracker({ maxRingSize: 50000 });
    const trials = [];
    // 3 runs
    for (let r = 0; r < 3; r++) {
      s.reset();
      const t0 = nowMs();
      for (let i = 0; i < 100000; i++) {
        s.record('k' + (i % 17), (i * 37) % 1000, (i % 97) !== 0, i % 500 === 0 ? 't-'+(i%10) : undefined);
      }
      trials.push(nowMs() - t0);
    }
    console.log('[PERF-T2.1] 100k record times (ms): ' + trials.map(t => t.toFixed(0)).join(', '));
    const worst = Math.max(...trials);
    assert.ok(worst <= 500, `最差 100k record 耗时 ${worst.toFixed(0)}ms > 预算 500ms（splice 退化导致？）`);
  });

  it('2) capacity 50k 充满后 100 次 snapshot ≤ 2000ms（含分位数排序）', function () {
    const s = new SloTracker({ maxRingSize: 50000 });
    for (let i = 0; i < 50000; i++) s.record('k' + (i % 31), i % 1200, (i % 89) !== 0);
    const trials = [];
    for (let r = 0; r < 5; r++) {
      const t0 = nowMs();
      for (let i = 0; i < 100; i++) s.snapshot({ domains: null, objectiveP99Ms: 1000, objectiveSuccess: 0.99 });
      trials.push(nowMs() - t0);
    }
    console.log('[PERF-T2.2] 100 snapshot x 5runs ms: ' + trials.map(t => t.toFixed(0)).join(', '));
    const worst = Math.max(...trials);
    assert.ok(worst <= 2000, `100 次 snapshot 最差 ${worst.toFixed(0)}ms > 2000ms 预算`);
  });

  it('3) 滚动写 1M 条（容量 50k，头指针环形绕 20 圈）后单条平均 ≤ 5μs（无数组搬移退化）', function () {
    const s = new SloTracker({ maxRingSize: 50000 });
    const t0 = nowMs();
    for (let i = 0; i < 1000000; i++) {
      s.record('k' + (i % 7), (i * 13) % 800, (i % 53) !== 0);
    }
    const totalMs = nowMs() - t0;
    const perUs = (totalMs * 1000) / 1000000;
    console.log('[PERF-T2.3] 1M records total=' + totalMs.toFixed(0) + 'ms, per record avg=' + perUs.toFixed(2) + 'μs');
    assert.ok(perUs <= 5, `单条平均 ${perUs.toFixed(2)}μs > 预算 5μs（splice O(N) 会达到 40-60μs）`);
    const snap = s.snapshot();
    assert.strictEqual(snap.ring_capacity, 50000);
    assert.strictEqual(snap.sample_count, 50000);
  });

  it('4) 环形缓冲正确性：写 70k 进 50k 容量，sample_count 应 = 50000 且最新样本存在', function () {
    const s = new SloTracker({ maxRingSize: 50000 });
    for (let i = 0; i < 70000; i++) s.record('mykey', i, true);
    const snap = s.snapshot();
    assert.strictEqual(snap.sample_count, 50000, '应只保留最新 50k 条');
    // 最新条目 latency max 应接近 69999（最后一条）
    const latestWindow = snap.windows && snap.windows['1h'];
    assert.ok(latestWindow && Number(latestWindow.latency_max_ms) >= 69000, `最新 max=${latestWindow && latestWindow.latency_max_ms}，应为 ~69999`);
  });
});
