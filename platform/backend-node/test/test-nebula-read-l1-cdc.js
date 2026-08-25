'use strict';

/**
 * T3 测试：Nebula 适配器：远程读端优先 + L1 TTL + CDC 失效
 * 覆盖：AC-6, TR-3.1, TR-3.2
 * ============================================================
 * 预期：
 *   ① 插入 N1、N2、E1(N1→N2) 后：getNode(N1) 触发 MockRemote._tick('getNode')：计数 = 1
 *   ② 再次 getNode(N1)：L1 命中，计数仍 1
 *   ③ createNode 修改 N1（等价更新），CDC 触发失效后 getNode(N1)：计数再 +1
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const assert = require('assert');

const TMP_ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t3-green-'));
const TMP_DATA = path.join(TMP_ROOT, 'data');
fs.mkdirSync(TMP_DATA, { recursive: true });

const configPath = path.resolve(__dirname, '..', 'src', 'config.js');
delete require.cache[require.resolve(configPath)];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'storage', 'index.js'))];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'nebulagraph-adapter.js'))];

const { config } = require(configPath);
config.storage.providers.sqlite.path = path.join(TMP_DATA, 'ous.db');

const { StorageFactory, resetStorage } = require('../src/storage');
const { NebulaGraphAdapter, resetNebulaGraphAdapter } = require('../src/nebulagraph-adapter');
const { MockRemoteGraphDriver } = require('../src/graph/remote-graph-driver');

let passed = 0, failed = 0;
const pendingAsync = [];
function test(name, fn) {
  try {
    const maybePromise = fn();
    if (maybePromise && typeof maybePromise.then === 'function') {
      pendingAsync.push(
        maybePromise
          .then(() => { passed++; console.log('  PASS (async)', name); })
          .catch(e => { failed++; console.error('  FAIL (async)', name, '\n     ', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 4).join('\n')); })
      );
    } else {
      passed++; console.log('  PASS ', name);
    }
  } catch (e) { failed++; console.error('  FAIL ', name, '\n     ', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 4).join('\n')); }
}

function makeStorage(label) {
  resetStorage();
  const prov = StorageFactory.create('memory', {});
  prov.connect();
  return prov;
}

// TR-3.1: [1,0,1] 序列
test('TR-3.1: 首读→命中 L1→CDC 失效→再读，远程调用计数 1,0,1', () => {
  resetNebulaGraphAdapter();
  const storage = makeStorage('t3-green');
  const driver = new MockRemoteGraphDriver();
  const adapter = new NebulaGraphAdapter({ driver, storage });

  // 建节点 N1,N2；建边 E1
  adapter.createNode({ id: 'N1', kind: 'Project', name: 'P-087' });
  adapter.createNode({ id: 'N2', kind: 'Document', name: '需求文档 RBAC' });
  adapter.createEdge('N1', 'N2', 'HAS_DOC', { weight: 1 });

  // 此时 driver 中 N1/N2 已同步 upsertNode。getNode(N1) 应走远程并 _tick('getNode')。
  driver.resetStats();

  const before1 = driver.callCounts.getNode || 0;
  const r1 = adapter.getNode('N1');
  assert.ok(r1, '应读到 N1');
  const after1 = driver.callCounts.getNode || 0;
  assert.strictEqual(after1 - before1, 1, `首次读 N1：远程 getNode 调用次数 +1（实际 ${after1 - before1}）`);

  // 第二次：命中 L1，不应调用
  const before2 = driver.callCounts.getNode || 0;
  const r2 = adapter.getNode('N1');
  assert.ok(r2);
  const after2 = driver.callCounts.getNode || 0;
  assert.strictEqual(after2 - before2, 0, `二次读 N1：L1 命中，不应调远程（实际 ${after2 - before2}）`);

  // 修改 N1：触发 CDC，L1 失效 → 再读应 +1
  adapter.updateNode('N1', { description: 'Updated' });
  const before3 = driver.callCounts.getNode || 0;
  const r3 = adapter.getNode('N1');
  assert.ok(r3);
  const after3 = driver.callCounts.getNode || 0;
  assert.strictEqual(after3 - before3, 1, `CDC 失效后再读：远程 +1（实际 ${after3 - before3}）`);

  storage.disconnect();
});

// TR-3.2：远程 503 → 仍可从本地/缓存拿（降级）
test('TR-3.2: 驱动挂掉时，adapter.getNode 仍能通过本地/L1 返回（降级标记）', async () => {
  resetNebulaGraphAdapter();
  const storage = makeStorage('t3-degrade');
  const driver = new MockRemoteGraphDriver();
  // 让驱动 getNode 抛错：通过 Object.defineProperty 重写
  const orig = driver.getNode.bind(driver);
  driver.getNode = function () { throw new Error('remote 503'); };
  const adapter = new NebulaGraphAdapter({ driver, storage });
  adapter.createNode({ id: 'X1', kind: 'Test' });
  const got = adapter.getNode('X1');
  assert.ok(got, '远程挂掉时，本地仍可读');
  assert.strictEqual(got.kind, 'Test');

  // 或者用 LruCache：L1 里放后清除驱动后再读，仍拿得到
  adapter.l1.set('node:X2', { node: { id: 'X2', kind: 'FakedByCache' }, source: 'cache' });
  const cached = adapter.getNode('X2');
  assert.ok(cached && cached.kind === 'FakedByCache', 'L1 缓存命中时不依赖远程驱动');

  storage.disconnect();
});

// LruCache 基础：size/ttl 语义
test('T3-aux: LruCache TTL 与容量边界', async () => {
  const { LruCache } = require('../src/nebulagraph-adapter');
  const c = new LruCache({ max: 2, ttlMs: 10 });
  c.set('a', 1); c.set('b', 2); c.set('c', 3); // max=2 → a 被淘汰
  assert.strictEqual(c.get('a'), undefined, 'max=2 下应淘汰 a');
  assert.strictEqual(c.get('b'), 2);
  assert.strictEqual(c.get('c'), 3);
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.strictEqual(c.get('b'), undefined, 'TTL 过期 b 应被清');
});

// CdcEventBus DLQ：消费抛错 → 进入 dlq
test('T3-aux: CdcEventBus 监听器抛错 → 写入 DLQ', () => {
  const { CdcEventBus } = require('../src/nebulagraph-adapter');
  const bus = new CdcEventBus();
  bus.on('graph:node_updated', () => { throw new Error('boom'); });
  const evt = bus.emitEvent('graph:node_updated', { id: 'x' });
  assert.strictEqual(bus.dlq.length, 1, 'DLQ 应含 1 条');
  assert.ok(evt && evt.seq >= 1);
});

console.log(`\n[GREEN T3] 已提交：${passed} passed / ${failed} failed；等待异步用例...`);
Promise.all(pendingAsync).then(() => {
  console.log(`[GREEN T3] 最终：${passed} passed / ${failed} failed`);
  process.exit(failed === 0 ? 0 : 1);
});
