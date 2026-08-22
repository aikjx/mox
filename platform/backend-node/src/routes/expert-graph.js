'use strict';

/**
 * 路由域：专家图谱
 * /expert-graph/* 能力图谱、协作网络、最优组队
 */
module.exports = function registerExpertGraphRoutes(ctx) {
  const { path, url, expertGraph, ok, fail, readBody, reg } = ctx;

  // ===== 专家能力图谱与协作网络 =====
  reg('get', '/expert-graph/stats', (req, res) => {
    ok(res, expertGraph.getGraphStats());
  });

  reg('get', '/expert-graph', (req, res) => {
    ok(res, expertGraph.export());
  });

  reg('get', '/expert-graph/neighbors/:id', (req, res, params) => {
    ok(res, { expert_id: params.id, neighbors: expertGraph.getNeighbors(params.id) });
  });

  reg('get', '/expert-graph/collaborators/:id', (req, res, params) => {
    const limit = parseInt(url.parse(req.url, true).query.limit) || 5;
    ok(res, { expert_id: params.id, collaborators: expertGraph.findTopCollaborators(params.id, limit) });
  });

  reg('get', '/expert-graph/path/:source/:target', (req, res, params) => {
    ok(res, { source: params.source, target: params.target, ...expertGraph.getCollaborationPath(params.source, params.target) });
  });

  reg('get', '/expert-graph/communities', (req, res) => {
    ok(res, { communities: expertGraph.detectCommunities() });
  });

  reg('post', '/expert-graph/optimal-team', async (req, res) => {
    const body = await readBody(req);
    try {
      ok(res, expertGraph.findOptimalTeam(body.question || '', body.size || 3));
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/expert-graph/rebuild', (req, res) => {
    ok(res, { success: true, stats: expertGraph.rebuild() });
  });

};
