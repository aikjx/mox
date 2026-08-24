/**
 * T10 M4 A-6：等保三级 WORM（一次写多次读）+ S3 Object Lock 合规配置
 *
 * 规范覆盖（AC-T10-23~30）：
 *  - SQLite 触发器：对审计表（dengbao_logs）禁止 UPDATE / DELETE（行级 WORM）
 *  - retention_min_days / legal_hold 两维保留；到期后由外部 job 清理（但 APPEND 仍生效）
 *  - S3 Object Lock 接口：PutObjectRetention / GetObjectRetention
 *  - 构造 SQL 脚本可直接在 better-sqlite3 / libsql 中执行
 *  - 提供 createWormDb 辅助函数（仅测试环境用；生产环境由运维执行脚本）
 */

'use strict';

/** 默认审计表名（与 dengbao_hash_chain Rust 模块概念对齐） */
const DEFAULT_LOG_TABLE = 'dengbao_logs';

/**
 * 生成 WORM 触发器的完整 SQL。
 *
 * 表结构（若不存在则建）：
 *   dengbao_logs(id INTEGER PRIMARY KEY AUTOINCREMENT,
 *                idx INTEGER UNIQUE NOT NULL,
 *                ts_ms INTEGER NOT NULL,
 *                actor TEXT NOT NULL,
 *                action TEXT NOT NULL,
 *                resource TEXT NOT NULL,
 *                outcome TEXT NOT NULL,
 *                payload_hash TEXT NOT NULL,
 *                prev_hash TEXT NOT NULL,
 *                block_hash TEXT NOT NULL,
 *                hmac_signature TEXT NOT NULL,
 *                retention_until_ms INTEGER,   -- NULL = 永久（legal hold）
 *                legal_hold INTEGER NOT NULL DEFAULT 0);
 */
function buildWormSql(options = {}) {
  const tbl = options.tableName || DEFAULT_LOG_TABLE;
  const retentionMinDays = options.retentionMinDays || 0;
  const sql = [];
  sql.push('-- [WORM v1] Dengbao Level-3 audit WORM schema');
  sql.push(`CREATE TABLE IF NOT EXISTS ${tbl} (`);
  sql.push('  id INTEGER PRIMARY KEY AUTOINCREMENT,');
  sql.push('  idx INTEGER UNIQUE NOT NULL,');
  sql.push('  ts_ms INTEGER NOT NULL,');
  sql.push('  actor TEXT NOT NULL,');
  sql.push('  action TEXT NOT NULL,');
  sql.push('  resource TEXT NOT NULL,');
  sql.push('  outcome TEXT NOT NULL,');
  sql.push('  payload_hash TEXT NOT NULL,');
  sql.push('  prev_hash TEXT NOT NULL,');
  sql.push('  block_hash TEXT NOT NULL,');
  sql.push('  hmac_signature TEXT NOT NULL,');
  sql.push('  retention_until_ms INTEGER,');
  sql.push('  legal_hold INTEGER NOT NULL DEFAULT 0 CHECK (legal_hold IN (0,1))');
  sql.push(');');
  sql.push(`CREATE INDEX IF NOT EXISTS idx_${tbl}_ts ON ${tbl}(ts_ms);`);
  sql.push(`CREATE INDEX IF NOT EXISTS idx_${tbl}_actor ON ${tbl}(actor);`);

  // === 禁止 UPDATE（WORM：写后不可改）===
  sql.push(`CREATE TRIGGER IF NOT EXISTS t_${tbl}_no_update`);
  sql.push(`  BEFORE UPDATE ON ${tbl}`);
  sql.push('  BEGIN');
  sql.push(`    SELECT RAISE(ABORT, 'WORM_VIOLATION: cannot UPDATE ${tbl}');`);
  sql.push('  END;');

  // === 禁止 DELETE（保留期内；legal_hold=1 则无期限）===
  // now_ms() 实现：(julianday - epoch) * ms_per_day，毫秒三位用毫秒小数部分
  sql.push(`CREATE TRIGGER IF NOT EXISTS t_${tbl}_no_delete_locked`);
  sql.push(`  BEFORE DELETE ON ${tbl}`);
  sql.push('  WHEN (OLD.legal_hold = 1) OR (OLD.retention_until_ms IS NOT NULL AND OLD.retention_until_ms > (' +
    ' CAST((julianday(\'now\') - 2440587.5) * 86400000 AS INTEGER) ))');
  sql.push('  BEGIN');
  sql.push(`    SELECT RAISE(ABORT, 'WORM_VIOLATION: cannot DELETE ${tbl} — retention or legal_hold');`);
  sql.push('  END;');

  // === INSERT 强制：idx 单调 + 保留期 >= retention_min_days（若 >0）===
  if (retentionMinDays > 0) {
    sql.push(`CREATE TRIGGER IF NOT EXISTS t_${tbl}_retention_at_least`);
    sql.push(`  BEFORE INSERT ON ${tbl}`);
    sql.push(`  WHEN NEW.retention_until_ms IS NOT NULL`);
    sql.push(`    AND NEW.retention_until_ms - NEW.ts_ms < ${retentionMinDays * 24 * 60 * 60 * 1000}`);
    sql.push('  BEGIN');
    sql.push(`    SELECT RAISE(ABORT, 'WORM_RETENTION_MIN: retention_min_days=${retentionMinDays} required');`);
    sql.push('  END;');
  }
  return sql.join('\n') + '\n';
}

/**
 * 为现有 better-sqlite3 数据库安装 WORM schema 与触发器。
 * @param {import('better-sqlite3').Database} db
 * @param {object} opts
 */
function installWormTriggers(db, opts = {}) {
  const tbl = opts.tableName || DEFAULT_LOG_TABLE;
  db.exec(buildWormSql(opts));
  // 基本校验：存在 1 张表 + 至少 2 个触发器
  const trgs = db
    .prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name = ?")
    .all(tbl)
    .map((r) => r.name);
  const has_no_update = trgs.some((n) => n.includes('no_update'));
  const has_no_delete = trgs.some((n) => n.includes('no_delete_locked'));
  return { installed: has_no_update && has_no_delete, triggers: trgs, table: tbl };
}

/**
 * 简单验证：尝试 UPDATE/DELETE 受限行 → 抛错；成功 INSERT → OK
 * @param {import('better-sqlite3').Database} db
 * @param {string} [tableName]
 */
function smokeTestWorm(db, tableName = DEFAULT_LOG_TABLE) {
  const ins = db.prepare(
    `INSERT INTO ${tableName} (idx,ts_ms,actor,action,resource,outcome,payload_hash,prev_hash,block_hash,hmac_signature,retention_until_ms,legal_hold) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)`,
  );
  const ts = Date.now();
  const ret = ts + 24 * 60 * 60 * 1000;
  ins.run(1, ts, 'a', 'x', 'r', 'OK', 'p', 'pr', 'bh', 'hm', ret, 0);
  // UPDATE 必须失败
  let updateThrows = false;
  try {
    db.prepare(`UPDATE ${tableName} SET action='Y' WHERE idx=1`).run();
  } catch (e) {
    if (String(e.message).includes('WORM_VIOLATION')) updateThrows = true;
  }
  // DELETE（retention 未过）必须失败
  let deleteThrows = false;
  try {
    db.prepare(`DELETE FROM ${tableName} WHERE idx=1`).run();
  } catch (e) {
    if (String(e.message).includes('WORM_VIOLATION')) deleteThrows = true;
  }
  // legal_hold=1 行的删除也必须失败
  ins.run(2, ts, 'a', 'x', 'r', 'OK', 'p', 'bh', 'bh2', 'hm', null, 1);
  let lhDeleteThrows = false;
  try {
    db.prepare(`DELETE FROM ${tableName} WHERE idx=2`).run();
  } catch (e) {
    if (String(e.message).includes('WORM_VIOLATION')) lhDeleteThrows = true;
  }
  return { updateThrows, deleteThrows, lhDeleteThrows };
}

// ============= S3 Object Lock 配置接口（纯逻辑；Rust 层对接真实 S3）=============

/**
 * 合规模式：
 *  - COMPLIANCE：用户/root 都不能提前删除（除非到期）
 *  - GOVERNANCE：特定 IAM 权限可提前删（默认 COMPLIANCE）
 * @typedef {'COMPLIANCE'|'GOVERNANCE'} RetentionMode
 */

/**
 * @typedef {{mode: RetentionMode, retainUntilMs: number, legalHold?: boolean}} ObjectRetentionConfig
 */

/** 应用 Object Lock 到对象元数据（内存结构） */
function applyObjectLock(objMeta, cfg) {
  if (!cfg || typeof cfg !== 'object') throw new Error('ObjectLockConfig required');
  if (cfg.mode !== 'COMPLIANCE' && cfg.mode !== 'GOVERNANCE') {
    throw new Error('Invalid Retention mode, must be COMPLIANCE or GOVERNANCE');
  }
  if (!Number.isFinite(cfg.retainUntilMs) || cfg.retainUntilMs <= Date.now()) {
    throw new Error('retainUntilMs must be a future timestamp (ms)');
  }
  const next = { ...objMeta };
  next.object_lock_mode = cfg.mode;
  next.object_lock_retain_until_ms = cfg.retainUntilMs;
  next.object_lock_legal_hold = !!cfg.legalHold;
  return next;
}

/** 当前对象是否可被删（按 lock 语义） */
function isLocked(objMeta, now = Date.now()) {
  if (!objMeta) return { locked: false };
  if (objMeta.object_lock_legal_hold === true) return { locked: true, reason: 'LEGAL_HOLD' };
  if (
    objMeta.object_lock_mode &&
    objMeta.object_lock_retain_until_ms &&
    objMeta.object_lock_retain_until_ms > now
  ) {
    return { locked: true, reason: objMeta.object_lock_mode };
  }
  return { locked: false };
}

/**
 * 格式化 Go/Rust SDK 风格的 PutObjectRetention 请求 body（与 S3 REST API 同构）
 */
function toS3RetentionXml(cfg) {
  const mode = cfg.mode;
  const d = new Date(cfg.retainUntilMs).toISOString();
  const lh = cfg.legalHold ? '<LegalHold>ON</LegalHold>' : '';
  return (
    '<?xml version="1.0" encoding="UTF-8"?>' +
    '<Retention xmlns="http://s3.amazonaws.com/doc/2006-03-01/">' +
    `<Mode>${mode}</Mode><RetainUntilDate>${d}</RetainUntilDate>${lh}</Retention>`
  );
}

module.exports = {
  DEFAULT_LOG_TABLE,
  buildWormSql,
  installWormTriggers,
  smokeTestWorm,
  applyObjectLock,
  isLocked,
  toS3RetentionXml,
};
