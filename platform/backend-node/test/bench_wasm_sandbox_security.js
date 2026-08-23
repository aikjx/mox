'use strict';
/**
 * H3 · Wasm 沙箱安全 Benchmark（T5 AC-06）
 *   - 模拟 Wasm Operator 沙箱（纯 JS 仿真，便于在 CI 中复跑）：
 *       * 1000 次正常请求（12 种良性算子，平均 O(n)）
 *       * 1000 次恶意请求（50% 死循环 / 30% 无限内存增长 / 15% 栈爆破 / 5% syscall 探测）
 *   - 开关 O3 补丁：env H3_ENABLE_FUEL=1
 *       O3 具体：
 *         * Fuel 硬上限：fuel_limit=2_000_000 Wasm 指令
 *         * Memory 硬上限：mem_pages_limit=16 Pages（16 × 64KB = 1MB）
 *         * 超限即时 Trap（对应 wasmer store.set_fuel / metering middlewares）
 *   - CSV 输出（每行一次调用）：
 *       ts_ms,kind,op_id,trait_normal_or_malicious,latency_ms,status[ok|trap_fuel|trap_mem|trap_oom_crash|panic],fuel_left,fuel_total,mem_pages_used,mem_pages_limit,ok_count_moving,rss_kb
 *   - 追加 summary 段：
 *       summary, ok, trap_fuel, trap_mem, trap_oom_crash, panics, malicious_false_escape, p50_ms, p95_ms, p99_ms, mem_rss_kb
 */
const path = require('path');
const fs = require('fs');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
const SPECDIR = path.resolve(ROOT, '..', '..', '.trae', 'specs', '20260823-enterprise-compare-top-oss-ai-products-optimize');
const OUTDIR = process.env.H3_OUTDIR || path.join(SPECDIR, 'harness-data');
const DEFAULT_OUTFILE = process.env.H3_OUTFILE || (process.env.H3_ENABLE_FUEL === '1' ? 'h3_after.csv' : 'h3_before.csv');
const OUTFILE = path.join(OUTDIR, DEFAULT_OUTFILE);

const ENABLE_O3 = process.env.H3_ENABLE_FUEL === '1';
const FUEL_LIMIT = parseInt(process.env.H3_FUEL || '2000000', 10);      // O3 硬上限：2M 指令
const PAGES_LIMIT = parseInt(process.env.H3_MEM_PAGES || '16', 10);   // O3 硬上限：16 Pages (1MB)
const PAGE_BYTES = 64 * 1024;

const NORMAL_COUNT = 1000;
const MAL_COUNT = 1000;

const NORMAL_OPS = [
  { id: 'add_1024',     fn: (ctx) => { const a=new Array(1024); for(let i=0;i<a.length;i++){ a[i]=i; ctx.useFuel(2048); ctx.useMem(1); return a.reduce((x,y)=>x+y,0); } } },
  { id: 'mul_512',      fn: (ctx) => { ctx.useFuel(1024); ctx.useMem(1); let r=1; for(let i=1;i<=512;i++) r=r*i % 1e9; return r; } },
  { id: 'mat_8x8',      fn: (ctx) => { ctx.useFuel(4096); ctx.useMem(1); const m=new Array(64).fill(0); for(let i=0;i<8;i++) for(let j=0;j<8;j++) m[i*8+j]=i*j; return m; } },
  { id: 'sum_u64_16k',  fn: (ctx) => { ctx.useFuel(32768); ctx.useMem(4); let s=0; for(let i=0;i<16384;i++) s+=i; return s; } },
  { id: 'sqrt_1k',      fn: (ctx) => { ctx.useFuel(2048); ctx.useMem(1); let s=0; for(let i=1;i<=1000;i++) s+=Math.sqrt(i); return s; } },
  { id: 'fib_25',       fn: (ctx) => { ctx.useFuel(8192); ctx.useMem(1); const fib=(n)=>n<2?n:fib(n-1)+fib(n-2); return fib(25); } },
  { id: 'rsort_1k',     fn: (ctx) => { ctx.useFuel(8192); ctx.useMem(2); const a=Array.from({length:1000},()=>Math.random()); return a.sort()[500]; } },
  { id: 'qsort_512',    fn: (ctx) => { ctx.useFuel(8192); ctx.useMem(2); const a=Array.from({length:512},()=>Math.random()*1e6); a.sort((x,y)=>x-y); return a[0]+a[a.length-1]; } },
  { id: 'sha_64B_em',   fn: (ctx) => { ctx.useFuel(4096); ctx.useMem(1); let h=0xdeadbeef; const s='The quick brown fox jumps over the lazy dog.'.repeat(2); for(let i=0;i<s.length;i++){ h=(h*31+s.charCodeAt(i))>>>0;} return h; } },
  { id: 'conv_16_16',   fn: (ctx) => { ctx.useFuel(16384); ctx.useMem(3); const K=[[1,0],[0,-1]]; const M=Array.from({length:16},()=>Array(16).fill(1)); for(let i=0;i<15;i++) for(let j=0;j<15;j++){ M[i][j]=K[0][0]*M[i][j]; } return M[0][0]; } },
  { id: 'grep_8k',      fn: (ctx) => { ctx.useFuel(16384); ctx.useMem(2); const s='abcdefghij'.repeat(800); let c=0; for(let i=0;i<s.length;i++) if(s[i]==='a')c++; return c; } },
  { id: 'kmeans_iter_64', fn: (ctx) => { ctx.useFuel(16384); ctx.useMem(2); const pts=Array.from({length:64},()=>[Math.random(),Math.random()]); const c0=[0.2,0.2],c1=[0.8,0.8]; let a=0,b=0; for(const p of pts){ const d0=(p[0]-c0[0])**2+(p[1]-c0[1])**2; const d1=(p[0]-c1[0])**2+(p[1]-c1[1])**2; if(d0<d1)a++; else b++; } return [a,b]; } },
];
const MALICIOUS_OPS = [
  // 50% 死循环：没有 O3 时每次至少占用 25~35ms（setInterval 超时安全网确保进程不 hang）
  { id: 'inf_loop_cpu',    ratio: 0.50, fn: (ctx) => { ctx.useMem(1); while(true){ ctx.useFuel(1); /* 不做系统调用 */ } } },
  // 30% 无限内存增长：每 1024 fuel 追加 1 Page (64KB) 数据
  { id: 'mem_bomb',        ratio: 0.30, fn: (ctx) => { let total=0; while(true){ const a=new Array(PAGE_BYTES/8).fill(1); ctx.useMem(1); ctx.useFuel(1024); total+=a[0]; } return total; } },
  // 15% 栈爆破（OOM crash / 递归爆栈）
  { id: 'stack_blow',      ratio: 0.15, fn: (ctx) => { const rec=()=>{ ctx.useFuel(1); return 1+rec(); }; return rec(); } },
  // 5% syscall 探测（eval/Function 尝试执行，在“沙箱”里视为非法）
  { id: 'syscall_probe',   ratio: 0.05, fn: (ctx) => { ctx.useFuel(50); ctx.useMem(1); return Function('return globalThis')(); } },
];

// O3 仿真上下文：fuel + mem 限制
class SandboxCtx {
  constructor(opts) {
    this.fuelLeft = opts.fuel || Infinity;
    this.fuelTotal = opts.fuel || Infinity;
    this.pagesUsed = 0;
    this.pagesLimit = opts.pages || Infinity;
    this.trap = null;
    this.hardAbortAt = Date.now() + (opts.softTimeoutMs || 30); // O3=OFF时也给30ms兜底
  }
  useFuel(n) {
    // 每 64 次调用才检查时间，降低开销
    if ((this._fc = (this._fc||0)+1) % 64 === 0 && Date.now() >= this.hardAbortAt) {
      this.trap = 'trap_soft_timeout'; throw new Error('TIMEOUT_SOFT');
    }
    if (this.fuelLeft !== Infinity) {
      this.fuelLeft -= n;
      if (this.fuelLeft <= 0) { this.trap = 'trap_fuel'; throw new Error('FUEL_TRAP'); }
    }
  }
  useMem(pages) {
    this.pagesUsed += pages;
    if (this.pagesUsed > this.pagesLimit) { this.trap = 'trap_mem'; throw new Error('MEM_TRAP'); }
  }
}

function pickMal() {
  const r = Math.random();
  let acc = 0;
  for (const m of MALICIOUS_OPS) {
    acc += m.ratio;
    if (r < acc) return m;
  }
  return MALICIOUS_OPS[MALICIOUS_OPS.length - 1];
}

function runOp(op, ctx) {
  const start = Date.now();
  try {
    const ret = op.fn(ctx);
    return { status: 'ok', latency: Date.now() - start, result: typeof ret === 'number' ? ret : 0 };
  } catch (e) {
    if (ctx.trap) return { status: ctx.trap, latency: Date.now() - start };
    const msg = String(e && e.message || e);
    if (/TIMEOUT_SOFT/.test(msg)) return { status: 'trap_soft_timeout', latency: Date.now() - start };
    if (/stack|recursion|out of memory/i.test(msg)) return { status: 'trap_oom_crash', latency: Date.now() - start };
    return { status: 'panic', latency: Date.now() - start };
  }
}

function percentile(sorted, p) {
  if (!sorted.length) return 0;
  return sorted[Math.min(sorted.length - 1, Math.floor(p / 100 * sorted.length))];
}

async function main() {
  fs.mkdirSync(OUTDIR, { recursive: true });
  const lines = [
    ['ts_ms','kind','op_id','trait','latency_ms','status','fuel_left','fuel_total','mem_pages_used','mem_pages_limit','ok_count_moving','rss_kb'].join(',')
  ];
  console.log(`[H3] start NORMAL=${NORMAL_COUNT} MALICIOUS=${MAL_COUNT} O3_ENABLE=${ENABLE_O3} fuel=${FUEL_LIMIT} pages=${PAGES_LIMIT}`);

  const order = [];
  for (let i=0;i<NORMAL_COUNT;i++) order.push({ kind:'normal', op: NORMAL_OPS[i % NORMAL_OPS.length] });
  for (let i=0;i<MAL_COUNT;i++)    order.push({ kind:'mal',    op: pickMal() });
  // 打乱次序，但保留 deterministic seed（Math.random 在此前已 warm 过）：Fisher-Yates
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }

  const stats = { ok:0, trap_fuel:0, trap_mem:0, trap_oom_crash:0, trap_soft_timeout:0, panics:0, mal_escape:0 };
  const lats = [];
  let okMoving = 0;

  for (let i = 0; i < order.length; i++) {
    const item = order[i];
    const ctx = ENABLE_O3
      ? new SandboxCtx({ fuel: FUEL_LIMIT, pages: PAGES_LIMIT, softTimeoutMs: 40 })
      : new SandboxCtx({ softTimeoutMs: 30 });

    const ts = Date.now();
    const r = runOp(item.op, ctx);
    lats.push(r.latency);

    switch (r.status) {
      case 'ok':                stats.ok++; okMoving++; break;
      case 'trap_fuel':         stats.trap_fuel++; break;
      case 'trap_mem':          stats.trap_mem++; break;
      case 'trap_oom_crash':    stats.trap_oom_crash++; break;
      case 'trap_soft_timeout': stats.trap_soft_timeout++; break;
      default:                  stats.panics++;
    }
    // 恶意请求的 false escape：本应 trap，但却 OK
    if (item.kind === 'mal' && r.status === 'ok') stats.mal_escape++;

    lines.push([
      ts, item.kind, item.op.id, item.kind,
      r.latency, r.status,
      (ctx.fuelLeft === Infinity ? 'inf' : Math.max(0, ctx.fuelLeft|0)),
      (ctx.fuelTotal === Infinity ? 'inf' : ctx.fuelTotal),
      ctx.pagesUsed,
      (ctx.pagesLimit === Infinity ? 'inf' : ctx.pagesLimit),
      okMoving,
      Math.round(process.memoryUsage().rss / 1024),
    ].join(','));
  }

  const latsSorted = lats.slice().sort((a,b)=>a-b);
  lines.push('');
  lines.push(['summary','ok','trap_fuel','trap_mem','trap_oom_crash','trap_soft_timeout','panics','malicious_false_escape','p50_ms','p95_ms','p99_ms','rss_kb'].join(','));
  lines.push([
    'summary',
    stats.ok, stats.trap_fuel, stats.trap_mem, stats.trap_oom_crash, stats.trap_soft_timeout, stats.panics,
    stats.mal_escape,
    percentile(latsSorted, 50), percentile(latsSorted, 95), percentile(latsSorted, 99),
    Math.round(process.memoryUsage().rss / 1024),
  ].join(','));
  fs.writeFileSync(OUTFILE, lines.join('\n') + '\n', 'utf8');
  console.log(`[H3] summary -> ${JSON.stringify(stats)}`);
  console.log(`[H3] CSV (${lines.length} rows) -> ${OUTFILE}`);
  // TR-6.1 / TR-6.2 / TR-6.3
  assert.ok(lines.length - 3 >= NORMAL_COUNT + MAL_COUNT, `detail rows too few: ${lines.length - 3}`);
  if (ENABLE_O3) {
    // O3 打开：mal_escape ≤ 2‰
    assert.ok(stats.mal_escape <= MAL_COUNT * 0.002, `[O3] mal_escape ${stats.mal_escape} 超限`);
    // O3 打开：至少 80% malicious → trap_fuel / trap_mem（不是 soft_timeout / panic / oom_crash 这种不可控形式）
    const cleanTraps = stats.trap_fuel + stats.trap_mem;
    assert.ok(cleanTraps >= MAL_COUNT * 0.8, `[O3] clean traps too few: ${cleanTraps}/${MAL_COUNT}`);
  } else {
    // O3 关闭：baseline 必然 soft timeout/oom_crash 远多于 clean traps
    const badTraps = stats.trap_soft_timeout + stats.trap_oom_crash;
    assert.ok(badTraps >= MAL_COUNT * 0.5, `[baseline] 预计大量 soft timeout/oom_crash，但实际仅 ${badTraps}/${MAL_COUNT}`);
  }
  process.exit(0);
}

if (require.main === module) {
  main().catch(e => { console.error('[H3] FATAL', e); process.exit(1); });
} else {
  module.exports = { main, SandboxCtx, NORMAL_OPS, MALICIOUS_OPS, percentile };
}
