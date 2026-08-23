'use strict';

/**
 * 璇玑全维知识图谱归一化体系 · 全链路企业级测试
 * ------------------------------------------------------------------
 * 验证三大关联维度 + 全域治理（一处更新、全域联动）：
 *   [1] 云端文档资源维度：doc→实体→域 三层绑定管道
 *       （四类实体抽取 / 共现关系挖掘 / 评分制域映射 / 幂等同步 / 删除自愈）
 *   [2] 业务流程与架构模块维度：需求归一化流水线 N1-N7
 *       （IR 归一化 → 语句拆解 → 域映射 → 模块拆分 → 算法关联 → 运行落盘 → 变更传播）
 *   [3] 本地代码工程维度：代码图谱桥接
 *       （全量扫描绑定 / 代码溯源 / 三方对账 / 变更建议）
 *   [4] 全域统一治理：三维看板 + 跨维全链路溯源链
 *   [5] 无破窗联动复验（W11-W13）+ 数据快照自愈回归
 * 运行：node test/test-normalization-pipeline.js
 */

const atlas = require('../src/project-atlas');
const kb = require('../src/kb');
const { readJSON, writeJSON } = require('../src/lib/json-store');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail ? ' -> ' + detail : ''}`); }
}

// ---------- 测试夹具：数据快照 + 测试文档注入 ----------
const DOC_ID = 'kb_doc_test_norm';
const AFFECTED = [
  'kb_documents.json', 'doc_graph_links.json', 'graph_nodes.json',
  'graph_edges.json', 'normalization_runs.json', 'code_graph_bindings.json', 'kb_history.json'
];
const SNAPSHOT = {};
AFFECTED.forEach(f => { SNAPSHOT[f] = JSON.parse(JSON.stringify(readJSON(f, []))); });

const TEST_DOC_CONTENT = [
  '# 璇玑全维归一化验证文档',
  '',
  'REQ-001: 知识图谱、知识库 文档双向关联（核心）',
  'REQ-002: 专家联盟 咨询必须接入图谱增强',
  '需求3：任务管理 自动化编排调度',
  '',
  'RULE-1: 图谱验证必须无破窗',
  '所有实体必须归属业务域，禁止幽灵引用，不得脱离图谱单点存在。',
  '',
  '【架构】前端视图层：全域治理看板',
  '架构：数据层 SQLite 双写持久化存储',
  '',
  'MODULE: 归一化流水线引擎 - 需求归一化与变更传播 依赖：知识图谱、专家联盟',
  '模块：代码桥接服务 - 图谱与本地代码双向映射'
].join('\n');

(function seedDoc() {
  const docs = readJSON('kb_documents.json', []);
  writeJSON('kb_documents.json', [
    ...docs.filter(d => d.id !== DOC_ID),
    {
      id: DOC_ID, title: '璇玑全维归一化验证文档', content: TEST_DOC_CONTENT,
      type: 'markdown', category: 'business.requirement', tags: ['归一化', '图谱'],
      description: '全链路测试夹具', status: 'active', version: 1,
      created_at: new Date().toISOString()
    }
  ]);
})();

// ---------- [1] 云端文档资源维度 ----------
console.log('[1] 云端文档资源维度：doc→实体→域 三层绑定');

const entities = kb.extractAllEntities(TEST_DOC_CONTENT);
const types = [...new Set(entities.map(e => e.type))];
check('四类实体全维抽取（需求/规则/架构/模块）',
  ['requirement', 'business_rule', 'architecture', 'module_def'].every(t => types.includes(t)),
  JSON.stringify(types));
check('技术实体复用既有口径（technical_term）', types.includes('technical_term'), JSON.stringify(types));
check('实体总量护栏（8-200）', entities.length >= 8 && entities.length <= 200, String(entities.length));

const relations = kb.mineEntityRelations(entities, TEST_DOC_CONTENT);
check('同节共现关系挖掘 ≥ 1 条', relations.length >= 1, String(relations.length));

const domains = atlas.getAtlasDomains();
const reqEntity = entities.find(e => e.type === 'requirement');
const dm = kb.matchEntityToDomains(reqEntity, domains);
check('评分制域映射（top 域得分 ≥ 5）', dm.length >= 1 && dm[0].score >= 5,
  JSON.stringify(dm.map(x => `${x.domainId}:${x.score}`)));

const pipeline = kb.getDocGraphPipeline();
const r1 = pipeline.autoSyncDocument(DOC_ID);
check('文档自动化管道运行成功', r1.ok === true, JSON.stringify(r1.error || ''));
check('三层绑定：实体入谱 ≥ 8', r1.entities >= 8, String(r1.entities));
check('三层绑定：域映射 ≥ 3（图谱/联盟/任务域命中）', r1.domainBindings >= 3, JSON.stringify(r1.boundDomains));
check('图谱新增节点（文档节点 + 实体节点）', r1.newNodes >= 5, String(r1.newNodes));
check('绑定记录落盘 doc_graph_links（溯源真相源）', pipeline.getBindings(DOC_ID).length === 1);

const r2 = pipeline.autoSyncDocument(DOC_ID);
check('幂等重跑：节点零新增', r2.newNodes === 0 && r2.totalGraphNodes === r1.totalGraphNodes,
  `r1=${r1.totalGraphNodes} r2=${r2.totalGraphNodes}`);
check('幂等重跑：绑定记录仍为 1 条', pipeline.getBindings(DOC_ID).length === 1);

const cov = pipeline.getCoverage();
check('覆盖率统计聚合（已绑定/活跃文档）', cov.docs >= 1 && cov.boundDocs >= 1,
  `docs=${cov.docs} bound=${cov.boundDocs}`);

// ---------- [2] 业务流程与架构模块维度 ----------
console.log('\n[2] 业务流程与架构模块维度：需求归一化流水线（N1-N7）');

const REQ_CONTENT = [
  '知识库 文档全生命周期管理。',
  '专家联盟 咨询必须接入图谱增强。',
  '任务管理 自动化编排调度。',
  '量子退火炉温控校准。'
].join('\n');
const nr = atlas.runNormalization({ title: '全维归一化验证需求', content: REQ_CONTENT, source: 'test' });
check('需求归一化运行成功（N1-N7 全流水线）', nr.ok === true, JSON.stringify(nr.error || nr.errors || ''));
const run = nr.run;
check('N1 IR 归一化（类别合法 + 关键词）',
  ['feature', 'architecture', 'bugfix', 'optimization', 'general'].includes(run.category) && run.ir.keywords.length > 0,
  `${run.category} / ${run.ir.keywords.length} 词`);
check('N2 语句拆解 ≥ 4 条', run.statements.length >= 4, String(run.statements.length));
check('N3 域映射覆盖 ≥ 3 语句', run.stats.mappedStatements >= 3, JSON.stringify(run.stats));
check('N3 映射命中真实域（kb/expert-alliance/tasks）',
  run.mappings.some(m => m.best === 'kb') &&
  run.mappings.some(m => m.best === 'expert-alliance') &&
  run.mappings.some(m => m.best === 'tasks'),
  JSON.stringify(run.mappings.map(m => m.best)));
check('N4 模块拆分计划 ≥ 3 域承接', run.modulePlan.plans.length >= 3, String(run.modulePlan.plans.length));
check('N4 既有引擎承接（moduleType=existing + 引擎清单）',
  run.modulePlan.plans.some(p => p.moduleType === 'existing' && p.engines.length > 0));
check('N4 无匹配语句 → 新模块建议（独立项目化种子）',
  run.modulePlan.newModules.length >= 1, JSON.stringify(run.modulePlan.newModules.map(m => m.name)));
check('N5 算法反推关联（kb 域 → LCS/文档分析/实体抽取）',
  run.algorithmBindings.some(b => b.domainId === 'kb' &&
    b.algorithms.some(a => ['algo-lcs', 'algo-docanalyze', 'algo-entity-extract'].includes(a.id))),
  JSON.stringify(run.algorithmBindings.map(b => b.domainId)));
check('N7 运行记录落盘可查', atlas.getNormalizationRuns('requirement').some(r => r.id === run.id));
check('空正文快速失败（前置守卫）', atlas.runNormalization({ title: 'x', content: '' }).ok === false);

const pr = atlas.runPropagation({ nodeId: 'kb', changeType: 'module', note: '全链路测试传播' });
check('N6 变更传播运行成功', pr.ok === true, JSON.stringify(pr.error || ''));
check('传播计划含高优先回归动作（算法/引擎/数据）',
  pr.ok === true && pr.run.plan.actions.some(a => a.priority === 'high'),
  JSON.stringify(pr.run && pr.run.plan.summary));
check('幽灵节点传播被拒', atlas.runPropagation({ nodeId: 'ghost-node' }).ok === false);
check('传播运行落盘', atlas.getNormalizationRuns('propagation').some(r => r.id === pr.run.id));

// ---------- [3] 本地代码工程维度 ----------
console.log('\n[3] 本地代码工程维度：代码图谱桥接（图谱↔本地代码双向映射）');

const scan = atlas.scanCodeBindings();
check('全量扫描绑定成功', scan.ok === true, JSON.stringify(scan));
check('图谱单元全绑定（bound=units, missing=0）',
  scan.bound === scan.units && scan.missing === 0,
  `units=${scan.units} bound=${scan.bound} missing=${scan.missing}`);
check('代码实体抽取总量 ≥ 500（函数/类/导出/路由）', scan.totalEntities >= 500, String(scan.totalEntities));

const kbBinding = atlas.getCodeBindings({ unitId: 'kb' });
check('kb 单元绑定可查（unitId 幂等键）', kbBinding.length === 1 && kbBinding[0].entityCount > 0,
  JSON.stringify(kbBinding.map(b => b.entityCount)));

const tc = atlas.traceCode('kb');
check('代码溯源：文件与实体定位', tc && tc.exists === true && tc.files.length >= 1,
  tc ? tc.codePath : 'null');
check('溯源含实体统计（函数/类/路由）',
  tc && tc.totals.functions + tc.totals.classes + tc.totals.routes > 0,
  tc ? JSON.stringify(tc.totals) : 'null');

const vc = atlas.verifyCodeConsistency();
check('三方对账一致（绑定↔磁盘↔图谱）', vc.ok === true && vc.failed === 0, `failed=${vc.failed}`);

const sug = atlas.suggestCodeChanges('kb');
check('变更建议清单（图谱变更→代码动作）', sug.codeUnits >= 1, String(sug.codeUnits));

// ---------- [4] 全域统一治理 ----------
console.log('\n[4] 全域统一治理：三维看板 + 跨维溯源');

const dash = atlas.getGovernanceDashboard();
check('三维看板结构齐备（cloudDocs/businessFlow/localCode）',
  ['cloudDocs', 'businessFlow', 'localCode'].every(k => dash.dimensions[k]),
  JSON.stringify(Object.keys(dash.dimensions)));
check('云端文档维度聚合（实体 ≥ 8 / 域绑定 ≥ 3）',
  dash.dimensions.cloudDocs.entities >= 8 && dash.dimensions.cloudDocs.domainBindings >= 3,
  JSON.stringify(dash.dimensions.cloudDocs));
check('业务流程维度聚合（运行 ≥ 2 / 映射语句 ≥ 3）',
  dash.dimensions.businessFlow.runs >= 2 && dash.dimensions.businessFlow.mappedStatements >= 3,
  JSON.stringify({ runs: dash.dimensions.businessFlow.runs, mapped: dash.dimensions.businessFlow.mappedStatements }));
check('本地代码维度聚合（全单元绑定 + 实体统计）',
  dash.dimensions.localCode.units >= 70 &&
  dash.dimensions.localCode.bound === dash.dimensions.localCode.units &&
  dash.dimensions.localCode.functions > 0,
  JSON.stringify(dash.dimensions.localCode.units));
check('综合健康分 ∈ (0,100]', dash.health.score > 0 && dash.health.score <= 100, String(dash.health.score));
check('看板内嵌无破窗验证结果', dash.verification.ok === true, `failed=${dash.verification.failed}`);

const chain = atlas.traceChain('tasks');
check('跨维溯源链可查（tasks 域）', chain.ok === true, JSON.stringify(chain.error || ''));
check('上游项目归属（proj-ai-platform）',
  chain.ok === true && chain.chain.owners.some(o => o.projectId === 'proj-ai-platform'),
  JSON.stringify(chain.chain.owners));
check('下游引擎展开（expert-alliance）',
  chain.ok === true && chain.chain.engines.some(e => e.id === 'expert-alliance'));
check('下游数据资产（tasks.json）',
  chain.ok === true && chain.chain.dataAssets.some(d => d.name === 'tasks.json'));
check('下游文档关联 ≥ 1', chain.ok === true && chain.chain.documents.length >= 1);
check('代码维度反查（绑定实体数 > 0）', chain.ok === true && chain.counts.codeEntities > 0,
  JSON.stringify(chain.counts));
check('需求维度反查（归一化语句映射到该域）',
  chain.ok === true && chain.chain.requirements.length >= 1, String(chain.chain.requirements.length));
check('幽灵节点溯源被拒', atlas.traceChain('ghost-node').ok === false);

// ---------- [5] 无破窗联动复验 + 数据自愈回归 ----------
console.log('\n[5] 无破窗联动复验（W11-W13）+ 数据自愈回归');

const vMid = atlas.verifyAtlas();
const w11 = vMid.checks.find(c => c.name.includes('W11'));
const w12 = vMid.checks.find(c => c.name.includes('W12'));
const w13 = vMid.checks.find(c => c.name.includes('W13'));
check('W11 文档图谱绑定完整（测试文档三层绑定对账）', w11?.ok === true, w11?.detail);
check('W12 归一化运行引用完整（域映射真实）', w12?.ok === true, w12?.detail);
check('W13 代码绑定一致（单元真实 + 路径存在）', w13?.ok === true, w13?.detail);
check('全量无破窗验证通过（W1-W13）', vMid.ok === true, `failed: ${vMid.summary.failed}`);

// 删除态自愈：文档删除 → 图谱清理（闭环验证）
(function markDeleted() {
  const docs = readJSON('kb_documents.json', []);
  const d = docs.find(x => x.id === DOC_ID);
  if (d) { d.status = 'deleted'; writeJSON('kb_documents.json', docs); }
})();
const r3 = pipeline.autoSyncDocument(DOC_ID);
check('删除态文档触发图谱自愈清理（cleaned）', r3.ok === true && r3.cleaned === true, JSON.stringify(r3));
check('清理移除文档节点与提及边', r3.removedNodes >= 1, JSON.stringify(r3));
check('清理后绑定记录移除', pipeline.getBindings(DOC_ID).length === 0);

// 快照恢复（数据自愈回归：测试产物零残留）
AFFECTED.forEach(f => writeJSON(f, SNAPSHOT[f]));
const vFinal = atlas.verifyAtlas();
check('数据恢复后无破窗验证整体通过', vFinal.ok === true, `failed: ${vFinal.summary.failed}`);
check('测试产物零残留（文档/运行记录/绑定恢复基线）',
  !readJSON('kb_documents.json', []).some(d => d.id === DOC_ID) &&
  !readJSON('normalization_runs.json', []).some(r => r.source === 'test') &&
  !readJSON('doc_graph_links.json', []).some(l => l.docId === DOC_ID),
  '存在残留');

// ---------- 汇总 ----------
console.log('\n===== 璇玑全维归一化体系全链路测试汇总 =====');
console.log(`通过: ${passed} 项，失败: ${failed} 项`);
console.log(`治理看板健康分: ${dash.health.score}（${dash.health.level}）· 验证项: ${vFinal.summary.total}`);
process.exit(failed > 0 ? 1 : 0);
