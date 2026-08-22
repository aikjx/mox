'use strict';

/**
 * 路由域：无穷维度优化
 * /ai/infinite-optimize/* CEM 寻优、多引擎对比与最优配置应用
 */
module.exports = function registerOptimizerRoutes(ctx) {
  const { infiniteOptimizer, config, ok, fail, readBody, appendLog, reg } = ctx;

  // ==================== 无穷维度优化引擎 ====================
  reg('get', '/ai/infinite-optimize/benchmarks', async (req, res) => {
    ok(res, { benchmarks: infiniteOptimizer.getBenchmarks(), objective_weights: require('../infinite-dimension-optimizer').OBJECTIVE_WEIGHTS });
  });

  reg('post', '/ai/infinite-optimize/start', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = infiniteOptimizer.start(body || {});
      appendLog({ type: 'infinite-optimize', msg: 'started', run_id: result.run_id, dimensions: result.dimensions });
      ok(res, result);
    } catch (e) {
      fail(res, 400, e.message);
    }
  });

  reg('post', '/ai/infinite-optimize/stop', async (req, res) => {
    ok(res, infiniteOptimizer.stop());
  });

  reg('get', '/ai/infinite-optimize/status', async (req, res) => {
    ok(res, infiniteOptimizer.getStatus());
  });

  reg('get', '/ai/infinite-optimize/results', async (req, res) => {
    ok(res, infiniteOptimizer.getResults());
  });

  reg('post', '/ai/infinite-optimize/compare', async (req, res) => {
    try {
      const result = await infiniteOptimizer.runComparison();
      appendLog({ type: 'infinite-optimize', msg: 'comparison done', engines: result.rows.filter((r) => r.configured).length });
      ok(res, result);
    } catch (e) {
      fail(res, 500, '引擎对比失败: ' + e.message);
    }
  });

  reg('get', '/ai/infinite-optimize/comparison', async (req, res) => {
    const result = infiniteOptimizer.getComparison();
    if (!result) {
      ok(res, { at: null, rows: [], note: '尚未运行对比，请先调用 POST /ai/infinite-optimize/compare' });
      return;
    }
    ok(res, result);
  });

  reg('post', '/ai/infinite-optimize/apply', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = infiniteOptimizer.applyBest(body && body.run_id);
      appendLog({ type: 'infinite-optimize', msg: 'applied best config', run_id: result.run_id, applied: result.applied });
      ok(res, result);
    } catch (e) {
      fail(res, 400, e.message);
    }
  });

};
