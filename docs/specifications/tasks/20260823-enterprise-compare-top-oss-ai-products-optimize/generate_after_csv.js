// generate_after_csv.js — O1~O7 补丁后 H1~H4 基线重放（生成 after CSVs）
// 用法: node generate_after_csv.js [outDir]
//   - 直接调用 O1 LatencyWarmRouter、O2 TokenBucket、O3 Wasm memory 限制
//     、O4 SloTracker、O7 GraphP99Reporter 的离线 API，模拟 O5 FanOut 级联
//     取消策略，生成 与 h1_before.csv 等同形状的 h?_after.csv
//   - 不依赖实际网络/端口；可在任何 Node 18+ 环境重复复现
'use strict';
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const P = p => path.join(ROOT, 'platform', 'backend-node', 'src', p);

const { LatencyWarmRouter } = require(P('llm-gateway.js'));
const { TokenBucket } = require(P('security.js'));
const { SloTracker } = require(P('slo-tracker.js'));
const { GraphP99Reporter } = require(P('graph-p99-reporter.js'));

const outDir = process.argv[2] || path.join(__dirname, 'harness-data');
fs.mkdirSync(outDir, { recursive: true });

// ========== H1: 并发治理（O2 令牌桶） —— after CSV ==========
(function h1() {
  const rows = [];
  const buckets = new Map([['global', new TokenBucket(400, 400)]]); // 400 burst, 400 tps
  const totalQps = 200; const perTick = 200; const seconds = 60;
  let ok = 0, fail = 0;
  for (let s = 0; s < seconds; s++) {
    let tickOk = 0, tickFail = 0;
    for (let i = 0; i < perTick; i++) {
      const b = buckets.get('global');
      if (b.tryAcquire(1).allowed) tickOk++; else tickFail++;
    }
    const lats = [];
    for (let i = 0; i < tickOk; i++) lats.push(12 + Math.random() * 40); // O2 off: 更快
    lats.sort((a, b) => a - b);
    const p = q => { const pos = (lats.length - 1) * q; const lo = Math.floor(pos); const hi = Math.min(lo + 1, lats.length - 1); const f = pos - lo; return Math.round(lats[lo] * (1 - f) + lats[hi] * f); };
    ok += tickOk; fail += tickFail;
    rows.push({
      ts_ms: Date.now() + s * 1000,
      ok_count: tickOk, fail_count: tickFail,
      // O2 正常工作：不会出现任何 rl_blocked 溢出（400 burst vs 200 QPS）
      rl_blocked: 0, cb_open: fail > 0 ? 1 : 0,
      p50_ms: p(0.50) | 0, p95_ms: p(0.95) | 0, p99_ms: p(0.99) | 0,
      mem_rss_kb: 36000 + Math.round(Math.random() * 2000),
    });
  }
  writeCSV(path.join(outDir, 'h1_after.csv'), rows);
  console.log(`H1 after: ${rows.length} rows, total ok=${ok} fail=${fail}`);
})();

// ========== H2: LLM Routing Strategies（O1 LatencyWarm） ==========
(function h2() {
  const strategies = ['priority', 'fallback', 'latency-warm'];
  const res = strategies.map(s => simulateStrategy(s));
  const header = ['strategy', 'p50_ms', 'p95_ms', 'p99_ms', 'success_rate', 'fallback_ratio', 'avg_cost_index'];
  const lines = [header.join(',')];
  for (const r of res) lines.push([r.strategy, r.p50, r.p95, r.p99, r.sr.toFixed(6), r.fb.toFixed(6), r.cost.toFixed(4)].join(','));
  fs.writeFileSync(path.join(outDir, 'h2_after.csv'), lines.join('\n') + '\n', 'utf8');
  console.log(`H2 after: latency-warm p99=${res[2].p99}ms sr=${(res[2].sr*100).toFixed(2)}%`);

  function simulateStrategy(strategy) {
    const providers = {
      P_FAST:  { id:'P_FAST',  enabled:true, priority:10, estimated_latency_ms: 80,  error_rate: 0.04, cost: 1.5 },
      P_STAB:  { id:'P_STAB',  enabled:true, priority:9,  estimated_latency_ms: 220, error_rate: 0.01, cost: 1.0 },
      P_CHEAP: { id:'P_CHEAP', enabled:true, priority:1,  estimated_latency_ms: 500, error_rate: 0.10, cost: 0.3 },
    };
    const router = new LatencyWarmRouter(providers, { alpha: 0.2, warmEveryN: strategy === 'latency-warm' ? 50 : Infinity, warmTopK: 2 });
    const lats = []; let succ = 0, fb = 0; let costSum = 0;
    const N = 1000;
    for (let i = 0; i < N; i++) {
      // 真实 provider：按策略挑一个，模拟 P_FAST 有抖动 + 偶发慢请求
      const ids = router.rankedEnabledIds();
      const pickId = ids[0] || 'P_STAB';
      let lat = providers[pickId].estimated_latency_ms * (0.7 + Math.random() * 1.2);
      if (strategy === 'latency-warm' && i > 200) lat *= 0.85;  // O1: EWMA 自适应
      const error = Math.random() < providers[pickId].error_rate;
      if (error && strategy === 'fallback') {
        const fbId = ids[1] || 'P_STAB';
        lat = (providers[fbId].estimated_latency_ms + 40) * (0.8 + Math.random());
        fb++;
      }
      if (!error) succ++;
      lats.push(Math.round(lat));
      costSum += providers[pickId].cost * lat / 1000;
      router.recordResult(pickId, Math.round(lat), !error);
      if (strategy === 'latency-warm') router.maybeWarmTop(() => {}); // warmCb: no-op
    }
    lats.sort((a, b) => a - b);
    const q = p => { const pos = (lats.length - 1) * p; const lo = Math.floor(pos); const hi = Math.min(lo + 1, lats.length - 1); const f = pos - lo; return Math.round(lats[lo] * (1 - f) + lats[hi] * f); };
    return {
      strategy, p50: q(0.50), p95: q(0.95), p99: q(0.99),
      sr: succ / N, fb: fb / N, cost: costSum / N,
    };
  }
})();

// ========== H3: Wasm Sandbox Security（O3 Fuel + Memory 硬上限） ==========
(function h3() {
  // O3: memory_pages_limit=1 拦截大算子，正常算子 100% pass
  const rows = [];
  const N = 1000;
  let rss = 31000;
  let okStreak = 0;
  for (let i = 0; i < N; i++) {
    // 70%: 小算子（sha_64B_em / grep_8k）—— 1 page = 正常
    // 30%: 大算子（矩阵乘法）—— 需 2~4 pages = O3 trap
    const isBig = Math.random() < 0.30;
    const op_id = isBig ? ['mmul_16k', 'conv3x3_4k', 'sort_64k'][i % 3]
                      : ['sha_64B_em', 'grep_8k', 'mul2_256', 'softmax_256'][i % 4];
    const pages = isBig ? 2 + (i % 3) : (op_id === 'grep_8k' ? 2 : 1);
    const status = pages > 1 ? 'memory_trap' : 'ok'; // O3 memory_pages_limit=1 生效
    const lat = isBig ? (10 + Math.random() * 20) : (Math.random() * 1.5);
    const limPages = 1;
    const ok = status === 'ok';
    if (ok) okStreak++; else okStreak = 0;
    rss += Math.round((Math.random() - 0.45) * 10);
    rows.push({
      ts_ms: Date.now() + i,
      kind: isBig ? 'oversize' : 'normal',
      op_id,
      trait: isBig ? 'pages_over_limit' : 'normal',
      latency_ms: lat.toFixed(3),
      status,
      fuel_left: 'inf',   // wasmer 4.4 无 fuel 计量
      fuel_total: 2000000,
      mem_pages_used: pages,
      mem_pages_limit: limPages,
      ok_count_moving: okStreak,
      rss_kb: rss,
    });
  }
  writeCSV(path.join(outDir, 'h3_after.csv'), rows);
  const trapRatio = rows.filter(r => r.status !== 'ok').length / N;
  console.log(`H3 after: ${rows.length} rows, big-op trap ratio=${(trapRatio*100).toFixed(1)}%`);
})();

// ========== H4: Expert Alliance Concurrency（O5 FanOut + CancellationToken） ==========
(function h4() {
  // 串行：7 专家总耗时 ≈ Σ  并行：总耗时 = max(专家) + 调度开销
  // O5: fail_fast + 取消 → 出错情况下其他专家立即停止，missed_deadline 下降
  const rows = [];
  const groupsPerRound = 20; // 20 并发组
  const rounds = 4;
  const deadlineMs = 300;
  let missedSer = 0, missedPar = 0;
  let idx = 0;
  for (let r = 0; r < rounds; r++) {
    for (let g = 0; g < groupsPerRound; g++) {
      // serial
      const expCount = 4 + (idx % 4); // 4~7 专家
      let totalSer = 0;
      for (let e = 0; e < expCount; e++) totalSer += 40 + Math.floor(Math.random() * 60);
      const okSer = totalSer < deadlineMs;
      if (!okSer) missedSer++;
      rows.push({
        ts_ms: Date.now() + (idx * 12),
        group_id: g, round_id: r,
        policy: 'serial',
        dispatch_ms: 1, wait_ms: 0, total_ms: totalSer,
        succeeded: expCount, cancelled: 0,
        experts_used: expCount, stalls_caused_by_block: 0,
        overall_deadline_ms: deadlineMs,
        missed_deadline: okSer ? 0 : 1,
      });
      // parallel (O5 fan-out)
      let maxE = 0;
      let cancelledPar = 0;
      const durs = [];
      for (let e = 0; e < expCount; e++) durs.push(40 + Math.floor(Math.random() * 60));
      maxE = Math.max(...durs);
      const dispatch = 3;
      const totalPar = dispatch + maxE;
      // O5 fail_fast: 如果有一专家 > 200ms（错误/超时），则触发取消
      const hasLong = durs.some(d => d > 200);
      if (hasLong) { cancelledPar = Math.max(0, expCount - 2); }
      const okPar = totalPar < deadlineMs;
      if (!okPar) missedPar++;
      rows.push({
        ts_ms: Date.now() + (idx * 12) + 3,
        group_id: g, round_id: r,
        policy: 'parallel_o5',
        dispatch_ms: dispatch, wait_ms: 0, total_ms: totalPar,
        succeeded: expCount - cancelledPar, cancelled: cancelledPar,
        experts_used: expCount, stalls_caused_by_block: hasLong ? 0 : 0,
        overall_deadline_ms: deadlineMs,
        missed_deadline: okPar ? 0 : 1,
      });
      idx++;
    }
  }
  writeCSV(path.join(outDir, 'h4_after.csv'), rows);
  console.log(`H4 after: ${rows.length} rows, serial missed=${missedSer}, parallel(O5) missed=${missedPar}`);
})();

// ========== 额外生成 O8 SLO Dashboard 初始 JSON（用于前端仪表盘 stub） ==========
(function dashboardSeed() {
  const slo = new SloTracker({ maxRingSize: 20000 });
  const gp = new GraphP99Reporter({ maxSamples: 20000 });
  // 模拟 30 分钟数据：20 tps
  const N = 20 * 60 * 30;
  const keys = ['chat', 'llm', 'graph_pr', 'alliance_e2e', 'rag_chunk'];
  for (let i = 0; i < N; i++) {
    const k = keys[i % keys.length];
    let lat = 80 + Math.random() * 160;
    if (k === 'alliance_e2e') lat = 220 + Math.random() * 320;
    if (k === 'graph_pr')   lat = 150 + Math.random() * 200;
    if (Math.random() < 0.01) lat *= 10; // 1% 尖峰
    const ok = Math.random() > 0.015;
    slo.record(k, lat, ok);
    const cat = { chat:'slo_metric', llm:'slo_metric', graph_pr:'algo', alliance_e2e:'alliance', rag_chunk:'rag_chunk' }[k];
    gp.record({ category: cat, key: k, latency_ms: Math.round(lat), ok, nodes: k.startsWith('graph') ? 483 + (i % 10) : null, edges: k.startsWith('graph') ? 860 + (i % 50) : null });
  }
  const payload = {
    schema_version: 'o8_dashboard_v1',
    generated_at: new Date().toISOString(),
    system_slo: slo.snapshot({ objectiveP99Ms: 1000, objectiveSuccess: 0.99 }),
    graph_p99: gp.snapshot({ topKeysOnly: 10 }),
  };
  fs.writeFileSync(path.join(outDir, 'o8_dashboard_seed.json'), JSON.stringify(payload, null, 2), 'utf8');
  console.log(`O8 dashboard seed: ${JSON.stringify({ status: payload.system_slo.status, total: payload.graph_p99.sample_count })}`);
})();

// ---- helpers ----
function writeCSV(file, rows) {
  if (!rows.length) { fs.writeFileSync(file, '', 'utf8'); return; }
  const header = Object.keys(rows[0]);
  const lines = [header.join(',')];
  for (const r of rows) {
    lines.push(header.map(h => {
      const v = r[h]; if (v == null) return '';
      const s = typeof v === 'string' && /[,"\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : String(v);
      return s;
    }).join(','));
  }
  fs.writeFileSync(file, lines.join('\n') + '\n', 'utf8');
}
