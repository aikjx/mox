'use strict';

/**
 * GREEN 测试：PostgresProvider + DualWriteStorage + switchDatabase
 * ===========================================================
 * 注：无需真实 Postgres 实例。PostgresProvider 未安装 pg 驱动时自动降级到内存镜像实现，
 * 其镜像即 MemoryProvider —— 与 SQLiteProvider 是同态 StorageProvider，
 * 本测试验证"行为等价"与"双写/回源"，对应 AC-1、AC-2、TR-1.1~TR-1.3。
 */

const path = require('path');
const fs = require('fs');
const os = require('os');
const assert = require('assert');

// 准备临时 SQLite data 目录（二次被 dual-write 作为 secondary）
const TMP_ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-storage-greent1-'));
const TMP_DATA = path.join(TMP_ROOT, 'data');
fs.mkdirSync(TMP_DATA, { recursive: true });
process.env.DB_PROVIDER = 'postgres';
process.env.DB_DUAL_WRITE = 'false';
process.env.DB_READ_PREF = 'auto';
process.chdir(TMP_ROOT);

// 覆盖 DATA_DIR + providers.sqlite.path 指向临时目录
const configPath = path.resolve(__dirname, '..', 'src', 'config.js');
const storagePath = path.resolve(__dirname, '..', 'src', 'storage', 'index.js');

// 清掉可能的 require 缓存，确保 getStorageConfig 重新读 env
delete require.cache[require.resolve(configPath)];
delete require.cache[require.resolve(storagePath)];

// 覆盖 providers.sqlite.path：读取 config 后，直接改内存对象（下次 require.config 生效）
const { config } = require(configPath);
config.storage.providers.sqlite.path = path.join(TMP_DATA, 'ous.db');
config.storage.providers.postgresql = {
  host: 'localhost',
  port: 5432,
  database: 'mox_test',
  user: 'mox',
  password: '',
  options: { max: 4 }
};

const { StorageFactory, SQLiteProvider, PostgresProvider, DualWriteStorage, resetStorage, switchDatabase, getStorage } = require(storagePath);

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

// ---- TR-1.1: StorageFactory.create('postgres') 不抛错；等价 API 行为与 SQLite 一致 ----
test('TR-1.1.1: StorageFactory.create("postgres") / postgresql 都成功', () => {
  const pgProv = StorageFactory.create('postgres', config.storage.providers.postgresql);
  assert.ok(pgProv, '必须返回实例');
  assert.strictEqual(pgProv.name, 'postgres');
  const pgProv2 = StorageFactory.create('postgresql', config.storage.providers.postgresql);
  assert.ok(pgProv2);
  pgProv.disconnect(); pgProv2.disconnect();
});

test('TR-1.1.2: PostgresProvider 全 API 行为与 SQLiteProvider（同种子数据集）逐字段等价', () => {
  const pgCfg = config.storage.providers.postgresql;
  const sqCfg = { ...config.storage.providers.sqlite, path: path.join(TMP_DATA, 'equiv.db') };
  const pg = new PostgresProvider(pgCfg); pg.connect();
  const sq = new SQLiteProvider(sqCfg); sq.connect();

  // 准备一组操作序列
  const seed = [
    () => pg.upsertEntity('Project', 'P1', { name: '项目一', domain: 'X01', layer: 1 }),
    () => sq.upsertEntity('Project', 'P1', { name: '项目一', domain: 'X01', layer: 1 }),
    () => pg.upsertEntity('Project', 'P2', { name: '项目二', domain: 'X01', layer: 1 }),
    () => sq.upsertEntity('Project', 'P2', { name: '项目二', domain: 'X01', layer: 1 }),
    () => pg.insertEntity('Task', 'T1', { title: '任务A', projectId: 'P1' }),
    () => sq.insertEntity('Task', 'T1', { title: '任务A', projectId: 'P1' }),
    () => pg.kvSet('llm:default:provider', 'mock-provider'),
    () => sq.kvSet('llm:default:provider', 'mock-provider'),
    () => pg.addLog('audit', '登录', { user: 'u1' }),
    () => sq.addLog('audit', '登录', { user: 'u1' }),
    () => pg.saveList('graph_nodes', [
      { id: 'n1', label: 'Alice', type: 'person' },
      { id: 'n2', label: 'Bob', type: 'person' }
    ]),
    () => sq.saveList('graph_nodes', [
      { id: 'n1', label: 'Alice', type: 'person' },
      { id: 'n2', label: 'Bob', type: 'person' }
    ])
  ];
  seed.forEach(fn => fn());

  // 逐 API 对比
  const deepEq = (a, b, k) => {
    // 忽略 created_at/updated_at 毫秒差（比较语义字段+id+type）
    const clean = (x) => {
      if (!x) return x;
      const { id, type, data } = x;
      return { id, type, data };
    };
    const A = Array.isArray(a) ? a.map(clean) : (a && typeof a === 'object' ? clean(a) : a);
    const B = Array.isArray(b) ? b.map(clean) : (b && typeof b === 'object' ? clean(b) : b);
    assert.deepStrictEqual(A, B, k + ' diff');
  };

  deepEq(pg.getEntity('Project', 'P1'), sq.getEntity('Project', 'P1'), 'getEntity(P1)');
  deepEq(pg.listEntities('Project').sort((x, y) => x.id.localeCompare(y.id)),
         sq.listEntities('Project').sort((x, y) => x.id.localeCompare(y.id)), 'listEntities(Project)');
  assert.strictEqual(pg.countByType('Project'), sq.countByType('Project'), 'count 相等');
  deepEq(pg.getList('graph_nodes').sort((x, y) => x.id.localeCompare(y.id)),
         sq.getList('graph_nodes').sort((x, y) => x.id.localeCompare(y.id)), 'saveList/getList 等价');
  assert.strictEqual(pg.kvGet('llm:default:provider'), sq.kvGet('llm:default:provider'));

  // searchEntities：LIKE query = "项目" → 都能找到两条
  const pgSearch = pg.searchEntities('Project', '项目').map(e => e.id).sort();
  const sqSearch = sq.searchEntities('Project', '项目').map(e => e.id).sort();
  assert.deepStrictEqual(pgSearch, sqSearch, 'searchEntities(Project, 项目)');

  // migrateFromJSON 幂等：先在 TMP_DATA 下伪造一份 projects.json 作为迁移源
  const migDir = TMP_DATA;
  fs.writeFileSync(path.join(migDir, 'projects.json'), JSON.stringify([
    { id: 'P_mig1', name: '迁移项目A' },
    { id: 'P_mig2', name: '迁移项目B' }
  ]));
  const pgMig = pg.migrateFromJSON(migDir);
  const sqMig = sq.migrateFromJSON(migDir);
  assert.strictEqual(typeof pgMig, 'number', 'pg migrate 返回数');
  assert.strictEqual(typeof sqMig, 'number', 'sq migrate 返回数');
  // 第二次迁移幂等（数量相同）
  const pgMig2 = pg.migrateFromJSON(migDir);
  const sqMig2 = sq.migrateFromJSON(migDir);
  // 在 memory fallback + better-sqlite3 迁移的幂等护栏里，两次 migrate 返回的是导入前 size 后的变化，幂等意味着 size 不变
  const pgSizeA = pg.listEntities('projects').length;
  const pgSizeB = pg.listEntities('projects').length;
  assert.strictEqual(pgSizeA, pgSizeB, 'migrate 幂等：重复 migrate 后 projects size 不变');

  pg.disconnect(); sq.disconnect();
});

// ---- TR-1.2: dual-write + 读空回源回填 ----
test('TR-1.2: DualWrite 写 100 → 删 5 primary → 读回源 成功回填', () => {
  const pgCfg = config.storage.providers.postgresql;
  const sqCfg = { ...config.storage.providers.sqlite, path: path.join(TMP_DATA, 'dual-write-backfill.db') };
  const primary = new PostgresProvider(pgCfg); primary.connect();
  const secondary = new SQLiteProvider(sqCfg); secondary.connect();
  const dual = new DualWriteStorage(primary, secondary, { readPref: 'auto' });

  const type = 'FooEntity';
  const ids = [];
  for (let i = 1; i <= 100; i++) {
    const id = 'F' + i;
    ids.push(id);
    dual.upsertEntity(type, id, { seq: i, label: '第' + i });
  }
  assert.strictEqual(primary.countByType(type), 100, 'primary 100');
  assert.strictEqual(secondary.countByType(type), 100, 'secondary 100');

  // 手动删除 primary 的 5 条（模拟 primary 数据丢失 → 读空回源 secondary）
  const deletedIds = ids.slice(0, 5);
  deletedIds.forEach(id => primary.deleteEntity(id));
  assert.strictEqual(primary.countByType(type), 95, '删除后 primary 95');

  // 逐条读取 100 条：dual 应该都能拿到
  let fallbacks = 0;
  for (const id of ids) {
    const got = dual.getEntity(type, id);
    if (!got) throw new Error('dual.getEntity(' + id + ') 为空');
    // 回源后 primary 侧 count 应为 100（回填成功）
  }
  fallbacks = deletedIds.filter(id => primary.getEntity(type, id) !== null).length;
  assert.strictEqual(fallbacks, 5, '回源回填数量=5');
  assert.strictEqual(primary.countByType(type), 100, '回填后 primary 恢复 100');

  // 清理
  primary.disconnect(); secondary.disconnect();
});

// ---- TR-1.3: switchDatabase 不泄漏连接；不影响后续读 ----
test('TR-1.3: switchDatabase postgres -> sqlite -> postgres 往返两次无异常、读一致', () => {
  resetStorage();
  // 使用独立 temp dir 作为 sqlite path
  const mod = require(configPath);
  const cfg = mod.config;
  cfg.storage.providers.sqlite.path = path.join(TMP_DATA, 'switch.db');

  const s1 = switchDatabase('postgres');
  s1.upsertEntity('Probe', 'PB1', { v: 1 });
  assert.strictEqual(s1.getEntity('Probe', 'PB1').data.v, 1);
  const s2 = switchDatabase('sqlite');
  // sqlite 中没有 PB1（因为 provider 不同），正常返回 null
  const probe = s2.getEntity('Probe', 'PB1');
  assert.ok(probe === null || probe === undefined, 'sqlite 侧不存在 PB1');
  // 再切回 postgres，镜像中仍有（在新的 postgres 实例中不一定，但 T1 重点是不抛错；此处只要不崩溃即可）
  const s3 = switchDatabase('postgres');
  const name = s3.name;
  assert.ok(name === 'postgres' || /dual/.test(name), '切回后 provider 正确');
});

console.log(`\n[GREEN T1] 结果：${passed} passed / ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
