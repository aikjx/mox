'use strict';

/**
 * T3-C / T12 对账脚本：Rust 单源真相 CLI vs Node GraphFormulas
 * ==============================================================
 * 10 fixtures × 7 algorithms = 70 assertions，Δ ≤ 1e-6
 *
 * 运行：node test/test-algo-rust-node-diff.js
 * 前置：cargo build --release -p graph-algorithms --bin compare_with_node
 *
 * RED 阶段（Node 未重构前）：预期有失败（算法差异 / 迭代次数 / 展开策略）
 * GREEN 阶段（Node 重构后，核心循环移除，委托 Rust CLI）：70/70 通过
 */

const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const { GraphFormulas, expandRawEdges } = require('../src/graph/graph-formulas');

// 精度护栏：严禁修改
const PPR_D = 0.85;
const PPR_MAX_ITER = 30;
const TOL = 1e-6;

// ---------- 定位 Rust CLI ----------
// __dirname = platform/backend-node/test → 上 3 级到 workspace 根（Cargo.toml）
const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..');
const RELEASE_BIN = path.join(WORKSPACE_ROOT, 'target', 'release', 'compare_with_node.exe');
const DEBUG_BIN = path.join(WORKSPACE_ROOT, 'target', 'debug', 'compare_with_node.exe');

function resolveRustCli() {
  // 优先级：预编译 release → 预编译 debug → cargo run（fallback，spawn 慢）
  if (fs.existsSync(RELEASE_BIN)) return { kind: 'exe', path: RELEASE_BIN };
  if (fs.existsSync(DEBUG_BIN)) return { kind: 'exe', path: DEBUG_BIN };
  return { kind: 'cargo', path: null };
}

function callRust(name, payload) {
  const inputJson = JSON.stringify(payload);
  const cli = resolveRustCli();
  let cmd, args;
  if (cli.kind === 'exe') {
    cmd = cli.path;
    args = ['--name', name, '--input', '-', '--output', '-'];
  } else {
    cmd = 'cargo';
    args = [
      'run', '--release',
      '-p', 'graph-algorithms',
      '--bin', 'compare_with_node',
      '--manifest-path', path.join(WORKSPACE_ROOT, 'Cargo.toml'),
      '--', '--name', name, '--input', '-', '--output', '-',
    ];
  }
  const res = spawnSync(cmd, args, {
    input: inputJson,
    encoding: 'utf-8',
    maxBuffer: 50 * 1024 * 1024,
    cwd: WORKSPACE_ROOT,
  });
  if (res.error) {
    throw new Error(`Rust CLI spawn 失败 (${cmd}): ${res.error.message}`);
  }
  if (res.status !== 0) {
    throw new Error(
      `Rust CLI 非零退出 (name=${name}, code=${res.status}):\nSTDERR: ${res.stderr}\nSTDOUT: ${res.stdout}`
    );
  }
  try {
    return JSON.parse(res.stdout.trim() || '{}');
  } catch (e) {
    throw new Error(`Rust CLI 输出非 JSON (name=${name}): ${res.stdout.slice(0, 200)}`);
  }
}

// ---------- 工具 ----------
function N(ids) { return ids.map(id => ({ id })); }
function E(list) {
  return list.map(([s, t, w]) => {
    const e = { source: s, target: t };
    if (w !== undefined) e.weight = w;
    return e;
  });
}

function maxMapDiff(a, b, tol = TOL) {
  // 返回两个 map-like 对象之间的最大 |a_i - b_i|；若 id 集不一致也返回 Infinity（tol+1）
  const keysA = Object.keys(a || {});
  const keysB = Object.keys(b || {});
  if (keysA.length !== keysB.length) return Infinity;
  let max = 0;
  for (const k of keysA) {
    const va = Number(a[k]);
    const vb = Number(b[k]);
    if (!Number.isFinite(va) || !Number.isFinite(vb)) return Infinity;
    const d = Math.abs(va - vb);
    if (d > max) max = d;
  }
  return max;
}

// ---------- 计数器 ----------
let total = 0, passed = 0, failed = 0;
const cases = []; // for reporting

function register(label, ok, detail) {
  total++;
  if (ok) passed++;
  else {
    failed++;
    cases.push({ label, detail });
  }
}

// ---------- 10 fixtures ----------
const FIXTURES = [
  // F1：星型（无向 RAW）：c 连 4 叶
  {
    name: 'T1-Star',
    nodes: N(['c', 's1', 's2', 's3', 's4']),
    edges: E([['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']]),
  },
  // F2：有向链 a→b→c→d→e
  {
    name: 'T2-ChainDir',
    nodes: N(['a', 'b', 'c', 'd', 'e']),
    edges: E([['a', 'b'], ['b', 'c'], ['c', 'd'], ['d', 'e']]),
    directed: true, // brandes/harmonic/ppr 视为有向；degree/density 仍按 RAW 语义
  },
  // F3：双团+桥（CNM 标准测试图）
  {
    name: 'T3-TwoCliquesBridge',
    nodes: N(['a', 'b', 'c', 'd', 'e', 'f']),
    edges: E([
      ['a', 'b'], ['a', 'c'], ['b', 'c'],
      ['d', 'e'], ['d', 'f'], ['e', 'f'],
      ['b', 'd'],
    ]),
  },
  // F4：双环（有向 a↔b）
  {
    name: 'T4-DoubleRing',
    nodes: N(['a', 'b']),
    edges: E([['a', 'b'], ['b', 'a']]),
    directed: true,
  },
  // F5：孤立 3 节点
  {
    name: 'T5-Isolated',
    nodes: N(['x', 'y', 'z']),
    edges: [],
  },
  // F6：有向星（c→叶）
  {
    name: 'T6-DirStar',
    nodes: N(['c', 's1', 's2', 's3', 's4']),
    edges: E([['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']]),
    directed: true,
  },
  // F7：K4 完全图（无向 RAW）
  {
    name: 'T7-K4Complete',
    nodes: N(['w', 'x', 'y', 'z']),
    edges: E([
      ['w', 'x'], ['w', 'y'], ['w', 'z'],
      ['x', 'y'], ['x', 'z'],
      ['y', 'z'],
    ]),
  },
  // F8：8 节点双向环
  {
    name: 'T8-BidiRing8',
    nodes: N(['r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7', 'r8']),
    edges: E([
      ['r1', 'r2'], ['r2', 'r1'],
      ['r2', 'r3'], ['r3', 'r2'],
      ['r3', 'r4'], ['r4', 'r3'],
      ['r4', 'r5'], ['r5', 'r4'],
      ['r5', 'r6'], ['r6', 'r5'],
      ['r6', 'r7'], ['r7', 'r6'],
      ['r7', 'r8'], ['r8', 'r7'],
      ['r8', 'r1'], ['r1', 'r8'],
    ]),
    directed: true,
  },
  // F9：不连通（两独立边 RAW）
  {
    name: 'T9-Disconnected',
    nodes: N(['a', 'b', 'c', 'd']),
    edges: E([['a', 'b'], ['c', 'd']]),
  },
  // F10：加权图（权重非 1，验证加权传播）
  {
    name: 'T10-Weighted',
    nodes: N(['p', 'q', 'r', 's']),
    edges: E([
      ['p', 'q', 2.5],
      ['q', 'r', 1.2],
      ['r', 's', 3.0],
      ['p', 'r', 0.7],
    ]),
  },
];

// ---------- 7 算法断言 ----------
function runFixture(fixture) {
  const label = (algo) => `${fixture.name} × ${algo}`;

  // ===== 1. degree =====
  {
    const jsRes = GraphFormulas.degreeCentrality(fixture.nodes, fixture.edges, { expandRaw: true, legacyShape: false });
    // JS 侧 degreeCentrality 语义：RAW 边计次 / (N-1)。对应 Rust 传 directed=true
    // （禁止内部 RAW-expand，保证 in_degree + out_degree 恰好是 raw 边出现次数）。
    const rustRes = callRust('degree', { nodes: fixture.nodes, edges: fixture.edges, directed: true });
    const md = maxMapDiff(jsRes, rustRes);
    const ok = md <= TOL;
    register(label('degree'), ok, ok ? `maxΔ=${md.toExponential(2)}` : `maxΔ=${md} > ${TOL}`);
  }

  // ===== 2. brandes =====
  {
    const dirOpt = { directed: !!fixture.directed };
    const jsRes = GraphFormulas.betweennessCentrality(fixture.nodes, fixture.edges, dirOpt);
    const rustRes = callRust('brandes', {
      nodes: fixture.nodes, edges: fixture.edges, directed: fixture.directed || false,
    });
    const md = maxMapDiff(jsRes, rustRes);
    const ok = md <= TOL;
    register(label('brandes'), ok, ok ? `maxΔ=${md.toExponential(2)}` : `maxΔ=${md} > ${TOL}`);
  }

  // ===== 3. harmonic =====
  {
    const dirOpt = { directed: !!fixture.directed };
    const jsRes = GraphFormulas.closenessCentrality(fixture.nodes, fixture.edges, dirOpt);
    const rustRes = callRust('harmonic', {
      nodes: fixture.nodes, edges: fixture.edges, directed: fixture.directed || false,
    });
    const md = maxMapDiff(jsRes, rustRes);
    const ok = md <= TOL;
    register(label('harmonic'), ok, ok ? `maxΔ=${md.toExponential(2)}` : `maxΔ=${md} > ${TOL}`);
  }

  // ===== 4. ppr (personalizedPageRank, d=0.85, maxIter=30, 无个性化向量→均匀) =====
  {
    // 对齐 T3-B：GraphFormulas.personalizedPageRank(nodes, edges, {}, {d, maxIter})
    const jsRes = GraphFormulas.personalizedPageRank(
      fixture.nodes, fixture.edges, {}, { d: PPR_D, maxIter: PPR_MAX_ITER }
    );
    const rustRes = callRust('ppr', { nodes: fixture.nodes, edges: fixture.edges });
    const md = maxMapDiff(jsRes, rustRes);
    const ok = md <= TOL;
    register(label('ppr'), ok, ok ? `maxΔ=${md.toExponential(2)}` : `maxΔ=${md} > ${TOL}`);
  }

  // ===== 5. cnm (modularity + community count) =====
  {
    const jsRes = GraphFormulas.communityDetectionCNM(fixture.nodes, fixture.edges);
    const rustRes = callRust('cnm', { nodes: fixture.nodes, edges: fixture.edges });
    const modDiff = Math.abs(Number(jsRes.modularity) - Number(rustRes.modularity));
    const countMatch = jsRes.communities.length === rustRes.communities.length;
    // 为了「一个 case」聚合一个断言：模块化度 ≤ tol 并且 社区数一致 或 模块化度 tol 内（社区数是整数，不一致即整个 fail）
    const ok = modDiff <= TOL && countMatch;
    const detail = ok
      ? `modΔ=${modDiff.toExponential(2)}; commCount=${jsRes.communities.length}`
      : `modΔ=${modDiff}, countMatch=${countMatch} (js=${jsRes.communities.length}, rust=${rustRes.communities.length})`;
    register(label('cnm'), ok, detail);
  }

  // ===== 6. density =====
  {
    // JS：density(N, E) 使用 RAW 边去重后的 E
    const uniqueRawEdges = (() => {
      const set = new Set();
      for (const e of fixture.edges) {
        const [a, b] = [e.source, e.target].sort();
        set.add(`${a}|${b}`);
      }
      return set.size;
    })();
    const jsRes = GraphFormulas.density(fixture.nodes.length, uniqueRawEdges);
    const rustRes = callRust('density', {
      nodes: fixture.nodes,
      edges: fixture.edges,
      nodeCount: fixture.nodes.length,
      edgeCount: uniqueRawEdges,
    });
    const diff = Math.abs(Number(jsRes.value) - Number(rustRes.value));
    const sameFormula = jsRes.formula === rustRes.formula;
    const ok = diff <= TOL && sameFormula;
    register(label('density'), ok,
      ok ? `Δ=${diff.toExponential(2)}; formula=ok`
         : `Δ=${diff}; formulaMatch=${sameFormula}`);
  }

  // ===== 7. raw_expand =====
  {
    const jsArr = expandRawEdges(fixture.edges, { directed: false });
    const rustArr = callRust('raw_expand', { nodes: fixture.nodes, edges: fixture.edges });
    // 对比：(a) 展开后边数一致；(b) 每对 (s,t) 权重累加一致（忽略顺序）
    let countOk = Array.isArray(jsArr) && Array.isArray(rustArr) && jsArr.length === rustArr.length;
    const sumKey = (a, b) => a < b ? `${a}|${b}` : `${b}|${a}`;
    const mapJS = new Map(), mapRust = new Map();
    if (Array.isArray(jsArr)) for (const e of jsArr) {
      const k = sumKey(e.source, e.target);
      mapJS.set(k, (mapJS.get(k) || 0) + Number(e.weight || 1));
    }
    if (Array.isArray(rustArr)) for (const e of rustArr) {
      const k = sumKey(e.source, e.target);
      mapRust.set(k, (mapRust.get(k) || 0) + Number(e.weight || 1));
    }
    let maxWtDiff = 0;
    if (countOk) {
      for (const [k, v] of mapJS) {
        const r = mapRust.get(k) || 0;
        maxWtDiff = Math.max(maxWtDiff, Math.abs(v - r));
      }
      for (const [k, v] of mapRust) {
        if (!mapJS.has(k)) { maxWtDiff = Infinity; break; }
      }
    }
    const ok = countOk && maxWtDiff <= TOL;
    register(label('raw_expand'), ok,
      ok ? `len=${jsArr.length}; maxWtΔ=${maxWtDiff.toExponential(2)}`
         : `countOk=${countOk} (js=${jsArr.length}, rust=${rustArr ? rustArr.length : 'n/a'}); maxWtΔ=${maxWtDiff}`);
  }
}

// ---------- main ----------
function main() {
  console.log('='.repeat(92));
  console.log('T3-C / T12 对账：Rust CLI 单源真相 vs Node GraphFormulas（10 fixtures × 7 algos = 70 cases）');
  console.log(`TOL = ${TOL}；PPR_D=${PPR_D}；PPR_MAX_ITER=${PPR_MAX_ITER}`);
  console.log('Rust CLI:', resolveRustCli().kind === 'exe'
    ? (resolveRustCli().path)
    : 'cargo run --release fallback');
  console.log('='.repeat(92));

  for (const fix of FIXTURES) {
    console.log(`\n[fixture] ${fix.name} (N=${fix.nodes.length}, E=${fix.edges.length}, directed=${!!fix.directed})`);
    try {
      runFixture(fix);
    } catch (e) {
      console.error(`  ⚠ fixture ${fix.name} 异常中断: ${e.message}`);
    }
  }

  console.log('\n' + '-'.repeat(92));
  console.log('失败清单：');
  if (cases.length === 0) console.log('  （无）');
  else cases.forEach((c, i) => console.log(`  ${i + 1}. [${c.label}] → ${c.detail}`));
  console.log('-'.repeat(92));
  console.log(`对账总计：${passed} 通过 / ${failed} 失败 / ${total} 断言（要求 70/70 GREEN）`);
  console.log('='.repeat(92));

  if (total === 70 && failed === 0) {
    console.log('\n[T3-C / T12 GREEN] 70/70 通过，Δ≤1e-6，Node↔Rust 数值完全对齐。');
    process.exit(0);
  } else {
    if (total !== 70) {
      console.error(`[约束违反] 断言数量错误：要求 70，实际 ${total}。`);
    }
    if (failed !== 0) {
      console.error(`[RED / 未对齐] 存在 ${failed} 项失败，见上方清单（若尚未实施 T3-B，属预期 RED 阶段）。`);
    }
    process.exit(1);
  }
}

main();
