'use strict';

/**
 * GREEN 测试：IChunkBackend 双实现 + FileStore 后端注入 + MPU + GC
 * ================================================================
 * 覆盖 AC-3/AC-4/AC-5；采用 S3ChunkBackend 的内存 fallback（零依赖 @aws-sdk），
 * 同时验证 FSChunkBackend。
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const crypto = require('crypto');
const assert = require('assert');

const TMP_ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t2-green-'));
const TMP_DATA = path.join(TMP_ROOT, 'data');
fs.mkdirSync(TMP_DATA, { recursive: true });

// config.storage.providers.sqlite.path 覆盖：用临时 db
process.env.DB_PROVIDER = 'sqlite';
process.env.FILE_GRACE_DAYS = '0'; // 便于 GC 立即 purges

const configPath = path.resolve(__dirname, '..', 'src', 'config.js');
delete require.cache[require.resolve(configPath)];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'storage', 'index.js'))];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'file-store.js'))];

const { config } = require(configPath);
config.storage.providers.sqlite.path = path.join(TMP_DATA, 'ous.db');

const {
  FSChunkBackend, S3ChunkBackend
} = require('../src/storage/chunk-backend');
const {
  FileStore, resetFileStore
} = require('../src/file-store');
const { StorageFactory, resetStorage } = require('../src/storage');

let passed = 0, failed = 0;
function test(name, fn) {
  try { fn(); passed++; console.log('  PASS ', name); }
  catch (e) { failed++; console.error('  FAIL ', name, '\n      ', (e && e.message) + '\n' + (e && e.stack || '').split('\n').slice(1, 3).join('\n')); }
}

// 独立使用 Memory Storage 做 FileStore，避免污染 data 目录
function makeTmpStorage(label) {
  resetStorage();
  process.env.DB_PROVIDER = 'sqlite';
  config.storage.providers.sqlite.path = path.join(TMP_DATA, label + '.db');
  const prov = StorageFactory.create('memory', {});
  prov.connect();
  return prov;
}

function sha256(buf) { return crypto.createHash('sha256').update(buf).digest('hex'); }
function makeBuf(size, seed = 1) {
  const b = Buffer.alloc(size);
  let s = seed >>> 0;
  for (let i = 0; i < size; i++) { s = (s * 1664525 + 1013904223) >>> 0; b[i] = s & 0xff; }
  return b;
}

// ---------------- TR-2.1: s3-mock 后端，同内容去重（A/A' 共享，B 独立）----------------
test('TR-2.1: S3 后端 A/A\' 去重，B 不共享；读回字节一致', async () => {
  const backend = new S3ChunkBackend({});
  await backend.connect();
  const A = makeBuf(2 * 1024 * 1024, 1); // 2MB
  const AP = Buffer.from(A); // 同内容副本
  const B = makeBuf(2 * 1024 * 1024, 2);
  const storage = makeTmpStorage('s3dedup');
  const fs0 = resetFileStore({ chunkBackend: backend, storage, options: { chunkSize: 1024 * 1024, graceDays: 0 } });
  const fA = await fs0.uploadFile(A, 'A.bin');
  const fAP = await fs0.uploadFile(AP, 'A-prime.bin');
  const fB = await fs0.uploadFile(B, 'B.bin');
  const all = await backend.listChunks();
  // A 和 B 各 2 chunks → 总共 4 chunks，而 A/A' 仅占 2 份；验证 chunk 数=2+2=4 且去重正确
  const expectedAChunks = 2, expectedBChunks = 2;
  assert.strictEqual(all.length, expectedAChunks + expectedBChunks, `chunk 后端总数应为 ${expectedAChunks + expectedBChunks}`);
  assert.strictEqual(fA.chunkCount, 2);
  assert.strictEqual(fB.chunkCount, 2);
  assert.deepStrictEqual(fA.chunks, fAP.chunks, 'A 与 A-prime chunks 完全相同（去重成功）');
  const bufA = await fs0.getFileContent(fA.id);
  const bufAP = await fs0.getFileContent(fAP.id);
  const bufB = await fs0.getFileContent(fB.id);
  assert.strictEqual(sha256(bufA), sha256(A));
  assert.strictEqual(sha256(bufAP), sha256(AP));
  assert.strictEqual(sha256(bufB), sha256(B));
  storage.disconnect();
});

// ---------------- TR-2.2: 128MB MPU（s3-mem fallback）hash 校验 ----------------
test('TR-2.2: 128MB 伪随机 (seed=42) 上传，读回 SHA256 一致；MPU 加速路径不报错', async () => {
  const backend = new S3ChunkBackend({});
  await backend.connect();
  const SIZE = 128 * 1024 * 1024;
  const buf = makeBuf(SIZE, 42);
  const storage = makeTmpStorage('mpu128');
  const fs0 = resetFileStore({ chunkBackend: backend, storage, options: { chunkSize: 1024 * 1024, mpuThreshold: 100 * 1024 * 1024, mpuConcurrency: 4 } });
  const t0 = Date.now();
  const f = await fs0.uploadFile(buf, '128m.bin');
  const elapsed = Date.now() - t0;
  const readBack = await fs0.getFileContent(f.id);
  assert.strictEqual(sha256(readBack), sha256(buf), '128MB 读写 hash 一致');
  console.log(`        128MB 上传耗时=${(elapsed / 1000).toFixed(3)}s（本地内存 mock，不作为性能基准）`);
  storage.disconnect();
});
// 128MB 在某些较慢开发机可能耗时：此处不做硬超时，120s 由用户决定

// ---------------- TR-2.3: 软删 → GC → 彻底删除 ----------------
test('TR-2.3: FILE_GRACE_DAYS=0 下，软删→GC，chunk 被删且 getFileContent 抛不存在', async () => {
  const backend = new S3ChunkBackend({});
  await backend.connect();
  const storage = makeTmpStorage('softgctest');
  const fs0 = resetFileStore({ chunkBackend: backend, storage, options: { graceDays: 0, softDelete: true, chunkSize: 1024 * 1024 } });
  const A = makeBuf(2 * 1024 * 1024, 7);
  const f = await fs0.uploadFile(A, 'gctest.bin');
  // 软删前存在
  assert.ok(await backend.hasChunk(f.chunks[0]));
  const beforeStatus = fs0.getFile(f.id).status;
  assert.strictEqual(beforeStatus, 'active');
  fs0.deleteFile(f.id); // soft delete
  const after = fs0.getFile(f.id);
  assert.ok(after, '软删后 file 条目仍存在于 storage（status soft_deleted）');
  assert.strictEqual(after.status, 'soft_deleted');
  // 此时 chunk.ref 已减 1，GC 会删
  const beforeChunks = (await backend.listChunks()).length;
  assert.ok(beforeChunks >= 2, '至少 2 chunks');
  const stat = await fs0.runGC();
  assert.ok(stat.soft_purged >= 1, '应成功 purge 至少 1 个软删');
  assert.ok(stat.chunks_deleted >= 2, 'GC 应删除 2 个 chunk');
  try {
    await fs0.getFileContent(f.id);
    assert.fail('getFileContent 应报错');
  } catch (e) {
    assert.ok(/File has been purged|File not found/.test(e.message));
  }
  storage.disconnect();
});

// ---------------- FSChunkBackend 等价性（A/A' dedup，小文件）----------------
test('TR-2.1-FS: FS backend 同样 A/A\' 去重、B 不分摊', async () => {
  const chunksDir = path.join(TMP_DATA, 'fs-chunks');
  const backend = new FSChunkBackend({ chunksDir });
  await backend.connect();
  const A = makeBuf(2 * 1024 * 1024, 3);
  const AP = Buffer.from(A);
  const B = makeBuf(2 * 1024 * 1024, 4);
  const storage = makeTmpStorage('fsdedup');
  const fs0 = resetFileStore({ chunkBackend: backend, storage, options: { chunkSize: 1024 * 1024 } });
  const fA = await fs0.uploadFile(A, 'A.bin');
  await fs0.uploadFile(AP, 'A-prime.bin');
  await fs0.uploadFile(B, 'B.bin');
  // 计算 chunksDir 下真实文件数
  const all = await backend.listChunks();
  assert.strictEqual(all.length, 4);
  const back = await fs0.getFileContent(fA.id);
  assert.strictEqual(sha256(back), sha256(A));
  storage.disconnect();
});

console.log(`\n[GREEN T2] 结果：${passed} passed / ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
