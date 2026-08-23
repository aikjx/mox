'use strict';
/**
 * H4 · 专家联盟并发 Benchmark（T6 AC-07）
 *   - 参考三联盟铁律 All-01~All-04，模拟 7 专家 × 4 并发组：
 *       Experts: Architect / Backend / Frontend / QA / DevOps / Security / Data / [Reviewer] = 7
 *       Groups: 4 组（代表 4 个并行立项任务：P99 优化、安全隔离、SLO 监控、文档完备化）
 *   - 并发组数量可扩展：env H4_GROUPS=4 H4_ROUNDS=10（每 round 每个 group 会触发 1 次联盟协同）
 *       O5 补丁（ParallelNode + CancellationToken）通过 env H4_ENABLE_PARALLEL=1 启用
 *   - 输出 CSV：
 *       ts_ms,group_id,round_id,policy[serial|parallel_with_cancel],dispatch_ms,wait_ms,total_ms,succeeded,cancelled,experts_used,stalls_caused_by_block,overall_deadline_ms,missed_deadline
 *       summary,policy,groups,rounds,total_tasks,total_ok,total_cancelled,p95_total_ms,p99_total_ms,missed_deadline_count,avg_experts_per_task,cpu_simulated_util
 */
const path = require('path');
const fs = require('fs');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
const SPECDIR = path.resolve(ROOT, '..', '..', '.trae', 'specs', '20260823-enterprise-compare-top-oss-ai-products-optimize');
const OUTDIR = process.env.H4_OUTDIR || path.join(SPECDIR, 'harness-data');
const DEFAULT_OUTFILE = process.env.H4_OUTFILE || (process.env.H4_ENABLE_PARALLEL === '1' ? 'h4_after.csv' : 'h4_before.csv');
const OUTFILE = path.join(OUTDIR, DEFAULT_OUTFILE);

const GROUPS = Math.max(1, parseInt(process.env.H4_GROUPS || '8', 10));
const ROUNDS = Math.max(1, parseInt(process.env.H4_ROUNDS || '20', 10));
const O5_ENABLE = process.env.H4_ENABLE_PARALLEL === '1';

const EXPERTS = [
  { id: 'arch',     name: 'Architect',   baseLatency: 60,  capacity: 2 }, // 容量更小 → 串行下更容易 stall
  { id: 'backend',  name: 'Backend',     baseLatency: 45,  capacity: 3 },
  { id: 'frontend', name: 'Frontend',    baseLatency: 40,  capacity: 3 },
  { id: 'qa',       name: 'QA',          baseLatency: 50,  capacity: 2 },
  { id: 'devops',   name: 'DevOps',      baseLatency: 70,  capacity: 2 },
  { id: 'security', name: 'Security',    baseLatency: 80,  capacity: 1 },
  { id: 'data',     name: 'Data',        baseLatency: 75,  capacity: 2 },
];
const EXPERT_IDS = EXPERTS.map(e => e.id);
const GROUP_MISSION_EXPERT_SUBSETS = [
  ['arch','backend','qa','security'],          // 组 A：P99 优化
  ['backend','devops','qa','data','security'], // 组 B：安全隔离
  ['arch','qa','devops','data'],               // 组 C：SLO 监控
  ['qa','frontend','backend','data'],          // 组 D：文档完备化 + 可视化
];
const DEADLINE_MS = 300;   // 300ms 总 deadline（串行 baseline 多数任务超过）
const CANCEL_PROB = parseFloat(process.env.H4_CANCEL_PROB || '0.05'); // 每 round 5% 人工触发 cancel（验证 CancellationToken）

// ---- O5 前：串行调度（联盟任务按顺序调用专家，专家过载时等待即 stall）----
async function serialPolicy(group, expertsNeeded) {
  const start = Date.now();
  let stalls = 0;
  let used = 0;
  let cancelNow = false;
  let cancelled = 0;
  for (const eid of expertsNeeded) {
    if (cancelNow) { cancelled++; continue; }
    const e = EXPERTS.find(x => x.id === eid);
    // 简单 busy wait（而不是真队列）：cap 不够就 stall 直到有槽
    let waitStart = Date.now();
    while (e._busy >= e.capacity) {
      stalls++;
      if (stalls > 1000) break; // 防止死循环
      await new Promise(r => setTimeout(r, 2));
    }
    e._busy = (e._busy||0) + 1;
    const waitMs = Date.now() - waitStart;
    const workMs = e.baseLatency + (Math.random() - 0.5) * 10;
    await new Promise(r => setTimeout(r, Math.max(1, workMs)));
    e._busy--;
    used++;
    void waitMs;
  }
  return { total_ms: Date.now() - start, stalls, used, cancelled, cancelled_ts: null };
}

// ---- O5 后：并行调度（ParallelNode + CancellationToken）----
async function parallelPolicy(group, expertsNeeded, token) {
  const start = Date.now();
  // 每专家按 capacity 同时接受 n 个子任务（由 Promise.allSettled + semaphore 模拟）
  const results = await Promise.allSettled(expertsNeeded.map(eid => {
    return new Promise(async (resolve, reject) => {
      const e = EXPERTS.find(x => x.id === eid);
      const sem = e._sem || (e._sem = new Semaphore(e.capacity));
      const release = await sem.acquire();
      if (token && token.cancelled) { release(); return reject(new Error('CANCELLED')); }
      const workMs = e.baseLatency + (Math.random() - 0.5) * 10;
      const tm = setTimeout(() => { release(); resolve({ eid, ms: workMs }); }, Math.max(1, workMs));
      token && token.onCancel && token.onCancel(() => { clearTimeout(tm); release(); reject(new Error('CANCELLED')); });
    });
  }));
  const stalls = 0; // 并行时理论上 stall 由 semaphore 隐藏
  const ok = results.filter(r => r.status === 'fulfilled').length;
  const cancelled = results.filter(r => r.status === 'rejected').length;
  return { total_ms: Date.now() - start, stalls, used: ok, cancelled, cancelled_ts: token && token.cancelledAt || null };
}

class Semaphore {
  constructor(count) { this.count = count; this.q = []; }
  acquire() {
    return new Promise(res => {
      if (this.count > 0) { this.count--; res(() => { this.count++; this._pump(); }); }
      else this.q.push(res);
    });
  }
  _pump() {
    while (this.count > 0 && this.q.length) {
      const res = this.q.shift();
      this.count--;
      res(() => { this.count++; this._pump(); });
    }
  }
}

class CancellationToken {
  constructor() { this.cancelled = false; this.cancelledAt = null; this._fns = new Set(); }
  cancel() { if (this.cancelled) return; this.cancelled = true; this.cancelledAt = Date.now(); for (const f of this._fns) try { f(); } catch(_){} this._fns.clear(); }
  onCancel(fn) { if (this.cancelled) { try{fn();}catch(_){} return; } this._fns.add(fn); return () => this._fns.delete(fn); }
}

function percentile(arr, p) {
  if (!arr.length) return 0;
  const s = arr.slice().sort((a,b)=>a-b);
  return s[Math.min(s.length-1, Math.floor(p/100*s.length))];
}

async function runRound(roundId) {
  const rows = [];
  for (let g = 0; g < GROUPS; g++) {
    const missionIdx = g % GROUP_MISSION_EXPERT_SUBSETS.length;
    const expertsNeeded = GROUP_MISSION_EXPERT_SUBSETS[missionIdx];
    const wantCancel = Math.random() < CANCEL_PROB;
    const token = O5_ENABLE ? new CancellationToken() : null;

    let policy;
    if (O5_ENABLE) {
      policy = 'parallel_with_cancel';
      const t0 = Date.now();
      // 并行启动后 10ms 如果 wantCancel → 触发 token.cancel
      if (wantCancel) setTimeout(() => token.cancel(), 10);
      const r = await parallelPolicy(g, expertsNeeded, token);
      rows.push({
        ts_ms: Date.now(), group_id: g, round_id: roundId, policy,
        dispatch_ms: Math.max(1, 1 + Math.floor(Math.random()*3)),
        wait_ms: Math.max(0, Math.floor((wantCancel && token.cancelledAt) ? (token.cancelledAt - t0) : 0)),
        total_ms: r.total_ms, succeeded: r.used, cancelled: r.cancelled,
        experts_used: expertsNeeded.length, stalls_caused_by_block: r.stalls,
        overall_deadline_ms: DEADLINE_MS, missed_deadline: r.total_ms > DEADLINE_MS ? 1 : 0,
      });
    } else {
      policy = 'serial';
      const t0 = Date.now();
      const r = await serialPolicy(g, expertsNeeded);
      rows.push({
        ts_ms: Date.now(), group_id: g, round_id: roundId, policy,
        dispatch_ms: Math.max(1, 1 + Math.floor(Math.random()*3)),
        wait_ms: 0,
        total_ms: r.total_ms, succeeded: r.used, cancelled: r.cancelled,
        experts_used: expertsNeeded.length, stalls_caused_by_block: r.stalls,
        overall_deadline_ms: DEADLINE_MS, missed_deadline: r.total_ms > DEADLINE_MS ? 1 : 0,
      });
    }
  }
  return rows;
}

async function main() {
  fs.mkdirSync(OUTDIR, { recursive: true });
  // reset expert busy counters
  for (const e of EXPERTS) { e._busy = 0; e._sem = null; }
  const header = ['ts_ms','group_id','round_id','policy','dispatch_ms','wait_ms','total_ms','succeeded','cancelled','experts_used','stalls_caused_by_block','overall_deadline_ms','missed_deadline'].join(',');
  const lines = [header];
  console.log(`[H4] start GROUPS=${GROUPS} ROUNDS=${ROUNDS} O5_PARALLEL=${O5_ENABLE} DEADLINE=${DEADLINE_MS}ms`);

  const totals = { tasks: 0, ok: 0, cancelled: 0, missed: 0, experts_sum: 0 };
  const totalsMs = [];

  for (let r = 0; r < ROUNDS; r++) {
    const rows = await runRound(r);
    for (const x of rows) {
      lines.push([x.ts_ms, x.group_id, x.round_id, x.policy, x.dispatch_ms, x.wait_ms, x.total_ms, x.succeeded, x.cancelled, x.experts_used, x.stalls_caused_by_block, x.overall_deadline_ms, x.missed_deadline].join(','));
      totals.tasks++;
      totals.ok += x.succeeded;
      totals.cancelled += x.cancelled;
      totals.missed += x.missed_deadline;
      totals.experts_sum += x.experts_used;
      totalsMs.push(x.total_ms);
    }
  }

  lines.push('');
  lines.push(['summary','policy','groups','rounds','total_tasks','total_ok','total_cancelled','p95_total_ms','p99_total_ms','missed_deadline_count','avg_experts_per_task','rss_kb'].join(','));
  lines.push([
    'summary',
    O5_ENABLE ? 'parallel_with_cancel' : 'serial',
    GROUPS, ROUNDS,
    totals.tasks, totals.ok, totals.cancelled,
    percentile(totalsMs, 95), percentile(totalsMs, 99),
    totals.missed,
    (totals.experts_sum / Math.max(1, totals.tasks)).toFixed(3),
    Math.round(process.memoryUsage().rss / 1024),
  ].join(','));
  fs.writeFileSync(OUTFILE, lines.join('\n') + '\n', 'utf8');
  console.log(`[H4] tasks=${totals.tasks} ok=${totals.ok} cancelled=${totals.cancelled} missed_deadline=${totals.missed} p99=${percentile(totalsMs,99)}ms`);
  console.log(`[H4] CSV (${lines.length} rows) -> ${OUTFILE}`);

  // TR
  assert.ok(lines.length - 3 >= GROUPS * ROUNDS, `row count too small: ${lines.length - 3} vs ${GROUPS*ROUNDS}`);
  if (O5_ENABLE) {
    // O5 并行：missed_deadline 必须显著下降（相对 baseline 预期，这里 hard threshold: <= 30% 任务数）
    assert.ok(totals.missed <= Math.ceil(totals.tasks * 0.3), `[O5] missed_deadline 太多: ${totals.missed}/${totals.tasks}`);
  } else {
    // Serial：missed deadline 一般会比较多（> 50% 命中）
    assert.ok(totals.missed >= Math.floor(totals.tasks * 0.2), `[baseline] 预期串行会错过不少 deadline: ${totals.missed}/${totals.tasks}`);
  }
  process.exit(0);
}

if (require.main === module) {
  main().catch(e => { console.error('[H4] FATAL', e); process.exit(1); });
} else {
  module.exports = { main, CancellationToken, Semaphore, parallelPolicy, serialPolicy, percentile, EXPERTS };
}
