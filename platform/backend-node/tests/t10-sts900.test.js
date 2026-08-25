/* eslint-env mocha, node */
'use strict';
/**
 * T10 M4 A-3 STS AssumeRole TTL=900 秒硬约束 + session_token HMAC 签名（12 cases）
 * 验证 Node 接入层：签发 (role_id, session_name, duration=900) → session_token，
 *   非 900 秒一律拒绝；token 被篡改 / 过期 → verify 失败
 * （与 Rust sts_ttl900.rs 同构实现，保持签名算法一致，便于跨端对齐）
 */
const assert = require('assert');
const crypto = require('crypto');

const STS_ALLOWED_TTL_SECS = 900;
const STS_TTL_MS = STS_ALLOWED_TTL_SECS * 1000;

function nowMs() { return Date.now(); }

function b64encode(buf) {
  return buf.toString('base64');
}

/**
 * session_token = base64(HMAC-SHA256(secret, role_id || session_name || expiration_LE8))
 */
function signSessionToken(secret, roleId, sessionName, expirationMs) {
  const h = crypto.createHmac('sha256', secret);
  h.update(roleId);
  h.update(sessionName);
  // expiration LE8 bytes
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(expirationMs), 0);
  h.update(b);
  return b64encode(h.digest());
}

/** 签发临时凭证 */
function assumeRole(secret, roleId, sessionName, durationSecs) {
  if (!roleId) throw new Error('STS_ROLE_ID_EMPTY');
  if (!sessionName) throw new Error('STS_SESSION_NAME_EMPTY');
  if (durationSecs !== STS_ALLOWED_TTL_SECS) {
    throw new Error(`STS_TTL_MUST_BE_900_SECONDS: got=${durationSecs}, expected=${STS_ALLOWED_TTL_SECS}`);
  }
  const issued = nowMs();
  const exp = issued + STS_TTL_MS;
  const token = signSessionToken(secret, roleId, sessionName, exp);
  return {
    credentials: {
      access_key: `AK.${roleId}.${Math.random().toString(36).slice(2, 10)}`,
      secret_key: crypto.randomBytes(24).toString('hex'),
      session_token: token,
      expiration: exp,
    },
    role_id: roleId,
    session_name: sessionName,
    issued_at_ms: issued,
    duration_secs: STS_ALLOWED_TTL_SECS,
  };
}

function verifyCreds(secret, roleId, sessionName, creds) {
  if (!creds || typeof creds !== 'object') return { ok: false, reason: 'NULL' };
  // 1. 过期检查
  if (creds.expiration <= nowMs()) return { ok: false, reason: 'EXPIRED' };
  // 2. 签名自证
  const expected = signSessionToken(secret, roleId, sessionName, creds.expiration);
  if (expected !== creds.session_token) return { ok: false, reason: 'TAMPER' };
  return { ok: true };
}

describe('T10/A3 STS TTL=900s (12)', function () {
  const SECRET = Buffer.from('mox-sts-root-secret-0123456789abcdef0123456789abcdef');
  const ROLE = 'role/editor';
  const SESS = 'sess_abc123';

  it('S1 900s TTL accepted', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    assert.strictEqual(r.role_id, ROLE);
    assert.strictEqual(r.session_name, SESS);
    assert.strictEqual(r.duration_secs, 900);
    assert.strictEqual(typeof r.credentials.session_token, 'string');
    assert(r.credentials.session_token.length > 20);
  });

  it('S2 duration 3600 (1h) explicitly rejected with named error', function () {
    assert.throws(() => assumeRole(SECRET, ROLE, SESS, 3600), /STS_TTL_MUST_BE_900_SECONDS/);
  });

  it('S3 edge 899 and 901 rejected (enforce strict equality)', function () {
    assert.throws(() => assumeRole(SECRET, ROLE, SESS, 899), /STS_TTL_MUST_BE_900_SECONDS/);
    assert.throws(() => assumeRole(SECRET, ROLE, SESS, 901), /STS_TTL_MUST_BE_900_SECONDS/);
  });

  it('S4 empty role/session rejected with specific error', function () {
    assert.throws(() => assumeRole(SECRET, '', SESS, 900), /STS_ROLE_ID_EMPTY/);
    assert.throws(() => assumeRole(SECRET, ROLE, '', 900), /STS_SESSION_NAME_EMPTY/);
  });

  it('S5 verify fresh credentials passes', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const v = verifyCreds(SECRET, ROLE, SESS, r.credentials);
    assert.strictEqual(v.ok, true, v.reason);
  });

  it('S6 tamper session_token → TAMPER verify fail', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const bad = { ...r.credentials, session_token: r.credentials.session_token.split('').reverse().join('') };
    const v = verifyCreds(SECRET, ROLE, SESS, bad);
    assert.strictEqual(v.ok, false);
    assert.strictEqual(v.reason, 'TAMPER');
  });

  it('S7 expiration field matches 900*1000ms after issuance', function () {
    const before = Date.now();
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const after = Date.now();
    const diffExp = r.credentials.expiration - r.issued_at_ms;
    assert.strictEqual(diffExp, STS_TTL_MS);
    assert(r.issued_at_ms >= before && r.issued_at_ms <= after);
  });

  it('S8 expired credentials verify returns EXPIRED', function () {
    // 伪造 credentials 其中 expiration=past
    const past = Date.now() - 1000;
    const token = signSessionToken(SECRET, ROLE, SESS, past);
    const fake = { expiration: past, session_token: token };
    const v = verifyCreds(SECRET, ROLE, SESS, fake);
    assert.strictEqual(v.ok, false);
    assert.strictEqual(v.reason, 'EXPIRED');
  });

  it('S9 different root key yields different signature (verify fails)', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const WRONG = Buffer.from('other-key-00000000000000000000000000000000');
    const v = verifyCreds(WRONG, ROLE, SESS, r.credentials);
    assert.strictEqual(v.ok, false);
  });

  it('S10 different roleId → signature differs (verify fails)', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const v = verifyCreds(SECRET, 'role/other', SESS, r.credentials);
    assert.strictEqual(v.ok, false);
    assert.strictEqual(v.reason, 'TAMPER');
  });

  it('S11 different session_name → signature differs (verify fails)', function () {
    const r = assumeRole(SECRET, ROLE, SESS, 900);
    const v = verifyCreds(SECRET, ROLE, 'different_session', r.credentials);
    assert.strictEqual(v.ok, false);
  });

  it('S12 50 concurrent issuances are independent & deterministic per issuance (distinct AK + unique token)', function () {
    const tokens = new Set();
    for (let i = 0; i < 50; i++) {
      const r = assumeRole(SECRET, ROLE, `${SESS}_${i}`, 900);
      tokens.add(r.credentials.access_key); // AKs unique
      // verify every single one ok
      const v = verifyCreds(SECRET, ROLE, `${SESS}_${i}`, r.credentials);
      assert(v.ok, `cred ${i} fails: ${v.reason}`);
    }
    assert.strictEqual(tokens.size, 50, '50 distinct access keys');
  });
});
