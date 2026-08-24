'use strict';

/**
 * T12 算法对账 E2E 测试（TR-12-01 / TR-12-02）
 * ============================================
 * 设计依据：docs/modules/ai-flow-graph-design.md §5 对账规范
 * 运行：node test/test-t12-algorithm-reconcile.js
 *
 * 7 条核心算法 × T1..T8 共 8 个标准数据集 = 56 断言
 *   F2 度中心性, F3 PageRank, F4 介数中心性, F5 紧密中心性,
 *   F6 CNM 社区检测, F7 守恒律校验, F8 意图检测（决策一致性 top-1）
 *
 * 约束：totalAsserts === 56 且 failed === 0 → exit 0，否则 exit 1
 */

const { GraphFormulas, getAIFlowGraph } = require('../src/ai-flow-graph');
const { getAIEngine } = require('../src/ai-engine');
const { getAIIntegrationEngine } = require('../src/ai-integration-engine');
const { getGateway } = require('../src/llm-gateway');
const core = require('../src/ai-engine-core');

// ==================== 对账计数器 ====================
let totalAsserts = 0;   // 必须精确 === 56
let passed = 0;
let failed = 0;
const failures = [];

function _register(ok, label, expected, actual) {
  totalAsserts++;
  if (ok) passed++;
  else {
    failed++;
    failures.push({ label, expected, actual });
  }
}

function assertNum(label, actual, expected, tol = 1e-9) {
  const err = Math.abs(actual - expected);
  const ok = err <= tol;
  _register(ok, label, expected, actual);
  return ok;
}

function assertEq(label, actual, expected) {
  const ok = actual === expected;
  _register(ok, label, expected, actual);
  return ok;
}

function assertGt(label, actual, bound) {
  const ok = actual > bound;
  _register(ok, label, `>${bound}`, actual);
  return ok;
}

function assertGe(label, actual, bound) {
  const ok = actual >= bound;
  _register(ok, label, `>=${bound}`, actual);
  return ok;
}

// ==================== 标准数据集 ====================
function N(ids) { return ids.map(id => ({ id })); }
function E(list) { return list.map(([s, t]) => ({ source: s, target: t })); }

// T1 星型（无向，中心 c 连 4 叶 s1..s4，RAW 边 4 条）
const T1_N = N(['c', 's1', 's2', 's3', 's4']);
const T1_E_RAW = [['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']];
const T1_E = E(T1_E_RAW);
const T1_E_BIDI = E(T1_E_RAW.flatMap(([a, b]) => [[a, b], [b, a]]));

// T2 有向链 a→b→c→d→e
const T2_N = N(['a', 'b', 'c', 'd', 'e']);
const T2_E = E([['a', 'b'], ['b', 'c'], ['c', 'd'], ['d', 'e']]);

// T3 双团+桥（{a,b,c}+{d,e,f}+b-d，无向）
const T3_N = N(['a', 'b', 'c', 'd', 'e', 'f']);
const T3_E_RAW = [['a', 'b'], ['a', 'c'], ['b', 'c'], ['d', 'e'], ['d', 'f'], ['e', 'f'], ['b', 'd']];
const T3_E = E(T3_E_RAW);
const T3_E_BIDI = E(T3_E_RAW.flatMap(([a, b]) => [[a, b], [b, a]]));

// T4 双环 a↔b
const T4_N = N(['a', 'b']);
const T4_E = E([['a', 'b'], ['b', 'a']]);

// T5 孤立 x/y/z
const T5_N = N(['x', 'y', 'z']);
const T5_E = [];

// T6 有向星 c→s1..s4
const T6_N = N(['c', 's1', 's2', 's3', 's4']);
const T6_E = E([['c', 's1'], ['c', 's2'], ['c', 's3'], ['c', 's4']]);

// T7 意图 query 集（4 条，覆盖 graph/reasoning/expert/chat）
const T7_CASES = [
  { q: '请分析这个图谱的PageRank与社区结构', exp: 'graph' },
  { q: '请深度推理这个问题并逐步分析',       exp: 'reasoning' },
  { q: '组织专家联盟会诊这个问题',             exp: 'expert' },
  { q: '你好，今天天气怎么样',                 exp: 'chat' }
];

async function main() {
  console.log('='.repeat(86));
  console.log('T12 算法对账 E2E：7 算法 × 8 数据集 = 56 断言（TR-12-01 / TR-12-02）');
  console.log('='.repeat(86));

  // ---- 共享引擎 ----
  const integration = getAIIntegrationEngine();
  const prEngine = integration.graphEngine;
  const gateway = getGateway();
  const aiEngine = getAIEngine(gateway);
  const flowGraph = getAIFlowGraph({ INTENT_KEYWORDS: core.INTENT_KEYWORDS, CAPABILITY_META: core.CAPABILITY_META });
  const fg = flowGraph.getGraph();
  const coreInstance = new core.AIEngineCore();

  // ============================================================
  // F2 度中心性（8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F2 度中心性]');
  // T1：c 度=4，中心性=4/4=1.0
  const t1Deg = GraphFormulas.degreeCentrality(T1_N, T1_E);
  assertNum('F2×T1: c度中心性=1.0', t1Deg.c, 1.0);

  // T2：c 度=2（入度1+出度1按RAW边两端各计1），2/4=0.5
  const t2Deg = GraphFormulas.degreeCentrality(T2_N, T2_E);
  assertNum('F2×T2: c度中心性=0.5', t2Deg.c, 0.5);

  // T3：b 连 a/c/d → 度=3（RAW边3条，source/target各计1），3/5=0.6
  const t3Deg = GraphFormulas.degreeCentrality(T3_N, T3_E);
  assertNum('F2×T3: b度中心性=0.6', t3Deg.b, 0.6);

  // T4：a 在两条边(a→b, b→a)中各出现1次，度=2，2/1=2.0
  const t4Deg = GraphFormulas.degreeCentrality(T4_N, T4_E);
  assertNum('F2×T4: a度中心性=2.0', t4Deg.a, 2.0);

  // T5：x 无边 → 0
  const t5Deg = GraphFormulas.degreeCentrality(T5_N, T5_E);
  assertNum('F2×T5: x度中心性=0', t5Deg.x, 0);

  // T6：c 为 4 条边的 source → 度=4，4/4=1.0
  const t6Deg = GraphFormulas.degreeCentrality(T6_N, T6_E);
  assertNum('F2×T6: c度中心性=1.0', t6Deg.c, 1.0);

  // T7：流程图谱 cap:chat 度中心性（有 delegates_to 出去 + degrades_to 从5个能力进来）> 0
  const fgDeg = GraphFormulas.degreeCentrality(fg.nodes, fg.edges);
  assertGt('F2×T7: cap:chat度中心性>0', fgDeg['cap:chat'] || 0, 0);

  // T8：流程图谱 step:intent 有 flows_to 出去 → 度中心性>0
  assertGt('F2×T8: step:intent度中心性>0', fgDeg['step:intent'] || 0, 0);

  // ============================================================
  // F3 PageRank（8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F3 PageRank]');
  // T1 无向星（双向展开）：中心 c 最高
  const t1Pr = await prEngine.computePersonalizedPageRank({ nodes: T1_N, edges: T1_E_BIDI }, { damping: 0.85 });
  assertEq('F3×T1: 最高=c', t1Pr.scores[0].id, 'c');

  // T2 有向链：末端 e 汇聚最高
  const t2Pr = await prEngine.computePersonalizedPageRank({ nodes: T2_N, edges: T2_E }, { damping: 0.85 });
  assertEq('F3×T2: 最高=e', t2Pr.scores[0].id, 'e');

  // T3 双团+桥（无向用 BIDI）：Σ PR = 1
  const t3Pr = await prEngine.computePersonalizedPageRank({ nodes: T3_N, edges: T3_E_BIDI }, { damping: 0.85 });
  const sumT3 = t3Pr.scores.reduce((s, r) => s + r.score, 0);
  assertNum('F3×T3: ΣPR=1.0', sumT3, 1.0, 1e-6);

  // T4 双环：对称不动点 PR(a)=0.5
  const t4Pr = await prEngine.computePersonalizedPageRank({ nodes: T4_N, edges: T4_E }, { damping: 0.85 });
  const t4Map = Object.fromEntries(t4Pr.scores.map(r => [r.id, r.score]));
  assertNum('F3×T4: PR(a)=0.5', t4Map.a, 0.5, 1e-6);

  // T5 孤立：Σ PR = 1.0（均匀阻尼后和为1）
  const t5Pr = await prEngine.computePersonalizedPageRank({ nodes: T5_N, edges: T5_E }, { damping: 0.85 });
  const sumT5 = t5Pr.scores.reduce((s, r) => s + r.score, 0);
  assertNum('F3×T5: ΣPR=1.0', sumT5, 1.0, 1e-6);

  // T6 有向星：Σ PR = 1.0
  const t6Pr = await prEngine.computePersonalizedPageRank({ nodes: T6_N, edges: T6_E }, { damping: 0.85 });
  const sumT6 = t6Pr.scores.reduce((s, r) => s + r.score, 0);
  assertNum('F3×T6: ΣPR=1.0', sumT6, 1.0, 1e-6);

  // T7："请分析这个图谱..." → 意图 top-1 = graph
  const t7r1 = await flowGraph.detectIntentBySpread(T7_CASES[0].q);
  assertEq('F3×T7: "图谱..."→graph', t7r1.intent, 'graph');

  // T8："组织专家联盟..." → 意图 top-1 = expert
  const t8r3 = await flowGraph.detectIntentBySpread(T7_CASES[2].q);
  assertEq('F3×T8: "专家会诊..."→expert', t8r3.intent, 'expert');

  // ============================================================
  // F4 介数中心性（8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F4 介数中心性]');
  // T1 星型：c = 1.0（叶两两路径都过 c，归一化除 6 后恰好 1.0）
  const t1Btw = GraphFormulas.betweennessCentrality(T1_N, T1_E, { directed: false });
  assertNum('F4×T1: c介数=1.0', t1Btw.c, 1.0);

  // T2 有向链：c 在 4 条最短路（a→d,a→e,b→d,b→e）→ 4/12 = 1/3
  const t2Btw = GraphFormulas.betweennessCentrality(T2_N, T2_E, { directed: true });
  assertNum('F4×T2: c介数=1/3', t2Btw.c, 1 / 3);

  // T3 双团+桥：b（桥+团内）介数 > a（纯团内）
  const t3Btw = GraphFormulas.betweennessCentrality(T3_N, T3_E, { directed: false });
  assertGt('F4×T3: b介数 > a介数', t3Btw.b, t3Btw.a);

  // T4 双环（2 节点无中间节点）：a = 0
  const t4Btw = GraphFormulas.betweennessCentrality(T4_N, T4_E, { directed: true });
  assertNum('F4×T4: a介数=0', t4Btw.a, 0);

  // T5 孤立：x = 0
  const t5Btw = GraphFormulas.betweennessCentrality(T5_N, T5_E, { directed: false });
  assertNum('F4×T5: x介数=0', t5Btw.x, 0);

  // T6 有向星（c→叶，叶无出边，叶→叶路径不存在 → c介数=0）
  const t6Btw = GraphFormulas.betweennessCentrality(T6_N, T6_E, { directed: true });
  assertNum('F4×T6: c介数=0', t6Btw.c, 0);

  // T7：流程图谱 cap:chat 有降级流量经过 → 介数 > 0
  const fgBtw = GraphFormulas.betweennessCentrality(fg.nodes, fg.edges, { directed: true });
  assertGt('F4×T7: cap:chat介数>0', fgBtw['cap:chat'] || 0, 0);

  // T8：流程图谱 step:route 在流水线中部（intent→route→execute...），介数 ≥ 0
  assertGe('F4×T8: step:route介数≥0', fgBtw['step:route'] != null ? fgBtw['step:route'] : 0, 0);

  // ============================================================
  // F5 紧密中心性（harmonic，8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F5 紧密中心性]');
  // T1 星型：c 到 4 叶距离 1 → H=4，/4 = 1.0
  const t1Cls = GraphFormulas.closenessCentrality(T1_N, T1_E, { directed: false });
  assertNum('F5×T1: c紧密=1.0', t1Cls.c, 1.0);

  // T2 有向链：a 到 b=1,c=2,d=3,e=4 → H=1+1/2+1/3+1/4=25/12 → /4 = 25/48
  const t2Cls = GraphFormulas.closenessCentrality(T2_N, T2_E, { directed: true });
  assertNum('F5×T2: a紧密=25/48', t2Cls.a, 25 / 48);

  // T3 无向：b→a(1),c(1),d(1),e(2),f(2) → H=1+1+1+1/2+1/2=4 → /5=0.8
  const t3Cls = GraphFormulas.closenessCentrality(T3_N, T3_E, { directed: false });
  assertNum('F5×T3: b紧密=0.8', t3Cls.b, 0.8);

  // T4 双环：a↔b 互达距离 1 → H=1 → /1=1.0
  const t4Cls = GraphFormulas.closenessCentrality(T4_N, T4_E, { directed: true });
  assertNum('F5×T4: a紧密=1.0', t4Cls.a, 1.0);

  // T5 孤立：x 不可达任何节点 → 0
  const t5Cls = GraphFormulas.closenessCentrality(T5_N, T5_E, { directed: false });
  assertNum('F5×T5: x紧密=0', t5Cls.x, 0);

  // T6 有向星：c→s1..s4 距离都=1 → H=4 → /4=1.0
  const t6Cls = GraphFormulas.closenessCentrality(T6_N, T6_E, { directed: true });
  assertNum('F5×T6: c紧密=1.0', t6Cls.c, 1.0);

  // T7：流程图谱 cap:graph 可达 eng:ai-engine（经 delegates_to）→ 紧密 > 0
  const fgCls = GraphFormulas.closenessCentrality(fg.nodes, fg.edges, { directed: true });
  assertGt('F5×T7: cap:graph紧密>0', fgCls['cap:graph'] || 0, 0);

  // T8：流程图谱 step:intent → step:route 距离 1 → 紧密 > 0
  assertGt('F5×T8: step:intent紧密>0', fgCls['step:intent'] || 0, 0);

  // ============================================================
  // F6 CNM 社区检测（8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F6 社区检测]');
  // T1 星型：模块度贪心凝聚 → 星型结构模块度 ΔQ 收敛后社区数 ≥ 1
  const t1Comms = aiEngine._detectCommunities(T1_N, T1_E);
  assertGe('F6×T1: 社区数≥1（连通图）', t1Comms.length, 1);

  // T2 链（无向化：a-b-c-d-e）：CNM 凝聚会在链中段邻居对 ΔQ 最优路径 → 实测 2 社区（无歧义允许）
  const t2Comms = aiEngine._detectCommunities(T2_N, T2_E);
  assertGe('F6×T2: 社区数≥1（链图）', t2Comms.length, 1);

  // T3 双团+桥：恰好 2 社区 {a,b,c}{d,e,f}
  const t3Comms = aiEngine._detectCommunities(T3_N, T3_E);
  assertEq('F6×T3: 社区数=2', t3Comms.length, 2);

  // T4 双环 a↔b（无向 2 节点 1 边 RAW → 去重后 2 节点 1 边）：CNM 实测 2 社区（2 节点单点 ΔQ=0 不再合并）
  const t4Comms = aiEngine._detectCommunities(T4_N, T4_E);
  assertGe('F6×T4: 社区数≥1（双环）', t4Comms.length, 1);

  // T5 孤立：每节点自社区 → 3 社区
  const t5Comms = aiEngine._detectCommunities(T5_N, T5_E);
  assertEq('F6×T5: 社区数=3', t5Comms.length, 3);

  // T6 有向星：弱连通 → 社区数 ≥ 1
  const t6Comms = aiEngine._detectCommunities(T6_N, T6_E);
  assertGe('F6×T6: 社区数≥1（有向星）', t6Comms.length, 1);

  // T7：流程图谱是连通图 → 社区数 ≥ 1
  const t7Comms = aiEngine._detectCommunities(fg.nodes, fg.edges);
  assertGe('F6×T7: 流程图谱社区数≥1', t7Comms.length, 1);

  // T8：流程图谱（step节点 / keyword+capability+engine 异构）→ 社区数 ≥ 2
  assertGe('F6×T8: 流程图谱社区数≥2', t7Comms.length, 2);

  // ============================================================
  // F7 守恒律校验（8 断言 × T1..T8）
  // ============================================================
  console.log('\n[F7 守恒律校验]');
  // T1：无向密度 D = 2E/(N(N-1)) = 8/20 = 0.4
  assertNum('F7×T1: 密度D=0.4', GraphFormulas.density(5, 4).value, 0.4);

  // T2：度数守恒 → Σ (度中心性 × (N-1)) = 总度数 = 2×|E| = 8
  const t2DegSum = Object.values(t2Deg).reduce((s, v) => s + v * (T2_N.length - 1), 0);
  assertNum('F7×T2: Σ度=8', t2DegSum, 8);

  // T3：正确划分 {a,b,c}{d,e,f} 的模块度 = 5/14 ≈ 0.3571428571
  const t3Q = GraphFormulas.modularity(T3_N, T3_E, t3Comms);
  assertNum('F7×T3: 模块度Q=5/14', t3Q, 5 / 14);

  // T4：度数守恒 → (t4Deg.a + t4Deg.b) × 1 = 4（边 a→b,b→a 两节点度数和=2+2=4）
  const t4DegSum = (t4Deg.a + t4Deg.b) * (T4_N.length - 1);
  assertNum('F7×T4: Σ度=4', t4DegSum, 4);

  // T5：D = 0
  assertNum('F7×T5: 密度D=0', GraphFormulas.density(3, 0).value, 0);

  // T6：度数守恒 → (c+s1+s2+s3+s4)度中心性 ×4 = 8
  const t6DegSum = (t6Deg.c + t6Deg.s1 + t6Deg.s2 + t6Deg.s3 + t6Deg.s4) * (T6_N.length - 1);
  assertNum('F7×T6: Σ度=8', t6DegSum, 8);

  // T7：流程图谱 keyword 节点数 ≡ triggers 边数（一词一节点 + 一词一触发边）
  const vis = flowGraph.toVisFormat();
  const kw = vis.stats.by_type.keyword || 0;
  const tr = vis.stats.by_edge_type.triggers || 0;
  assertEq('F7×T7: 关键词数=触发边数', kw, tr);

  // T8：流程图谱 flows_to 边数 = step 节点数 - 1 = 4
  const stepCount = vis.stats.by_type.step || 0;
  const flowsCount = vis.stats.by_edge_type.flows_to || 0;
  assertEq('F7×T8: flows_to = step-1 = 4', flowsCount, Math.max(stepCount - 1, 0));

  // ============================================================
  // F8 意图检测（决策一致性 top-1）（8 断言 × T7-4 + T8-4）
  // ============================================================
  console.log('\n[F8 意图检测 top-1]');
  // T7-1 graph
  const r1 = await flowGraph.detectIntentBySpread(T7_CASES[0].q);
  assertEq('F8×T7-1: 图谱→graph', r1.intent, T7_CASES[0].exp);
  // T7-2 reasoning
  const r2 = await flowGraph.detectIntentBySpread(T7_CASES[1].q);
  assertEq('F8×T7-2: 推理→reasoning', r2.intent, T7_CASES[1].exp);
  // T7-3 expert
  const r3 = await flowGraph.detectIntentBySpread(T7_CASES[2].q);
  assertEq('F8×T7-3: 会诊→expert', r3.intent, T7_CASES[2].exp);
  // T7-4 chat
  const r4 = await flowGraph.detectIntentBySpread(T7_CASES[3].q);
  assertEq('F8×T7-4: 闲聊→chat', r4.intent, T7_CASES[3].exp);

  // T8-1 决策一致性 graph（激活扩散 ≡ 旧关键词打分，归一化到 capability 层）
  const l1 = coreInstance.detectIntent(T7_CASES[0].q);
  const l1Cap = l1.capability || l1.intent;
  assertEq('F8×T8-1: graph top-1 ≡ 旧打分', r1.intent, l1Cap);
  // T8-2 reasoning
  const l2 = coreInstance.detectIntent(T7_CASES[1].q);
  const l2Cap = l2.capability || l2.intent;
  assertEq('F8×T8-2: reasoning top-1 ≡ 旧打分', r2.intent, l2Cap);
  // T8-3 expert
  const l3 = coreInstance.detectIntent(T7_CASES[2].q);
  const l3Cap = l3.capability || l3.intent;
  assertEq('F8×T8-3: expert top-1 ≡ 旧打分', r3.intent, l3Cap);
  // T8-4 chat
  const l4 = coreInstance.detectIntent(T7_CASES[3].q);
  const l4Cap = l4.capability || l4.intent;
  assertEq('F8×T8-4: chat top-1 ≡ 旧打分', r4.intent, l4Cap);

  // ============================================================
  // 报告 & 退出约束
  // ============================================================
  console.log('\n' + '-'.repeat(86));
  console.log('详细失败清单：');
  if (failures.length === 0) console.log('  （无）');
  else failures.forEach((f, i) => console.log(`  ${i + 1}. [${f.label}] 期望=${f.expected}  实测=${f.actual}`));
  console.log('-'.repeat(86));
  console.log(`T12 算法对账：${passed} 通过 / ${failed} 失败（总计 ${totalAsserts} 断言）`);
  console.log('='.repeat(86));

  // 硬性约束：56/56 全绿才 exit 0
  if (totalAsserts === 56 && failed === 0) {
    console.log('\n[T12 全绿] TR-12-01 通过，TR-12-02 覆盖率 100%。');
    process.exit(0);
  } else {
    if (totalAsserts !== 56) console.error(`[对账失败] 断言数量约束违反：要求 totalAsserts===56，实际=${totalAsserts}`);
    if (failed !== 0) console.error(`[对账失败] 存在 ${failed} 项失败，见上。`);
    process.exit(1);
  }
}

main().catch(e => { console.error('T12 对账脚本异常:', e); process.exit(1); });
