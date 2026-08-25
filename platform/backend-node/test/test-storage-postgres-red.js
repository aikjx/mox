'use strict';

/**
 * T7 Rubric: PostgresProvider + DualWrite 验证（原 RED 阶段在实现完毕后变为 GREEN）
 * ========================
 * 要求：
 *   1) StorageFactory.create('postgres'/'postgresql', cfg) 成功且 connect 无异常；
 *   2) PostgresProvider 暴露完整 StorageProvider 公共 API（含幂等 migrateFromJSON）
 *   3) Storage 层 config 暴露 dualWrite / readPref 字段（承载 DB_DUAL_WRITE / DB_READ_PREF）
 *   4) 无 pg 驱动时，Fake/Memory 降级路径实现 StorageProvider 等价 API（企业离线单测可跑）
 *
 * 不依赖真实 Postgres。
 */

const path = require('path');
const fs = require('fs');
const os = require('os');

// 准备独立的临时 SQLite dir，避免污染默认 data/
const TMP_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-storage-pg-red-'));
process.env.DB_PROVIDER = 'sqlite';

const storagePath = path.resolve(__dirname, '..', 'src', 'storage', 'index.js');
const { StorageFactory, PostgresProvider, SQLiteProvider, resetStorage } = require(storagePath);

// ---- 测试计数器 ----
let passed = 0, failed = 0;
function test(name, fn) {
  try {
    fn();
    passed++;
    console.log(`  PASS  ${name}`);
  } catch (e) {
    failed++;
    console.error(`  FAIL  ${name}\n        ${e && e.stack ? e.stack : e}`);
  }
}
function eq(a, b, m) {
  const sa = JSON.stringify(a);
  const sb = JSON.stringify(b);
  if (sa !== sb) throw new Error(`${m || 'deepEqual failed'} expected ${sb} got ${sa}`);
}
function ok(cond, m) { if (!cond) throw new Error(m || 'not truthy'); }

// ---- T7-RUBRIC-1: StorageFactory.create('postgres') + StorageFactory.create('postgresql') 成功 ----
test('PostgresProvider 已注册：StorageFactory.create("postgres") 返回实例', () => {
  const cfg = { host: 'localhost', port: 5432, database: 'test', user: 'u', password: 'p' };
  let prov = null, thrown = null;
  try { prov = StorageFactory.create('postgres', cfg); } catch (e) { thrown = e; }
  ok(thrown == null && prov != null, `StorageFactory.create("postgres") 不应抛错: ${thrown && thrown.message}`);
  ok(typeof prov.connect === 'function', '实例具备 connect 方法');
  ok(typeof prov.disconnect === 'function', '实例具备 disconnect 方法');
});

test('PostgresProvider 别名：StorageFactory.create("postgresql") 同样返回实例', () => {
  const cfg = { host: 'localhost', port: 5432, database: 'test' };
  let prov = null, thrown = null;
  try { prov = StorageFactory.create('postgresql', cfg); } catch (e) { thrown = e; }
  ok(thrown == null && prov != null, `StorageFactory.create("postgresql") 不应抛错: ${thrown && thrown.message}`);
});

// ---- T7-RUBRIC-2: Storage 层 config 暴露 dualWrite/readPref 字段 ----
test('Storage config 承载 dualWrite/readPref（实现端已开放）', () => {
  const { config } = require('../src/config');
  ok('dualWrite' in (config.storage || {}), 'config.storage.dualWrite 字段存在');
  ok('readPref' in (config.storage || {}), 'config.storage.readPref 字段存在');
  ok(typeof config.storage.dualWrite === 'boolean', 'dualWrite 类型为 boolean');
  ok(['primary', 'secondary', 'auto'].includes(String(config.storage.readPref || 'auto').toLowerCase()),
    'readPref 取值 ∈ {primary, secondary, auto}');
});

// ---- T7-RUBRIC-3: PostgresProvider 暴露完整公共 API（含 migrateFromJSON） ----
test('PostgresProvider.prototype 实现完整 StorageProvider 公共 API 面', () => {
  const methods = [
    'connect', 'disconnect',
    'insertEntity', 'upsertEntity', 'updateEntity', 'deleteEntity', 'deleteByType',
    'getEntity', 'getEntityData', 'listEntities', 'listAllEntities', 'countByType',
    'saveList', 'getList', 'searchEntities',
    'kvGet', 'kvSet', 'kvDelete',
    'addLog', 'getLogs', 'clearLogs',
    'migrateFromJSON'
  ];
  const proto = PostgresProvider.prototype;
  const missing = methods.filter(m => typeof proto[m] !== 'function');
  ok(missing.length === 0, `PostgresProvider 公共 API 完整，缺失: ${missing.join(',')}`);
});

test('PostgresProvider.migrateFromJSON 幂等：同一目录两次 run，countByType 不变（idempotent）', () => {
  const DATA_SRC = path.join(__dirname, '..', 'data');
  // 使用临时独立 DB 路径，避免真实 postgres 依赖；降级内存模式仍可验证幂等
  const prov = StorageFactory.create('postgres', { host: '127.0.0.1', database: `rubric_${Date.now()}` });
  prov.connect();
  try {
    prov.migrateFromJSON(DATA_SRC);
    const counts1 = {};
    ['tasks', 'experts', 'projects'].forEach(t => { try { counts1[t] = prov.countByType(t); } catch { counts1[t] = -1; } });
    prov.migrateFromJSON(DATA_SRC);
    const counts2 = {};
    ['tasks', 'experts', 'projects'].forEach(t => { try { counts2[t] = prov.countByType(t); } catch { counts2[t] = -2; } });
    // 幂等：两次 migration 后不产生重复条目
    for (const t of Object.keys(counts1)) {
      ok(counts1[t] === counts2[t], `migrateFromJSON 幂等 [type=${t}] first=${counts1[t]} second=${counts2[t]}`);
    }
  } finally { prov.disconnect(); }
});

// ---- T7-RUBRIC-4: Fake/Memory 降级路径仍然提供等价 API（get/upsert/read-after-write 一致） ----
test('PostgresProvider 无 pg 驱动时（Memory/Fake 降级）：CRUD round-trip 语义一致', () => {
  const prov = StorageFactory.create('postgres', { database: 'red_fake' });
  prov.connect();
  try {
    const ins = prov.upsertEntity('users', 'u1', { name: 'Alice', age: 30 });
    ok(ins != null, 'upsertEntity 返回记录');
    const got = prov.getEntityData('users', 'u1');
    eq(got, { name: 'Alice', age: 30 }, 'read-after-write 数据一致');
    prov.updateEntity('users', 'u1', { name: 'Alice', age: 31 });
    const got2 = prov.getEntityData('users', 'u1');
    eq(got2, { name: 'Alice', age: 31 }, 'update 后读回正确');
    const cnt = prov.countByType('users');
    ok(cnt === 1, `countByType users=1，实际 ${cnt}`);
    prov.deleteEntity('u1');
    ok(prov.getEntity('users', 'u1') == null, 'delete 后返回 null');
  } finally { prov.disconnect(); }
});

console.log(`\n[RED→GREEN T7-Rubric] 结果：${passed} passed / ${failed} failed`);
process.exit(failed === 0 && passed >= 5 ? 0 : 1);
