'use strict';

/**
 * 路由域：引擎宇宙图谱
 * 技术图谱管理所有引擎链接的统一查询入口 + 全链路验证。
 */
module.exports = function registerEngineUniverseRoutes(ctx) {
  const { ok, fail, log, reg } = ctx;
  const universe = require('../engine-universe');

  // 完整引擎宇宙图谱（节点+边+统计）
  reg('get', '/engine-universe', (req, res) => {
    ok(res, universe.getUniverse());
  });

  // 引擎清单（含关键功能描述，支持 category/capability 过滤）
  reg('get', '/engine-universe/engines', (req, res) => {
    const q = require('url').parse(req.url, true).query;
    ok(res, universe.listEngines({
      category: q.category || undefined,
      capability: q.capability || undefined
    }));
  });

  // 单引擎详情：上下游关系 + 服务需求 + 代码路径
  reg('get', '/engine-universe/engines/:id', (req, res, params) => {
    const detail = universe.getEngineDetail(params.id);
    if (!detail) return fail(res, 404, `引擎不存在: ${params.id}`);
    ok(res, detail);
  });

  // 链路追踪：任意两节点 BFS 最短路径（支持 edgeType 过滤）
  reg('get', '/engine-universe/trace', (req, res) => {
    const q = require('url').parse(req.url, true).query;
    if (!q.from || !q.to) return fail(res, 400, 'from 和 to 为必填');
    ok(res, universe.trace(q.from, q.to, q.type || null));
  });

  // 需求归一化链：每一环由哪些引擎服务
  reg('get', '/engine-universe/requirement-chain', (req, res) => {
    ok(res, universe.requirementChain());
  });

  // 全链路验证：代码路径/边完整性/需求链连通/降级链收敛/能力承接/无孤岛
  reg('get', '/engine-universe/verify', (req, res) => {
    ok(res, universe.verifyFullChain());
  });

  log('Engine universe endpoints registered: universe graph, engine detail, trace, requirement chain, full-chain verify');
};
