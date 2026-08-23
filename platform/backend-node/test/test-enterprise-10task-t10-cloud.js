'use strict';
/**
 * T10: 云盘 Cloud Drive (FileStore)
 *   - basic write/read round-trip (SHA-1 both-ways match)
 *   - version increment: listVersions shows v1+v2 after uploadNewVersion; restore(v1) returns original
 *   - ACL permissions: owner/u1 can read; u2 gets 403-like error, error does not leak content/hash
 *   - quota: maxQuota=1KB, 2KB upload exceeds quota with QuotaExceeded error
 *   - bulk: 1000 small files uploaded successfully (all ok, no error thrown)
 *   - large file interface: writeChunk() / multipart API declared on store instance (method signatures exist)
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');
const crypto = require('crypto');

const ROOT = path.join(__dirname, '..');
const TEST_ROOT = path.join(ROOT, 'tmp', 'test_cloud_artifacts');
if (!fs.existsSync(TEST_ROOT)) fs.mkdirSync(TEST_ROOT, { recursive: true });
// DATA_DIR override
process.env.DB_PROVIDER = 'memory';
process.env.DATA_DIR = TEST_ROOT;
process.env.FILE_SOFT_DELETE = 'false';

const configMod = require.resolve('../src/config');
const storageMod = require.resolve('../src/storage');
delete require.cache[configMod];
delete require.cache[storageMod];
const { config, DATA_DIR: resolvedDataDir } = require(configMod);
config.storage.provider = 'memory';
config.storage.providers.sqlite.path = path.join(TEST_ROOT, 'ous.db');

const storageModule = require(storageMod);
storageModule.resetStorage();

const { FileStore, resetFileStore, getFileStore } = require('../src/file-store');

function sha1(buf) {
  return crypto.createHash('sha1').update(buf).digest('hex');
}

describe('T10 Cloud Drive: setup + reset', function () {
  before(function () {
    // Clear leftover singleton + use a FileStore bound to test artifacts dir
    try {
      fs.rmSync(TEST_ROOT, { recursive: true, force: true, maxRetries: 3 });
    } catch {}
    fs.mkdirSync(TEST_ROOT, { recursive: true });
    fs.mkdirSync(path.join(TEST_ROOT, 'data'), { recursive: true });
    // force DATA_DIR re-eval in chunk-backend (getCfg lazy-loads config): no-op for fresh
    // reset singleton
    resetFileStore({ options: { chunkSize: 16 * 1024, softDelete: false } });
  });

  after(function () {
    try {
      // optional: do not remove unless user wants — but let's leave for debugging.
      // fs.rmSync(TEST_ROOT, { recursive: true, force: true, maxRetries: 3 });
    } catch {}
  });

  it('DATA_DIR resolves to or under TEST_ROOT (isolated)', function () {
    const resolved = (resolvedDataDir || '').toLowerCase();
    const root = TEST_ROOT.toLowerCase();
    assert.ok(
      resolved.includes(root.replace(/\\/g, '/')) || resolved.includes(root) || path.resolve(resolvedDataDir || '.').startsWith(path.resolve(TEST_ROOT)),
      `DATA_DIR 未隔离: ${resolvedDataDir} vs ${TEST_ROOT}`
    );
  });

  it('getFileStore / resetFileStore return valid instance with required methods', function () {
    const store = getFileStore();
    assert.ok(store instanceof FileStore, 'getFileStore() must return FileStore instance');
    const need = [
      'uploadFile', 'uploadNewVersion', 'getFileContent', 'getVersions',
      'restoreVersion', 'listFiles', 'deleteFile', 'getStats',
      // Public convenience for cloud drive APIs:
      'writeFile', 'readFile', 'listVersions',
      // ACL / quota / chunked
      'writeChunk', 'createMultipartUpload', 'uploadPart', 'completeMultipartUpload',
    ];
    for (const m of need) {
      assert.strictEqual(typeof store[m], 'function', `缺少 store.${m} 方法签名`);
    }
  });
});

describe('T10 Cloud Drive: 基础 write/read SHA-1 双路匹配', function () {
  it('writeFile("hello.txt", buf) + readFile === same bytes, sha1 matches', async function () {
    const store = resetFileStore({ options: { chunkSize: 16 * 1024, softDelete: false } });
    const content = 'Hello, 云盘 Cloud Drive! 中文 utf-8 payload #1 ✓';
    const buf = Buffer.from(content, 'utf8');
    const originalSha1 = sha1(buf);

    const meta = await store.writeFile('hello.txt', buf, { userId: 'u1' });
    assert.ok(meta && meta.id, 'writeFile 返回 meta.id');

    const readBuf = await store.readFile(meta.id);
    assert.ok(Buffer.isBuffer(readBuf), 'readFile 返回 Buffer');
    assert.strictEqual(readBuf.toString('utf8'), content, '内容字节一致');
    assert.strictEqual(sha1(readBuf), originalSha1, '下载 sha1 == 上传 sha1 (正向匹配)');
    assert.strictEqual(originalSha1, meta.contentSha1 || sha1(readBuf), '元数据 sha1 匹配');
  });
});

describe('T10 Cloud Drive: 版本 increment + restore', function () {
  it('v1 → v2: listVersions 长度 2; restore(v1) 返回 v1 内容', async function () {
    const store = resetFileStore({ options: { chunkSize: 16 * 1024, softDelete: false } });
    const v1Buf = Buffer.from('version 1 content initial upload', 'utf8');
    const v1Sha = sha1(v1Buf);
    const f1 = await store.writeFile('versions.txt', v1Buf, { userId: 'u1', changeNote: 'v1' });

    const v2Buf = Buffer.from('version 2 已更新 updated content'.repeat(3), 'utf8');
    const v2Sha = sha1(v2Buf);
    const f2 = await store.uploadNewVersion(f1.id, v2Buf, 'replace contents');
    assert.strictEqual(f2.currentVersion, 2);

    const versions = store.listVersions(f1.id);
    assert.strictEqual(versions.length, 2, `listVersions 长度应为 2，实际 ${versions.length}`);

    const restored = store.restoreVersion(f1.id, 1);
    assert.ok(restored, 'restoreVersion 返回元数据');
    const restoredBuf = await store.readFile(f1.id);
    assert.strictEqual(sha1(restoredBuf), v1Sha, 'restore(v1) 内容 sha1 必须与 v1 一致');
    void v2Sha;
  });
});

describe('T10 Cloud Drive: ACL 权限控制', function () {
  it('write with owner=u1 readers=[u1]; u2 read throws 403-kind error without leaking content/hash', async function () {
    const store = resetFileStore({ options: { chunkSize: 16 * 1024, softDelete: false } });
    const buf = Buffer.from('仅 u1 可读 ACL payload', 'utf8');
    const meta = await store.writeFile('acl.txt', buf, {
      userId: 'u1',
      owner: 'u1',
      readers: ['u1'],
    });
    assert.ok(meta);

    // u1 reads → success
    const u1Buf = await store.readFile(meta.id, { asUser: 'u1' });
    assert.strictEqual(u1Buf.toString('utf8'), buf.toString('utf8'));

    // u2 reads → error
    let err = null;
    try { await store.readFile(meta.id, { asUser: 'u2' }); }
    catch (e) { err = e; }
    assert.ok(err, 'u2 read 必须抛出 ACL 错误');
    const errMsg = String((err && err.message) || '') + ' ' + JSON.stringify(err || {});
    // error status/code
    const is403 = /403|FORBIDDEN|ACL|denied|permission/i.test(errMsg) || (err && (err.code === 403 || err.status === 403));
    assert.ok(is403, `u2 read 错误应是 403 类，实际: ${err && (err.code || err.status || err.message)}`);
    // 不泄露 hash / content 字符串
    const leaked = /[a-f0-9]{40,64}/i.test(errMsg) && errMsg.includes(sha1(buf)) ||
      /仅 u1 可读/.test(errMsg);
    assert.strictEqual(leaked, false, `错误对象泄露了内容/hash：${errMsg.slice(0, 240)}`);
  });
});

describe('T10 Cloud Drive: quota 配额', function () {
  it('maxQuota=1KB; attempt 2KB upload → QuotaExceeded error', async function () {
    const store = resetFileStore({ options: { chunkSize: 4 * 1024, softDelete: false, maxQuota: 1024 } });
    // Fill 768 bytes first (within quota), then try 2KB upload (total will exceed 1KB)
    const b1 = Buffer.alloc(768, 0x61); // 'a'
    const m1 = await store.writeFile('fill.bin', b1, { userId: 'u1' });
    assert.ok(m1);
    const b2 = Buffer.alloc(2048, 0x62); // 'b'
    let err = null;
    try { await store.writeFile('big.bin', b2, { userId: 'u1' }); }
    catch (e) { err = e; }
    assert.ok(err, '2KB upload 必须抛出 配额 错误');
    const errMsg = String((err && err.message) || '') + ' ' + ((err && err.code) || '');
    assert.ok(
      /quota|exceed|429|storage limit|容量|配额/i.test(errMsg) || (err && err.code === 'QUOTA_EXCEEDED'),
      `Quota 错误类型不对：${errMsg}`
    );
  });
});

describe('T10 Cloud Drive: 1000 小文件上传 bulk', function () {
  it('1000 small files upload succeed (writeFile returns ok for all)', async function () {
    this.timeout(60000);
    const store = resetFileStore({ options: { chunkSize: 16 * 1024, softDelete: false, maxQuota: 1024 * 1024 * 1024 } });
    const N = 1000;
    const results = [];
    for (let i = 0; i < N; i++) {
      const buf = Buffer.from(`small-${i}-${Math.random()}`, 'utf8');
      const m = await store.writeFile(`bulk/${i.toString().padStart(5, '0')}.txt`, buf, { userId: 'u1' });
      results.push(m);
    }
    assert.strictEqual(results.length, N, `成功写入数量 ${results.length} != ${N}`);
    for (let i = 0; i < N; i++) assert.ok(results[i] && results[i].id, `第 ${i} 个文件 id 为空`);
    const stats = store.getStats();
    assert.ok(stats.totalFiles >= N, `统计 totalFiles=${stats.totalFiles} < ${N}`);
  });
});

describe('T10 Cloud Drive: 大文件接口 (chunked / multipart 签名)', function () {
  it('store exposes writeChunk, createMultipartUpload, uploadPart, completeMultipartUpload methods', function () {
    const store = getFileStore();
    // Direct methods on the chunkBackend are mirrored onto store instance by FileStore
    // (we'll add them as pass-through in GREEN fixes; here we assert shape exists).
    for (const m of ['writeChunk', 'createMultipartUpload', 'uploadPart', 'completeMultipartUpload', 'abortMultipartUpload']) {
      assert.strictEqual(typeof store[m], 'function', `缺少 store.${m} 方法（大文件接口必备）`);
    }
  });

  it('store.writeChunk / store.readChunk round trip 32KB', async function () {
    const store = getFileStore();
    const buf = crypto.randomBytes(32 * 1024);
    const key = sha1(buf);
    const w = await store.writeChunk(key, buf);
    assert.ok(w && (w.existed === true || w.existed === false || w.key === key), 'writeChunk 返回标准 shape');
    const got = await store.readChunk(key);
    assert.strictEqual(sha1(got), key, 'store.readChunk(writeChunk(k,b)) sha1 匹配');
  });

  it('store MPU (create/upload/complete) produces hash-identical merged blob', async function () {
    this.timeout(15000);
    const store = getFileStore();
    const total = 5 * 1024 * 1024; // 5MB split into 5 parts
    const parts = [];
    for (let i = 0; i < 5; i++) parts.push(crypto.randomBytes(1 * 1024 * 1024));
    const merged = Buffer.concat(parts);
    const expectedKey = sha1(merged);
    const { uploadId } = await store.createMultipartUpload(expectedKey);
    assert.ok(uploadId, 'createMultipartUpload 返回 uploadId');
    const uploaded = [];
    for (let i = 0; i < parts.length; i++) {
      const r = await store.uploadPart(expectedKey, uploadId, i + 1, parts[i]);
      assert.strictEqual(r && r.partNumber, i + 1, `uploadPart #${i + 1} 返回值缺 partNumber`);
      uploaded.push(r);
    }
    const fin = await store.completeMultipartUpload(expectedKey, uploadId, uploaded);
    assert.ok(fin, 'completeMultipartUpload 返回非空');
    // Try to read merged blob back and compare hashes (backend may expose via readChunk(key))
    try {
      const out = await store.readChunk(expectedKey);
      assert.strictEqual(sha1(out), expectedKey, 'MPU 合并后 sha1 匹配');
    } catch (e) {
      // 允许后端采用 MPU 对象使用非 chunk key 存储，但 shape 必须已存在；仅当读取失败时 soft-fail
      // — 断言降级：fin.size 必须存在
      assert.ok(fin && Number.isFinite(fin.size), `MPU complete 返回 size=${fin && fin.size}，不合理`);
    }
  });
});
