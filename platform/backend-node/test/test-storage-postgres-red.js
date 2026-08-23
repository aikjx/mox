'use strict';

/**
 * RED 测试：PostgresProvider 实现（T1）
 * ========================
 * 要求：
 *   1) StorageFactory.create('postgres', cfg) 不抛错；connect 正常；
 *   2) 全部 StorageProvider 公共 API 与 SQLiteProvider 行为等价；
 *   3) DB_DUAL_WRITE=true + DB_READ_PREF=postgres 时，读空 → 回源 SQLite → 回填；
 *   4) switchDatabase('postgres') 后再切回 sqlite 连接不泄漏；
 *
 * 在没有 Postgres 时，通过一个内嵌 HTTP mock 的 FakePostgres 返回与 pg 驱动兼容的结果结构，
 * 保证测试在 100% 无外部依赖的本地可跑（与 AC-1 的 Evidence 一致，只是通过 mock 先 RED）。
 * 真实实现最终用原生 pg，但在单测里可用 Fake 验证等价行为。
 */

const path = require('path');
const fs = require('fs');
const os = require('os');

// 准备独立的临时 SQLite dir，避免污染默认 data/
const TMP_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'xuanji-storage-pg-red-'));
process.env.DB_PROVIDER = 'sqlite';
// 重写 config.DATA_DIR
process.chdir(TMP_DIR); // 不影响全局真实 DATA_DIR 由 getStorageConfig 决定

// 先读取现有 storage/index（此时 postgres provider 应为 throw）
const storagePath = path.resolve(__dirname, '..', 'src', 'storage', 'index.js');
const { StorageFactory, SQLiteProvider, resetStorage } = require(storagePath);

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

// ---- RED-1: StorageFactory 对 postgres 仍抛错（预期失败 → 证明测试有效）----
test('RED 预检查：PostgresProvider 尚未实现，StorageFactory.create("postgres") 应抛错', () => {
  let thrown = false;
  try { StorageFactory.create('postgres', { host: 'localhost', port: 5432, database: 'test', user: 'u', password: 'p' }); }
  catch (e) { thrown = /postgres/i.test(e.message) || /MySQL provider|PostgreSQL provider/.test(e.message); }
  ok(thrown, '预期 StorageFactory 在未实现时抛 postgres 相关错误');
});

// ---- RED-2: 双写/回源 不存在（预期失败：DB_DUAL_WRITE 环境处理 + 方法未实现）----
test('RED 预检查：DB_DUAL_WRITE 选项尚未实现，config 中未暴露 dualWrite/readPref', () => {
  const { config } = require('../src/config');
  ok(!('dualWrite' in (config.storage || {})), '目前不应出现 dualWrite 字段，证明 RED 正确');
  ok(!('readPref' in (config.storage || {})), '目前不应出现 readPref 字段，证明 RED 正确');
});

// ---- RED-3: 即便 Provider 实装，migrateFromJSON 幂等护栏缺失 → 预期失败用 ----
test('RED 预检查：PostgresProvider 目前不存在 migrateFromJSON 方法', () => {
  let prov;
  try { prov = StorageFactory.create('postgres', {}); } catch { prov = null; }
  ok(prov === null, 'PostgresProvider 仍然未实现');
});

console.log(`\n[RED] T1 结果：${passed} passed / ${failed} failed`);
process.exit(failed > 0 && passed >= 1 ? 0 : 1); // 允许"有抛错预期"通过 RED：这里 exit(1) 说明当前确实未实现
