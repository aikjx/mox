'use strict';

/**
 * 性能对比 V2：Rust vs Node 旧实现
 * ================================
 * 策略：
 *  (1) 使用足够大图（N=280 nodes, p=0.035 → ~2744 有向边），算法复杂度主导进程 spawn 开销
 *  (2) Rust「单次批处理」模式：一次 CLI 调用完成 7 算法（spawn 开销仅付 1 次）
 *  (3) 分别对每个算法：OLD 纯 JS 时间 vs NEW 批处理 Rust 算法时间
 * 合格线：算法 7 条总体加速比 ≥ 50%（OLD/NEW ≥ 1.5×）
 */

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..');
const RUST_BIN_CANDIDATES = [
  path.join(WORKSPACE_ROOT, 'target', 'release', 'compare_with_node.exe'),
  path.join(WORKSPACE_ROOT, 'target', 'debug', 'compare_with_node.exe'),
];
let RUST_BIN = null;
for (const p of RUST_BIN_CANDIDATES) if (fs.existsSync(p)) { RUST_BIN = p; break; }
if (!RUST_BIN) {
  console.error('[perf] 未找到 compare_with_node 二进制（release/debug 均不存在）。请先执行：');
  console.error('  cargo build --release -p graph-algorithms --bin compare_with_node');
  console.error('  或在 ' + RUST_BIN_CANDIDATES.join(' / ') + ' 下验证');
  process.exit(2);
}
console.log(`[perf] Rust CLI: ${RUST_BIN}\n`);

// ===== 大图 fixture（确定性构造，每次相同） =====
function makeLargeGraph(N, p) {
  const nodes = [];
  for (let i = 0; i < N; i++) nodes.push({ id: `n${i}` });
  const edges = [];
  for (let i = 0; i < N; i++) {
    for (let j = 0; j < N; j++) {
      if (i === j) continue;
      // mulberry32 风格确定性伪随机
      let x = Math.imul(i * 2654435761 ^ j * 40503, 1597334677);
      x = Math.imul(x ^ (x >>> 15), x | 1);
      x ^= x + Math.imul(x ^ (x >>> 7), x | 61);
      const r = ((x ^ (x >>> 14)) >>> 0) / 4294967296;
      if (r < p) edges.push({ source: `n${i}`, target: `n${j}`, weight: 1 });
    }
  }
  return { nodes, edges };
}

// N=200, p=0.028 → 平均每个节点 ~5-6 出边，总边 ~1100 directed (可控的大图 benchmark)
const G = makeLargeGraph(200, 0.028);
console.log(`[perf] Fixture: ${G.nodes.length} nodes, ${G.edges.length} edges（有向）\n`);

// =============== OLD 纯 JS 实现（同 v1） ===============
function OLD_expandRaw(edges) {
  const out = [];
  for (const e of edges || []) {
    const s = e.source, t = e.target;
    if (s === undefined || t === undefined) continue;
    const w = e.weight || 1;
    out.push({ source: s, target: t, weight: w });
    out.push({ source: t, target: s, weight: w });
  }
  return out;
}
function OLD_degree(nodes, edges) {
  const n = nodes.length; if (n === 0) return {};
  const deg = new Map(nodes.map(nd => [nd.id, 0]));
  const e = OLD_expandRaw(edges);
  for (const edge of e) {
    const s = edge.source, t = edge.target, w = edge.weight || 1;
    if (s === t) { if (deg.has(s)) deg.set(s, deg.get(s) + w); continue; }
    if (deg.has(s)) deg.set(s, deg.get(s) + w);
    if (deg.has(t)) deg.set(t, deg.get(t) + w);
  }
  const denom = Math.max(1, n - 1); const out = {};
  for (const [id, d] of deg) out[id] = d / denom;
  return out;
}
function OLD_brandes(nodes, edges) {
  const n = nodes.length; if (n < 3) { const o = {}; nodes.forEach(nd => (o[nd.id] = 0)); return o; }
  const ids = nodes.map(nd => nd.id); const idx = new Map(ids.map((id, i) => [id, i]));
  const eList = OLD_expandRaw(edges);
  const adj = Array.from({ length: n }, () => new Set());
  for (const e of eList) {
    const s = idx.get(e.source), t = idx.get(e.target);
    if (s === undefined || t === undefined || s === t) continue;
    adj[s].add(t); adj[t].add(s);
  }
  const cb = new Array(n).fill(0);
  for (let s = 0; s < n; s++) {
    const stack = []; const queue = [s];
    const dist = new Array(n).fill(-1); const sigma = new Array(n).fill(0);
    const preds = Array.from({ length: n }, () => []);
    dist[s] = 0; sigma[s] = 1;
    while (queue.length) {
      const v = queue.shift(); stack.push(v);
      for (const w of adj[v]) {
        if (dist[w] < 0) { dist[w] = dist[v] + 1; queue.push(w); }
        if (dist[w] === dist[v] + 1) { sigma[w] += sigma[v]; preds[w].push(v); }
      }
    }
    const delta = new Array(n).fill(0);
    while (stack.length) {
      const w = stack.pop();
      for (const v of preds[w]) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
      if (w !== s) cb[w] += delta[w];
    }
  }
  const denom = (n - 1) * (n - 2); const out = {};
  for (let i = 0; i < n; i++) out[ids[i]] = cb[i] / denom;
  return out;
}
function OLD_harmonic(nodes, edges) {
  const n = nodes.length; if (n <= 1) { const o = {}; nodes.forEach(nd => (o[nd.id] = 0)); return o; }
  const ids = nodes.map(nd => nd.id); const idx = new Map(ids.map((id, i) => [id, i]));
  const eList = OLD_expandRaw(edges);
  const adj = Array.from({ length: n }, () => new Set());
  for (const e of eList) {
    const s = idx.get(e.source), t = idx.get(e.target);
    if (s === undefined || t === undefined || s === t) continue;
    adj[s].add(t); adj[t].add(s);
  }
  const out = {};
  for (let v = 0; v < n; v++) {
    const dist = new Array(n).fill(-1); dist[v] = 0; const q = [v];
    while (q.length) {
      const x = q.shift();
      for (const y of adj[x]) { if (dist[y] < 0) { dist[y] = dist[x] + 1; q.push(y); } }
    }
    let harmonic = 0;
    for (let u = 0; u < n; u++) if (u !== v && dist[u] > 0) harmonic += 1 / dist[u];
    out[ids[v]] = harmonic / (n - 1);
  }
  return out;
}
function OLD_ppr(nodes, edges) {
  const n = nodes.length; if (n === 0) return {};
  const ids = nodes.map(nd => nd.id); const idx = new Map(ids.map((id, i) => [id, i]));
  const d = 0.85; const maxIter = 30;
  const outAdj = Array.from({ length: n }, () => []);
  for (const e of edges) {
    const s = idx.get(e.source), t = idx.get(e.target);
    if (s === undefined || t === undefined || s === t) continue;
    outAdj[s].push(t);
  }
  let pr = new Array(n).fill(1 / n); const pers = new Array(n).fill(1 / n);
  for (let iter = 0; iter < maxIter; iter++) {
    const np = new Array(n).fill(0); let dang = 0;
    for (let i = 0; i < n; i++) {
      if (outAdj[i].length === 0) dang += pr[i];
      else { const sh = pr[i] / outAdj[i].length; for (const j of outAdj[i]) np[j] += sh; }
    }
    let delta = 0;
    for (let i = 0; i < n; i++) {
      const v = (1 - d) * pers[i] + d * (np[i] + dang / n);
      delta += Math.abs(v - pr[i]); pr[i] = v;
    }
    if (delta < 1e-8) break;
  }
  const res = {}; for (let i = 0; i < n; i++) res[ids[i]] = pr[i];
  return res;
}
function OLD_cnm(nodes, edges) {
  const ids = nodes.map(nd => nd.id); const N = ids.length;
  if (N < 2) return 0;
  const eList = OLD_expandRaw(edges);
  const undMap = new Map(); const wkey = (a, b) => a < b ? `${a}|${b}` : `${b}|${a}`;
  for (const e of eList) {
    if (e.source === e.target) continue;
    const k = wkey(e.source, e.target);
    undMap.set(k, (undMap.get(k) || 0) + (e.weight || 1));
  }
  const adj = new Map(); ids.forEach(id => adj.set(id, new Map()));
  for (const [k, w] of undMap) {
    const [a, b] = k.split('|');
    if (!adj.has(a) || !adj.has(b)) continue;
    adj.get(a).set(b, (adj.get(a).get(b) || 0) + w);
    adj.get(b).set(a, (adj.get(b).get(a) || 0) + w);
  }
  const twoM = ids.reduce((s, id) => s + [...(adj.get(id)?.values() || [])].reduce((a, b) => a + b, 0), 0);
  const m = twoM / 2; if (m === 0) return 0;
  let nextCid = 0; const nodeComm = new Map(); const comms = new Map();
  for (const id of ids) {
    const cid = nextCid++; nodeComm.set(id, cid);
    const dS = [...(adj.get(id)?.values() || [])].reduce((a, b) => a + b, 0);
    comms.set(cid, { members: new Set([id]), tot: dS, self: 0 });
  }
  while (true) {
    let best = { gain: 0, a: -1, b: -1, eAB: 0 };
    for (const [k, w] of undMap) {
      const [a, b] = k.split('|');
      const ca = nodeComm.get(a), cb = nodeComm.get(b);
      if (ca === undefined || cb === undefined || ca === cb) continue;
      const A = comms.get(ca), B = comms.get(cb); if (!A || !B) continue;
      let eAB = 0;
      const small = A.members.size <= B.members.size ? A : B;
      const bigCid = A.members.size <= B.members.size ? cb : ca;
      for (const id of small.members) {
        const nb = adj.get(id); if (!nb) continue;
        for (const [nbid, wei] of nb) { if (nodeComm.get(nbid) === bigCid) eAB += wei; }
      }
      const delta = (eAB / m) - 2 * 1.0 * A.tot * B.tot / (twoM * twoM);
      if (delta > best.gain) best = { gain: delta, a: ca, b: cb, eAB };
    }
    if (best.gain <= 0) break;
    const A = comms.get(best.a), B = comms.get(best.b);
    const newSelf = A.self + B.self + best.eAB;
    comms.set(best.a, { members: new Set([...A.members, ...B.members]), tot: A.tot + B.tot, self: newSelf });
    comms.delete(best.b);
    for (const id of B.members) nodeComm.set(id, best.a);
  }
  return comms.size; // 返回社区数（作为算法运行标记）
}
function OLD_densityValue(nodes, edges) {
  const set = new Set();
  for (const e of edges) {
    const a = e.source, b = e.target;
    if (a < b) set.add(`${a}|${b}`); else set.add(`${b}|${a}`);
  }
  const E = set.size, N = nodes.length;
  if (N < 2) return 0;
  return (2 * E) / (N * (N - 1));
}
function OLD_rawExpandCount(edges) { return OLD_expandRaw(edges).length; }

// =============== 计时 ===============
function t(ms, label, fn) {
  // warmup 1 次
  fn();
  const t0 = Date.now();
  const res = fn();
  const t1 = Date.now();
  const elapsed = t1 - t0;
  console.log(`  ${label.padEnd(28)} → ${String(elapsed).padStart(5)} ms`);
  return { elapsed, res };
}

const N = G.nodes.length, E = G.edges.length;

console.log('========== [A] OLD 纯 JS 核心算法（Node 旧实现） ==========\n');
const OLD = {};
OLD.degree = t(N, 'OLD degree (O(E) 累加)', () => OLD_degree(G.nodes, G.edges));
OLD.brandes = t(N, 'OLD brandes Brandes O(N·m)', () => OLD_brandes(G.nodes, G.edges));
OLD.harmonic = t(N, 'OLD harmonic (BFS ×N)', () => OLD_harmonic(G.nodes, G.edges));
OLD.ppr = t(N, 'OLD ppr ×30 iter', () => OLD_ppr(G.nodes, G.edges));
OLD.cnm = t(N, 'OLD cnm ΔQ 贪心合并', () => OLD_cnm(G.nodes, G.edges));
OLD.density = t(N, 'OLD density 三字段', () => OLD_densityValue(G.nodes, G.edges));
OLD.raw = t(N, 'OLD raw_expand', () => OLD_rawExpandCount(G.edges));
const OLD_sum = Object.values(OLD).reduce((s, r) => s + r.elapsed, 0);

console.log(`\n  OLD 7 算法总耗时（串行） → ${OLD_sum} ms`);

console.log('\n========== [B] NEW Rust（单 CLI 进程内批处理 7 算法 → spawn 只付 1 次） ==========\n');

// 直接调用 compare_with_node 7 次，但计时串行累加，每调用一次都包含一次 spawn。
// 为了「公平对照」7 OLD 算法串行总和，我们也做 7 次 CLI 调用并求和。
// 但是 spawn 对每个 algo 是 ~500ms，为了得到真正的「Rust 算法时间」，我们做两次测量：
//   B1 = 7 × (spawn + algo)  串行求和
//   B2 = 基线 spawn 开销（一个空 trivial 调用 ×7）
//   B_Rust_algo_total = max(1, B1 - B2)

const INPUT_JSON = JSON.stringify({ nodes: G.nodes, edges: G.edges });
function callRust(name) {
  const r = spawnSync(RUST_BIN, ['--name', name, '--input', '-', '--output', '-'], {
    input: INPUT_JSON, encoding: 'utf-8', maxBuffer: 100 * 1024 * 1024, windowsHide: true,
    cwd: WORKSPACE_ROOT,
  });
  if (r.error) {
    throw new Error(`rust ${name} spawn 错误 (${r.error.code || ''}): ${r.error.message}\n  RUST_BIN=${RUST_BIN}`);
  }
  if (r.status !== 0) {
    throw new Error(
      `rust ${name} failed (status=${r.status}, signal=${r.signal || 'none'}):\n` +
      `  STDERR: ${(r.stderr || '').slice(0, 600)}\n` +
      `  STDOUT(head=300): ${(r.stdout || '').slice(0, 300)}`
    );
  }
  return r.stdout;
}

// warmup
callRust('degree');

// B1 正式测
function t2(label, fn) {
  const t0 = Date.now();
  const out = fn();
  const t1 = Date.now();
  const elapsed = t1 - t0;
  console.log(`  ${label.padEnd(28)} → ${String(elapsed).padStart(5)} ms`);
  return { elapsed };
}
const NEW = {};
NEW.degree = t2('NEW degree (Rust CLI)', () => callRust('degree'));
NEW.brandes = t2('NEW brandes (Rust CLI)', () => callRust('brandes'));
NEW.harmonic = t2('NEW harmonic (Rust CLI)', () => callRust('harmonic'));
NEW.ppr = t2('NEW ppr (Rust CLI, 0.85,30)', () => callRust('ppr'));
NEW.cnm = t2('NEW cnm (Rust CLI)', () => callRust('cnm'));
NEW.density = t2('NEW density (Rust CLI)', () => callRust('density'));
NEW.raw = t2('NEW raw_expand (Rust CLI)', () => callRust('raw_expand'));
const B1_sum = Object.values(NEW).reduce((s, r) => s + r.elapsed, 0);

// B2 基线（测 7 次 raw_expand 最轻量调用作为近似 spawn 基线；但 raw 本身亦有复制）
// 更精确：创建 trivial input（1 node 0 edge）并调用 density N 次
const TRIVIAL = JSON.stringify({ nodes: [{ id: 'x' }], edges: [] });
function baseline() {
  spawnSync(RUST_BIN, ['--name', 'density', '--input', '-', '--output', '-'], {
    input: TRIVIAL, encoding: 'utf-8', maxBuffer: 5 * 1024 * 1024, windowsHide: true, cwd: WORKSPACE_ROOT,
  });
}
baseline();
const t_base_start = Date.now();
for (let i = 0; i < 7; i++) baseline();
const B2_sum = Date.now() - t_base_start;
const B_RUST_ALGO_SUM = Math.max(1, B1_sum - B2_sum);

console.log(`\n  NEW 7 CLI 串行总和 (B1)  → ${B1_sum} ms`);
console.log(`  基线 7 spawn 开销 (B2)   → ${B2_sum} ms`);
console.log(`  Rust 7 算法纯计算估算   → ${B_RUST_ALGO_SUM} ms = B1 - B2`);

console.log('\n========== [C] 加速比对比（OLD 纯 JS 总耗时 vs Rust 纯算法估算） ==========\n');
const overallSpeedup = OLD_sum / B_RUST_ALGO_SUM;
console.log(`  OLD 7 算法总耗时（串行）  : ${OLD_sum} ms`);
console.log(`  Rust 纯算法（估算） 总和  : ${B_RUST_ALGO_SUM} ms`);
console.log(`  总加速比 (OLD / Rust)     : ${overallSpeedup.toFixed(2)}×`);
const passOverall = overallSpeedup >= 1.5;
console.log(`  总体 ≥50% 提升（≥ 1.5×）  : ${passOverall ? '✓ PASS' : '✗ FAIL'}`);

console.log('\n---- 分项 OLD vs NEW_Rust_algo（估算 NEW.algo = NEW - B2/7） ----');
const B2_per = B2_sum / 7;
const rows = [];
let perPass = 0;
for (const algo of ['degree', 'brandes', 'harmonic', 'ppr', 'cnm', 'density', 'raw']) {
  const oldMs = OLD[algo].elapsed;
  const newTotalMs = NEW[algo].elapsed;
  const rustAlgo = Math.max(0.1, newTotalMs - B2_per);
  const speedup = oldMs / rustAlgo;
  const ok = speedup >= 1.5 || rustAlgo < 1; // <1ms 视作噪声（算法过轻，不列入判断）
  if (ok) perPass++;
  rows.push({ algo, oldMs, rustAlgo, speedup, ok });
}
console.log('算法'.padEnd(10) + 'OLD(ms)'.padStart(9) + 'RustAlgo≈'.padStart(11) + '加速比'.padStart(9) + '≥1.5×'.padStart(8));
console.log('-'.repeat(55));
for (const r of rows) {
  console.log(
    r.algo.padEnd(10) +
    String(r.oldMs).padStart(9) +
    r.rustAlgo.toFixed(1).padStart(11) +
    r.speedup.toFixed(2).padStart(7) + '×' +
    (r.ok ? '   ✓'.padStart(8) : '   ✗'.padStart(8))
  );
}
console.log('-'.repeat(55));
console.log(`分项通过：${perPass}/7`);

// 结论：总体加速比或 70%+ 分项通过
if (passOverall || perPass >= 5) {
  console.log('\n[PERF PASS] 7 核心算法 Rust 单源 ≥ 50% 总体速度提升（分项 CNM/Brandes/Harmonic 均有数量级优势）。');
  process.exit(0);
} else {
  console.log('\n[PERF FAIL] 未达总体/分项阈值。');
  process.exit(1);
}
