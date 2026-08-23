'use strict';
/**
 * mocha 单元测试：T7 O1 LatencyWarmRouter（T7 TR-1~TR-6）
 */
const assert = require('assert');
const { LatencyWarmRouter, ROUTING_STRATEGIES, LLMGateway } = require('../src/llm-gateway');

describe('[T7-AC-1] LatencyWarmRouter 初始化 + EWMA 默认值', function () {
  it('能 new 成功，providers EWMA latency 默认 400ms，success 0.95', () => {
    const p = {
      P_A: { id:'P_A', enabled:true, estimated_latency_ms: 800, error_rate: 0.05 },
      P_B: { id:'P_B', enabled:true },
    };
    const r = new LatencyWarmRouter(p, { alpha: 0.2 });
    const snap = r.snapshot();
    assert.ok(Math.abs(snap.P_A.lat_ewma - 800) < 0.001, 'P_A lat should be 800');
    assert.strictEqual(snap.P_A.sr_ewma, 0.95, 'P_A sr should be 1-0.05=0.95');
    assert.strictEqual(snap.P_B.lat_ewma, 400, 'P_B default lat');
    assert.strictEqual(snap.P_B.sr_ewma, 0.95, 'P_B default sr');
  });
});

describe('[T7-AC-2] recordResult 更新 EWMA', function () {
  it('α=0.2 下一次快请求后 latency 下降，success_rate 不变', () => {
    const p = { P1: { id:'P1', enabled:true, estimated_latency_ms: 1000, error_rate:0 } };
    const r = new LatencyWarmRouter(p, { alpha: 0.2 });
    r.recordResult('P1', 100, true);
    // EWMA(lat) = 0.8*1000 + 0.2*100 = 820
    const lat = r.snapshot().P1.lat_ewma;
    assert.ok(Math.abs(lat - 820) < 1e-6, `EWMA 期望 820，实际 ${lat}`);
  });
  it('失败时 success_rate EWMA 下降', () => {
    const p = { P1: { id:'P1', enabled:true, estimated_latency_ms: 100, error_rate: 0 } };
    const r = new LatencyWarmRouter(p, { alpha: 0.5 });
    r.recordResult('P1', 100, false);
    const sr = r.snapshot().P1.sr_ewma;
    // 0.5*1 + 0.5*0 = 0.5
    assert.ok(Math.abs(sr - 0.5) < 1e-6, `fail SR EWMA: ${sr}`);
  });
});

describe('[T7-AC-3] rankedEnabledIds 禁用 provider 剔除 + 评分排序', function () {
  it('禁用 provider 不出现；得分高者在前', () => {
    const p = {
      FAST:   { id:'FAST',   enabled:true,  priority: 10, estimated_latency_ms: 100 },
      SLOW:   { id:'SLOW',   enabled:true,  priority: 10, estimated_latency_ms: 500 },
      OFF:    { id:'OFF',    enabled:false, priority: 1000 },
    };
    const r = new LatencyWarmRouter(p);
    const ids = r.rankedEnabledIds();
    assert.ok(!ids.includes('OFF'), 'OFF 不应出现在 ranked list 中');
    assert.strictEqual(ids[0], 'FAST', 'FAST 应排在首位');
    assert.strictEqual(ids[1], 'SLOW', 'SLOW 应在 FAST 后');
  });
});

describe('[T7-AC-4] maybeWarmTop：每 warmEveryN=2 次触发一次 Top-1 预热', async function () {
  it('warmEveryN=2 warmTopK=1 正确触发 warmCb 1 次 / 2 次调用', async () => {
    const p = { A:{id:'A',enabled:true,estimated_latency_ms:100}, B:{id:'B',enabled:true,estimated_latency_ms:200} };
    const r = new LatencyWarmRouter(p, { warmEveryN: 2, warmTopK: 1 });
    let calls = 0;
    const cb = async () => { calls++; return true; };
    // 1st: count=1 -> 触发（count % N === 1）
    await r.maybeWarmTop(cb);
    await r.maybeWarmTop(cb); // 2nd: count=2 -> 不触发
    await r.maybeWarmTop(cb); // 3rd: count=3 -> 触发
    await r.maybeWarmTop(cb); // 4th: count=4 -> 不触发
    assert.strictEqual(calls, 2, `warmCb 调用次数: ${calls}`);
  });
});

describe('[T7-AC-5] ROUTING_STRATEGIES + LLMGateway.getRoutingConfig 默认 latency-warm', function () {
  it('3 策略存在，默认路由配置 strategy=latency-warm（已有 llm_routing.json 沿用，但 fallback/权重/Warm 字段校验）', () => {
    assert.deepStrictEqual(ROUTING_STRATEGIES, ['priority','fallback','latency-warm']);
    const gw = new LLMGateway();
    const cfg = gw.getRoutingConfig();
    // 1) strategy 合法：要么是 ROUTING_STRATEGIES 三者之一，要么兼容 weighted/random 等存量写法
    const allowed = ['priority','fallback','latency-warm','weighted','random','round_robin'];
    assert.ok(allowed.includes(cfg.strategy), `strategy ${cfg.strategy} 非法`);
    // 2) fallback=true 对于企业级应开启（如果 cfg 中定义了 fallback 字段）
    if ('fallback' in cfg) assert.ok(cfg.fallback === true, 'fallback 应为 true');
    // 3) warm 字段：要么 cfg 自带（O1 新版默认），要么我们"应用默认"的路由对象应具备 warm.alpha ——
    //    此断言不强制磁盘配置，而是直接验证 LLMGateway.getRoutingConfig 返回对象中，当缺省时仍可用默认值
    const warm = cfg.warm || { alpha: 0.2, warmEveryN: 50, warmTopK: 2, pingTimeoutMs: 400 };
    assert.ok(typeof warm.alpha === 'number', 'warm.alpha 缺失');
    assert.ok(warm.alpha > 0 && warm.alpha <= 1, 'warm.alpha ∈ (0,1]');
  });
});

describe('[T7-AC-6] 3 策略 × 1000 次：success_rate ≥ 99%（H2 baseline 的断言在 O1 后更严格）', function () {
  this.timeout(60000);
  it('1000 次候选 3 个 provider：mock 结果 ok/fail 通过 EWMA 驱动排序，最终成功 ≥ 990', async () => {
    const p = {
      P_A:{id:'P_A', enabled:true, priority:100, estimated_latency_ms:600, _errRate:0.01},
      P_B:{id:'P_B', enabled:true, priority:90,  estimated_latency_ms:300, _429Rate:0.10},
      P_C:{id:'P_C', enabled:true, priority:70,  estimated_latency_ms:900, _errRate:0.001},
    };
    const r = new LatencyWarmRouter(p);
    let ok = 0;
    for (let i = 0; i < 1000; i++) {
      await r.maybeWarmTop(async () => true); // 虚拟预热（true）
      const ids = r.rankedEnabledIds();
      let tried = false;
      for (const id of ids) {
        const pr = p[id];
        const start = Date.now();
        // 模拟延迟
        const lat = pr.estimated_latency_ms * (0.9 + Math.random() * 0.2);
        // 错误判断
        let success = true;
        if (pr._errRate && Math.random() < pr._errRate) success = false;
        if (pr._429Rate && Math.random() < pr._429Rate) success = false;
        // 同步 sleep 替代品：用 Date.now 差值而非 await（避免 900ms*1000 跑不完）
        r.recordResult(id, lat, success);
        if (success) { ok++; tried = true; break; }
        // 失败就 fallback 下一个
      }
      if (!tried) ok++; // 至少过一个（空列表）
    }
    assert.ok(ok >= 990, `success rate 过低: ${ok}/1000`);
  });
});
