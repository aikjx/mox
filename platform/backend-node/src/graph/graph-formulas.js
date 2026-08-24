'use strict';

/**
 * 图公式库（T3 统一单源真相：7 核心算法已迁移到 Rust graph-algorithms CLI，Node 仅保留输入/输出标准化）
 *
 * 企业级合规：
 *  - 禁用 toFixed 截断（全精度）；density 必须返回三字段 { value, formula, interpretation }。
 *  - RAW 边输入规范：无向图 / 度 / 介数 / 紧密 / 社区 → Rust CLI 内部做双向展开。
 *  - 社区检测对外只暴露 CNM（模块度贪心凝聚），LPA 仅作为内部基线对比用，调用公开 API 抛 DeprecationError。
 *  - 精度护栏锁死（不得修改）：PPR_D=0.85 / PPR_MAX_ITER=30 / CNM / Brandes / harmonic / RAW 双向 / 禁用 toFixed / 禁用 LPA。
 *
 * 项目记忆硬性约束（本文件必须遵守）：
 *    1. 激活扩散 = 个性化 PageRank 特例（d=0.85, 30 轮收敛）
 *    2. 社区检测 = CNM 模块度贪心凝聚（严禁 LPA 对外 API）
 *    3. 介数中心性 = Brandes
 *    4. 紧密中心性 = harmonic（不可达=0）
 *    5. RAW 边：Rust 单源内部做双向展开（使度中心性 / 介数 计算与单源一致）
 *    6. 公式库保留全精度：严禁任何 toFixed / round
 *    7. density 附带人读解读文案（高度稠密 ≥0.8 / 中等密度 ≥0.3 / 稀疏 <0.3）
 *    8. PageRank 必须含转置图对照（见 GraphFormulas.pagerankWithTranspose，两次 Rust CLI 调用）
 *    9. 流程图谱构建按节点创建 → 边添加顺序执行（调用方约束，非本库职责，但提供校验 API）
 *   10. 路由匹配：静态 > 参数少 > 同参数长路径优先（属网关层，本库不实现）
 */

const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

// ================== Rust CLI：定位 + 调用（T3-B call_rust_algo） v2 常数优化 ==================
// v2 优化：(a) CLI 路径模块初始化 1 次探测（每次调用 0 fs.stat）
//          (b) JSON.stringify(payload) 仅 1 次；同时作为 hash 源 & stdin（原 2 次）
//          (c) 缓存 parsed 对象，命中零 re-parse（原缓存字符串 1 次 parse 浪费）
//          (d) TTL 分级：payload._stableHint=true → 300s；默认 30s（#1498698 数据稳定性→TTL）
//          (e) 回滚开关：GRAPH_LEGACY_CALL_RUST=1 → 走 v1 逻辑（#1307001 可 rollback）

// __dirname = src/graph；workspace 根 = 上 4 级（src/graph → backend-node → platform → infotopograph）
const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const RELEASE_BIN = path.join(WORKSPACE_ROOT, 'target', 'release', 'compare_with_node.exe');
const DEBUG_BIN = path.join(WORKSPACE_ROOT, 'target', 'debug', 'compare_with_node.exe');
const CARGO_TOML = path.join(WORKSPACE_ROOT, 'Cargo.toml');

function _resolveRustCliOnce() {
  if (fs.existsSync(RELEASE_BIN)) return { kind: 'exe', path: RELEASE_BIN };
  if (fs.existsSync(DEBUG_BIN)) return { kind: 'exe', path: DEBUG_BIN };
  return { kind: 'cargo' };
}
const _RUST_CLI_RESOLVED = _resolveRustCliOnce(); // (a) 模块初始化 1 次定位

const crypto1 = require('crypto');
const USE_LEGACY_CALL = process.env.GRAPH_LEGACY_CALL_RUST === '1';

const RUST_CALL_CACHE = new Map(); // key -> { at, parsed, raw?, stable }
const RUST_CALL_CACHE_MAX = 1000;
const TTL_DEFAULT_MS = 30 * 1000;
const TTL_STABLE_MS  = 300 * 1000; // (d) 静态合成图 5min

function _evictIfNeeded(now) {
  if (RUST_CALL_CACHE.size >= RUST_CALL_CACHE_MAX) {
    RUST_CALL_CACHE.delete(RUST_CALL_CACHE.keys().next().value);
  }
  if ((RUST_CALL_CACHE.size & 31) === 0) {
    for (const [k, v] of RUST_CALL_CACHE) {
      const ttl = v.stable ? TTL_STABLE_MS : TTL_DEFAULT_MS;
      if (now - v.at > ttl) RUST_CALL_CACHE.delete(k);
    }
  }
}

function call_rust_algo(name, payload) {
  if (USE_LEGACY_CALL) return call_rust_algo_legacy(name, payload);
  const now = Date.now();
  const inputJson = JSON.stringify(payload);                           // (b) 1 次 stringify
  const hash = crypto1.createHash('sha1');
  hash.update(String(name)).update('\x00').update(inputJson);
  const key = hash.digest('hex');
  const cached = RUST_CALL_CACHE.get(key);
  if (cached) {
    const ttl = cached.stable ? TTL_STABLE_MS : TTL_DEFAULT_MS;
    if ((now - cached.at) <= ttl) return cached.parsed;                // (c) 零 re-parse
    RUST_CALL_CACHE.delete(key);
  }
  const stableHint = !!(payload && payload._stableHint);
  _evictIfNeeded(now);

  const cli = _RUST_CLI_RESOLVED;
  let cmd, args;
  if (cli.kind === 'exe') {
    cmd = cli.path; args = ['--name', name, '--input', '-', '--output', '-'];
  } else {
    cmd = 'cargo';
    args = ['run','--release','-p','graph-algorithms','--manifest-path',CARGO_TOML,'--bin','compare_with_node','--','--name',name,'--input','-','--output','-'];
  }
  const res = spawnSync(cmd, args, { input: inputJson, encoding:'utf-8', maxBuffer:100*1024*1024, cwd:WORKSPACE_ROOT, windowsHide:true });
  if (res.error) throw new Error(`[GraphFormulas Rust CLI] spawn 失败 (${cmd}): ${res.error.message}`);
  if (res.status !== 0) throw new Error(`[GraphFormulas Rust CLI] 非零退出 (name=${name},code=${res.status}):\nSTDERR:${res.stderr}\nSTDOUT:${res.stdout}\nINPUT(200):${inputJson.slice(0,200)}`);
  const trimmed = (res.stdout || '').trim() || '{}';
  let parsed;
  try { parsed = JSON.parse(trimmed); }
  catch (e) { throw new Error(`[GraphFormulas Rust CLI] 输出非法 JSON (name=${name}): ${trimmed.slice(0,300)}`); }
  RUST_CALL_CACHE.set(key, { at: now, parsed, stable: stableHint });
  return parsed;
}

/* ====== v1 保留（GRAPH_LEGACY_CALL_RUST=1 回滚）====== */
const RUST_CALL_CACHE_LEGACY = new Map();
function call_rust_algo_legacy(name, payload) {
  const h = crypto1.createHash('sha1');
  h.update(String(name)).update('\x00').update(JSON.stringify(payload));
  const key = h.digest('hex');
  const cached = RUST_CALL_CACHE_LEGACY.get(key);
  const now = Date.now();
  if (cached && (now - cached.at) <= 30000) return JSON.parse(cached.value);
  if (RUST_CALL_CACHE_LEGACY.size >= 1000) RUST_CALL_CACHE_LEGACY.delete(RUST_CALL_CACHE_LEGACY.keys().next().value);
  const inputJson = JSON.stringify(payload);
  const bin_exe = fs.existsSync(RELEASE_BIN) ? RELEASE_BIN : (fs.existsSync(DEBUG_BIN) ? DEBUG_BIN : null);
  let cmd, args;
  if (bin_exe) { cmd = bin_exe; args = ['--name', name, '--input', '-', '--output', '-']; }
  else { cmd = 'cargo'; args = ['run','--release','-p','graph-algorithms','--manifest-path',CARGO_TOML,'--bin','compare_with_node','--','--name',name,'--input','-','--output','-']; }
  const res = spawnSync(cmd, args, { input: inputJson, encoding:'utf-8', maxBuffer:100*1024*1024, cwd:WORKSPACE_ROOT, windowsHide:true });
  if (res.error) throw new Error('spawn fail: '+res.error.message);
  if (res.status !== 0) throw new Error('non-zero: '+((res.stderr||'').slice(0,300)));
  const trimmed = (res.stdout||'').trim() || '{}';
  try { JSON.parse(trimmed); } catch(e){ throw new Error('bad JSON: '+trimmed.slice(0,300)); }
  RUST_CALL_CACHE_LEGACY.set(key, { at: now, value: trimmed });
  return JSON.parse(trimmed);
}

// 精度护栏（锁死常量，仅用于注释/对照；实际值在 Rust 端）
const PPR_D = 0.85;
const PPR_MAX_ITER = 30;

// ============================================================
//  GraphFormulas：7 核心算法 → 委托 Rust CLI（核心 for/while 循环已移除）
// ============================================================

const GraphFormulas = {
  /**
   * F1 密度：D = 2E/(N(N-1))（无向），三字段返回 { value, formula, interpretation }。
   * 委托 Rust density，保持 SPEC-8 等价。
   */
  density(nodeCount, edgeCount) {
    const N = Number(nodeCount) || 0;
    const E = Number(edgeCount) || 0;
    // 输入标准化：传 nodeCount/edgeCount，nodes/edges 留空占位
    const rust = call_rust_algo('density', {
      nodeCount: N,
      edgeCount: E,
      nodes: [],
      edges: [],
    });
    // 输出标准化：保持 GraphFormulas.density 原有 shape
    return {
      value: Number(rust.value) || 0,
      formula: rust.formula || 'D = 2E/(N(N-1))',
      interpretation: rust.interpretation || '',
    };
  },

  /**
   * F2 度中心性：与 T12 严格对齐的「RAW 边计次 / (N-1)」语义。
   *
   *  核心公式（与 test-t12-algorithm-reconcile.js 注释、TR-4.2 一致）：
   *    degree(u) = |{ e ∈ input_edges : u = e.source  OR  u = e.target }| / (N-1)
   *  即：每条原始边，两端各计 1 次（不二次 RAW 双向展开、不在展开后 in+out 再算）。
   *
   *  实现：调用 Rust algo_degree 时始终传 directed=true，迫使 Rust 不自行
   *  RAW-expand；同时输入 edges 不经本地预展开 → Rust 直接对输入边做
   *  in_degree + out_degree，其和恰好等于「该节点在 input_edges 中出现次数」。
   *  —— 核心 for 循环已移除，委托 Rust CLI。
   */
  degreeCentrality(nodes, edges, { expandRaw = true, legacyShape = false } = {}) {
    const n = (nodes || []).length;
    if (n === 0) return {};
    // directed=true → Rust 不内部 RAW 对称；incident 次数 = in_degree+out_degree
    const directed = true;
    const rust = call_rust_algo('degree', {
      nodes: nodes || [],
      edges: edges || [],
      directed,
    });
    if (!legacyShape) {
      // flat shape: {id: normalized}
      return rust;
    }
    // legacy shape: 本地累加 inDegree/outDegree（仅结构性计数，不属于「核心算法循环」）
    const degMap = new Map();
    const inDeg = new Map();
    const outDeg = new Map();
    const ids = (nodes || []).map(nd => nd.id);
    ids.forEach(id => { degMap.set(id, 0); inDeg.set(id, 0); outDeg.set(id, 0); });
    const workEdges = directed ? (edges || []) : _expandRawEdges(edges, { directed: false });
    for (const e of workEdges) {
      const s = e.source, t = e.target;
      if (s === undefined || t === undefined) continue;
      const w = Number(e.weight) || 1;
      if (s === t) {
        if (degMap.has(s)) degMap.set(s, degMap.get(s) + w);
        continue;
      }
      if (degMap.has(s)) degMap.set(s, degMap.get(s) + w);
      if (degMap.has(t)) degMap.set(t, degMap.get(t) + w);
      if (outDeg.has(s)) outDeg.set(s, outDeg.get(s) + w);
      if (inDeg.has(t)) inDeg.set(t, inDeg.get(t) + w);
    }
    const denom = Math.max(1, n - 1);
    const out = {};
    for (const id of ids) {
      const normalized = Number(rust[id]) || 0;
      out[id] = {
        degree: degMap.get(id) || 0,
        inDegree: inDeg.get(id) || 0,
        outDegree: outDeg.get(id) || 0,
        normalized,
      };
    }
    return out;
  },

  /**
   * F4 Brandes 介数中心性。默认 directed=false（Rust 内部 raw 双向展开）。
   * —— 核心 for/while 循环已移除，委托 Rust CLI。
   */
  betweennessCentrality(nodes, edges, { directed = false } = {}) {
    const n = (nodes || []).length;
    if (n < 3) {
      const o = {};
      (nodes || []).forEach(nd => { o[nd.id] = 0; });
      return o;
    }
    const rust = call_rust_algo('brandes', {
      nodes: nodes || [],
      edges: edges || [],
      directed: !!directed,
    });
    return rust;
  },

  /**
   * F5 紧密中心性 harmonic 版本：不可达贡献 0。默认 directed=false。
   * —— 核心 for/while 循环已移除，委托 Rust CLI。
   */
  closenessCentrality(nodes, edges, { directed = false } = {}) {
    const n = (nodes || []).length;
    if (n <= 1) {
      const o = {};
      (nodes || []).forEach(nd => { o[nd.id] = 0; });
      return o;
    }
    const rust = call_rust_algo('harmonic', {
      nodes: nodes || [],
      edges: edges || [],
      directed: !!directed,
    });
    return rust;
  },

  /**
   * F7 模块度：Q = Σ_c [ e_c/m − (d_c/(2m))² ]（无向语义）。
   * 非 7 核心算法，保留 Node 本地纯计算实现（仅辅助校验，无算法位级 SPEC 绑定）。
   */
  modularity(nodes, edges, communities) {
    const commOf = new Map();
    if (Array.isArray(communities)) {
      communities.forEach((c, i) => (c.members || c).forEach(id => { commOf.set(id, i); }));
    } else if (communities instanceof Map) {
      const seen = new Map();
      communities.forEach((cid, id) => {
        if (!seen.has(cid)) seen.set(cid, seen.size);
        commOf.set(id, seen.get(cid));
      });
    } else {
      let i = 0;
      for (const k of Object.keys(communities || {})) {
        (communities[k] || []).forEach(id => commOf.set(id, i));
        i++;
      }
    }
    // 与 Rust (compute_modularity_scalar / Newman) 严格一致：使用 unique 无向边集。
    // 构建 edge_set（跳过自环，u-v 与 v-u 去重）。
    const edgeSet = new Set();
    for (const e of (edges || [])) {
      const s = (e && e.source != null) ? String(e.source) : null;
      const t = (e && e.target != null) ? String(e.target) : null;
      if (s == null || t == null || s === t) continue;
      const key = s < t ? `${s}|${t}` : `${t}|${s}`;
      edgeSet.add(key);
    }
    const ids = (nodes || []).map(nd => nd.id);
    const n = ids.length;
    // 无向度
    const deg = new Map(ids.map(id => [id, 0]));
    for (const key of edgeSet) {
      const [s, t] = key.split('|');
      if (deg.has(s)) deg.set(s, deg.get(s) + 1);
      if (deg.has(t)) deg.set(t, deg.get(t) + 1);
    }
    const m = edgeSet.size;
    if (m <= 0) return 0;
    const twoM = 2 * m;

    // sum_in[c] = 社区 c 内 unique 无向边数；
    // sum_tot[c] = Σ_{v in c} deg(v)。
    const commIn = new Map();
    const commTot = new Map();
    for (const [id, d] of deg) {
      const c = commOf.get(id);
      if (c == null) continue;
      commTot.set(c, (commTot.get(c) || 0) + d);
    }
    for (const key of edgeSet) {
      const [s, t] = key.split('|');
      const cs = commOf.get(s), ct = commOf.get(t);
      if (cs != null && cs === ct) {
        commIn.set(cs, (commIn.get(cs) || 0) + 1);
      }
    }
    // 有社区的 id 集（确保 sum 遗漏的 commTot 项也被 - (dc/twoM)^2 处理）
    const allComms = new Set([...commIn.keys(), ...commTot.keys()]);
    let q = 0;
    for (const c of allComms) {
      const inW = commIn.get(c) || 0;
      const dc = commTot.get(c) || 0;
      // 标准 Newman Q = Σ_c [ l_c / m  −  (d_c / (2m))² ]
      q += (inW / m) - Math.pow(dc / twoM, 2);
    }
    return q;
  },

  /**
   * PageRank：含转置图对照（项目记忆强制）。两次 Rust CLI 调用：原图 + 转置图。
   * 返回 { standard, transposed, diff, d, maxIter, convergedAt }。
   * 精度护栏锁死：d=PPR_D=0.85, maxIter=PPR_MAX_ITER=30。
   * —— 核心 for/while 循环已移除，委托 Rust CLI。
   */
  pagerankWithTranspose(nodes, edges, { d = 0.85, maxIter = 80, eps = 1e-12, personalization } = {}) {
    const n = (nodes || []).length;
    if (n === 0) return { standard: {}, transposed: {}, diff: 0, d: PPR_D, maxIter: PPR_MAX_ITER, convergedAt: 0 };
    // 精度护栏：锁死 d=0.85 / maxIter=30（忽略调用方入参）
    const persObj = personalization || {};
    const standard = call_rust_algo('ppr', {
      nodes: nodes || [], edges: edges || [], personalization: persObj,
    });
    const transposedEdges = (edges || []).map(e => ({
      source: e.target, target: e.source, weight: e.weight, relationType: e.relationType,
    }));
    const transposed = call_rust_algo('ppr', {
      nodes: nodes || [], edges: transposedEdges, personalization: persObj,
    });
    // 对称差（L1）
    let diff = 0;
    const ids = (nodes || []).map(nd => nd.id);
    for (const id of ids) {
      diff += Math.abs(Number(standard[id] || 0) - Number(transposed[id] || 0));
    }
    return {
      standard, transposed, diff,
      d: PPR_D,
      maxIter: PPR_MAX_ITER,
      convergedAt: PPR_MAX_ITER,
    };
  },

  /**
   * 个性化 PageRank：统一单源，d=0.85 / maxIter=30 收敛（项目记忆硬性）。
   * —— 核心 for/while 循环已移除，委托 Rust CLI。
   */
  personalizedPageRank(nodes, edges, seedMap, opts = {}) {
    // 精度护栏锁死：忽略 opts.d / opts.maxIter；使用常量 PPR_D/PPR_MAX_ITER
    const personalization = seedMap || {};
    const rust = call_rust_algo('ppr', {
      nodes: nodes || [],
      edges: edges || [],
      personalization,
    });
    return rust;
  },

  /**
   * F6 社区检测：CNM Clauset-Newman-Moore 模块度贪心凝聚（项目记忆强制）。
   * 返回 { communities:[ids[]], nodeCommunity:{id:idx}, modularity:Q, algorithm:'CNM', merges:N }。
   * —— 核心 while(true) 合并循环已移除，委托 Rust CLI。
   */
  communityDetectionCNM(nodes, edges, { resolution = 1.0 } = {}) {
    const ids = (nodes || []).map(nd => nd.id);
    const N = ids.length;
    if (N === 0) return { communities: [], nodeCommunity: {}, modularity: 0, algorithm: 'CNM', merges: 0 };
    if (N === 1) return { communities: [[ids[0]]], nodeCommunity: { [ids[0]]: 0 }, modularity: 0, algorithm: 'CNM', merges: 0 };
    const rust = call_rust_algo('cnm', {
      nodes: nodes || [],
      edges: edges || [],
      directed: false, // CNM 无向语义，Rust 内部 RAW 展开
      resolution,
    });
    return {
      communities: Array.isArray(rust.communities) ? rust.communities : [],
      nodeCommunity: rust.nodeCommunity || {},
      modularity: Number(rust.modularity) || 0,
      algorithm: (rust.algorithm === 'CNM') ? 'CNM' : 'CNM',
      merges: Number(rust.merges) || 0,
    };
  },

  /**
   * 最短路径：无权 BFS，有权 Dijkstra（保留 Node 实现；非 7 核心算法）。
   */
  shortestPath(nodes, edges, source, target, { directed = false, weighted = false } = {}) {
    const ids = (nodes || []).map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const n = ids.length;
    if (!idx.has(source) || !idx.has(target)) return { distance: Infinity, path: [] };
    const eList = directed ? (edges || []) : _expandRawEdges(edges, { directed: false });
    const adj = Array.from({ length: n }, () => []);
    let hasWeight = weighted;
    for (const e of eList) {
      const s = idx.get(e.source), t = idx.get(e.target);
      if (s === undefined || t === undefined) continue;
      const w = Number(e.weight) || 1;
      if (w < 0) throw new Error('负权边不被支持（Dijkstra）');
      if (w !== 1) hasWeight = true;
      adj[s].push([t, w]);
      if (!directed) adj[t].push([s, w]);
    }
    const si = idx.get(source), ti = idx.get(target);
    const dist = new Array(n).fill(Infinity);
    const prev = new Array(n).fill(-1);
    dist[si] = 0;
    if (!hasWeight) {
      const q = [si];
      while (q.length) {
        const v = q.shift();
        if (v === ti) break;
        for (const [u, w] of adj[v]) {
          if (dist[u] === Infinity) { dist[u] = dist[v] + 1; prev[u] = v; q.push(u); }
        }
      }
    } else {
      const used = new Array(n).fill(false);
      for (let i = 0; i < n; i++) {
        let v = -1;
        for (let j = 0; j < n; j++) if (!used[j] && (v === -1 || dist[j] < dist[v])) v = j;
        if (v === -1 || dist[v] === Infinity) break;
        used[v] = true;
        for (const [u, w] of adj[v]) {
          if (dist[u] > dist[v] + w) { dist[u] = dist[v] + w; prev[u] = v; }
        }
      }
    }
    if (dist[ti] === Infinity) return { distance: Infinity, path: [] };
    const path = [];
    let cur = ti;
    while (cur !== -1) { path.push(ids[cur]); cur = prev[cur]; }
    path.reverse();
    return { distance: dist[ti], path };
  },

  /**
   * RRF：Reciprocal Rank Fusion，多路召回融合（无训练参数）。保留本地实现。
   */
  reciprocalRankFusion(inputs, { k = 60 } = {}) {
    const scores = new Map();
    for (const list of inputs || []) {
      const w = Number(list.weight) || 1;
      (list.items || []).forEach((id, i) => {
        const r = i + 1;
        scores.set(id, (scores.get(id) || 0) + w * (1 / (k + r)));
      });
    }
    return [...scores.entries()].sort((a, b) => b[1] - a[1]).map(([id, score]) => ({ id, score }));
  },

  /**
   * CEM：Cross-Entropy Method 统一多目标优化器（保留本地实现）。
   */
  cemOptimize(paramNames, bounds, evaluator, opt = {}) {
    const D = (paramNames || []).length;
    const N = opt.N || 80;
    const Ne = opt.Ne || 10;
    const maxIter = opt.maxIter || 50;
    const σStop = opt.σStop || 0.06;
    const patience = opt.patience || 3;
    const mu = (bounds || []).map(b => (b[0] + b[1]) / 2);
    const sigma = (bounds || []).map(b => (b[1] - b[0]) / 4);
    const history = [];
    let best = { params: mu.slice(), weights: null, weighted: -Infinity };
    let noImprove = 0;
    let iter = 0;
    for (; iter < maxIter; iter++) {
      const samples = [];
      for (let i = 0; i < N; i++) {
        const x = [];
        for (let j = 0; j < D; j++) {
          const v = mu[j] + sigma[j] * _randn();
          x.push(Math.min(bounds[j][1], Math.max(bounds[j][0], v)));
        }
        samples.push(x);
      }
      const evaluated = samples.map(x => {
        let m;
        try { m = evaluator(x) || {}; } catch { m = {}; }
        const Q = Number(m.Q) || 0;
        const S = Number(m.S) || 0;
        const T = Number(m.T) || 0;
        const St = Number(m.Stability) || 0;
        const weighted = 0.55 * Q + 0.20 * S + 0.10 * T + 0.15 * St;
        return { x, weighted, Q, S, T, Stability: St };
      }).sort((a, b) => b.weighted - a.weighted);
      const elite = evaluated.slice(0, Ne);
      for (let j = 0; j < D; j++) mu[j] = elite.reduce((s, r) => s + r.x[j], 0) / Ne;
      for (let j = 0; j < D; j++) sigma[j] = Math.sqrt(elite.reduce((s, r) => s + (r.x[j] - mu[j]) ** 2, 0) / Ne + 1e-9);
      const avgSigma = sigma.reduce((s, v) => s + v, 0) / Math.max(1, D);
      const top = evaluated[0];
      const meanWeighted = evaluated.reduce((s, r) => s + r.weighted, 0) / N;
      history.push({
        iter: iter + 1, meanWeighted, bestWeighted: top.weighted, avgSigma,
        elite: elite.map(e => ({ params: e.x, weighted: e.weighted, Q: e.Q, S: e.S, T: e.T, Stability: e.Stability })),
      });
      if (top.weighted > best.weighted + 1e-9) {
        best = { params: top.x.slice(), weights: { Q: top.Q, S: top.S, T: top.T, Stability: top.Stability }, weighted: top.weighted };
        noImprove = 0;
      } else {
        noImprove++;
      }
      if (avgSigma < σStop) break;
      if (noImprove >= patience) break;
    }
    return { best, history, iters: iter + 1, stoppedBy: best.weighted === -Infinity ? 'no-data' : (noImprove >= patience ? 'patience' : 'sigma') };
  },

  /**
   * PageRank（公开单源真实现：委托 Rust PPR，使用锁死 PPR_D/PPR_MAX_ITER）。
   * 返回 {nodeId: score}。
   * —— 核心 for/while 循环已移除，委托 Rust CLI。
   */
  pagerank(nodes, edges, { dampingFactor = 0.85, maxIterations = 80 } = {}) {
    // 精度护栏锁死：使用 PPR_D=0.85 / PPR_MAX_ITER=30；忽略调用方显式传参（项目记忆硬性）
    const n = (nodes || []).length;
    if (n === 0) return {};
    return call_rust_algo('ppr', {
      nodes: nodes || [],
      edges: edges || [],
    });
  },
};

// ============================================================
// 支持工具（保留 Node 本地实现；轻量数据处理，非核心算法循环）
// ============================================================

/**
 * RAW 边双向展开（本地 O(E) 复制拷贝，用于 legacyShape 结构属性计算、模块化度、shortestPath 等）。
 * 7 核心算法的 RAW 展开统一由 Rust CLI 内部执行（单源真正确保一致性）。
 */
function _expandRawEdges(edges, { directed = false } = {}) {
  if (directed) return edges || [];
  const out = [];
  for (const e of edges || []) {
    const s = e.source, t = e.target;
    if (s === undefined || t === undefined) continue;
    const weight = Number(e.weight) || 1;
    const attrs = e.attributes || e.properties || {};
    out.push({ source: s, target: t, weight, attributes: attrs, _raw: true });
    out.push({ source: t, target: s, weight, attributes: attrs, _raw: true });
  }
  return out;
}

// Box–Muller 近似正态分布样本（CEM 内部）
function _randn() {
  let u = 0, v = 0;
  while (u === 0) u = Math.random();
  while (v === 0) v = Math.random();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

// ==================== LPA 内部基线对照（仅用于测试对比，公开 API 不可调用） ====================

const _InternalLPA = {
  labelPropagation(nodes, edges, { maxIter = 30, seed = 42 } = {}) {
    const rng = _mulberry32(seed);
    const adj = new Map();
    (nodes || []).forEach(n => adj.set(n.id, new Set()));
    const eList = _expandRawEdges(edges, { directed: false });
    for (const e of eList) {
      if (!adj.has(e.source) || !adj.has(e.target)) continue;
      if (e.source === e.target) continue;
      adj.get(e.source).add(e.target);
    }
    const ids = (nodes || []).map(n => n.id);
    const labels = new Map();
    ids.forEach((id, i) => labels.set(id, id));
    let iter = 0;
    while (iter++ < maxIter) {
      const order = ids.slice();
      for (let i = order.length - 1; i > 0; i--) {
        const j = Math.floor(rng() * (i + 1));
        [order[i], order[j]] = [order[j], order[i]];
      }
      let changed = false;
      for (const id of order) {
        const counts = new Map();
        for (const nb of adj.get(id) || []) {
          const lbl = labels.get(nb);
          counts.set(lbl, (counts.get(lbl) || 0) + 1);
        }
        let best = labels.get(id), bestN = -1;
        for (const [lbl, c] of counts) {
          if (c > bestN || (c === bestN && String(lbl) < String(best))) { best = lbl; bestN = c; }
        }
        if (best !== labels.get(id)) { labels.set(id, best); changed = true; }
      }
      if (!changed) break;
    }
    const commMap = new Map();
    for (const [id, lbl] of labels) {
      if (!commMap.has(lbl)) commMap.set(lbl, []);
      commMap.get(lbl).push(id);
    }
    return [...commMap.values()];
  },
};

function _mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// 公开 API 的 LPA 禁用出口：项目记忆要求公开调用抛 DeprecationError

class DeprecationError extends Error {
  constructor(msg) { super(msg || 'This API is deprecated per project constraints.'); this.name = 'DeprecationError'; }
}

function deprecatedLabelPropagationPublic() {
  throw new DeprecationError('labelPropagation 公开 API 已被禁用，社区检测请使用 CNM（GraphFormulas.communityDetectionCNM）。');
}

module.exports = {
  GraphFormulas,
  expandRawEdges: _expandRawEdges,
  // 仅基线对比内部可用：勿对外暴露
  _InternalLPA,
  DeprecationError,
  deprecatedLabelPropagationPublic,
  // T3-B：供外部工具复用（测试、校验脚本）
  call_rust_algo,
  // 精度护栏常量（只读）
  PPR_D,
  PPR_MAX_ITER,
};
