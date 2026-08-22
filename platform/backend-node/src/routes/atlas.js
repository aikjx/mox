'use strict';

/**
 * 路由域：项目全息图谱（Project Atlas）
 * 整个项目机器图谱化：24 业务域 + 4 模块 + 18 引擎 + 15 算法 + 34 数据资产 + 34 文档，
 * 全部关联本地代码路径；AI 对话经专家联盟架构师专家图谱增强回答。
 */
module.exports = function registerAtlasRoutes(ctx) {
  const { ok, fail, readBody, log, reg } = ctx;
  const atlas = require('../project-atlas');
  const { getAlliance } = require('../expert-alliance');

  // 完整全息图谱（129 节点 + 173 边 + 统计）
  reg('get', '/atlas', (req, res) => {
    ok(res, atlas.getAtlas());
  });

  // 无破窗验证（145 项：动态比对路由域/数据目录/代码路径/文档/连通性）
  reg('get', '/atlas/verify', (req, res) => {
    ok(res, atlas.verifyAtlas());
  });

  // 单域全景：功能/引擎/算法/数据/文档一屏尽览
  reg('get', '/atlas/domains/:id', (req, res, params) => {
    const detail = atlas.getDomainDetail(params.id);
    if (!detail) return fail(res, 404, `业务域不存在: ${params.id}`);
    ok(res, detail);
  });

  // 影响面分析：改动一个节点波及哪些引擎/算法/数据/文档
  reg('get', '/atlas/impact/:id', (req, res, params) => {
    const result = atlas.impact(params.id);
    if (!result) return fail(res, 404, `图谱节点不存在: ${params.id}`);
    ok(res, result);
  });

  // 图谱资产检索（自然语言关键词）
  reg('get', '/atlas/search', (req, res) => {
    const q = require('url').parse(req.url, true).query;
    if (!q.q) return fail(res, 400, 'q 为必填');
    ok(res, atlas.searchAtlas(q.q));
  });

  // AI 图谱对话：架构师专家 + 全息图谱上下文增强回答
  reg('post', '/atlas/consult', async (req, res) => {
    const body = await readBody(req);
    if (!body.question) return fail(res, 400, 'question 为必填');
    try {
      const alliance = getAlliance();
      const result = await alliance.consultAtlas(body.question, {
        temperature: body.temperature,
        problemContext: body.context
      });
      ok(res, result);
    } catch (e) {
      fail(res, 500, `图谱咨询失败: ${e.message}`);
    }
  });

  log('Project atlas endpoints registered: graph, verify, domain detail, impact, search, AI consult');
};
