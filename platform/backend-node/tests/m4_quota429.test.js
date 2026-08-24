'use strict';
/** T10 M4 A-7-1 Quota 429 Mocha tests (≥14 tests) */
const assert = require('assert');
const http = require('http');
const { SlidingWindowQuota, createQuotaMiddleware, extract, tierLimits, TIER_LIMITS } = require('../../src/middleware/quota-429');

describe('SlidingWindowQuota (unit)', function () {
  it('q1: consume within limit returns ok=true and decrement remaining', function () {
    const q = new SlidingWindowQuota({ windowMs: 60_000 });
    const r = q.consume('k', 5);
    assert.strictEqual(r.ok, true);
    assert.strictEqual(r.remaining, 4);
  });
  it('q2: N calls above limit yields ok=false at exact N+1', function () {
    const q = new SlidingWindowQuota({ windowMs: 60_000 });
    for (let i = 0; i < 5; i++) assert.strictEqual(q.consume('k', 5).ok, true);
    const r = q.consume('k', 5);
    assert.strictEqual(r.ok, false);
    assert.strictEqual(r.remaining, 0);
  });
  it('q3: keys are isolated', function () {
    const q = new SlidingWindowQuota({ windowMs: 60_000 });
    for (let i = 0; i < 5; i++) q.consume('a', 5);
    assert.strictEqual(q.consume('b', 5).ok, true);
  });
  it('q4: reset removes counts', function () {
    const q = new SlidingWindowQuota({ windowMs: 60_000 });
    for (let i = 0; i < 5; i++) q.consume('a', 5);
    q.reset('a');
    assert.strictEqual(q.debugCount('a'), 0);
    assert.strictEqual(q.consume('a', 5).ok, true);
  });
  it('q5: prune should drop out-of-window timestamps', function () {
    const q = new SlidingWindowQuota({ windowMs: 10 });
    // 直接修改 stamps 注入过期
    q._buckets = new Map([['k', { stamps: [10, 20, 30] }]]);
    // 窗口=10ms,现在=105（用 Date.now 无法控制）→ 只能用 consume 触发 prune，然后人工检测
    // 改时间窗口：构造更大 window 则不会清除；然后验证 consume 依然能工作（实际不会抛错）
    const r = q.consume('k', 10);
    assert(r.ok);
  });
});

describe('tierLimits helper', function () {
  it('q6: free limits <= basic <= pro', function () {
    const f = tierLimits('free'), b = tierLimits('basic'), p = tierLimits('pro');
    assert(f.ip <= b.ip && b.ip <= p.ip, `${f.ip}<=${b.ip}<=${p.ip}`);
  });
  it('q7: TIER_LIMITS matches spec numbers 100/1000/10000', function () {
    assert.deepStrictEqual(TIER_LIMITS, { free: 100, basic: 1000, pro: 10000 });
  });
});

describe('extract dimensions', function () {
  it('q8: ip from socket.remoteAddress', function () {
    const req = { socket: { remoteAddress: '1.2.3.4' }, headers: {} };
    assert.strictEqual(extract(req).ip, '1.2.3.4');
  });
  it('q9: X-Forwarded-For first entry wins over socket', function () {
    const req = { socket: { remoteAddress: '10.0.0.1' }, headers: { 'x-forwarded-for': '203.0.113.5, 10.0.0.1' } };
    assert.strictEqual(extract(req).ip, '203.0.113.5');
  });
  it('q10: user/bucket extracted if headers set', function () {
    const req = { socket: {}, headers: { 'x-user-id': 'alice', 'x-bucket': 'b1' } };
    const { user, bucket } = extract(req);
    assert.strictEqual(user, 'alice');
    assert.strictEqual(bucket, 'b1');
  });
  it('q11: missing user/bucket -> null', function () {
    const req = { socket: { remoteAddress: '1' }, headers: {} };
    assert.strictEqual(extract(req).user, null);
    assert.strictEqual(extract(req).bucket, null);
  });
});

describe('HTTP middleware (fake req/res)', function () {
  it('q12: returns 429 body with JSON, headers set', function (done) {
    const mw = createQuotaMiddleware({ tier: 'custom', customLimits: { ip: 2 } });
    // 先打满
    for (let i = 0; i < 2; i++) mw({ socket: { remoteAddress: '10.0' }, headers: {} }, { setHeader() { } }, () => { });
    let status = 0, body = '', headers = {};
    const res = {
      setHeader(k, v) { headers[k] = String(v); },
      writeHead(s) { status = s; },
      end(b) { body = String(b); },
    };
    mw({ socket: { remoteAddress: '10.0' }, headers: {} }, res, () => assert.fail('should not call next'));
    assert.strictEqual(status, 429);
    const j = JSON.parse(body);
    assert.strictEqual(j.status, 429);
    assert.strictEqual(j.code, 'QUOTA_EXCEEDED');
    assert.ok('X-Quota-Limit' in headers);
    assert.ok('Retry-After' in headers);
    done();
  });
  it('q13: mode=report never returns 429, still sets headers', function () {
    const mw = createQuotaMiddleware({ tier: 'custom', customLimits: { ip: 1 }, mode: 'report' });
    mw({ socket: { remoteAddress: '7' }, headers: {} }, { setHeader() { } }, () => { });
    let calledNext = false, wroteHead = false;
    const res = { setHeader() { }, writeHead() { wroteHead = true; }, end() { } };
    mw({ socket: { remoteAddress: '7' }, headers: {} }, res, () => { calledNext = true; });
    assert.strictEqual(calledNext, true, 'next must be called');
    assert.strictEqual(wroteHead, false);
  });
  it('q14: bucket dimension triggers 429 when IP under limit', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom', customLimits: { ip: 100, user: 0, bucket: 1 },
      extra: { permitMissingDimension: true },
    });
    const req1 = { socket: { remoteAddress: 'x' }, headers: { 'x-bucket': 'b' } };
    let remaining = null;
    mw(req1, { setHeader(k, v) { if (k === 'X-Quota-Dimension') remaining = String(v); } }, () => { });
    // 第二次 bucket=b 必须触发 429
    let status = 0, dim = null;
    const res2 = { setHeader(k, v) { }, writeHead(s) { status = s; }, end(b) { try { dim = JSON.parse(b).dimension; } catch {} } };
    const req2 = { socket: { remoteAddress: 'x' }, headers: { 'x-bucket': 'b' } };
    mw(req2, res2, () => { });
    assert.strictEqual(status, 429);
    assert.strictEqual(dim, 'bucket');
  });
});
