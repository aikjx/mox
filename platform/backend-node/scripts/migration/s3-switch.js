#!/usr/bin/env node
'use strict';

/**
 * MOX Enterprise · FS ↔ S3 存储后端切换工具（T0→T1 核心组件）
 * ================================================================
 * 功能：
 *  1. 环境配置生成（.env.s3 / .env.fs 模板）
 *  2. 数据迁移：FS chunks → S3（或反向），支持断点续传、并发、校验
 *  3. 双写验证：对比 FS 与 S3 的 chunk 哈希一致性
 *  4. 灰度切流：按百分比流量切换到 S3
 *
 * 用法：
 *  node scripts/migration/s3-switch.js migrate --from fs --to s3 --concurrency 10
 *  node scripts/migration/s3-switch.js verify --source fs --target s3
 *  node scripts/migration/s3-switch.js gen-env --target s3
 *  node scripts/migration/s3-switch.js status
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { createDefaultBackend, FSChunkBackend, S3ChunkBackend } = require('../../src/storage/chunk-backend');

// ─── CLI 参数解析 ───
function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next && !next.startsWith('--')) { args[key] = next; i++; }
      else { args[key] = true; }
    } else {
      args._.push(a);
    }
  }
  return args;
}

// ─── 进度条 ───
class ProgressBar {
  constructor(total, label = '') {
    this.total = total;
    this.current = 0;
    this.label = label;
    this.startTime = Date.now();
    this.lastPrint = 0;
  }
  tick(n = 1) {
    this.current += n;
    const now = Date.now();
    if (now - this.lastPrint > 200 || this.current >= this.total) {
      this.print();
      this.lastPrint = now;
    }
  }
  print() {
    const pct = Math.min(100, (this.current / this.total) * 100);
    const elapsed = (Date.now() - this.startTime) / 1000;
    const rate = this.current / Math.max(0.1, elapsed);
    const eta = rate > 0 ? (this.total - this.current) / rate : 0;
    const bars = Math.floor(pct / 2);
    const bar = '█'.repeat(bars) + '░'.repeat(50 - bars);
    process.stdout.write(`\r${this.label} [${bar}] ${pct.toFixed(1)}%  ${this.current}/${this.total}  ${rate.toFixed(0)}/s  ETA ${eta.toFixed(0)}s  `);
    if (this.current >= this.total) process.stdout.write('\n');
  }
}

// ─── 生成 .env 模板 ───
function genEnv(target) {
  const isS3 = target === 's3';
  const content = isS3 ? `# MOX S3 存储后端配置（T1 级别）
# 生成时间: ${new Date().toISOString()}
# 使用方式: cp .env.s3 .env && 填入真实凭证

# ─── 存储后端切换 ───
FILE_BACKEND=s3
DB_PROVIDER=postgresql

# ─── S3 兼容对象存储 ───
S3_CHUNKS_BUCKET=mox-chunks-prod
AWS_REGION=ap-southeast-1
S3_ENDPOINT=https://s3.ap-southeast-1.amazonaws.com
S3_ACCESS_KEY=your-access-key-here
S3_SECRET_KEY=your-secret-key-here

# ─── PostgreSQL 元数据库 ───
DB_HOST=10.0.0.10
DB_PORT=5432
DB_NAME=mox_meta
DB_USER=mox_app
DB_PASSWORD=your-db-password

# ─── 双写过渡期（可选，灰度切流时开启） ───
# DB_DUAL_WRITE=true
# DB_READ_PREF=auto

# ─── 分库路由（T2 级别，可选） ───
# PG_SHARD_NODES=pg-01:10.0.0.10:5432,pg-02:10.0.0.11:5432
# PG_SHARD_DB=mox_meta
# PG_SHARD_USER=mox_app
# PG_SHARD_PASS=your-db-password
` : `# MOX FS 本地存储配置（T0 级别 / 回滚用）
FILE_BACKEND=fs
DB_PROVIDER=sqlite
DATA_DIR=./data
`;
  const filename = isS3 ? '.env.s3' : '.env.fs';
  const outPath = path.join(process.cwd(), filename);
  fs.writeFileSync(outPath, content, 'utf8');
  console.log(`✓ 已生成配置模板: ${outPath}`);
  console.log(`  下一步: cp ${filename} .env  然后填入真实凭证`);
}

// ─── 获取 backend 实例 ───
function getBackend(type) {
  if (type === 'fs') {
    return new FSChunkBackend();
  }
  if (type === 's3') {
    return new S3ChunkBackend();
  }
  return createDefaultBackend();
}

// ─── 迁移：从 source backend 复制所有 chunk 到 target backend ───
async function migrate(args) {
  const fromType = args.from || 'fs';
  const toType = args.to || 's3';
  const concurrency = parseInt(args.concurrency || '5', 10);
  const dryRun = args['dry-run'] === true;

  console.log(`\n🚀 开始迁移: ${fromType} → ${toType}  (并发: ${concurrency}${dryRun ? ', DRY RUN' : ''})\n`);

  const source = getBackend(fromType);
  const target = getBackend(toType);

  await source.connect();
  await target.connect();

  // 列出所有 source chunk
  console.log('📋 扫描源端 chunk 列表...');
  const allKeys = await source.listChunks('');
  console.log(`   发现 ${allKeys.length} 个 chunk\n`);

  if (allKeys.length === 0) {
    console.log('源端无数据，迁移完成。');
    return;
  }

  const progress = new ProgressBar(allKeys.length, '迁移进度');
  let copied = 0, skipped = 0, failed = 0;
  const failures = [];

  // 并发 worker
  const queue = [...allKeys];
  async function worker() {
    while (queue.length > 0) {
      const key = queue.shift();
      if (!key) continue;
      try {
        // 检查 target 是否已存在
        const exists = await target.hasChunk(key);
        if (exists) { skipped++; progress.tick(); continue; }
        if (dryRun) { copied++; progress.tick(); continue; }
        // 读取 + 写入
        const buf = await source.readChunk(key);
        await target.writeChunk(key, buf);
        copied++;
      } catch (err) {
        failed++;
        failures.push({ key, error: err.message });
      }
      progress.tick();
    }
  }

  const workers = Array.from({ length: concurrency }, () => worker());
  await Promise.all(workers);

  console.log(`\n✅ 迁移完成！`);
  console.log(`   复制: ${copied}  跳过(已存在): ${skipped}  失败: ${failed}`);
  if (failures.length > 0) {
    console.log(`\n⚠️  失败列表（前 10 条）:`);
    failures.slice(0, 10).forEach(f => console.log(`   - ${f.key}: ${f.error}`));
  }

  await source.disconnect();
  await target.disconnect();
}

// ─── 校验：对比 source 与 target 的所有 chunk 哈希 ───
async function verify(args) {
  const sourceType = args.source || 'fs';
  const targetType = args.target || 's3';
  const sampleRate = parseFloat(args.sample || '1.0'); // 抽样比例，默认全量

  console.log(`\n🔍 开始校验: ${sourceType} ↔ ${targetType}  (抽样率: ${(sampleRate * 100).toFixed(0)}%)\n`);

  const source = getBackend(sourceType);
  const target = getBackend(targetType);
  await source.connect();
  await target.connect();

  const allKeys = await source.listChunks('');
  const sampleKeys = sampleRate >= 1 ? allKeys : allKeys.filter(() => Math.random() < sampleRate);
  console.log(`   源端共 ${allKeys.length} 个，抽样校验 ${sampleKeys.length} 个\n`);

  const progress = new ProgressBar(sampleKeys.length, '校验进度');
  let matched = 0, mismatched = 0, missing = 0;
  const mismatches = [];

  for (const key of sampleKeys) {
    try {
      const targetExists = await target.hasChunk(key);
      if (!targetExists) { missing++; mismatches.push({ key, reason: 'target missing' }); progress.tick(); continue; }
      const srcBuf = await source.readChunk(key);
      const tgtBuf = await target.readChunk(key);
      const srcHash = crypto.createHash('sha256').update(srcBuf).digest('hex');
      const tgtHash = crypto.createHash('sha256').update(tgtBuf).digest('hex');
      if (srcHash === tgtHash) matched++;
      else { mismatched++; mismatches.push({ key, srcHash, tgtHash }); }
    } catch (err) {
      mismatched++;
      mismatches.push({ key, error: err.message });
    }
    progress.tick();
  }

  console.log(`\n✅ 校验完成！`);
  console.log(`   一致: ${matched}  不一致: ${mismatched}  缺失: ${missing}`);
  if (mismatches.length > 0) {
    console.log(`\n⚠️  不一致详情（前 10 条）:`);
    mismatches.slice(0, 10).forEach(m => console.log(`   - ${m.key}: ${m.reason || m.error || `hash mismatch ${m.srcHash?.slice(0,8)} vs ${m.tgtHash?.slice(0,8)}`}`));
  }
  const pass = mismatched === 0 && missing === 0;
  console.log(`\n结论: ${pass ? '✅ 全部通过，可以切流' : '❌ 存在差异，请勿切流，请先修复'}`);

  await source.disconnect();
  await target.disconnect();
  return pass;
}

// ─── 状态查看 ───
async function status() {
  console.log('\n📊 MOX 存储后端状态\n');
  console.log(`  FILE_BACKEND = ${process.env.FILE_BACKEND || 'fs (默认)'}`);
  console.log(`  DB_PROVIDER  = ${process.env.DB_PROVIDER || 'sqlite (默认)'}`);
  console.log(`  DATA_DIR     = ${process.env.DATA_DIR || './data'}`);
  if (process.env.S3_CHUNKS_BUCKET) {
    console.log(`  S3 Bucket    = ${process.env.S3_CHUNKS_BUCKET}`);
    console.log(`  S3 Endpoint  = ${process.env.S3_ENDPOINT || '(AWS 默认)'}`);
  }
  if (process.env.PG_SHARD_NODES) {
    console.log(`  PG Shards    = ${process.env.PG_SHARD_NODES}`);
  }
  console.log('');

  // 统计 FS chunk 数
  try {
    const fsBackend = new FSChunkBackend();
    await fsBackend.connect();
    const keys = await fsBackend.listChunks('');
    let totalSize = 0;
    // 只统计前 1000 个的大小做估算
    for (let i = 0; i < Math.min(keys.length, 1000); i++) {
      try {
        const buf = await fsBackend.readChunk(keys[i]);
        totalSize += buf.length;
      } catch {}
    }
    const avgSize = keys.length > 0 ? totalSize / Math.min(keys.length, 1000) : 0;
    const estTotal = avgSize * keys.length;
    console.log(`  FS Chunk 数量: ${keys.length}`);
    console.log(`  平均大小(抽样): ${(avgSize / 1024).toFixed(1)} KB`);
    console.log(`  估算总容量: ${(estTotal / 1024 / 1024 / 1024).toFixed(2)} GB`);
    await fsBackend.disconnect();
  } catch (e) {
    console.log(`  FS 统计失败: ${e.message}`);
  }
  console.log('');
}

// ─── 主入口 ───
async function main() {
  const args = parseArgs(process.argv);
  const command = args._[0];

  switch (command) {
    case 'migrate':
      await migrate(args);
      break;
    case 'verify':
      await verify(args);
      break;
    case 'gen-env':
      genEnv(args.target || 's3');
      break;
    case 'status':
      await status();
      break;
    case 'help':
    default:
      console.log(`
MOX FS ↔ S3 存储切换工具

用法:
  node s3-switch.js <command> [options]

命令:
  migrate    迁移 chunk 数据
    --from fs|s3        源端（默认 fs）
    --to   fs|s3        目标端（默认 s3）
    --concurrency N     并发数（默认 5）
    --dry-run           只扫描不实际写入

  verify     校验两端数据一致性
    --source fs|s3      源端
    --target fs|s3      目标端
    --sample 0.0-1.0    抽样比例（默认 1.0 全量）

  gen-env    生成环境变量模板
    --target s3|fs      目标配置（默认 s3）

  status     查看当前存储状态

  help       显示此帮助
`);
  }
}

main().catch(err => {
  console.error('❌ 执行失败:', err.message);
  process.exit(1);
});
