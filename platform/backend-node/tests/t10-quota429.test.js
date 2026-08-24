/* eslint-env mocha, node */
'use strict';
/**
 * T10 M4 A-4 Quota 429 中间件测试（14 cases，与 spec.md AC-T10-13~22 对齐）
 */
const assert = require('assert');
const http = require('http');
const {
  SlidingWindowQuota,
  createQuotaMiddleware,
  tierLimits,
  extract,
  TIER_LIMITS,
} = require('../src/middleware/quota-429');

function mkReq(headers = {}, remote = '127.0.0.1') {
  return {
    headers: Object.assign({}, headers),
    socket: { remoteAddress: remote },
  };
}
function mkRes() {
  const r = {
    _hdrs: {},
    _status: 200,
    _body: null,
    _ended: false,
    setHeader(k, v) { r._hdrs[k.toLowerCase()] = String(v); },
    getHeader(k) { return r._hdrs[(k || '').toLowerCase()]; },
    writeHead(code) { r._status = code; },
    end(body) { r._ended = true; r._body = body; },
  };
  return r;
}

describe('T10/A4 Quota 429 Middleware (14)', function () {

  it('Q1 TIER_LIMITS free=100 basic=1000 pro=10000', function () {
    assert.strictEqual(TIER_LIMITS.free, 100);
    assert.strictEqual(TIER_LIMITS.basic, 1000);
    assert.strictEqual(TIER_LIMITS.pro, 10_000);
  });

  it('Q2 tierLimits returns 3 dim dict with ascending ip/user/bucket per tier', function () {
    const free = tierLimits('free');
    assert(free.ip <= free.user && free.user <= free.bucket, 'free dim scales');
    const basic = tierLimits('basic');
    assert(basic.ip <= basic.user && basic.user <= basic.bucket, 'basic dim scales');
  });

  it('Q3 extract picks ip from socket.remoteAddress by default', function () {
    const r = mkReq({}, '10.0.0.5');
    const e = extract(r);
    assert.strictEqual(e.ip, '10.0.0.5');
    assert.strictEqual(e.user, null);
    assert.strictEqual(e.bucket, null);
  });

  it('Q4 extract honors X-Forwarded-For[0] over socket', function () {
    const r = mkReq({ 'x-forwarded-for': '203.0.113.9, 10.0.0.1' }, '127.0.0.1');
    assert.strictEqual(extract(r).ip, '203.0.113.9');
  });

  it('Q5 custom tier with customLimits works', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 5, user: 0, bucket: 0 },
    });
    assert.deepStrictEqual(mw._limits, { ip: 5, user: 0, bucket: 0 });
  });

  it('Q6 basic tier accepts N=basic.ip requests within window (return 200)', function () {
    const mw = createQuotaMiddleware({
      tier: 'free', // 100 ip
      windowMs: 60_000,
    });
    for (let i = 0; i < TIER_LIMITS.free; i++) {
      const req = mkReq({});
      const res = mkRes();
      let called = false;
      mw(req, res, () => { called = true; });
      assert(called, 'next should be called within limit');
      assert.strictEqual(res._status, 200);
    }
  });

  it('Q7 exceeding limit returns 429 with correct JSON body', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 3, user: 0, bucket: 0 },
      windowMs: 60_000,
    });
    for (let i = 0; i < 3; i++) mw(mkReq({}), mkRes(), () => {});
    const res = mkRes();
    mw(mkReq({}), res, () => assert.fail('next should not be called on 429'));
    assert.strictEqual(res._status, 429);
    const body = JSON.parse(res._body);
    assert.strictEqual(body.code, 'QUOTA_EXCEEDED');
    assert.strictEqual(body.status, 429);
    assert.strictEqual(body.dimension, 'ip');
    assert.strictEqual(body.limit, 3);
    assert.strictEqual(body.count, 3);
    assert(Number.isFinite(body.retry_after_sec) && body.retry_after_sec > 0);
  });

  it('Q8 429 responses include all 4 quota headers', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom', customLimits: { ip: 2, user: 0, bucket: 0 },
    });
    mw(mkReq({}), mkRes(), () => {});
    mw(mkReq({}), mkRes(), () => {});
    const res = mkRes();
    mw(mkReq({}), res, () => {});
    assert(res.getHeader('X-Quota-Limit'), 'X-Quota-Limit present');
    assert(res.getHeader('X-Quota-Remaining') !== undefined, 'X-Quota-Remaining present');
    assert(res.getHeader('X-Quota-Reset'), 'X-Quota-Reset present');
    assert(res.getHeader('Retry-After'), 'Retry-After present');
    assert.strictEqual(res.getHeader('X-Quota-Dimension'), 'ip');
  });

  it('Q9 sliding window does not double count across window boundary', function () {
    // 构造一个短窗口的 quota，手动填充历史时间戳（跨 2 个时间段）
    const W = 100; // 100ms 短窗
    const sw = new SlidingWindowQuota({ windowMs: W });
    // 先让边界跨过：我们直接 consume 2 次
    sw.consume('k', 2);
    sw.consume('k', 2);
    // 2/2 consumed, next will fail
    const r1 = sw.consume('k', 2);
    assert.strictEqual(r1.ok, false, 'exact cap blocks');
    // advance time past window
    const waitMs = W + 10;
    const start = Date.now();
    // 忙等（短时间）
    while (Date.now() - start < waitMs) { /* spin */ }
    const r2 = sw.consume('k', 2);
    assert.strictEqual(r2.ok, true, 'after window pass, freed');
  });

  it('Q10 User ID dimension limits independently of IP', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 0, user: 2, bucket: 0 },
    });
    mw(mkReq({ 'x-user-id': 'u1' }), mkRes(), (e) => assert.ifError(e));
    mw(mkReq({ 'x-user-id': 'u1' }), mkRes(), (e) => assert.ifError(e));
    const res = mkRes();
    mw(mkReq({ 'x-user-id': 'u1' }), res, () => assert.fail('u1 3rd should 429'));
    assert.strictEqual(res._status, 429);
    assert.strictEqual(res.getHeader('X-Quota-Dimension'), 'user');
    // 不同 user 不限
    let passed = false;
    mw(mkReq({ 'x-user-id': 'u2' }), mkRes(), () => { passed = true; });
    assert(passed, 'u2 fresh passes');
  });

  it('Q11 Bucket dimension enforces independently', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 0, user: 0, bucket: 1 },
    });
    mw(mkReq({ 'x-bucket': 'b1' }), mkRes(), () => {});
    const res = mkRes();
    mw(mkReq({ 'x-bucket': 'b1' }), res, () => {});
    assert.strictEqual(res._status, 429);
    assert.strictEqual(res.getHeader('X-Quota-Dimension'), 'bucket');
  });

  it('Q12 report mode never returns 429 (only writes headers)', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 1, user: 0, bucket: 0 },
      mode: 'report',
    });
    mw(mkReq({}), mkRes(), () => {}); // 1/1
    let calledNext = false;
    const res = mkRes();
    mw(mkReq({}), res, () => { calledNext = true; }); // 2/1 in report => must still call next
    assert(calledNext, 'report mode calls next even over limit');
    assert.strictEqual(res._status, 200, 'no 429 in report');
    assert(res.getHeader('X-Quota-Remaining') !== undefined);
  });

  it('Q13 remaining header decreases monotonically until 0 at 429 boundary', function () {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 5, user: 0, bucket: 0 },
    });
    const seq = [];
    for (let i = 0; i < 5; i++) {
      const res = mkRes();
      mw(mkReq({}), res, () => {});
      seq.push(Number(res.getHeader('X-Quota-Remaining')));
    }
    // remaining = limit - count; after 5 successive consume last remaining=0 before next request
    for (let i = 1; i < seq.length; i++) {
      assert(seq[i - 1] >= seq[i], `monotonic non-increase: ${seq[i - 1]} >= ${seq[i]}`);
    }
    assert.strictEqual(seq[seq.length - 1], 0, 'last in-window remaining should hit 0 after 5 consumes if consumed at cap; logic: count=5 => remaining=0');
  });

  it('Q14 real http-server integration with createServer handler chain', function (done) {
    const mw = createQuotaMiddleware({
      tier: 'custom',
      customLimits: { ip: 2, user: 0, bucket: 0 },
    });
    const srv = http.createServer((req, res) => {
      mw(req, res, () => {
        res.statusCode = 200;
        res.end('ok');
      });
    });
    srv.listen(0, () => {
      const port = srv.address().port;
      const url = `http://127.0.0.1:${port}/`;
      const codes = [];
      const doReq = (i, cb) => {
        http.get(url, (r) => {
          codes.push(r.statusCode);
          // drain body
          r.resume();
          r.on('end', cb);
        });
      };
      doReq(0, () => doReq(1, () => doReq(2, () => {
        srv.close();
        assert.deepStrictEqual(codes, [200, 200, 429]);
        done();
      })));
    });
  });
});
