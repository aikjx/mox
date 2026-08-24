/* eslint-env mocha, node */
'use strict';
/**
 * T10 M4 A-6 WORM SQLite 触发器 + S3 Object Lock 测试（12 cases）
 */
const assert = require('assert');
const os = require('os');
const path = require('path');
const fs = require('fs');

let Database;
try {
  Database = require('better-sqlite3');
} catch (e) { /* 若无 better-sqlite3，仅测 S3 ObjectLock 纯逻辑部分 */ }

const {
  DEFAULT_LOG_TABLE,
  buildWormSql,
  installWormTriggers,
  smokeTestWorm,
  applyObjectLock,
  isLocked,
  toS3RetentionXml,
} = require('../src/worm/worm-and-object-lock');

function tempDb() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 't10-worm-'));
  const f = path.join(tmp, 'audit.db');
  return { tmp, f };
}

describe('T10/A6 WORM + S3 ObjectLock (12)', function () {

  describe('buildWormSql (static)', function () {
    it('W1 generates CREATE TABLE + 2 triggers + indexes', function () {
      const s = buildWormSql();
      assert(/CREATE TABLE/.test(s), 'contains create table');
      assert(/CREATE TRIGGER[\s\S]*no_update/.test(s), 'no_update trigger');
      assert(/CREATE TRIGGER[\s\S]*no_delete_locked/.test(s), 'no_delete_locked trigger');
      assert(/CREATE INDEX/.test(s), 'indexes');
      assert(s.includes('WORM_VIOLATION'));
    });

    it('W2 with retentionMinDays=30 adds retention_at_least trigger', function () {
      const s = buildWormSql({ retentionMinDays: 30 });
      assert(/retention_min_days=30/.test(s) || /retention_at_least/.test(s));
      assert(/t_.*_retention_at_least/.test(s));
    });

    it('W3 custom tableName applied in SQL', function () {
      const s = buildWormSql({ tableName: 'mylogs' });
      assert(s.includes('mylogs'));
      assert(!s.includes('dengbao_logs') || s.includes(DEFAULT_LOG_TABLE));
    });
  });

  describe('installWormTriggers (better-sqlite3)', function () {
    before(function () {
      if (!Database) this.skip();
    });

    it('W4 installs triggers with basic+ flags', function () {
      const { f } = tempDb();
      const db = new Database(f);
      const r = installWormTriggers(db, { retentionMinDays: 7 });
      assert.strictEqual(r.installed, true);
      assert(r.triggers.length >= 3, 'no_update + no_delete + retention >= 3 triggers');
      db.close();
    });

    it('W5 INSERT OK / UPDATE throws WORM_VIOLATION / DELETE throws WORM_VIOLATION / legal_hold blocks delete', function () {
      const { f } = tempDb();
      const db = new Database(f);
      installWormTriggers(db);
      const r = smokeTestWorm(db);
      assert(r.updateThrows, 'update must throw WORM_VIOLATION');
      assert(r.deleteThrows, 'delete during retention must throw WORM_VIOLATION');
      assert(r.lhDeleteThrows, 'legal_hold=1 delete must throw WORM_VIOLATION');
      db.close();
    });

    it('W6 retention expired row deletable (in the future from db perspective via set ts_ms=very old, retention_until_ms=near past)', function () {
      const { f } = tempDb();
      const db = new Database(f);
      installWormTriggers(db, { retentionMinDays: 0 });
      const ins = db.prepare(
        `INSERT INTO ${DEFAULT_LOG_TABLE} (idx,ts_ms,actor,action,resource,outcome,payload_hash,prev_hash,block_hash,hmac_signature,retention_until_ms,legal_hold) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)`,
      );
      const past = Date.now() - 1000;
      const expired = Date.now() - 500; // retention already passed
      ins.run(10, past, 'u', 'a', 'r', 'S', 'p', 'pr', 'bh', 'hm', expired, 0);
      let deleted = false;
      try {
        db.prepare(`DELETE FROM ${DEFAULT_LOG_TABLE} WHERE idx=10`).run();
        deleted = true;
      } catch (e) {
        // not ok
      }
      assert.strictEqual(deleted, true, 'expired rows should be deletable');
      db.close();
    });
  });

  describe('applyObjectLock / isLocked / toS3RetentionXml', function () {
    it('W7 applyObjectLock enriches meta with 3 fields', function () {
      const future = Date.now() + 86_400_000;
      const m = applyObjectLock({ id: 1 }, { mode: 'COMPLIANCE', retainUntilMs: future, legalHold: true });
      assert.strictEqual(m.object_lock_mode, 'COMPLIANCE');
      assert.strictEqual(m.object_lock_retain_until_ms, future);
      assert.strictEqual(m.object_lock_legal_hold, true);
    });

    it('W8 applyObjectLock rejects invalid mode or past date', function () {
      const future = Date.now() + 86_400_000;
      assert.throws(() => applyObjectLock({}, { mode: 'BAD', retainUntilMs: future }), /Invalid Retention mode/);
      assert.throws(() => applyObjectLock({}, { mode: 'COMPLIANCE', retainUntilMs: Date.now() - 1000 }), /future timestamp/);
      assert.throws(() => applyObjectLock({}, null), /required/);
    });

    it('W9 isLocked legal_hold blocks regardless of retention', function () {
      const r1 = isLocked({ object_lock_legal_hold: true, object_lock_mode: 'COMPLIANCE', object_lock_retain_until_ms: Date.now() - 1 });
      assert.strictEqual(r1.locked, true);
      assert.strictEqual(r1.reason, 'LEGAL_HOLD');
    });

    it('W10 isLocked compliance blocks until retainUntilMs passes, then free', function () {
      const future = Date.now() + 10_000;
      const meta = { object_lock_mode: 'COMPLIANCE', object_lock_retain_until_ms: future };
      assert.strictEqual(isLocked(meta).locked, true);
      assert.strictEqual(isLocked({ ...meta, object_lock_retain_until_ms: Date.now() - 1 }).locked, false);
    });

    it('W11 toS3RetentionXml produces Mode + RetainUntilDate + optional LegalHold', function () {
      const future = new Date('2030-01-15T10:00:00Z').getTime();
      const xml1 = toS3RetentionXml({ mode: 'GOVERNANCE', retainUntilMs: future });
      assert(xml1.includes('<Mode>GOVERNANCE</Mode>'));
      assert(xml1.includes('<RetainUntilDate>2030-01-15T10:00:00.000Z</RetainUntilDate>'));
      assert(!xml1.includes('<LegalHold>ON</LegalHold>'));

      const xml2 = toS3RetentionXml({ mode: 'COMPLIANCE', retainUntilMs: future, legalHold: true });
      assert(xml2.includes('<Mode>COMPLIANCE</Mode>'));
      assert(xml2.includes('<LegalHold>ON</LegalHold>'));
    });

    it('W12 empty meta / null safely returns unlocked (not throw)', function () {
      assert.deepStrictEqual(isLocked(null), { locked: false });
      assert.deepStrictEqual(isLocked({}), { locked: false });
    });
  });
});
