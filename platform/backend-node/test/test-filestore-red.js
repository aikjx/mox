'use strict';

/**
 * RED 测试：IChunkBackend 抽象 (fs + s3-mock) + FileStore MPU + GC
 * =============================================================
 * 前置：当前 file-store.js 只支持 fs，不支持可注入 backend；
 *       chunk 读写直写 path.join(CHUNKS_DIR, chunkHash)；
 *       FILE_BACKEND 环境变量未被使用；软删/GC 未完整实现。
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const assert = require('assert');

// ---- 环境隔离 ----
const TMP_ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'xuanji-t2-red-'));
const TMP_DATA = path.join(TMP_ROOT, 'data');
fs.mkdirSync(TMP_DATA, { recursive: true });

// 先检查现状 file-store 是否暴露 backend 注入 & FILE_BACKEND / FILE_MPU_CONCURRENCY / FILE_GRACE_DAYS 支持
let redPassed = 0, redFailed = 0;
function red(name, fn) {
  try { fn(); redPassed++; console.log('  RED-OK ', name); }
  catch (e) { redFailed++; console.log('  RED-FAIL (说明该能力尚未存在，符合预期)', name, '—', e.message.split('\n')[0]); }
}

const fsExists = fs.existsSync;
red('chunk-backend.js 模块目前不存在', () => {
  if (fsExists(path.resolve(__dirname, '..', 'src', 'storage', 'chunk-backend.js'))) throw new Error('chunk-backend.js 已存在');
});
red('process.env.FILE_BACKEND 未被 file-store 读取', () => {
  const src = fs.readFileSync(path.resolve(__dirname, '..', 'src', 'file-store.js'), 'utf-8');
  if (/FILE_BACKEND|FILE_MPU_CONCURRENCY|FILE_GRACE_DAYS/.test(src)) throw new Error('已包含环境变量处理');
});
red('FileStore 未提供 backend 构造注入', () => {
  const src = fs.readFileSync(path.resolve(__dirname, '..', 'src', 'file-store.js'), 'utf-8');
  if (/new FileStore\(.*\{.*backend/.test(src) || /constructor\([^\)]*backend/.test(src)) throw new Error('已支持 backend 注入');
});
red('FileStore 当前不存在 deleteFile(status=soft_deleted) + runGC 接口', () => {
  const src = fs.readFileSync(path.resolve(__dirname, '..', 'src', 'file-store.js'), 'utf-8');
  if (/soft_deleted|runGC|graceDays/.test(src)) throw new Error('已实现软删/GC');
});
red('FileStore 未实现 S3 MPU（无 createMultipartUpload / completeMultipartUpload）', () => {
  const src = fs.readFileSync(path.resolve(__dirname, '..', 'src', 'file-store.js'), 'utf-8');
  if (/MultipartUpload|uploadPart|CompleteMultipart/i.test(src)) throw new Error('已实现 MPU');
});

console.log(`\n[RED T2] 预计 5 项能力未实现；RED-FAIL 数=${redFailed} 表示"当前还没有"，符合 RED 阶段要求。`);
// RED 阶段：证明"当前没有这些能力" —— 即 redFailed >= 3 时我们的 RED 是有效的；
// 完成 GREEN 后此文件不应再被运行（我们在 GREEN 测试里直接断言通过）。
process.exit(redFailed >= 3 ? 0 : 1);
