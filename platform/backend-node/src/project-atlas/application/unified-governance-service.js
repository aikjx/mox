'use strict';

/**
 * 项目全息图谱 · 全域统一治理服务（application 层 · 跨维聚合）
 * ------------------------------------------------------------------
 * 璇玑全维归一化体系的治理中枢，聚合三大维度：
 *   ① 云端文档资源维度  文档→实体→图谱自动化管道（doc_graph_links）
 *   ② 业务流程与架构模块维度  需求归一化流水线（normalization_runs）
 *   ③ 本地代码工程维度  代码图谱桥接（code_graph_bindings）
 *
 * getDashboard  全域治理看板（三维覆盖 + 图谱规模 + 无破窗验证 + 综合健康分）
 * traceChain    跨维全链路溯源：任意图谱节点 → 上游项目/文档实体 → 下游引擎/
 *               算法/数据/流程/代码实体（一处查询、全域可见）
 *
 * 依赖注入：getAtlasView / getDocGraphCoverage / getNormalizationStats /
 *           getCodeBridgeStats / getTraceSources（全部只读访问器）
 */

function createUnifiedGovernanceService({
  getAtlasView, getDocGraphCoverage, getNormalizationStats,
  getCodeBridgeStats, getTraceSources
}) {

  // ============ 全域治理看板 ============

  function getDashboard() {
    const view = getAtlasView();
    const verification = view.verify ? view.verify() : { ok: true, rules: [] };
    const docCoverage = getDocGraphCoverage();
    const normStats = getNormalizationStats();
    const codeStats = getCodeBridgeStats();

    const atlasStats = {
      nodes: (view.nodes || []).length,
      edges: (view.edges || []).length,
      projects: (view.nodes || []).filter(n => n.kind === 'project').length,
      domains: (view.nodes || []).filter(n => n.kind === 'domain' || n.kind === 'module').length,
      engines: (view.nodes || []).filter(n => n.kind === 'engine').length,
      algorithms: (view.nodes || []).filter(n => n.kind === 'algorithm').length,
      flows: (view.flows || []).length
    };

    // 综合健康分：三维覆盖率（各 25%）+ 无破窗验证（25%）
    const verifyScore = verification.ok ? 1 : 0.5;
    const score = Math.round((
      docCoverage.coverage * 0.25 +
      normStats.domainCoverage * 0.25 +
      codeStats.coverage * 0.25 +
      verifyScore * 0.25
    ) * 100);

    return {
      dimensions: {
        cloudDocs: {
          key: 'cloudDocs', name: '云端文档资源维度',
          docs: docCoverage.docs, boundDocs: docCoverage.boundDocs,
          entities: docCoverage.entities, entityTypes: docCoverage.entityTypes,
          domainBindings: docCoverage.domainBindings,
          coverage: round2(docCoverage.coverage)
        },
        businessFlow: {
          key: 'businessFlow', name: '业务流程与架构模块维度',
          runs: normStats.runs, requirementRuns: normStats.requirementRuns,
          propagationRuns: normStats.propagationRuns,
          statements: normStats.statements, mappedStatements: normStats.mappedStatements,
          mappingCoverage: round2(normStats.mappingCoverage),
          domainsCovered: normStats.domainsCovered, totalDomains: normStats.totalDomains,
          domainCoverage: round2(normStats.domainCoverage),
          newModulesSuggested: normStats.newModulesSuggested
        },
        localCode: {
          key: 'localCode', name: '本地代码工程维度',
          units: codeStats.units, bound: codeStats.bound,
          codeEntities: codeStats.codeEntities,
          functions: codeStats.functions, classes: codeStats.classes, routes: codeStats.routes,
          consistent: codeStats.consistent, inconsistent: codeStats.inconsistent,
          coverage: round2(codeStats.coverage)
        }
      },
      atlas: atlasStats,
      verification: {
        ok: verification.ok,
        passed: (verification.rules || []).filter(r => r.ok !== false).length,
        failed: (verification.rules || []).filter(r => r.ok === false).length,
        summary: verification.summary || null
      },
      health: { score, level: score >= 90 ? 'excellent' : score >= 70 ? 'good' : score >= 50 ? 'fair' : 'poor' },
      generatedAt: new Date().toISOString()
    };
  }

  // ============ 跨维全链路溯源 ============

  /**
   * 节点全链路溯源（一处查询、全域可见）：
   *   上游：拥有项目（owns_domain 反向）
   *   自身：节点详情 + 代码绑定
   *   下游：引擎/算法/数据/文档（atlas 边）+ 流程步骤 + 需求映射 + 代码实体
   */
  function traceChain(nodeId) {
    const view = getAtlasView();
    const node = (view.nodes || []).find(n => n.id === nodeId);
    if (!node) return { ok: false, error: `图谱节点不存在: ${nodeId}` };

    const edges = view.edges || [];
    const nodeById = new Map((view.nodes || []).map(n => [n.id, n]));
    const out = edges.filter(e => e.from === nodeId);
    const inc = edges.filter(e => e.to === nodeId);

    const expand = (edgeList, dir) => edgeList
      .map(e => ({ edge: e.type, node: nodeById.get(dir === 'out' ? e.to : e.from) }))
      .filter(x => x.node)
      .map(x => ({ relation: x.edge, ...x.node }));

    const sources = getTraceSources();

    // ① 上游：拥有该域的项目
    const owners = expand(inc, 'in').filter(x => x.relation === 'owns_domain')
      .map(x => ({ projectId: x.id, name: x.name, status: x.status }));

    // ② 下游分类展开
    const downstream = expand(out, 'out');
    const byKind = {
      engines: downstream.filter(x => x.relation === 'uses_engine' || x.relation === 'delegates_to'),
      algorithms: downstream.filter(x => x.relation === 'implements_algo'),
      dataAssets: downstream.filter(x => x.relation === 'persists_to' || x.relation === 'reads' || x.relation === 'writes'),
      documents: downstream.filter(x => x.relation === 'documented_by'),
      domains: downstream.filter(x => x.relation === 'owns_domain' || x.relation === 'flow_of')
    };

    // ③ 文档实体维度：映射到该域的 KB 实体（云端文档资源维度反向链）
    const kbEntities = sources.kbEntitiesByDomain ? sources.kbEntitiesByDomain(nodeId) : [];

    // ④ 归一化维度：需求运行中映射到该域的语句
    const requirements = sources.requirementsByDomain ? sources.requirementsByDomain(nodeId) : [];

    // ⑤ 代码维度：绑定实体 + 变更建议
    const codeBinding = sources.codeBinding ? sources.codeBinding(nodeId) : null;

    // ⑥ 流程维度：委托到该节点（引擎）的流程步骤
    const flowSteps = [];
    (view.nodes || []).forEach(n => {
      if (n.kind === 'flow_step') {
        const delegate = edges.find(e => e.from === n.id && e.to === nodeId && e.type === 'delegates_to');
        const belong = edges.find(e => e.from === n.id && e.to === nodeId && e.type === 'flow_of');
        if (delegate || belong) flowSteps.push({ stepId: n.id, name: n.name, flowName: n.flowName, relation: delegate ? 'delegates_to' : 'flow_of' });
      }
    });

    return {
      ok: true,
      node: { id: node.id, kind: node.kind, name: node.name, codePath: node.codePath || null },
      chain: {
        owners,
        engines: byKind.engines.map(x => ({ id: x.id, name: x.name, codePath: x.codePath })),
        algorithms: byKind.algorithms.map(x => ({ id: x.id, name: x.name, principle: x.principle, codePath: x.codePath })),
        dataAssets: byKind.dataAssets.map(x => ({ id: x.id, name: x.name })),
        documents: byKind.documents.map(x => ({ id: x.id, name: x.name, path: x.path })),
        flowSteps,
        kbEntities,
        requirements,
        codeBinding
      },
      counts: {
        owners: owners.length,
        engines: byKind.engines.length,
        algorithms: byKind.algorithms.length,
        dataAssets: byKind.dataAssets.length,
        documents: byKind.documents.length,
        flowSteps: flowSteps.length,
        kbEntities: kbEntities.length,
        requirements: requirements.length,
        codeEntities: codeBinding ? codeBinding.entityCount : 0
      },
      tracedAt: new Date().toISOString()
    };
  }

  /** 三维联动状态总览（前端导航卡数据源） */
  function getDimensionStatus() {
    const docCoverage = getDocGraphCoverage();
    const normStats = getNormalizationStats();
    const codeStats = getCodeBridgeStats();
    return {
      dimensions: [
        { key: 'cloudDocs', name: '云端文档资源维度', icon: 'Collection',
          primary: `${docCoverage.boundDocs}/${docCoverage.docs} 文档已图谱化`,
          secondary: `${docCoverage.entities} 实体 · ${docCoverage.domainBindings} 域映射`,
          coverage: round2(docCoverage.coverage) },
        { key: 'businessFlow', name: '业务流程与架构模块维度', icon: 'Operation',
          primary: `${normStats.domainsCovered}/${normStats.totalDomains} 域已归一化`,
          secondary: `${normStats.runs} 次运行 · ${normStats.statements} 条子需求`,
          coverage: round2(normStats.domainCoverage) },
        { key: 'localCode', name: '本地代码工程维度', icon: 'Monitor',
          primary: `${codeStats.bound}/${codeStats.units} 单元已绑定`,
          secondary: `${codeStats.codeEntities} 代码实体 · ${codeStats.inconsistent} 处不一致`,
          coverage: round2(codeStats.coverage) }
      ],
      pipeline: '云端文档沉淀 → 图谱关联建模 → 业务流程归一 → 模块项目化拆分 → 本地代码联动落地',
      generatedAt: new Date().toISOString()
    };
  }

  return { getDashboard, traceChain, getDimensionStatus };
}

function round2(x) { return Math.round((x || 0) * 100) / 100; }

module.exports = { createUnifiedGovernanceService };
