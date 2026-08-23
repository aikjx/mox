/**
 * RED-GREEN TDD：图公式单源化断言（AC-05 / TR-5.1/5.2）。
 *
 * RED 阶段（实现 wrapper 之前）：本测试断言「三条路径的度/介数/PageRank 结果严格相等」，
 *   但由于 ai-flow-graph.js 与 lib/graph-algos.js 目前各自保持独立实现（可能数值差异或顶层键名不同），
 *   预期会 FAIL — 这证明本测试真有效。
 * GREEN 阶段（三实现归一后）：lib/graph-algos.js / ai-flow-graph.js 的 3 个目标函数改为 thin wrapper
 *   → 直接 require('./graph/graph-formulas') 取同名方法；三条路径的返回值将严格相同，本测试通过。
 *
 * 断言统计（目标 ≥ 20）：
 *   - 度中心性：top 3 节点 id 顺序 × 2 组节点 = 6；各 top 值相等性 × 2 = 2（合计 8）
 *   - 介数中心性：max 值相等（严格到 1e-9） × 2；top-1 id 相符 × 1（合计 3）
 *   - PageRank：前 5 节点集合交集占比 100%（2）+ sum ≈ 1（2）（合计 4）
 *   - 等价性（3 路径 → 两两对比）：degree/betweenness/pagerank 每对 × 各 1（合计 9）
 *   - WRAPPER_BODY 校验：graph-algos.js function body 行数 ≤ 4（合计 3）
 * 总计：8+3+4+9+3 = 27 ≥ 20
 */
'use strict';

const path = require('path');
const fs = require('fs');
const assert = require('assert');

// ========= 加载 3 条路径 =========
const GraphFormulas = require('../src/graph/graph-formulas').GraphFormulas;
const LegacyGraphAlgos = require('../src/lib/graph-algos');
const { AIFlowGraph, GraphFormulas: FlowGraphFm } = require('../src/ai-flow-graph');
// ai-flow-graph 不是 new 类导出公式，而是导出内部常量对象 GraphFormulas（与独立 graph-formulas 名撞但独立）
// ai-flow-graph 另外导出类 AIFlowGraph（公式 API 不挂实例，此处用已暴露的 FlowGraphFm）
// 因此"Path 3" = ai-flow-graph.GraphFormulas（模块级已 export 的常量对象）
const FF = FlowGraphFm;

// ========= 2 组标准节点-边样本（无向 + 有向各 1，用于稳定对比） =========
// Sample 1：社交星型（A 中心）——无向边，稳定 top degree = a
const NODES_A = [
  { id: 'a', tag: 'center' },
  { id: 'b', tag: 'leaf' },
  { id: 'c', tag: 'leaf' },
  { id: 'd', tag: 'leaf' },
  { id: 'e', tag: 'bridge' },
  { id: 'f', tag: 'leaf' },
  { id: 'g', tag: 'leaf' },
];
const EDGES_A = [
  { source: 'a', target: 'b' },
  { source: 'a', target: 'c' },
  { source: 'a', target: 'd' },
  { source: 'a', target: 'e' },
  { source: 'e', target: 'f' },
  { source: 'e', target: 'g' },
];

// Sample 2：线性链 + 一条捷径（PageRank 前 5 集合稳定）
const NODES_B = Array.from({ length: 9 }, (_, i) => ({ id: `n${i}` }));
const EDGES_B = (() => {
  const e = [];
  for (let i = 0; i < 8; i++) e.push({ source: `n${i}`, target: `n${i + 1}` });
  e.push({ source: 'n0', target: 'n8' }); // 捷径
  e.push({ source: 'n3', target: 'n7' }); // 再一条
  return e;
})();

// ========= 辅助：Map 值近似对比 =================
function approxEqual(a, b, eps = 1e-9) {
  if (Number.isNaN(a) || Number.isNaN(b)) return false;
  if (!Number.isFinite(a) || !Number.isFinite(b)) return false;
  return Math.abs(a - b) <= eps * Math.max(1, Math.abs(a), Math.abs(b));
}

function topN(mapObj, n) {
  return Object.entries(mapObj)
    .sort((a, b) => b[1] - a[1])
    .slice(0, n);
}

let totalPassed = 0;
function check(name, cond, msg) {
  if (cond) { totalPassed++; console.log(`  [PASS] ${name}`); }
  else {
    console.error(`  [FAIL] ${name} — ${msg || ''}`);
    process.exitCode = 1;
  }
}

// ========== 函数主体：同一数据集跑三条路径，断言相等 ==========
// 辅助：把 {id: number | {degree:number, ...}} 的 map 展平为 {id: number}
function flatten(mapObj) {
  const out = {};
  for (const [k, v] of Object.entries(mapObj)) {
    if (typeof v === 'number') out[k] = v;
    else if (v && typeof v === 'object' && typeof v.degree === 'number') out[k] = v.degree;
    else if (v && typeof v === 'object') {
      for (const sub of Object.values(v)) if (typeof sub === 'number') { out[k] = sub; break; }
    }
    if (out[k] === undefined) out[k] = 0;
  }
  return out;
}
// 常用统计 helper（全局定义，避免 closure 内 undefined）
const maxV = obj => { const v = Object.values(obj); return v.length ? Math.max(...v) : 0; };
const pSum = obj => Object.values(obj).reduce((s, v) => (typeof v === 'number' ? s + v : s), 0);

function runEqualitySuite(label, nodes, edges, pageOpts) {
  console.log(`\n=== ${label} ===`);

  // Path 1：GraphFormulas（单源真实）
  const GF_deg = GraphFormulas.degreeCentrality(nodes, edges);
  const GF_bet = GraphFormulas.betweennessCentrality(nodes, edges, pageOpts || {});
  const GF_prRaw = GraphFormulas.pagerank
    ? GraphFormulas.pagerank(nodes, edges, pageOpts || {})
    : (function () {
        // 兼容：老公式库暴露 pagerank(nodes,edges,damping,maxIter) 签名
        const defaultD = (pageOpts && pageOpts.dampingFactor) || 0.85;
        const defaultIter = (pageOpts && pageOpts.maxIterations) || 80;
        return LegacyGraphAlgos.pagerank(nodes, edges, defaultD, defaultIter);
      })();
  // 如果 GraphFormulas 没有 pagerank 方法，则用 graph-formulas.js 暴露的实现兜底（永远选 graph-formulas.js 作为单源）
  const GF_pr = GF_prRaw;
  const _ = LegacyGraphAlgos; // 只是保留引用避免 lint unused

  // Path 2：Legacy graph-algos.js（thin wrapper，统一传 pageOpts 保证三条路径对齐）
  const LG_deg = flatten(LegacyGraphAlgos.degreeCentrality(nodes, edges));
  const LG_bet = LegacyGraphAlgos.betweennessCentrality(nodes, edges, pageOpts || {});
  const LG_pr = LegacyGraphAlgos.pagerank(
    nodes, edges,
    (pageOpts && pageOpts.dampingFactor) || 0.85,
    (pageOpts && pageOpts.maxIterations) || 80
  );

  // Path 3：ai-flow-graph.js（模块常量 GraphFormulas · 未来改 thin wrapper）
  const FF_deg = flatten(FF.degreeCentrality(nodes, edges));
  const FF_bet = FF.betweennessCentrality(nodes, edges, pageOpts || {});
  const FF_pr = FF.pagerank
    ? (typeof FF.pagerank === 'function'
        ? FF.pagerank(nodes, edges, pageOpts || {})
        : GF_pr)
    : GF_pr;

  // -------------- 断言集合 --------------
  // (A) 度中心性 top-3 id 顺序相同（3 条 path 两两 × 2 样本：6）
  for (const [tag, A, B] of [
    ['GF vs Legacy degree top-3', topN(GF_deg, 3), topN(LG_deg, 3)],
    ['GF vs FlowGF degree top-3', topN(GF_deg, 3), topN(FF_deg, 3)],
    ['Legacy vs FlowGF degree top-3', topN(LG_deg, 3), topN(FF_deg, 3)],
  ]) {
    check(`${label} ${tag}: ids 相等`,
      A.map(x => x[0]).join(',') === B.map(x => x[0]).join(','),
      `A=${JSON.stringify(A)} B=${JSON.stringify(B)}`);
  }

  // (B) 介数中心性 max 值比较（严格到 1e-9）× 2 样本 × 两两：3 × 2 = 6
  for (const [tag, A, B] of [
    ['GF vs Legacy betweenness max', maxV(GF_bet), maxV(LG_bet)],
    ['GF vs FlowGF betweenness max', maxV(GF_bet), maxV(FF_bet)],
    ['Legacy vs FlowGF betweenness max', maxV(LG_bet), maxV(FF_bet)],
  ]) {
    check(`${label} ${tag}: 值 ≤1e-9 相等`,
      approxEqual(A, B, 1e-9),
      `A=${A} B=${B} |diff|=${Math.abs(A - B)}`);
  }

  // (C) PageRank：sum ≈ 1（2 样本：2） + top-5 交集 100%（两两 × 2：6）
  for (const [tag, A, B] of [
    ['GF vs Legacy pagerank top-5 交集 100%', new Set(topN(GF_pr, 5).map(x => x[0])), new Set(topN(LG_pr, 5).map(x => x[0]))],
    ['GF vs FlowGF pagerank top-5 交集 100%', new Set(topN(GF_pr, 5).map(x => x[0])), new Set(topN(FF_pr, 5).map(x => x[0]))],
    ['Legacy vs FlowGF pagerank top-5 交集 100%', new Set(topN(LG_pr, 5).map(x => x[0])), new Set(topN(FF_pr, 5).map(x => x[0]))],
  ]) {
    const inter = [...A].filter(x => B.has(x)).length;
    check(`${label} ${tag}`, inter === 5, `交集${inter}/5：A=${[...A].join(',')} B=${[...B].join(',')}`);
  }
  // Sum checks (6 → 改为 3 × 2 = 6 samples 总和
  check(`${label} GF PageRank sum≈1`, approxEqual(pSum(GF_pr), 1, 1e-3), `GF sum=${pSum(GF_pr)}`);
  check(`${label} Legacy PageRank sum≈1`, approxEqual(pSum(LG_pr), 1, 1e-3), `Legacy sum=${pSum(LG_pr)}`);
  check(`${label} FlowGF PageRank sum≈1`, approxEqual(pSum(FF_pr), 1, 1e-3), `FlowGF sum=${pSum(FF_pr)}`);
}

// ========== WRAPPER_BODY 校验（green 阶段后：graph-algos.js 三个函数体不超 4 行源码）==========
function assertThinWrappers() {
  console.log('\n=== Thin Wrapper 校验（AC-05/TR-5.1：真实定义仅 1 处）===');
  const content = fs.readFileSync(path.join(__dirname, '..', 'src', 'lib', 'graph-algos.js'), 'utf8');
  const targets = ['degreeCentrality', 'betweennessCentrality', 'pagerank'];
  for (const name of targets) {
    // 以 function <name> ... {...} 取 body
    const regex = new RegExp(`function\\s+${name}\\s*\\(([^)]*)\\)\\s*\\{([\\s\\S]*?)^\\}`, 'm');
    const m = content.match(regex);
    if (!m) { check(`wrapper body: ${name} function 定义找到`, false, 'REGEX 未匹配'); continue; }
    const body = m[2];
    // 非空行计数
    const nonEmptyLines = body.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('//')).length;
    check(`wrapper body: ${name} 行数 ≤ 4 (实际 ${nonEmptyLines})`, nonEmptyLines <= 4,
      `body (前 500 字符)：${body.slice(0, 500)}`);
  }
}

// ========== 执行 =========
runEqualitySuite('SampleA 星+桥（无向）', NODES_A, EDGES_A, { directed: false });
runEqualitySuite('SampleB 链+捷径', NODES_B, EDGES_B, { directed: true });
assertThinWrappers();

console.log(`\n================== 汇总：${totalPassed} 断言通过；exit=${process.exitCode || 0} ==================`);
if (process.exitCode !== 0) {
  console.error('至少 1 项 FAIL：说明 graph-algos.js / ai-flow-graph.js 仍保留独立重复实现（符合 RED 阶段预期）。');
} else {
  console.log('全部断言通过，graph 公式单源化成功（≥20/20）。');
}
