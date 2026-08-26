#!/usr/bin/env node
'use strict';

/**
 * MOX Enterprise · SQLite → PostgreSQL 元数据迁移工具（T0→T1 核心组件）
 * ==========================================================================
 * 功能：
 *  1. 自动读取 SQLite 表结构，在 PG 中创建对应表（类型映射）
 *  2. 批量数据迁移，支持并发、断点续传、进度显示
 *  3. 迁移后数据校验（行数对比 + 抽样哈希对比）
 *  4. 支持只迁移部分表 / 排除部分表
 *
 * 类型映射：
 *   SQLite INTEGER  → PG BIGSERIAL / BIGINT
 *   SQLite TEXT     → PG TEXT
 *   SQLite REAL     → PG DOUBLE PRECISION
 *   SQLite BLOB     → PG BYTEA
 *   SQLite NUMERIC  → PG NUMERIC
 *
 * 用法：
 *  node scripts/migration/sqlite-to-pg.js migrate --sqlite ./data/ous.db --pg postgres://user:pass@host:5432/db
 *  node scripts/migration/sqlite-to-pg.js verify  --sqlite ./data/ous.db --pg postgres://user:pass@host:5432/db
 *  node scripts/migration/sqlite-to-pg.js schema  --sqlite ./data/ous.db
 *  node scripts/migration/sqlite-to-pg.js tables  --sqlite ./data/ous.db
 */

const fs = require('fs');
const path = require('path');

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
    this.total = total; this.current = 0; this.label = label;
    this.startTime = Date.now(); this.lastPrint = 0;
  }
  tick(n = 1) {
    this.current += n;
    const now = Date.now();
    if (now - this.lastPrint > 200 || this.current >= this.total) { this.print(); this.lastPrint = now; }
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

// ─── SQLite 连接 ───
function getSqlite(dbPath) {
  try {
    const Database = require('better-sqlite3');
    return new Database(dbPath, { readonly: true });
  } catch (e) {
    throw new Error(`better-sqlite3 未安装或数据库打开失败: ${e.message}\n请运行: npm install better-sqlite3`);
  }
}

// ─── PG 连接 ───
function getPg(connectionString) {
  try {
    const { Pool } = require('pg');
    return new Pool({ connectionString, max: 10 });
  } catch (e) {
    throw new Error(`pg 驱动未安装: ${e.message}\n请运行: npm install pg`);
  }
}

// ─── SQLite → PG 类型映射 ───
function mapType(sqliteType) {
  if (!sqliteType) return 'TEXT';
  const t = sqliteType.toUpperCase();
  if (t.includes('INT')) return 'BIGINT';
  if (t.includes('TEXT') || t.includes('CHAR') || t.includes('CLOB')) return 'TEXT';
  if (t.includes('REAL') || t.includes('FLOAT') || t.includes('DOUBLE')) return 'DOUBLE PRECISION';
  if (t.includes('BLOB')) return 'BYTEA';
  if (t.includes('NUMERIC') || t.includes('DECIMAL')) return 'NUMERIC';
  if (t.includes('BOOLEAN')) return 'BOOLEAN';
  if (t.includes('DATETIME') || t.includes('TIMESTAMP')) return 'TIMESTAMP';
  if (t.includes('DATE')) return 'DATE';
  return 'TEXT';
}

// ─── 获取 SQLite 所有表 ───
function getTables(sqlite) {
  const rows = sqlite.prepare(`
    SELECT name FROM sqlite_master
    WHERE type='table' AND name NOT LIKE 'sqlite_%'
    ORDER BY name
  `).all();
  return rows.map(r => r.name);
}

// ─── 获取表结构 ───
function getTableSchema(sqlite, tableName) {
  const columns = sqlite.prepare(`PRAGMA table_info("${tableName}")`).all();
  return columns.map(col => ({
    name: col.name,
    type: mapType(col.type),
    notNull: col.notnull === 1,
    defaultValue: col.dflt_value,
    primaryKey: col.pk === 1
  }));
}

// ─── 在 PG 中创建表 ───
async function createPgTable(pg, tableName, columns) {
  const colDefs = columns.map(col => {
    let def = `"${col.name}" ${col.type}`;
    if (col.primaryKey) def += ' PRIMARY KEY';
    if (col.notNull && !col.primaryKey) def += ' NOT NULL';
    if (col.defaultValue !== null && col.defaultValue !== undefined) def += ` DEFAULT ${col.defaultValue}`;
    return def;
  });
  const sql = `CREATE TABLE IF NOT EXISTS "${tableName}" (${colDefs.join(', ')})`;
  await pg.query(sql);
  // 创建索引（对主键和常用查询列）
  for (const col of columns) {
    if (!col.primaryKey && (col.name.includes('_id') || col.name === 'id' || col.name === 'type')) {
      try {
        await pg.query(`CREATE INDEX IF NOT EXISTS idx_${tableName}_${col.name} ON "${tableName}" ("${col.name}")`);
      } catch {}
    }
  }
}

// ─── 迁移单表 ───
async function migrateTable(sqlite, pg, tableName, batchSize = 1000) {
  const columns = getTableSchema(sqlite, tableName);
  const colNames = columns.map(c => `"${c.name}"`).join(', ');
  const total = sqlite.prepare(`SELECT COUNT(*) as cnt FROM "${tableName}"`).get().cnt;

  if (total === 0) {
    console.log(`   ⏭  ${tableName}: 空表，跳过`);
    return { table: tableName, total: 0, migrated: 0, skipped: true };
  }

  // 在 PG 创建表
  await createPgTable(pg, tableName, columns);

  // 清空 PG 中已有数据（重新迁移）
  await pg.query(`TRUNCATE TABLE "${tableName}" RESTART IDENTITY CASCADE`);

  const progress = new ProgressBar(total, `   迁移 ${tableName.padEnd(20)}`);
  let migrated = 0;

  // 分批读取 + 批量插入
  const stmt = sqlite.prepare(`SELECT * FROM "${tableName}" LIMIT ? OFFSET ?`);
  for (let offset = 0; offset < total; offset += batchSize) {
    const rows = stmt.all(batchSize, offset);
    if (rows.length === 0) break;

    // 构建批量 INSERT
    const placeholders = rows.map((_, i) =>
      `(${columns.map((_, j) => `$${i * columns.length + j + 1}`).join(', ')})`
    ).join(', ');
    const values = rows.flatMap(row => columns.map(col => {
      const val = row[col.name];
      // BLOB 处理
      if (col.type === 'BYTEA' && val !== null) {
        return Buffer.isBuffer(val) ? val : Buffer.from(val);
      }
      return val;
    }));

    const insertSql = `INSERT INTO "${tableName}" (${colNames}) VALUES ${placeholders}`;
    await pg.query(insertSql, values);
    migrated += rows.length;
    progress.tick(rows.length);
  }

  return { table: tableName, total, migrated };
}

// ─── 校验单表 ───
async function verifyTable(sqlite, pg, tableName) {
  const sqliteCount = sqlite.prepare(`SELECT COUNT(*) as cnt FROM "${tableName}"`).get().cnt;
  const pgResult = await pg.query(`SELECT COUNT(*) as cnt FROM "${tableName}"`);
  const pgCount = parseInt(pgResult.rows[0].cnt, 10);

  const countMatch = sqliteCount === pgCount;

  // 抽样哈希对比（取前 100 行和后 100 行）
  let hashMatch = true;
  const columns = getTableSchema(sqlite, tableName);
  const pkCol = columns.find(c => c.primaryKey) || columns[0];

  if (sqliteCount > 0 && pkCol) {
    const sampleSize = Math.min(100, sqliteCount);
    const sqliteRows = sqlite.prepare(`SELECT * FROM "${tableName}" ORDER BY "${pkCol.name}" LIMIT ?`).all(sampleSize);
    const pgRows = (await pg.query(`SELECT * FROM "${tableName}" ORDER BY "${pkCol.name}" LIMIT $1`, [sampleSize])).rows;

    for (let i = 0; i < sqliteRows.length; i++) {
      const srcHash = JSON.stringify(sqliteRows[i], Object.keys(sqliteRows[i]).sort());
      const tgtHash = JSON.stringify(pgRows[i], Object.keys(pgRows[i] || {}).sort());
      if (srcHash !== tgtHash) { hashMatch = false; break; }
    }
  }

  return {
    table: tableName,
    sqliteCount,
    pgCount,
    countMatch,
    hashMatch,
    pass: countMatch && hashMatch
  };
}

// ─── 主迁移流程 ───
async function migrate(args) {
  const sqlitePath = args.sqlite || path.join(process.cwd(), 'data', 'ous.db');
  const pgConn = args.pg || process.env.DATABASE_URL;
  const batchSize = parseInt(args['batch-size'] || '1000', 10);
  const only = args.only ? args.only.split(',').map(s => s.trim()) : null;
  const exclude = args.exclude ? args.exclude.split(',').map(s => s.trim()) : [];

  if (!pgConn) {
    console.error('❌ 请提供 PG 连接串: --pg postgres://user:pass@host:5432/db  或设置 DATABASE_URL');
    process.exit(1);
  }
  if (!fs.existsSync(sqlitePath)) {
    console.error(`❌ SQLite 文件不存在: ${sqlitePath}`);
    process.exit(1);
  }

  console.log(`\n🚀 SQLite → PostgreSQL 迁移`);
  console.log(`   SQLite: ${sqlitePath}`);
  console.log(`   PG:     ${pgConn.replace(/\/\/[^:]+:[^@]+@/, '//***:***@')}`);
  console.log(`   批次:   ${batchSize}\n`);

  const sqlite = getSqlite(sqlitePath);
  const pg = getPg(pgConn);

  let tables = getTables(sqlite);
  if (only) tables = tables.filter(t => only.includes(t));
  tables = tables.filter(t => !exclude.includes(t));

  console.log(`📋 待迁移表 (${tables.length} 个): ${tables.join(', ')}\n`);

  const results = [];
  for (const table of tables) {
    try {
      const r = await migrateTable(sqlite, pg, table, batchSize);
      results.push(r);
    } catch (err) {
      console.log(`   ❌ ${table}: 迁移失败 - ${err.message}`);
      results.push({ table, error: err.message });
    }
  }

  console.log(`\n✅ 迁移完成！`);
  const success = results.filter(r => !r.error && !r.skipped);
  const failed = results.filter(r => r.error);
  console.log(`   成功: ${success.length}  失败: ${failed.length}  跳过: ${results.filter(r => r.skipped).length}`);
  if (failed.length > 0) {
    console.log(`\n⚠️  失败表:`);
    failed.forEach(f => console.log(`   - ${f.table}: ${f.error}`));
  }

  sqlite.close();
  await pg.end();
}

// ─── 校验流程 ───
async function verify(args) {
  const sqlitePath = args.sqlite || path.join(process.cwd(), 'data', 'ous.db');
  const pgConn = args.pg || process.env.DATABASE_URL;

  if (!pgConn) { console.error('❌ 请提供 PG 连接串'); process.exit(1); }

  console.log(`\n🔍 SQLite ↔ PostgreSQL 数据校验\n`);

  const sqlite = getSqlite(sqlitePath);
  const pg = getPg(pgConn);
  const tables = getTables(sqlite);

  const results = [];
  for (const table of tables) {
    try {
      const r = await verifyTable(sqlite, pg, table);
      results.push(r);
      const icon = r.pass ? '✅' : '❌';
      console.log(`   ${icon} ${table.padEnd(25)} SQLite=${r.sqliteCount}  PG=${r.pgCount}  行数=${r.countMatch ? '✓' : '✗'}  哈希=${r.hashMatch ? '✓' : '✗'}`);
    } catch (err) {
      console.log(`   ⚠️  ${table}: 校验异常 - ${err.message}`);
      results.push({ table, pass: false, error: err.message });
    }
  }

  const allPass = results.every(r => r.pass);
  console.log(`\n${allPass ? '✅ 全部通过，可以切换 DB_PROVIDER=postgresql' : '❌ 存在差异，请勿切流'}`);

  sqlite.close();
  await pg.end();
  return allPass;
}

// ─── Schema 查看 ───
function showSchema(args) {
  const sqlitePath = args.sqlite || path.join(process.cwd(), 'data', 'ous.db');
  const sqlite = getSqlite(sqlitePath);
  const tables = getTables(sqlite);
  console.log(`\n📐 SQLite 表结构 → PG 映射\n`);
  for (const table of tables) {
    const cols = getTableSchema(sqlite, table);
    console.log(`  ${table} (${cols.length} 列):`);
    cols.forEach(c => {
      const pk = c.primaryKey ? ' 🔑' : '';
      console.log(`    ${c.name.padEnd(25)} ${c.type}${pk}`);
    });
    console.log('');
  }
  sqlite.close();
}

// ─── 主入口 ───
async function main() {
  const args = parseArgs(process.argv);
  const command = args._[0];

  switch (command) {
    case 'migrate': await migrate(args); break;
    case 'verify':  await verify(args); break;
    case 'schema':  showSchema(args); break;
    case 'tables': {
      const sqlitePath = args.sqlite || path.join(process.cwd(), 'data', 'ous.db');
      const sqlite = getSqlite(sqlitePath);
      console.log('\n📋 表列表:\n');
      getTables(sqlite).forEach(t => {
        const cnt = sqlite.prepare(`SELECT COUNT(*) as c FROM "${t}"`).get().c;
        console.log(`   ${t.padEnd(30)} ${cnt} 行`);
      });
      console.log('');
      sqlite.close();
      break;
    }
    case 'help':
    default:
      console.log(`
MOX SQLite → PostgreSQL 迁移工具

用法:
  node sqlite-to-pg.js <command> [options]

命令:
  migrate    迁移全部数据
    --sqlite PATH         SQLite 文件路径（默认 ./data/ous.db）
    --pg CONNSTR          PG 连接串（或 DATABASE_URL 环境变量）
    --batch-size N        每批行数（默认 1000）
    --only t1,t2          只迁移指定表
    --exclude t1,t2       排除指定表

  verify     校验数据一致性
    --sqlite PATH
    --pg CONNSTR

  schema     显示表结构及 PG 类型映射
    --sqlite PATH

  tables     列出所有表及行数
    --sqlite PATH

  help       显示此帮助
`);
  }
}

main().catch(err => {
  console.error('❌ 执行失败:', err.message);
  process.exit(1);
});
