'use strict';

/**
 * 路由域：联网搜索
 * /web-search/* 搜索引擎配置、连通测试与搜索执行
 */
module.exports = function registerWebSearchRoutes(ctx) {
  const { webSearchService, config, ok, fail, readBody, appendLog, reg } = ctx;

  // ==================== 联网搜索配置 ====================
  reg('get', '/web-search/config', async (req, res) => {
    ok(res, {
      config: webSearchService.getConfig(),
      engines: webSearchService.getEngines(),
      ready: webSearchService.isReady()
    });
  });

  reg('post', '/web-search/config', async (req, res) => {
    const body = await readBody(req);
    try {
      const config = webSearchService.updateConfig(body);
      ok(res, { config, ready: webSearchService.isReady() });
    } catch (e) {
      appendLog({ type: 'web-search', msg: 'config update failed', error: e.message });
      fail(res, 400, '联网搜索配置保存失败: ' + e.message);
    }
  });

  reg('post', '/web-search/test', async (req, res) => {
    const result = await webSearchService.test();
    appendLog({ type: 'web-search', msg: 'test', success: result.success, message: result.message });
    ok(res, result);
  });

  reg('post', '/web-search', async (req, res) => {
    const body = await readBody(req);
    if (!body.query || !String(body.query).trim()) {
      fail(res, 400, '缺少 query 参数');
      return;
    }
    try {
      const result = await webSearchService.search(body.query);
      ok(res, result);
    } catch (e) {
      fail(res, 500, '搜索失败: ' + e.message);
    }
  });

};
