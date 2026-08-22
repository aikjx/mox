'use strict';

/**
 * 引擎宇宙图谱 · 全链路验证测试
 * ------------------------------------------------------------------
 * 验证用户核心诉求：
 *   ① 技术图谱管理所有引擎链接：17 引擎节点化 + 关联边可直接查询
 *   ② 需求归一化链与引擎紧密关联（n_ingest→n_gate 每环有引擎服务）
 *   ③ 本地代码路径关联图谱（每个引擎 codePath 真实存在）
 *   ④ 全链路打通验证（降级链收敛 / 能力承接完备 / 无孤岛）
 *   ⑤ 业务/数据/算法流程图通过图谱完成（流程图谱引擎在宇宙中可达）
 * 运行：node test/test-engine-universe.js
 */

const assert = require('assert');
const universe = require('../src/engine-universe');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail ? ' -> ' + detail : ''}`); }
}

// ---------- ① 引擎节点化完备 ----------
console.log('[1] 引擎节点化（技术图谱管理所有引擎）');
const u = universe.getUniverse();
check('引擎节点数 = 18（17 引擎 + 引擎宇宙自身）', u.stats.engineCount === 18, String(u.stats.engineCount));
check('需求归一化链节点 = 5', u.stats.requirementNodeCount === 5);
check('关联边数 ≥ 30（依赖/委托/降级/数据流/服务）', u.stats.edgeCount >= 30, String(u.stats.edgeCount));
check('边类型覆盖 6 类（depends_on/delegates_to/degrades_to/data_flows_to/serves/flows_to）',
  ['depends_on', 'delegates_to', 'degrades_to', 'data_flows_to', 'serves', 'flows_to']
    .every(t => u.stats.edgeByType[t] > 0), JSON.stringify(u.stats.edgeByType));
check('每个引擎节点含关键功能描述（keyFunctions ≥ 3 条）',
  universe.ENGINES.every(e => Array.isArray(e.keyFunctions) && e.keyFunctions.length >= 3));
check('每个引擎节点含代码路径声明', universe.ENGINES.every(e => typeof e.codePath === 'string' && e.codePath.startsWith('src/')));

// 用户点名的引擎类别全部存在
const userNamed = {
  '记忆引擎': ['ultimate-ai-engine', 'session-store'],
  '计算引擎': ['ai-integration-engine'],
  '分析引擎': ['ai-engine'],
  '文档编写引擎': ['auto-dev-engine', 'kb'],
  '自动化引擎': ['auto-dev-engine', 'orchestration-engine'],
  '知识图谱引擎': ['knowledge-graph'],
  '流程图谱引擎': ['ai-flow-graph']
};
for (const [label, ids] of Object.entries(userNamed)) {
  check(`${label}已节点化（${ids.join('/')}）`, ids.every(id => universe.getEngine(id)));
}

// ---------- ② 需求归一化链关联 ----------
console.log('\n[2] 需求归一化链与引擎关联');
const chain = universe.requirementChain();
check('需求链 5 环全部有引擎服务', chain.every(c => c.servedBy.length > 0),
  chain.filter(c => c.servedBy.length === 0).map(c => c.node.id).join(','));
const ingest = chain.find(c => c.node.id === 'n_ingest');
check('需求采集环节由联网搜索+知识库服务', ingest.servedBy.some(s => s.engine === 'web-search-service') && ingest.servedBy.some(s => s.engine === 'kb'));
const norm = chain.find(c => c.node.id === 'n_norm');
check('归一化环节由编排核心+流程图谱服务（意图归一化）', norm.servedBy.some(s => s.engine === 'ai-engine-core') && norm.servedBy.some(s => s.engine === 'ai-flow-graph'));
const disp = chain.find(c => c.node.id === 'n_disp');
check('特派环节由专家联盟引擎服务', disp.servedBy.some(s => s.engine === 'expert-alliance-engine'));
const rec = chain.find(c => c.node.id === 'n_rec');
check('裁决环节由记忆推理引擎服务', rec.servedBy.some(s => s.engine === 'ultimate-ai-engine'));

// ---------- ③ 链路追踪 ----------
console.log('\n[3] 链路追踪（关联关系可直接查询）');
const t1 = universe.trace('ai-engine-core', 'llm-gateway');
check('编排核心 → LLM 网关 可追踪', t1.found, JSON.stringify(t1.path));
const t2 = universe.trace('ai-engine-core', 'expert-alliance', 'delegates_to');
check('编排核心 -[delegates_to]-> 专家联盟域包 可追踪', t2.found, JSON.stringify(t2.path));
const t3 = universe.trace('n_ingest', 'n_gate');
check('需求链 n_ingest → n_gate 可追踪', t3.found, t3.reason);
const t4 = universe.trace('expert-alliance-engine', 'ai-integration-engine');
check('联盟引擎 → 图计算引擎 跨层可达（协同链路）', t4.found, JSON.stringify(t4.path.map(p => `${p.from}->${p.to}`)));
const t5 = universe.trace('web-search-service', 'ultimate-ai-engine');
check('搜索服务 → 记忆推理引擎 跨层可达', t5.found, t5.reason);
check('不存在节点追踪返回 found=false 而非抛错', universe.trace('nope', 'llm-gateway').found === false);

// ---------- ④ 全链路验证 ----------
console.log('\n[4] 全链路验证（verifyFullChain）');
const v = universe.verifyFullChain();
check('全链路验证整体通过', v.ok);
check('验证项总数 ≥ 40（代码路径/边完整性/需求链/降级链/能力承接/无孤岛）', v.summary.total >= 40, String(v.summary.total));
const codeChecks = v.checks.filter(c => c.name.startsWith('代码路径存在') || c.name.startsWith('协作文件存在'));
check(`代码路径检查 ${codeChecks.length} 项全部通过`, codeChecks.every(c => c.ok));
const edgeChecks = v.checks.filter(c => c.name.startsWith('边两端节点存在'));
check(`边完整性检查 ${edgeChecks.length} 项全部通过`, edgeChecks.every(c => c.ok));
check('降级链全部收敛到 llm-gateway（chat 兜底不变式）',
  v.checks.find(c => c.name.includes('降级链全部收敛'))?.ok === true);
check('全域连通无孤岛（单一连通分量，技术图谱管理所有链接）',
  v.checks.find(c => c.name.includes('全域连通无孤岛'))?.ok === true);

// ---------- ⑤ 流程图谱在宇宙中可达（业务/数据/算法流程图统一承载） ----------
console.log('\n[5] 流程图引擎关联（业务/数据/算法流程图通过图谱完成）');
const flowDetail = universe.getEngineDetail('ai-flow-graph');
check('流程图谱引擎详情可查询', !!flowDetail);
check('流程图谱引擎上游含编排核心（能力矩阵注入）', flowDetail.relations.upstream.some(e => e.from === 'ai-engine-core'));
check('流程图谱引擎委托图计算引擎（F8 激活扩散）', flowDetail.relations.downstream.some(e => e.to === 'ai-integration-engine' && e.type === 'delegates_to'));

// 自动开发引擎：需求→代码全链路
const autoDev = universe.getEngineDetail('auto-dev-engine');
check('自动开发引擎数据流入知识图谱（架构图谱统一管理）', autoDev.relations.downstream.some(e => e.to === 'knowledge-graph' && e.type === 'data_flows_to'));
check('自动开发引擎数据流入知识库（文档沉淀）', autoDev.relations.downstream.some(e => e.to === 'kb' && e.type === 'data_flows_to'));
check('自动开发引擎代码文件关联 ≥ 2 个', autoDev.codeFiles.length >= 2);

// ---------- ⑥ 单引擎详情契约 ----------
console.log('\n[6] 单引擎详情（最关键功能描述）');
const detail = universe.getEngineDetail('ultimate-ai-engine');
check('记忆推理引擎含上下游关系', detail.relations.upstream.length > 0 && detail.relations.downstream.length > 0);
check('记忆推理引擎服务需求裁决环节', detail.servesRequirements.some(s => s.requirement === 'n_rec'));
check('记忆推理引擎代码文件关联', detail.codeFiles.length >= 1);
check('不存在的引擎返回 null', universe.getEngineDetail('nonexistent') === null);

// ---------- 汇总 ----------
console.log('\n===== 引擎宇宙全链路验证汇总 =====');
console.log(`通过: ${passed} 项，失败: ${failed} 项`);
console.log(`引擎: ${u.stats.engineCount} 节点 · 需求链: ${u.stats.requirementNodeCount} 环 · 关联边: ${u.stats.edgeCount} 条`);
process.exit(failed > 0 ? 1 : 0);
