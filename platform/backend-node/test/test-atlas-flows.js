'use strict';

/**
 * 业务处理流程图谱化 · 分析验证测试（EAF-STD-001）
 * ------------------------------------------------------------------
 * 验证闭环：
 *   [1] 流程注册表完整性：9 条流程覆盖核心域，结构规范
 *   [2] EAF-STD-001 参考实现：专家联盟六阶段 + 前置守卫 + 双降级链 + 回归主流
 *   [3] 图谱化正确性：flow_step 节点 + 六类流程边 + W8 全连通
 *   [4] 流程查询 API：getFlows / getFlowDetail 契约
 *   [5] W9 机器验证全绿（结构/引用/连通/覆盖/标准锚点）
 *   [6] 标准文档存在且图谱登记（EAF-STD-001 行业规范标准）
 * 运行：node test/test-atlas-flows.js
 */

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const atlas = require('../src/project-atlas');
const { FLOWS } = require('../src/project-atlas/domain/flow-registry');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail !== undefined ? ' -> ' + String(detail) : ''}`); }
}

const PROJECT_ROOT = path.join(__dirname, '..', '..', '..');

console.log('[1] 流程注册表完整性');
check('流程总数 ≥ 9（核心域全覆盖）', FLOWS.length >= 9, String(FLOWS.length));
const flowIds = new Set(FLOWS.map(f => f.id));
check('流程 id 唯一', flowIds.size === FLOWS.length);
check('全部流程步骤数 ≥3', FLOWS.every(f => f.steps.length >= 3));
check('全部流程含 transitions', FLOWS.every(f => f.transitions.length >= f.steps.length - 1));
const coreFlowDomains = ['expert-alliance', 'ai-engine', 'atlas', 'engine-kernel', 'auto-dev', 'kb', 'graph', 'chat', 'optimizer'];
const coveredDomains = new Set(FLOWS.map(f => f.domain));
check('九大核心域流程全覆盖', coreFlowDomains.every(d => coveredDomains.has(d)),
  coreFlowDomains.filter(d => !coveredDomains.has(d)).join(','));
check('全系统流程步骤总量 ≥ 40', FLOWS.reduce((s, f) => s + f.steps.length, 0) >= 40);
check('降级链总量 ≥ 4（韧性建模）', FLOWS.reduce((s, f) => s + f.transitions.filter(t => t.type === 'degrade').length, 0) >= 4);

console.log('\n[2] EAF-STD-001 参考实现（专家联盟六阶段）');
const eaf = FLOWS.find(f => f.standard === 'EAF-STD-001');
check('标准流程存在（flow-ea-consult）', !!eaf && eaf.id === 'flow-ea-consult');
const stageSteps = eaf.steps.filter(s => /阶段[一二三四五六]/.test(s.name));
check('六阶段完整（意图/组队/咨询辩论/综合/门禁/学习）', stageSteps.length === 6, String(stageSteps.length));
check('前置守卫存在（空问题快速失败）', eaf.steps.some(s => s.id === 'ea-guard' && s.detail.includes('100ms')));
check('归一化输出步骤存在（trace 落盘）', eaf.steps.some(s => s.id === 'ea-output' && s.writes.includes('alliance_traces.jsonl')));
const eafDegrades = eaf.transitions.filter(t => t.type === 'degrade');
check('双降级链（辩论→单专家 / LLM→启发式）', eafDegrades.length === 2 &&
  eafDegrades.some(t => t.from === 'ea-deliberate' && t.to === 'ea-single-fallback') &&
  eafDegrades.some(t => t.from === 'ea-synthesize' && t.to === 'ea-heuristic-synthesis'));
const backToGate = eaf.transitions.filter(t => t.to === 'ea-gate' && t.type === 'next');
check('降级路径回归主流（两条均汇入门禁）', backToGate.length >= 2, String(backToGate.length));
const deliberate = eaf.steps.find(s => s.id === 'ea-deliberate');
check('韧性约束声明（超时隔离60s/令牌900/共识0.6）',
  deliberate.detail.includes('60s') && deliberate.detail.includes('900') && deliberate.detail.includes('0.6'));
const eafEngines = new Set(eaf.steps.map(s => s.engine).filter(Boolean));
check('委托引擎 ≥ 4（联盟引擎/专家图谱/LLM网关/联盟域）', eafEngines.size >= 4, [...eafEngines].join(','));
check('意图先验读写闭环（ea-intent 读 + ea-learn 写）',
  eaf.steps.find(s => s.id === 'ea-intent').reads.includes('alliance_intent_priors.json') &&
  eaf.steps.find(s => s.id === 'ea-learn').writes.includes('alliance_intent_priors.json'));

console.log('\n[3] 图谱化正确性');
const a = atlas.getAtlas();
check('flow_step 节点入图（45 步）', a.stats.byKind.flow_step === FLOWS.reduce((s, f) => s + f.steps.length, 0), String(a.stats.byKind.flow_step));
const be = a.stats.byEdge;
check('flow_of 边 = 步骤数（每步归属域）', be.flow_of === a.stats.byKind.flow_step, String(be.flow_of));
check('next_step 边 ≥ 30（主干流转）', be.next_step >= 30, String(be.next_step));
check('degrades_to 边 ≥ 4（降级链）', be.degrades_to >= 4, String(be.degrades_to));
check('delegates_to 边 ≥ 40（步骤委托引擎）', be.delegates_to >= 40, String(be.delegates_to));
check('reads/writes 边存在（数据依赖）', (be.reads || 0) >= 5 && (be.writes || 0) >= 8, `reads=${be.reads} writes=${be.writes}`);
// 步骤节点挂标准锚点，可检索
const searchEaf = atlas.searchAtlas('六阶段');
check('流程步骤可自然语言检索（六阶段）', searchEaf.nodes.some(n => n.kind === 'flow_step'));
// 影响面：改动专家联盟引擎波及流程步骤
const impactEAE = atlas.impact('expert-alliance-engine');
check('影响面分析波及流程步骤（改引擎→影响委托步骤）',
  impactEAE.impacted.some(n => n.kind === 'flow_step' && n.name.includes('阶段三')));

console.log('\n[4] 流程查询 API');
const flows = atlas.getFlows();
check('getFlows 返回清单与统计', flows.flows.length === FLOWS.length && flows.stats.total === FLOWS.length);
check('统计含步骤总量/降级总量/覆盖域', flows.stats.totalSteps >= 40 && flows.stats.totalDegrades >= 4 && flows.stats.coveredDomains.length >= 9);
check('标准级流程标记（standardLevel）', flows.flows.filter(f => f.standardLevel).length === 1);
const d = atlas.getFlowDetail('flow-ea-consult');
check('getFlowDetail 返回步骤链（10 步）', d.steps.length === 10, String(d.steps.length));
check('步骤含入口标记', d.steps.some(s => s.entry === true));
check('步骤含委托引擎名', d.steps.every(s => !s.engine || s.engine.name));
check('详情含降级链', d.degrades.length === 2);
check('graphRef 暴露图谱节点 id（可关联图谱）', d.graphRef.stepNodeIds.length === 10 && d.graphRef.stepNodeIds[0] === 'step:flow-ea-consult/ea-guard');
check('未知流程返回 null', atlas.getFlowDetail('flow-not-exist') === null);

console.log('\n[5] W9 机器验证');
const v = atlas.verifyAtlas();
check('无破窗验证全绿（W1-W9）', v.ok, v.checks.filter(c => !c.ok).map(c => `${c.name}: ${c.detail}`).join(';'));
const w9 = v.checks.filter(c => c.name.startsWith('W9'));
check('W9 检查项 ≥ 50（9 流程 × 6 结构项 + 2 全局项）', w9.length >= 50, String(w9.length));
check('W9 全部通过', w9.every(c => c.ok));
check('验证总量 ≥ 250（含流程检查族）', v.summary.total >= 250, String(v.summary.total));

console.log('\n[6] 标准文档存在且图谱登记');
const stdPath = path.join(PROJECT_ROOT, 'docs', 'standards', 'expert-alliance-flow-standard.md');
check('EAF-STD-001 标准文档存在', fs.existsSync(stdPath));
const stdContent = fs.existsSync(stdPath) ? fs.readFileSync(stdPath, 'utf8') : '';
check('标准含六阶段规范', stdContent.includes('六阶段') && stdContent.includes('意图识别') && stdContent.includes('质量门禁'));
check('标准含韧性约束（60s/900/0.6）', stdContent.includes('60s') && stdContent.includes('900') && stdContent.includes('0.6'));
check('标准含 W9 机器验证映射', stdContent.includes('W9') && stdContent.includes('atlas/verify'));
check('标准文档已图谱登记（documented_by → expert-alliance 域）',
  a.edges.some(e => e.type === 'documented_by' && e.to === 'doc:docs/standards/expert-alliance-flow-standard.md' && e.from === 'expert-alliance'));
const atlasDomain = atlas.getDomainDetail('expert-alliance');
check('专家联盟域详情含标准文档', atlasDomain.docs.some(x => x.path === 'docs/standards/expert-alliance-flow-standard.md'));

console.log('\n===== 业务流程图谱化分析验证汇总 =====');
console.log(`通过: ${passed} 项，失败: ${failed} 项`);
process.exit(failed > 0 ? 1 : 0);
