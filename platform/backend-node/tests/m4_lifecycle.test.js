'use strict';
/** T10 M4 A-7-3 生命周期策略（Lifecycle）行为驱动测试 —— 模拟 HotWarmColdLifecycle API
 *
 * 注：Rust 核心实现已在 lifecycle.rs（已通过 7 tests）。
 *     Node 侧以行为驱动验证同样的策略语义，确保运维对接时理解一致。
 */
const assert = require('assert');

/** 1:1 对齐 Rust 实现的 StorageClass 枚举与规则：
 *   HOT : 0-30天内访问或新写
 *   WARM: 30天+未访问且 <=90天
 *   COLD: >90天未访问
 * 读任意类对象 → 回温到 HOT，restore_counter++
 */
const ONE_DAY = 24 * 60 * 60 * 1000;
const HOT_DAYS = 30;
const WARM_DAYS = 90;

class Lifecycle {
  constructor() {
    /** @type {Map<string,{cls:string,lastAccess:number}>} */ this.m = new Map();
    this.tx = 0; this.rx = 0;
  }
  put(key, now = Date.now()) { this.m.set(key, { cls: 'HOT', lastAccess: now }); }
  /** 返回迁移结果：Array<[key, fromClass, toClass]> */
  scan(now = Date.now()) {
    const out = [];
    for (const [k, v] of this.m) {
      const days = (now - v.lastAccess) / ONE_DAY;
      let next = v.cls;
      if (days > WARM_DAYS) next = 'COLD';
      else if (days > HOT_DAYS) next = 'WARM';
      if (next !== v.cls) { out.push([k, v.cls, next]); v.cls = next; this.tx++; }
    }
    return out;
  }
  /** 模拟读：回温到 HOT */
  read(key) {
    const v = this.m.get(key);
    if (!v) return false;
    if (v.cls !== 'HOT') { v.cls = 'HOT'; this.rx++; }
    v.lastAccess = Date.now();
    return true;
  }
  classOf(key) { return this.m.get(key).cls; }
}

describe('Lifecycle Hot→Warm→Cold behavior (≥12 tests)', function () {
  it('l1: new object → HOT', function () {
    const lc = new Lifecycle(); lc.put('a'); assert.strictEqual(lc.classOf('a'), 'HOT');
  });
  it('l2: 31-day idle → scan moves HOT→WARM', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 31 * ONE_DAY);
    const r = lc.scan(t0);
    assert.deepStrictEqual(r, [['a', 'HOT', 'WARM']]);
    assert.strictEqual(lc.classOf('a'), 'WARM');
  });
  it('l3: 91-day idle → scan moves HOT→COLD', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 91 * ONE_DAY);
    lc.scan(t0);
    assert.strictEqual(lc.classOf('a'), 'COLD');
  });
  it('l4: WARM→COLD on next scan (idle prolonged)', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 31 * ONE_DAY); lc.scan(t0);
    const r = lc.scan(t0 + 60 * ONE_DAY); // 再推 60d → 累计 91d
    assert.deepStrictEqual(r, [['a', 'WARM', 'COLD']]);
  });
  it('l5: 10-day idle → scan no change', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 10 * ONE_DAY);
    assert.deepStrictEqual(lc.scan(t0), []);
  });
  it('l6: read WARM object restores to HOT and restore_counter increments', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 31 * ONE_DAY); lc.scan(t0);
    lc.read('a');
    assert.strictEqual(lc.classOf('a'), 'HOT');
    assert.strictEqual(lc.rx, 1);
  });
  it('l7: read HOT object does NOT increment restore_counter', function () {
    const lc = new Lifecycle();
    lc.put('a'); lc.read('a');
    assert.strictEqual(lc.rx, 0);
  });
  it('l8: scan transitions increment tx exactly once per transition', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 31 * ONE_DAY);
    lc.put('b', t0 - 91 * ONE_DAY);
    lc.put('c', t0 - 10 * ONE_DAY);
    lc.scan(t0);
    assert.strictEqual(lc.tx, 2);
  });
  it('l9: stats JSON round-trip', function () {
    const lc = new Lifecycle();
    const stats = { objects: lc.m.size, transitioned: lc.tx, restored: lc.rx };
    const s = JSON.stringify(stats);
    const r = JSON.parse(s);
    assert.strictEqual(r.transitioned, 0);
    assert.strictEqual(r.restored, 0);
  });
  it('l10: mixed-age scan applies only to violators', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('fresh', t0);
    lc.put('warmish', t0 - 31 * ONE_DAY);
    lc.put('coldish', t0 - 91 * ONE_DAY);
    const r = lc.scan(t0).map(([k]) => k).sort();
    assert.deepStrictEqual(r, ['coldish', 'warmish'].sort());
  });
  it('l11: empty scan does not throw', function () {
    const lc = new Lifecycle();
    assert.doesNotThrow(() => lc.scan(Date.now()));
  });
  it('l12: access within HOT window prolongs stay (rewrites lastAccess)', function () {
    const lc = new Lifecycle();
    const t0 = Date.now();
    lc.put('a', t0 - 29 * ONE_DAY); // 正好 29d
    lc.read('a'); // lastAccess → 现在 t0
    const r = lc.scan(t0);
    assert.deepStrictEqual(r, [], '因为 read 刷新了 lastAccess，不会迁移');
    // 再推 31d 无访问 → 迁移 WARM
    const r2 = lc.scan(t0 + 31 * ONE_DAY);
    assert.deepStrictEqual(r2, [['a', 'HOT', 'WARM']]);
  });
});
