'use strict';
/** T10 M4 A-7-2 WORM + S3 ObjectLock tests (≥14 tests) */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { buildWormSql, installWormTriggers, smokeTestWorm, applyObjectLock, isLocked, toS3RetentionXml, DEFAULT_LOG_TABLE } = require('../../src/worm/worm-and-object-lock');

let Database;
try {
  Database = require('better-sqlite3');
} catch (_) {
  Database = null;
}

describe('buildWormSql (static generation)', function () {
  it('w1: output contains CREATE TABLE + 2 triggers minimum', function () {
    const s = buildWormSql();
    assert.ok(s.includes('CREATE TABLE IF NOT EXISTS dengbao_logs'));
    assert.ok(s.includes('no_update'));
    assert.ok(s.includes('no_delete_locked'));
  });
  it('w2: retentionMinDays emits t_retention_at_least trigger', function () {
    assert.ok(buildWormSql({ retentionMinDays: 30 }).includes('t_dengbao_logs_retention_at_least'));
    assert.ok(!buildWormSql().includes('t_dengbao_logs_retention_at_least'));
  });
  it('w3: custom table name reflected', function () {
    assert.ok(buildWormSql({ tableName: 'audit_t' }).includes('CREATE TABLE IF NOT EXISTS audit_t'));
  });
});

describe('SQLite WORM install + smoke (better-sqlite3)', function () {
  before(function () { if (!Database) this.skip(); });

  let db, tmpfile;
  beforeEach(function () {
    tmpfile = path.join(os.tmpdir(), `worm_t_${Date.now()}_${Math.random().toString(36).slice(2)}.db`);
    db = new Database(tmpfile);
    db.pragma('journal_mode = WAL');
  });
  afterEach(function () {
    if (db) db.close();
    if (tmpfile && fs.existsSync(tmpfile)) fs.unlinkSync(tmpfile);
    for (const ext of ['-wal', '-shm']) {
      const p = tmpfile + ext;
      if (fs.existsSync(p)) fs.unlinkSync(p);
    }
  });

  it('w4: installWormTriggers reports installed', function () {
    const r = installWormTriggers(db);
    assert.strictEqual(r.installed, true);
    assert.ok(r.triggers.length >= 2);
    assert.strictEqual(r.table, DEFAULT_LOG_TABLE);
  });
  it('w5: INSERT succeeds; UPDATE throws WORM_VIOLATION', function () {
    installWormTriggers(db);
    const r = smokeTestWorm(db);
    assert.strictEqual(r.updateThrows, true);
  });
  it('w6: DELETE while retention_until_ms in future throws', function () {
    installWormTriggers(db);
    const r = smokeTestWorm(db);
    assert.strictEqual(r.deleteThrows, true);
  });
  it('w7: DELETE row with legal_hold=1 throws', function () {
    installWormTriggers(db);
    const r = smokeTestWorm(db);
    assert.strictEqual(r.lhDeleteThrows, true);
  });
  it('w8: retentionMinDays = 365 rejects inserts with shorter retention', function () {
    installWormTriggers(db, { retentionMinDays: 365 });
    const ts = Date.now();
    // 保留期仅 1 天 → 应被 trigger 拦截
    const short = ts + 24 * 60 * 60 * 1000;
    let threw = false;
    try {
      db.prepare(
        `INSERT INTO ${DEFAULT_LOG_TABLE}(idx,ts_ms,actor,action,resource,outcome,payload_hash,prev_hash,block_hash,hmac_signature,retention_until_ms,legal_hold) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)`,
      ).run(1, ts, 'a', 'x', 'r', 'OK', 'p', 'pr', 'bh', 'hm', short, 0);
    } catch (e) {
      if (String(e.message).includes('WORM_RETENTION_MIN')) threw = true;
    }
    assert.strictEqual(threw, true);
  });
  it('w9: legal_hold column enforces 0/1 CHECK', function () {
    installWormTriggers(db);
    let threw = false;
    try {
      db.prepare(
        `INSERT INTO ${DEFAULT_LOG_TABLE}(idx,ts_ms,actor,action,resource,outcome,payload_hash,prev_hash,block_hash,hmac_signature,retention_until_ms,legal_hold) VALUES (?,?,?,?,?,?,?,?,?,?,?,2)`,
      ).run(1, Date.now(), 'a', 'x', 'r', 'OK', 'p', 'pr', 'bh', 'hm', Date.now() + 1e6);
    } catch (e) {
      threw = true;
    }
    assert.strictEqual(threw, true, 'legal_hold must be 0/1');
  });
});

describe('S3 Object Lock', function () {
  it('w10: applyObjectLock returns new object with mode/retainUntil/legalHold', function () {
    const future = Date.now() + 7 * 24 * 3600 * 1000;
    const r = applyObjectLock({ key: 'a' }, { mode: 'GOVERNANCE', retainUntilMs: future, legalHold: true });
    assert.strictEqual(r.object_lock_mode, 'GOVERNANCE');
    assert.strictEqual(r.object_lock_legal_hold, true);
    assert.strictEqual(r.key, 'a', 'original fields preserved');
  });
  it('w11: missing config throws', function () {
    assert.throws(() => applyObjectLock({}));
  });
  it('w12: invalid mode throws', function () {
    assert.throws(() => applyObjectLock({}, { mode: 'OTHER', retainUntilMs: Date.now() + 1e6 }));
  });
  it('w13: past retainUntilMs throws', function () {
    assert.throws(() => applyObjectLock({}, { mode: 'COMPLIANCE', retainUntilMs: Date.now() - 1e6 }));
  });
  it('w14: isLocked reports legal_hold locked', function () {
    const { locked, reason } = isLocked({ object_lock_legal_hold: true });
    assert.strictEqual(locked, true);
    assert.strictEqual(reason, 'LEGAL_HOLD');
  });
  it('w15 [bonus]: toS3RetentionXml contains Mode/RetainUntilDate', function () {
    const s = toS3RetentionXml({ mode: 'COMPLIANCE', retainUntilMs: Date.now() + 1e6 });
    assert.ok(s.includes('<Mode>COMPLIANCE</Mode>'));
    assert.ok(s.includes('<RetainUntilDate>'));
  });
});
