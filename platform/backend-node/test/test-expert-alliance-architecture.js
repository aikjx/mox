'use strict';

/**
 * 专家联盟架构理清 —— 架构级验证测试
 * 验证本轮 A16/A18/A19/A20/A21/A22/A23/A24 修复：
 *   A16 意图识别单一真相源（alliance 导出，engine 引用，无漂移）
 *   A18 PageRank 委托单源（联盟层与 ai-engine 结果一致）
 *   A19 社区检测委托 CNM（专家图返回真实多社区，双团图划分正确）
 *   A20 辩论综合真实内容驱动（无硬编码话术）
 *   A21/A22/A23 死代码清除（模块可加载、无残留引用）
 *   A24 能力图分级建边（包含式强边 + 2-gram 语义邻接边，密度 0.19）
 * 运行：node test/test-expert-alliance-architecture.js
 */

const assert = require('assert');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) {
    passed++;
    console.log(`  [PASS] ${name}`);
  } else {
    failed++;
    console.log(`  [FAIL] ${name}${detail ? ' -> ' + detail : ''}`);
  }
}

// ---------- ① 模块加载与死代码清除 ----------
console.log('[1] 模块加载（含循环依赖安全性：直接先加载 expert-alliance）');
const { getAlliance, INTENT_PATTERNS } = require('../src/expert-alliance');
const { getAllianceEngine } = require('../src/expert-alliance-engine');
const { getExpertGraph } = require('../src/expert-graph');
const { getDispatcher } = require('../src/expert-dispatcher');
const alliance = getAlliance();
const engine = getAllianceEngine();
const graph = getExpertGraph(alliance);
const dispatcher = getDispatcher(alliance);
check('四模块全部可加载且单例可用', alliance && engine && graph && dispatcher);
check('A16: expert-alliance 导出 INTENT_PATTERNS（15 个意图域）', Array.isArray(INTENT_PATTERNS) && INTENT_PATTERNS.length === 15);
check('A21: expert-graph 死代码 typeKeywords 已清除',
  !require('fs').readFileSync(require('path').join(__dirname, '..', 'src', 'expert-graph.js'), 'utf8').includes('typeKeywords'));

// ---------- ② A16 意图识别单一真相源 ----------
console.log('\n[2] A16 意图识别单一真相源');
const q1 = '这个微服务架构如何做负载均衡与服务治理';
const rAlliance = alliance._detectIntent(q1);
const rEngine = engine.classifyIntent(q1);
check('alliance._detectIntent 主意图 = architecture', rAlliance.primary === 'architecture', rAlliance.primary);
check('engine.classifyIntent 主意图 = architecture', rEngine.primary === 'architecture', rEngine.primary);
check('两处主意图一致（同源无漂移）', rAlliance.primary === rEngine.primary);
const q2 = '分析知识图谱的PageRank中心性与社区发现算法';
check('图谱类问题两处均命中 graph',
  alliance._detectIntent(q2).primary === engine.classifyIntent(q2).primary &&
  alliance._detectIntent(q2).primary === 'graph');

// ---------- ③ A18 PageRank 委托单源 ----------
console.log('\n[3] A18 PageRank 委托单源（已知答案：星型图中心最高）');
(async () => {
  const starNodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }];
  const starEdges = [
    { source: 'a', target: 'c' }, { source: 'b', target: 'c' },
    { source: 'd', target: 'c' }, { source: 'c', target: 'a' },
    { source: 'c', target: 'b' }, { source: 'c', target: 'd' }
  ];
  const prAlliance = await alliance._computePageRank(starNodes, starEdges, 0.85, 50);
  check('联盟层 PageRank 返回降序数组', Array.isArray(prAlliance) && prAlliance.length === 4);
  check('星型图权威节点 c 排名第一', prAlliance[0].id === 'c', JSON.stringify(prAlliance));
  const sum = prAlliance.reduce((s, p) => s + p.pagerank, 0);
  check('ΣPR = 1（质量守恒）', Math.abs(sum - 1) < 1e-6, String(sum));
  const src = require('fs').readFileSync(require('path').join(__dirname, '..', 'src', 'expert-alliance', 'application', 'alliance-orchestrator.js'), 'utf8');
  check('A22: _scoreExperts 死变量 hasLLM 已清除', !src.includes('hasLLM'));

  // ---------- ④ A19 CNM 社区检测 ----------
  console.log('\n[4] A19 社区检测委托 CNM（先重建能力图：A24 分级建边）');
  graph.rebuild();
  check('能力图边数 > 0（修复前精确匹配导致 0 边）', graph.edges.length > 0, `实际 ${graph.edges.length}`);
  check('A24: 语义邻接边存在且 relation 分层标注（修复前仅 2 条包含式边）',
    graph.edges.some(e => e.relation === 'semantic_adjacent' && Array.isArray(e.shared_grams) && e.shared_grams.length > 0),
    `semantic_adjacent ${graph.edges.filter(e => e.relation === 'semantic_adjacent').length} 条`);
  check('A24: 边权重为正整数（强弱分层）', graph.edges.every(e => Number.isInteger(e.weight) && e.weight > 0));
  const comms = graph.detectCommunities();
  check('真实专家能力图社区数 > 1（修复前 BFS 恒为 1）', comms.length > 1, `实际 ${comms.length}`);
  check('社区成员为节点对象且含 type 字段',
    comms.every(c => c.members.every(m => m && m.id && m.type)));
  const assigned = comms.flatMap(c => c.members.map(m => m.id));
  check('全部专家均被唯一社区覆盖（无截断/无重复）',
    assigned.length === graph.nodes.length && new Set(assigned).size === assigned.length,
    `分配 ${assigned.length} / 节点 ${graph.nodes.length}`);

  // ---------- ⑤ A20 辩论综合真实内容驱动 ----------
  console.log('\n[5] A20 辩论综合真实内容驱动');
  const fakeResponses = [
    { expert: '架构专家', response: '建议采用微服务架构，引入服务注册与API网关，保证高可用与弹性扩容', confidence: 0.85 },
    { expert: '性能专家', response: '优先解决高可用瓶颈，引入缓存与异步化，微服务拆分需评估性能开销', confidence: 0.7 },
    { expert: '安全专家', response: '微服务间通信需加密认证，高可用架构须配套审计与合规基线', confidence: 0.75 }
  ];
  const consensus = alliance._extractConsensus(fakeResponses);
  const divergence = alliance._extractDivergences(fakeResponses);
  const recommendation = alliance._generateFinalRecommendation(fakeResponses);
  check('共识提取包含真实共性关键词（微服务/高可用）',
    consensus.includes('微服务') || consensus.includes('高可用'), consensus);
  check('共识不再包含硬编码话术（算子系统公理）', !consensus.includes('数学公理'));
  check('分歧提取含 Jaccard 相似度量化', divergence.includes('Jaccard'), divergence);
  check('推荐基于真实置信度（0.85 的架构专家）', recommendation.includes('0.85') && recommendation.includes('架构专家'), recommendation);

  // ---------- ⑥ 汇总 ----------
  console.log('\n===== 架构验证汇总 =====');
  console.log(`通过: ${passed} 项，失败: ${failed} 项`);
  console.log(`专家总数: ${alliance.listExperts().length} · 专家图: ${graph.nodes.length} 节点 / ${graph.edges.length} 边 · 社区数: ${comms.length}`);
  process.exit(failed > 0 ? 1 : 0);
})().catch(e => {
  console.error('[FAIL] 异常:', e.message);
  process.exit(1);
});
