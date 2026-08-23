'use strict';

/**
 * 项目全息图谱 · 需求归一化流水线（application 层 · 用例编排）
 * ------------------------------------------------------------------
 * "业务流程与架构模块维度" 核心落地：需求输入 → 全流程自动归一化。
 * 五步流水线：
 *   ① ingest        原始需求 → 归一化 IR（类别推断 + 关键词）
 *   ② decompose     IR → 语句级子需求拆解
 *   ③ map           子需求 → 业务域映射（评分制 top-3）
 *   ④ split         映射结果 → 模块拆分计划（引擎承接 / 新模块建议）
 *   ⑤ bind          引擎 → 算法关联（consumers 反推）
 * 变更传播：propagateChange 任意图谱节点变更 → 影响面 → 结构化传播计划
 * 运行记录持久化 normalization_runs.json（N7 校验后落盘，全程可溯源）
 *
 * 依赖注入（可测性）：getView / runsIO / impact 由装配方注入。
 */

const { uid } = require('../../utils');
const {
  buildRequirementIR, decomposeRequirement, mapStatementsToDomains,
  planModuleSplit, bindAlgorithmsForPlans, buildPropagationPlan,
  validateNormalizationRun
} = require('../domain/normalization-rules');

function createNormalizationPipeline({ getView, runsIO, impact }) {

  /**
   * 需求归一化运行（全流水线）：
   * 入参 { title, content, category?, source? } → 运行记录（含五步产物）
   */
  function runRequirement({ title, content, category, source }) {
    const raw = String(content || '').trim();
    if (!raw) return { ok: false, error: 'content 为必填（需求正文）' };

    // ① 归一化 IR
    const ir = buildRequirementIR({ title, content: raw, category });
    // ② 拆解
    const statements = decomposeRequirement(ir);
    // ③ 域映射
    const view = getView();
    const domains = view.domains || [];
    const mappings = mapStatementsToDomains(statements, domains);
    // ④ 模块拆分
    const enginesById = new Map((view.engines || []).map(e => [e.id, e]));
    const modulePlan = planModuleSplit(mappings, domains, enginesById);
    // ⑤ 算法关联
    const algorithmBindings = bindAlgorithmsForPlans(modulePlan.plans, view.algorithms || []);

    const run = {
      id: uid('nrm'),
      type: 'requirement',
      title: ir.title, category: ir.category,
      source: source || 'manual',
      ir: { id: ir.id, title: ir.title, category: ir.category, keywords: ir.keywords },
      statements,
      mappings: mappings.map(m => ({
        statementId: m.statement.id, text: m.statement.text,
        best: m.best ? m.best.domainId : null,
        matches: m.matches
      })),
      modulePlan: { plans: modulePlan.plans, newModules: modulePlan.newModules },
      algorithmBindings,
      stats: {
        statements: statements.length,
        mappedStatements: mappings.filter(m => m.best).length,
        domains: modulePlan.plans.length,
        newModules: modulePlan.newModules.length,
        algorithmBindings: algorithmBindings.length,
        coverage: statements.length === 0 ? 0 : mappings.filter(m => m.best).length / statements.length
      },
      status: 'analyzed',
      createdAt: new Date().toISOString()
    };

    // N7 校验后落盘
    const existing = runsIO.read();
    const check = validateNormalizationRun(run, new Set(existing.map(r => r.id)), new Set(domains.map(d => d.id)));
    if (!check.valid) return { ok: false, error: '归一化运行校验失败', errors: check.errors };
    existing.unshift(run);
    runsIO.write(existing.slice(0, 500)); // 护栏：保留最近 500 次运行
    return { ok: true, run };
  }

  /**
   * 变更传播运行：图谱节点变更 → 影响面 → 传播计划
   * 入参 { nodeId, changeType, note? }，changeType ∈ requirement|architecture|module|algorithm|data
   */
  function runPropagation({ nodeId, changeType, note }) {
    const view = getView();
    const nodeById = new Map((view.nodes || []).map(n => [n.id, n]));
    if (!nodeById.has(nodeId)) return { ok: false, error: `图谱节点不存在: ${nodeId}` };

    const impactResult = typeof impact === 'function' ? impact(nodeId) : { seed: nodeId, reachableNodes: [] };
    const plan = buildPropagationPlan(impactResult, nodeById);

    const run = {
      id: uid('nrm'),
      type: 'propagation',
      nodeId, changeType: changeType || 'module',
      note: note || '',
      plan,
      status: 'planned',
      createdAt: new Date().toISOString()
    };

    const existing = runsIO.read();
    const check = validateNormalizationRun(run, new Set(existing.map(r => r.id)), new Set());
    if (!check.valid) return { ok: false, error: '传播运行校验失败', errors: check.errors };
    existing.unshift(run);
    runsIO.write(existing.slice(0, 500));
    return { ok: true, run };
  }

  /** 运行记录查询（?type= requirement|propagation 过滤） */
  function getRuns(type) {
    const runs = runsIO.read();
    return type ? runs.filter(r => r.type === type) : runs;
  }

  function getRun(id) { return runsIO.read().find(r => r.id === id) || null; }

  /** 归一化统计（治理看板数据源） */
  function getStats() {
    const runs = runsIO.read();
    const reqRuns = runs.filter(r => r.type === 'requirement');
    const propRuns = runs.filter(r => r.type === 'propagation');
    const domains = getView().domains || [];
    const domainIds = new Set(domains.map(d => d.id));

    // 域覆盖：被至少一次运行映射到的域
    const covered = new Set();
    reqRuns.forEach(r => (r.mappings || []).forEach(m =>
      (m.matches || []).forEach(x => { if (domainIds.has(x.domainId)) covered.add(x.domainId); })));

    const statements = reqRuns.reduce((s, r) => s + (r.stats ? r.stats.statements : 0), 0);
    const mapped = reqRuns.reduce((s, r) => s + (r.stats ? r.stats.mappedStatements : 0), 0);

    return {
      runs: runs.length, requirementRuns: reqRuns.length, propagationRuns: propRuns.length,
      statements, mappedStatements: mapped,
      mappingCoverage: statements === 0 ? 0 : mapped / statements,
      domainsCovered: covered.size, totalDomains: domains.length,
      domainCoverage: domains.length === 0 ? 0 : covered.size / domains.length,
      newModulesSuggested: reqRuns.reduce((s, r) => s + (r.stats ? r.stats.newModules : 0), 0),
      lastRunAt: runs.length > 0 ? runs[0].createdAt : null
    };
  }

  return { runRequirement, runPropagation, getRuns, getRun, getStats };
}

module.exports = { createNormalizationPipeline };
