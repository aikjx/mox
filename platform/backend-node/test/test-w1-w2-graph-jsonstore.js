'use strict';
/**
 * W1 立即修补 + W2 图谱骨架单元测试
 * 覆盖：
 *   [A] json-store 原子写：半写/模拟崩溃 0 损坏
 *   [B] L3.5 图谱 6 接口 CRUD 行为正确
 *   [C] 3 条金链 findPath 非空（需求根因 6 跳 / 切换审计 / 组织进化）
 *   [D] 红线 3：removeEdge 只 tombstone 不物理删，原因可回溯
 *   [E] 3 Provider 等价：SQLite / Memory / Postgres(fallback) 同构结果逐字段一致
 */
const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

// ========= 工具：临时目录 =========
const tmpRoot = path.join(os.tmpdir(), `mox-w1w2-${Date.now()}-${process.pid}`);
fs.mkdirSync(tmpRoot, { recursive: true });
const cleanup = [];
const tmpFile = (name) => { const f = path.join(tmpRoot, name); cleanup.push(f); return f; };
const removeSilent = (p) => { try { fs.rmSync(p, { recursive: true, force: true }); } catch {} };
after(() => { for (const p of cleanup) removeSilent(p); removeSilent(tmpRoot); });

// ========= [A] json-store 原子写（半写/崩溃不损坏） =========
describe('W1 · json-store writeJSON 原子写', () => {
  const dir = tmpFile('json-store');
  const atomicWrite = require('../src/lib/json-store')._testAtomicWrite;
  before(() => {
    fs.mkdirSync(dir, { recursive: true });
  });

  it('A1 · 普通 100 次读写：始终 parse 成功（直接测原子写函数落盘）', () => {
    const target = path.join(dir, 'settings.json');
    const orig = { v: 0, arr: Array.from({ length: 50 }, (_, i) => i) };
    atomicWrite(target, JSON.stringify(orig));
    assert.deepStrictEqual(JSON.parse(fs.readFileSync(target, 'utf8')), orig);
    for (let i = 0; i < 100; i++) {
      const next = { v: i + 1, huge: 'x'.repeat(12 * 1024), arr: Array.from({ length: i % 37 }, (_, j) => j) };
      atomicWrite(target, JSON.stringify(next));
      const got = JSON.parse(fs.readFileSync(target, 'utf8'));
      assert.deepStrictEqual(got.v, next.v, `第${i}次读回 v 不一致`);
      assert.doesNotThrow(() => JSON.stringify(got), `第${i}次读回损坏: ${got}`);
    }
  });

  it('A2 · 崩溃场景：写一半抛错时旧版本仍保持完整（原子 rename）', () => {
    // 先写旧版本
    const target = path.join(dir, 'golden.json');
    const OLD = { id: 'golden', payload: Array.from({ length: 200 }, (_, i) => i * i) };
    atomicWrite(target, JSON.stringify(OLD));
    assert.deepStrictEqual(JSON.parse(fs.readFileSync(target, 'utf8')), OLD);

    // monkey-patch：模拟 atomicWriteFileSync 中 write tmp 写一半抛错
    const origWrite = fs.writeFileSync;
    let injected = 0;
    fs.writeFileSync = function (p, data, enc) {
      // 只针对 *.tmp 文件：第 1 次写只写半截，然后抛错
      if (p.endsWith('.tmp') && injected === 0) {
        injected++;
        const half = String(data).slice(0, Math.floor(String(data).length / 2));
        origWrite.call(fs, p, half, enc); // 半写
        throw new Error('A2 simulated crash: power loss while writing tmp');
      }
      return origWrite.apply(fs, arguments);
    };
    const NEW_BAD = { id: 'bad', payload: 'CRASH'.repeat(10000) };
    assert.throws(() => atomicWrite(target, JSON.stringify(NEW_BAD)), /simulated crash/);
    fs.writeFileSync = origWrite; // 还原

    // 旧版本必须完全没动：直接读磁盘文件解析，不得损坏
    const parsedDisk = JSON.parse(fs.readFileSync(target, 'utf8'));
    assert.deepStrictEqual(parsedDisk, OLD, `崩溃后目标磁盘文件必须完整保留旧版本，不得半写 / 损坏 / 覆盖成半写内容`);

    // 残留 *.tmp 文件必须清理（atomicWriteFileSync catch 里会 unlink）
    const tmps = fs.readdirSync(dir).filter(n => n.endsWith('.tmp'));
    assert.deepStrictEqual(tmps, [], `崩溃后必须无残留临时文件（GC 干净）`);
  });
});

// ========= [B/C/D/E] L3.5 图谱 6 接口 =========
const { StorageFactory, resetStorage } = require('../src/storage');

function makeProviders(suffix) {
  const p = {};
  // SQLite：临时文件
  const sqliteDb = tmpFile(`graph-${suffix}.db`);
  p.sqlite = StorageFactory.create('sqlite', { path: sqliteDb, options: {} });
  p.sqlite.connect();
  // Memory
  p.memory = StorageFactory.create('memory', {});
  p.memory.connect();
  // Postgres（无 pg 驱动时自动降级到 memory fallback，行为完全同构；我们验证降级模式等价）
  p.pg = StorageFactory.create('postgres', { host: 'localhost', port: 5432, database: 'mox_test', user: 'u', password: 'p', options: {} });
  // connect() 可能返回 promise，但 pg 驱动缺失场景下是同步降级；这里同步调用完成后仍可用 mirror
  const r = p.pg.connect();
  // 如果返回 promise（pg 驱动存在），我们不做真实 PG 连接，只测 fallback 镜像即可（等价性目标）
  return Promise.resolve(r).then(() => p);
}

// 三条金链写入（需求根因链 6 跳 · 切换审计链 · 组织进化链）
function writeThreeGoldenChains(s) {
  // 1. 需求根因链 Requirement→Design→API→Code→TestCase→Bug（6跳=5边？不对：6个节点对应5条边，金链 1 是 6 跳 = 6 条边 = 7 个节点。我们按总纲 §4.2 写 7 节点 6 边：Requirement → Design → API_Contract → CodeFile → TestCase → Bug → Incident（更完整）
  const R = 'req:r1'; const D = 'design:d1'; const A = 'api:a1'; const C = 'code:c1'; const T = 'test:t1'; const B = 'bug:b1'; const I = 'inc:i1';
  s.addEdge(R, 'tracks_back_to', D, { section: '3.2' });
  s.addEdge(D, 'realized_by', A, { owner: 'arch' });
  s.addEdge(A, 'implements', C, { lang: 'ts' });
  s.addEdge(C, 'implements', T, { framework: 'mocha' });
  s.addEdge(T, 'found_in', B, { severity: 'critical' });
  s.addEdge(B, 'caused_by', I, { mttr: 180 });

  // 2. 切换审计链
  const CR = 'cr:001'; const PL = 'plan:sp1'; const MJ = 'job:mj1'; const VR = 'rpt:vr1'; const SN = 'snap:s1'; const HB = 'hash:hb1';
  s.addEdge(CR, 'targets', PL, { env: 'prod' });
  s.addEdge(PL, 'released_via', MJ, { size: '300GB' });
  s.addEdge(MJ, 'validates_end_to_end', VR, { mismatch: 0 });
  s.addEdge(VR, 'rollback_to', SN, { tag: 'pre-cutover' });
  s.addEdge(SN, 'contains', HB, { algo: 'sha256' });

  // 3. 组织进化闭环 improves_next
  const AAR = 'aar:1'; const PRJ_NEXT = 'project:next-2026q3';
  s.addEdge(AAR, 'improves_next', PRJ_NEXT, { lessons: ['双写预热', '对账窗口延长 7→14'] });
  return {
    chain1Nodes: [R, D, A, C, T, B, I],
    chain2Nodes: [CR, PL, MJ, VR, SN, HB],
    chain3Nodes: [AAR, PRJ_NEXT]
  };
}

describe('W2 · 知识图谱 L3.5 中枢 6 接口', () => {
  let providers;
  before(async () => { providers = await makeProviders('main'); });

  it('B1 · addEdge/removeEdge/neighbors 基本行为', () => {
    for (const name of ['sqlite', 'memory', 'pg']) {
      const s = providers[name];
      s.addEdge('a', 'r1', 'b', { v: 1 });
      s.addEdge('b', 'r1', 'c', { v: 2 });
      const nA = s.neighbors('a');
      assert.ok(nA.find(e => e.dst === 'b' && e.rel === 'r1'), `${name}: a→b 边丢失`);
      const nB = s.neighbors('b', 'in');
      assert.ok(nB.find(e => e.src === 'a'), `${name}: b 的入邻居应包含 a`);
      s.removeEdge('a', 'r1', 'b', 'deprecate');
      const nA2 = s.neighbors('a');
      assert.strictEqual(nA2.length, 0, `${name}: removeEdge 后应无 live 边`);
    }
  });

  it('C1 · 三条金链 findPath 非空且长度匹配（6跳/5跳/2跳）', () => {
    for (const name of ['sqlite', 'memory', 'pg']) {
      const s = providers[name];
      const ids = writeThreeGoldenChains(s);
      // 链1 6条边（R→D→A→C→T→B→I）从 R 到 I 最短 6 跳
      const p1 = s.findPath(ids.chain1Nodes[0], ids.chain1Nodes[ids.chain1Nodes.length - 1], 7);
      assert.ok(Array.isArray(p1), `${name}: 需求根因链 6 跳 必须有解, got ${p1}`);
      assert.strictEqual(p1.length, ids.chain1Nodes.length - 1, `${name}: 需求根因链应为 ${ids.chain1Nodes.length - 1} 条边, got ${p1.length}`);
      // 链2 5条边
      const p2 = s.findPath(ids.chain2Nodes[0], ids.chain2Nodes[ids.chain2Nodes.length - 1], 6);
      assert.ok(Array.isArray(p2), `${name}: 切换审计链 5 跳 必须有解`);
      assert.strictEqual(p2.length, ids.chain2Nodes.length - 1, `${name}: 切换审计链应为 ${ids.chain2Nodes.length - 1} 条边, got ${p2.length}`);
      // 链3 1条边
      const p3 = s.findPath(ids.chain3Nodes[0], ids.chain3Nodes[ids.chain3Nodes.length - 1], 2);
      assert.ok(Array.isArray(p3), `${name}: 组织进化链 1 跳 必须有解`);
      assert.strictEqual(p3.length, 1, `${name}: 组织进化链应为 1 边, got ${p3.length}`);
    }
  });

  it('D1 · 红线 3：removeEdge 永远只 tombstone，不物理删', () => {
    const s = providers.sqlite;
    const ST = (sql, obj) => s.db.prepare(sql).get(obj);
    // 新增一条边，统计 graph_edges 总数（命名参数防占位数误判）
    s.addEdge('x', 'rel_of_lineage', 'y', { k: 1 });
    const before = s.db.prepare('SELECT COUNT(*) as c FROM graph_edges').get().c;
    s.removeEdge('x', 'rel_of_lineage', 'y', '审计删除原因-2026');
    const after = s.db.prepare('SELECT COUNT(*) as c FROM graph_edges').get().c;
    assert.strictEqual(after, before, `红线3：removeEdge 前后记录总数必须完全相等（只 tombstone 不物理删）`);
    const row = ST('SELECT tombstone, reason FROM graph_edges WHERE src = @src AND rel = @rel AND dst = @dst',
      { src: 'x', rel: 'rel_of_lineage', dst: 'y' });
    assert.strictEqual(row.tombstone, 1, `红线3：tombstone 必须 = 1`);
    assert.strictEqual(row.reason, '审计删除原因-2026', `红线3：原因必须完整保留在 reason 字段，7 年内审计可回放`);
    // 再加回去：UNIQUE 冲突 do update tombstone=0 reason=null
    s.addEdge('x', 'rel_of_lineage', 'y', { k: 2 });
    const cntAfterReadd = s.db.prepare('SELECT COUNT(*) c FROM graph_edges WHERE src = @src AND rel = @rel AND dst = @dst')
      .get({ src: 'x', rel: 'rel_of_lineage', dst: 'y' }).c;
    const tsAfterReadd = ST('SELECT tombstone ts FROM graph_edges WHERE src = @src AND rel = @rel AND dst = @dst',
      { src: 'x', rel: 'rel_of_lineage', dst: 'y' }).ts;
    assert.strictEqual(cntAfterReadd, 1, `加回去后仍保持 1 条记录（UNIQUE 触发 upsert，不是新增）`);
    assert.strictEqual(tsAfterReadd, 0, `加回去后 tombstone 必须归零 = 0`);
  });

  it('E1 · 三 Provider 同构：同写入=同 neighbors / findPath / pageRank / neighborhood', () => {
    const names = ['sqlite', 'memory', 'pg'];
    const snapshots = {};
    // === 保证同构前提：先把每个 provider 之前测试写入的边全部清空，再写统一的 4 条 eq:* 边 ===
    for (const name of names) {
      const s = providers[name];
      if (name === 'sqlite') {
        s.db.prepare('DELETE FROM graph_edges').run();
      } else if (name === 'memory') {
        s.edges.clear();
      } else { // pg fallback 内存镜像
        const mirror = s._mirror ? s._mirror() : s;
        if (mirror && mirror.edges) mirror.edges.clear();
      }
    }
    for (const name of names) {
      const s = providers[name];
      s.addEdge('eq:a', 'knows', 'eq:b', { w: 0.3 });
      s.addEdge('eq:b', 'knows', 'eq:c', { w: 0.5 });
      s.addEdge('eq:c', 'knows', 'eq:a', { w: 0.9 }); // 环
      s.addEdge('eq:a', 'refers', 'eq:c', { w: 1 });
      snapshots[name] = {
        nb: s.neighbors('eq:a'),
        path: s.findPath('eq:a', 'eq:c', 3),
        sub: s.neighborhoodSubgraph(['eq:a'], 2, 100),
        pr: s.pageRank() instanceof Map ? Array.from(s.pageRank().entries()).sort((a, b) => a[0].localeCompare(b[0])) : null
      };
    }
    // neighbors 集合相等（不依赖顺序，用 src|rel|dst 排序后比）
    const keyE = (e) => `${e.src}|${e.rel}|${e.dst}`;
    const sortE = (arr) => arr.map(keyE).slice().sort();
    for (const n of names) {
      assert.deepStrictEqual(sortE(snapshots[n].nb), sortE(snapshots.sqlite.nb), `neighbors 不等: sqlite vs ${n}`);
      assert.deepStrictEqual(snapshots[n].path.map(keyE), snapshots.sqlite.path.map(keyE), `findPath 不等: sqlite vs ${n}`);
      // neighborhoodSubgraph：edges 排序比（nodes 长度相同即同）
      assert.strictEqual(snapshots[n].sub.nodes.length, snapshots.sqlite.sub.nodes.length, `subgraph nodes length 不等 vs ${n}`);
      assert.deepStrictEqual(sortE(snapshots[n].sub.edges), sortE(snapshots.sqlite.sub.edges), `subgraph edges 不等: sqlite vs ${n}`);
      // pageRank：数值近似相等 容差 1e-6（两边边集合完全一致后，20 次迭代应基本精确相等，浮点误差宽容避免抖动）
      for (let i = 0; i < snapshots.sqlite.pr.length; i++) {
        assert.strictEqual(snapshots[n].pr[i][0], snapshots.sqlite.pr[i][0], `pr key 不等 vs ${n}`);
        assert.ok(Math.abs(snapshots[n].pr[i][1] - snapshots.sqlite.pr[i][1]) < 1e-6,
          `pr value 不等: ${snapshots.sqlite.pr[i][0]} sqlite=${snapshots.sqlite.pr[i][1]} ${n}=${snapshots[n].pr[i][1]}`);
      }
    }
  });
});
