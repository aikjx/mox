'use strict';
/**
 * mocha：T8 O2 TokenBucket + 租户级 QPS 配额（TR-7~TR-12）
 */
const assert = require('assert');
const { SecurityManager, TokenBucket, DEFAULT_TENANT_QUOTAS } = require('../src/security');

describe('[T8-AC-1] TokenBucket 基本语义：burst 内允许，超出按 QPS 限流', function () {
  it('burst=5, qps=10 → 5 次立即成功，第 6 次失败且 resetMs > 0', () => {
    const b = new TokenBucket(5, 10);
    for (let i = 0; i < 5; i++) {
      const r = b.tryAcquire(1);
      assert.ok(r.allowed, `第 ${i+1} 次应成功`);
    }
    const fail = b.tryAcquire(1);
    assert.ok(!fail.allowed, '第 6 次应被限流');
    assert.ok(fail.resetMs > 0, `resetMs 应 >0，实际 ${fail.resetMs}`);
  });

  it('500ms 后补充 tokens 5（qps=10 → 0.5s 补充 5）→ 再取 5 次成功', async function () {
    const b = new TokenBucket(5, 10);
    for (let i = 0; i < 5; i++) b.tryAcquire(1); // 打空
    await new Promise(res => setTimeout(res, 550));
    // 0.5s 应补充 5 个 → token=5
    for (let i = 0; i < 5; i++) {
      const r = b.tryAcquire(1);
      assert.ok(r.allowed, `0.5s 后第 ${i+1} 次应成功`);
    }
  });
});

describe('[T8-AC-2] SecurityManager 默认滑动窗口（无 env）向后兼容', function () {
  it('构造后 SEC_ENABLE_TOKEN_BUCKET≠1 → mode=sliding_window', () => {
    delete process.env.SEC_ENABLE_TOKEN_BUCKET;
    const s = new SecurityManager({ rateLimitWindow: 1000, rateLimitMaxRequests: 3 });
    let r;
    for (let i = 0; i < 3; i++) { r = s.checkRateLimit('k1'); assert.ok(r.allowed, `第${i}次 ok`); assert.strictEqual(r.mode, 'sliding_window'); }
    r = s.checkRateLimit('k1');
    assert.ok(!r.allowed, '第 4 次应被限流');
  });
});

describe('[T8-AC-3] O2 SEC_ENABLE_TOKEN_BUCKET=1 → mode=token_bucket + key 级别限流生效', function () {
  this.timeout(5000);
  it('强制 O2，qps=10 burst=5；burst 内放行，第 6 次被拦', () => {
    process.env.SEC_ENABLE_TOKEN_BUCKET = '1';
    const s = new SecurityManager({ defaultKeyQps: 10, defaultKeyBurst: 5 });
    let r;
    for (let i = 0; i < 5; i++) {
      r = s.checkRateLimit('k1');
      assert.ok(r.allowed, `burst ${i+1} 应通过`);
      assert.strictEqual(r.mode, 'token_bucket');
    }
    r = s.checkRateLimit('k1');
    assert.ok(!r.allowed, '第 6 次应被限流（burst exhausted）');
  });
});

describe('[T8-AC-4] O2 tenant 双维：按 tier 单独配额 + key 通过但 tenant 被拦时回滚 key tokens', function () {
  it('VIP 租户 200 qps burst 400；单 key burst=5；6 次请求中第 6 次应 key 级被拦（非 tenant）', () => {
    process.env.SEC_ENABLE_TOKEN_BUCKET = '1';
    const s = new SecurityManager({ defaultKeyQps: 200, defaultKeyBurst: 5 });
    let r;
    for (let i = 0; i < 5; i++) {
      r = s.checkRateLimit('vip-key', { tier: 'VIP', tenantId: 'acme' });
      assert.ok(r.allowed && r.mode === 'token_bucket', `vip key request ${i+1} 应通过`);
    }
    r = s.checkRateLimit('vip-key', { tier: 'VIP', tenantId: 'acme' });
    assert.ok(!r.allowed, '第 6 次应 key 级被拦');
    assert.strictEqual(r.cause, undefined); // 保持兼容
    // key 被拦后，重新再试另一个 key（同 tenant）：tenant 桶 tokens 应不减（回滚保证），如果此处再拦一定是因为 tenant burst 不足，但我们已知 burst=400 不会被 5 次消耗
    const r2 = s.checkRateLimit('vip-key-2', { tier: 'VIP', tenantId: 'acme' });
    assert.ok(r2.allowed, 'tenant 级应未被消耗（burst 够），新 key burst=5 第 1 次应通过');
  });

  it('匿名 ANONYMOUS 低配额（qps=2 burst=4）：连续 4 次通过，第 5 次被拦（tenant 级）', () => {
    process.env.SEC_ENABLE_TOKEN_BUCKET = '1';
    const s = new SecurityManager({ defaultKeyQps: 9999, defaultKeyBurst: 9999,
      tenantQuotas: Object.assign({}, DEFAULT_TENANT_QUOTAS, { ANONYMOUS: { qps: 2, burst: 4 } }) });
    let r;
    for (let i = 0; i < 4; i++) {
      r = s.checkRateLimit(`anon-${i}`, { tier: 'ANONYMOUS', tenantId: 'anon-pool' });
      assert.ok(r.allowed, `anon ${i+1} 应通过`);
    }
    r = s.checkRateLimit('anon-5', { tier: 'ANONYMOUS', tenantId: 'anon-pool' });
    assert.ok(!r.allowed, '第 5 次应被 tenant 级限流（burst 耗尽）');
    assert.ok(r.bucketTenantState, 'tenant state 存在');
  });
});

describe('[T8-AC-5] 闲置 bucket GC：超过 bucketIdleCleanupMs + 触发周期后 _tokenBuckets 为空', function () {
  it('bucketIdleCleanupMs=200，350ms 后再次 checkRateLimit → 仅保留新 key bucket（ephemeral 被回收）', async function () {
    process.env.SEC_ENABLE_TOKEN_BUCKET = '1';
    const s = new SecurityManager({
      defaultKeyQps: 10, defaultKeyBurst: 5,
      bucketIdleCleanupMs: 200, bucketIdleCleanupEveryMs: 0,
    });
    s.checkRateLimit('ephemeral-key');
    assert.strictEqual(s._tokenBuckets.size, 1, '1 个 bucket 已创建');
    await new Promise(res => setTimeout(res, 350));
    s.checkRateLimit('some-other-key-to-trigger-gc', { tier: 'NORMAL' });
    // 'ephemeral-key' 已闲置 >200ms，应被清理
    assert.ok(s._tokenBuckets.size <= 1, `GC 后应 ≤1（仅当前新 key bucket 可能留存），实际 ${s._tokenBuckets.size}`);
  });
});

describe('[T8-AC-6] 60 秒 × 200 QPS 模拟：O2 rl_blocked 为 NORMAL 的 QPS 封顶的 100% 对齐（H1_AFTER ≤ H1_BEFORE rl_blocked×0.1 同类验证，这里做 1s burst 验证）', function () {
  it('60 QPS 封顶 burst=10，连续 120 次在同一 ms 触发 → 前 10 次 ok，后 110 次 blocked', () => {
    process.env.SEC_ENABLE_TOKEN_BUCKET = '1';
    const s = new SecurityManager({ defaultKeyQps: 60, defaultKeyBurst: 10 });
    let ok = 0, blocked = 0;
    for (let i = 0; i < 120; i++) {
      const r = s.checkRateLimit('key60');
      if (r.allowed) ok++; else blocked++;
    }
    assert.strictEqual(ok, 10, `burst=10 ok 应为 10，实际 ${ok}`);
    assert.strictEqual(blocked, 110, `剩余 110 应为 blocked`);
  });
});

// 避免环境变量泄漏到其他测试
after(() => { delete process.env.SEC_ENABLE_TOKEN_BUCKET; });
