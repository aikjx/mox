'use strict';
/**
 * D.1 精度护栏专项
 *  ============================================================
 *  D.1.1 扫描 graph 相关 JS 文件禁止 `.toFixed(` 代码中出现（注释允许）
 *  D.1.2 graph-algos.js 公开 API 不得暴露 LPA：
 *        - 旧公开 `labelPropagation` 应抛 DeprecationError
 *        - `_internalLabelPropagation` 仍可运行（仅内部/单测）
 *  D.1.3 RAW 双向展开：单边 u-v 生成 u<->v 两方向（T3 已有 test-algo-rust-node-diff 覆盖）
 *  D.1.4 PageRank / Activation 默认参数：d=0.85；maxIter=30
 *
 *  运行：
 *      cd platform/backend-node && node test/test-precision-guardrail.js
 */

const fs = require('fs');
const path = require('path');
const assert = require('assert');

const BACKEND_NODE = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(BACKEND_NODE, '..', '..');

const passes = [];
const failures = [];
function ck(cond, name, detail) {
  if (cond) passes.push(`✅ PASS: ${name}${detail ? ' — ' + detail : ''}`);
  else failures.push(`❌ FAIL: ${name}${detail ? ' — ' + detail : ''}`);
}

console.log('========== D.1 精度护栏专项启动 ==========');

// ————— D.1.1 禁止 toFixed (graph 相关文件) —————
{
  const graphTargets = [
    'src/graph/graph-formulas.js',
    'src/ai-flow-graph.js',
    'src/lib/graph-algos.js',
    'src/routes/graph.js',
    'src/modules/graph.js',
    'src/expert-graph.js',
    'src/ai-flow-graph.js',
  ];
  const offenders = [];
  for (const rel of graphTargets) {
    const abs = path.join(BACKEND_NODE, rel);
    if (!fs.existsSync(abs)) continue;
    const raw = fs.readFileSync(abs, 'utf8');
    // 去注释：块注释、行注释
    const stripped = raw
      .replace(/\/\*[\s\S]*?\*\//g, ' ')
      .replace(/\/\/[^\n]*/g, ' ');
    if (/\.toFixed\s*\(/.test(stripped)) {
      // 定位具体行
      raw.split(/\r?\n/).forEach((l, n) => {
        const noLineC = l.replace(/\/\/[^\n]*/g, '');
        if (/\.toFixed\s*\(/.test(noLineC)) {
          offenders.push(`${rel}:${n + 1}: ${l.trim().slice(0, 120)}`);
        }
      });
    }
    if (/Math\.round\s*\(/.test(stripped) || /\.toPrecision\s*\(/.test(stripped)) {
      offenders.push(`${rel}: 禁止 Math.round / .toPrecision 精度截断`);
    }
  }
  ck(offenders.length === 0,
    `D.1.1 graph 相关文件 (${graphTargets.length}) 代码中无 .toFixed/.round/.toPrecision 截断`,
    offenders.length ? offenders.join('\n  - ') : `扫描 ${graphTargets.filter(t => fs.existsSync(path.join(BACKEND_NODE, t))).length} 个文件，干净`);
}

// ————— D.1.2 graph-algos.js 公开 API 不得暴露 LPA —————
{
  let raised1 = false, raised2 = false;
  try {
    const { labelPropagation } = require('../src/lib/graph-algos');
    labelPropagation(
      [{ id: 'a' }, { id: 'b' }, { id: 'c' }],
      [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }],
      5
    );
  } catch (e) {
    if (e && (e.name === 'DeprecationError' || e.message && /LPA|labelPropagation|deprecated/i.test(e.message))) {
      raised1 = true;
    }
  }
  // graph-formulas deprecatedLabelPropagationPublic
  try {
    const { deprecatedLabelPropagationPublic } = require('../src/graph/graph-formulas');
    deprecatedLabelPropagationPublic && deprecatedLabelPropagationPublic();
  } catch (e) {
    if (e && (e.name === 'DeprecationError' || /LPA|deprecated/i.test(String(e.message)))) raised2 = true;
  }
  ck(raised1, `D.1.2 graph-algos.labelPropagation 公开出口抛 DeprecationError (${raised1})`);
  ck(raised2, `D.1.2 GraphFormulas deprecatedLabelPropagationPublic 抛 DeprecationError (${raised2})`);
  // 内部 _internalLabelPropagation 仍可运行
  try {
    const { _internalLabelPropagation } = require('../src/lib/graph-algos');
    const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
    const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }];
    const r = _internalLabelPropagation(nodes, edges, { maxIter: 30, seed: 42 });
    const ok = r && typeof r === 'object' && Object.keys(r).length === nodes.length;
    ck(ok, `D.1.2 内部 _internalLabelPropagation 仍可用 (result keys=${Object.keys(r).length})`);
  } catch (e) {
    ck(false, 'D.1.2 内部 _internalLabelPropagation 报错', String(e && e.message));
  }
}

// ————— D.1.3 RAW 双向展开：单边 u-v → u<->v (T3 已有对齐复用) —————
{
  const { expandRawEdges } = require('../src/graph/graph-formulas');
  const exp = expandRawEdges([{ source: 'u', target: 'v' }], { directed: false });
  const dirs = exp.map(e => `${e.source}->${e.target}`).sort();
  const ok = dirs.length === 2 && dirs[0] === 'u->v' && dirs[1] === 'v->u';
  ck(ok, `D.1.3 RAW 单边双向展开 (2 条方向对称)`, `${JSON.stringify(dirs)}`);
  // 度中心性一致（RAW 后 u、v 都 =1/(N-1)，N=3 → 0.5）
  const { GraphFormulas } = require('../src/graph/graph-formulas');
  const nodes3 = [{ id: 'u' }, { id: 'v' }, { id: 'w' }];
  const degs = GraphFormulas.degreeCentrality(nodes3, [{ source: 'u', target: 'v' }], { expandRaw: true });
  ck(Math.abs(degs.u - 0.5) < 1e-9 && Math.abs(degs.v - 0.5) < 1e-9 && degs.w === 0,
    `D.1.3 度中心性 u=v=0.5, w=0（RAW 语义）`,
    `实际 u=${degs.u},v=${degs.v},w=${degs.w}`);
}

// ————— D.1.4 PageRank 默认 d=0.85 / maxIter=30 —————
{
  const { GraphFormulas, PPR_D, PPR_MAX_ITER } = require('../src/graph/graph-formulas');
  const nodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
  const edges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }, { source: 'a', target: 'c' }];
  // 常量护栏
  ck(PPR_D === 0.85, `D.1.4 PPR_D 常量=0.85`, `实际=${PPR_D}`);
  ck(PPR_MAX_ITER === 30, `D.1.4 PPR_MAX_ITER 常量=30`, `实际=${PPR_MAX_ITER}`);
  // pagerankWithTranspose 返回值含 d & maxIter
  const pr = GraphFormulas.pagerankWithTranspose(nodes, edges);
  ck(pr.d === 0.85, `D.1.4 PageRank 返回 d=0.85`, `实际 d=${pr.d}`);
  ck(Number(pr.maxIter) === 30, `D.1.4 PageRank 返回 maxIter=30`, `实际 maxIter=${pr.maxIter}`);
  // Activation 默认 (activateSpread 直接 lib/graph-algos，T5 已验)
  try {
    const { activateSpread } = require('../src/lib/graph-algos');
    const r1 = activateSpread(nodes.filter(()=>true).concat([]), edges, 'a');
    const r2 = activateSpread(nodes, edges, 'a', 0.85);
    let s = 0;
    for (const k of Object.keys(r1 || {})) s += Math.abs((r1[k]||0)-(r2[k]||0));
    ck(s < 1e-9, `D.1.4 activateSpread 默认值=显式 0.85，sumΔ=${s.toExponential(2)}`);
  } catch (e) {
    ck(false, 'D.1.4 activateSpread 异常', String(e && e.message));
  }
  // PPR 护栏：忽略调用方 d/maxIter（若内部仍用 PPR_D / PPR_MAX_ITER，则不同传参应完全一致）
  try {
    const seed = { a: 1 };
    const pp1 = GraphFormulas.personalizedPageRank(nodes, edges, seed);
    const pp2 = GraphFormulas.personalizedPageRank(nodes, edges, seed, { d: 0.5, maxIter: 3 });
    let diff = 0;
    for (const k of Object.keys(pp1)) diff += Math.abs(pp1[k] - pp2[k]);
    ck(diff < 1e-9, `D.1.4 PPR 护栏：忽略调用方 d/maxIter 传参（diff=${diff.toExponential(2)}；应为 0）`);
  } catch (e) {
    ck(false, 'D.1.4 PPR 护栏异常', String(e && e.message));
  }
}

// ————— 输出 —————
console.log('\n—————— PASS ——————');
passes.forEach(p => console.log(p));
console.log('\n—————— FAIL ——————');
failures.forEach(f => console.log(f));
console.log(`\n========== 汇总：${passes.length} PASS / ${failures.length} FAIL ==========`);
console.log(failures.length === 0 ? '🟢 精度护栏全 GREEN' : '🔴 精度护栏存在失败项');

process.exit(failures.length === 0 ? 0 : 1);
