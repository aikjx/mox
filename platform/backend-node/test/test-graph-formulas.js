'use strict';

/**
 * 图谱公式逐个测试验证套件
 * ==========================
 * 设计依据：docs/modules/ai-flow-graph-design.md 第 5 节
 * 运行：node test/test-graph-formulas.js
 *
 * 每个公式（F1-F8）配已知解析答案的标准测试图（T1-T8），
 * 断言数值误差 < 1e-9（PageRank/LPA 容差 1e-6），输出人性化报告表格。
 */

const { GraphFormulas, getAIFlowGraph } = require('../src/ai-flow-graph');
const { getAIEngine } = require('../src/ai-engine');
const { getAIIntegrationEngine } = require('../src/ai-integration-engine');
const { getGateway } = require('../src/llm-gateway');

// ==================== 测试工具 ====================

const results = [];
let passed = 0, failed = 0;

function assert(name, testGraph, formulaId, actual, expected, tol = 1e-9) {
  const err = Math.abs(actual - expected);
  const ok_ = err <= tol || (Number.isInteger(expected) && Math.abs(actual - expected) < tol);
  if (ok_) passed++; else failed++;
  results.push({ ok: ok_, formula: formulaId, graph: testGraph, expected, actual: +actual.toFixed(10), err: +err.toExponential(2) });
  return ok_;
}

function assertEqual(name, testGraph, formulaId, actual, expected) {
  const ok_ = actual === expected;
  if (ok_) passed++; else failed++;
  results.push({ ok: ok_, formula: formulaId, graph: testGraph, expected, actual, err: 0 });
  return ok_;
}

function N(ids) { return ids.map(id => ({ id })); }
function E(list) { return list.map(([source, target, weight]) => ({ source, target, ...(weight !== undefined ? { weight } : {}) })); }

// ==================== 标准测试图 ====================

// T1 星型（无向）：中心 c 连 4 叶 s1..s4
// 无向图约定：RAW 边（单条），公式库内部负责双向展开；仅 PageRank（有向算法）用双向展开边
const T1_NODES = N(['c', 's1', 's2', 's3', 's4']);
const T1_EDGES_RAW = [['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']];
const T1_EDGES = E(T1_EDGES_RAW); // 无向 RAW
const T1_EDGES_BIDI = E(T1_EDGES_RAW.flatMap(([a, b]) => [[a, b], [b, a]])); // PageRank 用

// T2 链（有向）：a→b→c→d→e
const T2_NODES = N(['a', 'b', 'c', 'd', 'e']);
const T2_EDGES = E([['a', 'b'], ['b', 'c'], ['c', 'd'], ['d', 'e']]);

// T3 双团（无向）：{a,b,c} 全互连 + {d,e,f} 全互连 + 桥 b—d
const T3_NODES = N(['a', 'b', 'c', 'd', 'e', 'f']);
const T3_EDGES_RAW = [['a', 'b'], ['a', 'c'], ['b', 'c'], ['d', 'e'], ['d', 'f'], ['e', 'f'], ['b', 'd']];
const T3_EDGES = E(T3_EDGES_RAW); // 无向 RAW

// T4 双环（有向）：a↔b
const T4_NODES = N(['a', 'b']);
const T4_EDGES = E([['a', 'b'], ['b', 'a']]);

// T5 孤立：3 个点无边
const T5_NODES = N(['x', 'y', 'z']);
const T5_EDGES = [];

// T6 星型有向：中心指向 4 叶 c→s1..s4
const T6_NODES = N(['c', 's1', 's2', 's3', 's4']);
const T6_EDGES = E([['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']]);

async function main() {
  console.log('='.repeat(86));
  console.log('图谱公式逐个测试验证套件（设计依据：docs/modules/ai-flow-graph-design.md §5）');
  console.log('='.repeat(86));

  // ---------- F1 密度 ----------
  // T1 星型（N=5, E=4 无向）：D = 2*4/(5*4) = 0.4
  assert('T1', '星型图', 'F1 密度', GraphFormulas.density(5, 4).value, 0.4);
  // T5 孤立（N=3, E=0）：D = 0
  assert('T5', '孤立图', 'F1 密度', GraphFormulas.density(3, 0).value, 0);

  // ---------- F2 度中心性 ----------
  // T1：c 度=4 → 4/4=1.0；叶=1 → 1/4=0.25（无向 RAW 边：source/target 各计 1）
  const t1Deg = GraphFormulas.degreeCentrality(T1_NODES, T1_EDGES);
  assert('T1', '星型图', 'F2 度中心性(c)', t1Deg.c, 1.0);
  assert('T1', '星型图', 'F2 度中心性(s1)', t1Deg.s1, 0.25);

  // ---------- F4 介数中心性（Brandes） ----------
  // T1 无向星型：4 叶两两最短路都经 c → 未归一化 6；归一化除 (5-1)(5-2)/2=6 → c=1.0，叶=0
  const t1Btw = GraphFormulas.betweennessCentrality(T1_NODES, T1_EDGES, { directed: false });
  assert('T1', '星型图(无向)', 'F4 介数(c)', t1Btw.c, 1.0);
  assert('T1', '星型图(无向)', 'F4 介数(s1)', t1Btw.s1, 0);

  // T2 有向链：b 在 a→c/a→d/a→e 三条最短路上 → 3/12=0.25；
  //   c 在 a→d/a→e/b→d/b→e 四条路上 → 4/12=1/3；端点 a、e 恒 0
  const t2Btw = GraphFormulas.betweennessCentrality(T2_NODES, T2_EDGES, { directed: true });
  assert('T2', '链图(有向)', 'F4 介数(b)', t2Btw.b, 0.25);
  assert('T2', '链图(有向)', 'F4 介数(a)', t2Btw.a, 0);
  assert('T2', '链图(有向)', 'F4 介数(c)', t2Btw.c, 1 / 3);

  // T6 有向星型（中心→叶）：无任何路径经过中心 → c=0
  const t6Btw = GraphFormulas.betweennessCentrality(T6_NODES, T6_EDGES, { directed: true });
  assert('T6', '星型(有向出)', 'F4 介数(c)', t6Btw.c, 0);

  // ---------- F5 紧密中心性（harmonic） ----------
  // T1 无向星型：c 到各叶距离 1 → H=4，除 (N-1)=4 → 1.0；s1 到 c=1、到其他叶=2×3 → H=2.5 → 2.5/4=0.625
  const t1Cls = GraphFormulas.closenessCentrality(T1_NODES, T1_EDGES, { directed: false });
  assert('T1', '星型图(无向)', 'F5 紧密(c)', t1Cls.c, 1.0);
  assert('T1', '星型图(无向)', 'F5 紧密(s1)', t1Cls.s1, 0.625);

  // T2 有向链：a 到 b,c,d,e 距离 1,2,3,4 → H=1+1/2+1/3+1/4=25/12 → /(4)=25/48≈0.5208
  const t2Cls = GraphFormulas.closenessCentrality(T2_NODES, T2_EDGES, { directed: true });
  assert('T2', '链图(有向)', 'F5 紧密(a)', t2Cls.a, 25 / 48);
  assert('T2', '链图(有向)', 'F5 紧密(e)', t2Cls.e, 0); // e 不可达任何节点 → 0

  // T5 孤立：全 0
  const t5Cls = GraphFormulas.closenessCentrality(T5_NODES, T5_EDGES, { directed: false });
  assert('T5', '孤立图', 'F5 紧密(x)', t5Cls.x, 0);

  // ---------- F3 PageRank（委托统一单源实现） ----------
  const integration = getAIIntegrationEngine();
  const prEngine = integration.graphEngine;

  // T4 双环：各 0.5（对称不动点）
  const t4Pr = await prEngine.computePersonalizedPageRank({ nodes: T4_NODES, edges: T4_EDGES }, { damping: 0.85 });
  const t4Map = Object.fromEntries(t4Pr.scores.map(r => [r.id, r.score]));
  assert('T4', '双环图', 'F3 PR(a)', t4Map.a, 0.5, 1e-6);
  assert('T4', '双环图', 'F3 PR(b)', t4Map.b, 0.5, 1e-6);

  // T2 有向链：e 最高（汇聚末端）且 ΣPR=1
  const t2Pr = await prEngine.computePersonalizedPageRank({ nodes: T2_NODES, edges: T2_EDGES }, { damping: 0.85 });
  const t2Map = Object.fromEntries(t2Pr.scores.map(r => [r.id, r.score]));
  const sumPR = t2Pr.scores.reduce((s, r) => s + r.score, 0);
  assert('T2', '链图(有向)', 'F3 ΣPR=1', sumPR, 1.0, 1e-6);
  assertEqual('T2', '链图(有向)', 'F3 最高=e', t2Pr.scores[0].id, 'e');

  // T1 无向星型：中心最高（度大者权重高；PageRank 是有向算法，无向图需双向展开边）
  const t1Pr = await prEngine.computePersonalizedPageRank({ nodes: T1_NODES, edges: T1_EDGES_BIDI }, { damping: 0.85 });
  assertEqual('T1', '星型图(无向)', 'F3 最高=c', t1Pr.scores[0].id, 'c');

  // ---------- F6 社区检测（模块度贪心凝聚 CNM，ai-engine 单源） ----------
  const gateway = getGateway();
  const aiEngine = getAIEngine(gateway);

  // T3 双团+桥：应恰好 2 社区（LPA 因标签吞并失败，CNM 正确）
  const t3Comms = aiEngine._detectCommunities(T3_NODES, T3_EDGES);
  assertEqual('T3', '双团+桥', 'F6 社区数', t3Comms.length, 2);
  const t3Sets = t3Comms.map(c => [...c.members].sort().join(','));
  assertEqual('T3', '双团+桥', 'F6 划分正确',
    t3Sets.includes('a,b,c') && t3Sets.includes('d,e,f'), true);

  // T5 孤立：3 社区（每个节点自身）
  const t5Comms = aiEngine._detectCommunities(T5_NODES, T5_EDGES);
  assertEqual('T5', '孤立图', 'F6 社区数', t5Comms.length, 3);

  // ---------- F7 模块度 ----------
  // T3 双团+桥的正确划分 {a,b,c}{d,e,f}：m=7；e_c 各=3；d_c 各=7（团内 6 + 桥贡献 1）
  // Q = [3/7 − (7/14)²] + [3/7 − (7/14)²] = 2×(3/7 − 1/4) = 2×5/28 = 5/14 ≈ 0.3571
  const t3Q = GraphFormulas.modularity(T3_NODES, T3_EDGES_RAW.map(([a, b]) => ({ source: a, target: b })), t3Comms);
  assert('T3', '双团+桥', 'F7 模块度', t3Q, 5 / 14);

  // ---------- F8 激活扩散意图识别（图谱化） ----------
  const flowGraph = getAIFlowGraph();
  const cases = [
    ['请分析这个图谱的PageRank与社区结构', 'graph'],
    ['请深度推理这个问题并逐步分析', 'reasoning'],
    ['组织专家联盟会诊这个问题', 'expert'],
    ['你好，今天天气怎么样', 'chat']
  ];
  for (const [q, expected] of cases) {
    const r = await flowGraph.detectIntentBySpread(q);
    assertEqual('T7', `意图用例"${q.slice(0, 12)}..."`, 'F8 激活扩散', r.intent, expected);
  }

  // ---------- T8 流程图谱自检 + 等价性回归 ----------
  const vis = flowGraph.toVisFormat();
  // 数量守恒：keyword 节点数 = triggers 边数（一词一节点，一词一触发边）
  const kwCount = vis.stats.by_type.keyword || 0;
  const trigCount = vis.stats.by_edge_type.triggers || 0;
  assertEqual('T8', '流程图谱自检', '关键词数=触发边数', kwCount, trigCount);
  // capability 数量 = 6
  assertEqual('T8', '流程图谱自检', '能力节点数', vis.stats.by_type.capability || 0, 6);
  // flows_to 边 = 流水线步数-1 = 4
  assertEqual('T8', '流程图谱自检', '流水线边数', vis.stats.by_edge_type.flows_to || 0, 4);

  // Top-1 决策一致性回归：图谱激活扩散（默认 d=0.85）与旧关键词打分的路由决策一致
  const core = require('../src/ai-engine-core');
  const coreInstance = new core.AIEngineCore();
  for (const [q] of cases) {
    const spread = await flowGraph.detectIntentBySpread(q);
    const legacy = coreInstance.detectIntent(q);
    assertEqual('T8', `决策一致性"${q.slice(0, 10)}..."`, 'F8 top-1 ≡ 旧打分', spread.intent, legacy.intent);
  }

  // ==================== 报告 ====================
  console.log('\n' + '-'.repeat(86));
  console.log('详细结果：');
  console.log('状态 | 公式 | 测试图 | 期望 | 实测 | 误差');
  console.log('-'.repeat(86));
  for (const r of results) {
    const mark = r.ok ? '  ✓ ' : '  ✗ ';
    console.log(`${mark} | ${r.formula.padEnd(24)} | ${r.graph.padEnd(14)} | ${String(r.expected).padEnd(10)} | ${String(r.actual).padEnd(12)} | ${r.err}`);
  }
  console.log('-'.repeat(86));
  console.log(`总计: ${passed + failed} 项断言 | 通过 ${passed} | 失败 ${failed}`);
  console.log('='.repeat(86));
  if (failed > 0) {
    console.log('\n失败项需修复后重跑。');
    process.exit(1);
  } else {
    console.log('\n全部公式验证通过 —— 业务流程与算法流程已统一承载于图谱引擎。');
  }
}

main().catch(e => { console.error('测试套件异常:', e); process.exit(1); });
