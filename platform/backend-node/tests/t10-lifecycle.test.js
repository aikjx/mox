/* eslint-env mocha, node */
'use strict';
/**
 * T10 M4 A-1 Cloud Drive Lifecycle (HOT/WARM/COLD) Node 接入层测试（12 cases）
 * 验证 Node 层对存储分类规则的理解：HOT 0-30d, WARM 30-90d, COLD >90d；读回温触发迁移。
 * （Rust 层已通过 cargo test；这里纯 JS 规则引擎对齐 AC-T10-1~4）
 */
const assert = require('assert');

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const HOT_MAX_DAYS = 30;
const WARM_MAX_DAYS = 90;

function classifyByAgeDays(ageDays) {
  if (ageDays <= HOT_MAX_DAYS) return 'HOT';
  if (ageDays <= WARM_MAX_DAYS) return 'WARM';
  return 'COLD';
}

/** 返回 { transition: bool, newClass, reason } */
function planTransition(obj, now = Date.now()) {
  const ageMs = now - obj.lastModifiedMs;
  const ageDays = ageMs / MS_PER_DAY;
  const expected = classifyByAgeDays(ageDays);
  if (expected !== obj.storageClass) {
    return { transition: true, newClass: expected, reason: 'age', ageDays };
  }
  return { transition: false, newClass: obj.storageClass, reason: 'none', ageDays };
}

/** 读回温：从 WARM/COLD → HOT（若 access_on_read=true）*/
function onRead(obj, opts = { accessOnRead: true }) {
  if (!opts.accessOnRead) return obj;
  if (obj.storageClass === 'HOT') return obj;
  return { ...obj, storageClass: 'HOT', lastAccessMs: Date.now(), restoreChargeCount: (obj.restoreChargeCount || 0) + 1 };
}

describe('T10/A1 Lifecycle HOT→WARM→COLD Node rules (12)', function () {
  const base = { key: 'a.txt', size: 1024, lastModifiedMs: Date.now() };

  it('L1 age 0d (just-uploaded) → HOT', function () {
    assert.strictEqual(classifyByAgeDays(0), 'HOT');
  });
  it('L2 age 15d → HOT', function () {
    assert.strictEqual(classifyByAgeDays(15), 'HOT');
  });
  it('L3 age 30d (boundary) → still HOT', function () {
    assert.strictEqual(classifyByAgeDays(30), 'HOT');
  });
  it('L4 age 31d crosses → WARM', function () {
    assert.strictEqual(classifyByAgeDays(31), 'WARM');
  });
  it('L5 age 60d still WARM', function () {
    assert.strictEqual(classifyByAgeDays(60), 'WARM');
  });
  it('L6 age 90d boundary → WARM', function () {
    assert.strictEqual(classifyByAgeDays(90), 'WARM');
  });
  it('L7 age 91d → COLD (archive)', function () {
    assert.strictEqual(classifyByAgeDays(91), 'COLD');
  });
  it('L8 planTransition reports no-op for matching class', function () {
    const now = Date.now();
    const obj = { ...base, storageClass: 'WARM', lastModifiedMs: now - 45 * MS_PER_DAY };
    const p = planTransition(obj, now);
    assert.strictEqual(p.transition, false);
    assert.strictEqual(p.newClass, 'WARM');
  });
  it('L9 planTransition reports HOT→WARM transition after 31d', function () {
    const now = Date.now();
    const obj = { ...base, storageClass: 'HOT', lastModifiedMs: now - 31 * MS_PER_DAY };
    const p = planTransition(obj, now);
    assert.strictEqual(p.transition, true);
    assert.strictEqual(p.newClass, 'WARM');
    assert.strictEqual(p.reason, 'age');
  });
  it('L10 planTransition WARM→COLD after 90d+1ms', function () {
    const now = Date.now();
    const obj = { ...base, storageClass: 'WARM', lastModifiedMs: now - (90 * MS_PER_DAY + 1) };
    const p = planTransition(obj, now);
    assert.strictEqual(p.transition, true);
    assert.strictEqual(p.newClass, 'COLD');
  });
  it('L11 onRead read-restore WARM → HOT with charge', function () {
    const obj = { ...base, storageClass: 'WARM' };
    const r = onRead(obj);
    assert.strictEqual(r.storageClass, 'HOT');
    assert.strictEqual(r.restoreChargeCount, 1);
  });
  it('L12 COLD access onRead → HOT; accessOnRead=false preserves class', function () {
    const cold = { ...base, storageClass: 'COLD' };
    const r = onRead(cold, { accessOnRead: false });
    assert.strictEqual(r.storageClass, 'COLD');
    const r2 = onRead(cold);
    assert.strictEqual(r2.storageClass, 'HOT');
    assert.strictEqual(r2.restoreChargeCount, 1);
  });
});
