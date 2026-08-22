'use strict';

/**
 * 路由域：服务管理
 * /services/* 服务启停、日志、批量操作
 */
module.exports = function registerServicesRoutes(ctx) {
  const { serviceManager, ok, fail, readBody, reg } = ctx;

  // ===== 服务管理路由 =====
  reg('get', '/services', async (req, res) => {
    try {
      const status = await serviceManager.getAllStatus();
      ok(res, status);
    } catch (e) {
      console.error('[services-list]', e);
      fail(res, 500, '获取服务列表失败: ' + e.message);
    }
  });

  reg('get', '/services/:id', async (req, res, params) => {
    const serviceId = params.id;
    try {
      const status = await serviceManager.getServiceStatus(serviceId);
      ok(res, status);
    } catch (e) {
      console.error('[service-status]', e);
      fail(res, 500, '获取服务状态失败: ' + e.message);
    }
  });

  reg('post', '/services/:id/start', async (req, res, params) => {
    const serviceId = params.id;
    const body = await readBody(req);
    try {
      const result = await serviceManager.startService(serviceId, body.options || {});
      ok(res, result);
    } catch (e) {
      console.error('[service-start]', e);
      fail(res, 500, '启动服务失败: ' + e.message);
    }
  });

  reg('post', '/services/:id/stop', async (req, res, params) => {
    const serviceId = params.id;
    try {
      const result = await serviceManager.stopService(serviceId);
      ok(res, result);
    } catch (e) {
      console.error('[service-stop]', e);
      fail(res, 500, '停止服务失败: ' + e.message);
    }
  });

  reg('post', '/services/:id/restart', async (req, res, params) => {
    const serviceId = params.id;
    try {
      const result = await serviceManager.restartService(serviceId);
      ok(res, result);
    } catch (e) {
      console.error('[service-restart]', e);
      fail(res, 500, '重启服务失败: ' + e.message);
    }
  });

  reg('get', '/services/:id/logs', async (req, res, params) => {
    const serviceId = params.id;
    const lines = parseInt(req.query?.lines || '50', 10);
    try {
      const logs = serviceManager.getServiceLog(serviceId, lines);
      ok(res, { serviceId, lines, logs });
    } catch (e) {
      console.error('[service-logs]', e);
      fail(res, 500, '获取服务日志失败: ' + e.message);
    }
  });

  reg('post', '/services/:id/logs/clear', (req, res, params) => {
    const serviceId = params.id;
    try {
      const result = serviceManager.clearServiceLog(serviceId);
      ok(res, { success: result, serviceId });
    } catch (e) {
      console.error('[service-logs-clear]', e);
      fail(res, 500, '清理服务日志失败: ' + e.message);
    }
  });

  reg('post', '/services/batch/start', async (req, res) => {
    const body = await readBody(req);
    const ids = body.services || [];
    try {
      const result = await serviceManager.batchStart(ids.length > 0 ? ids : null);
      ok(res, result);
    } catch (e) {
      console.error('[batch-start]', e);
      fail(res, 500, '批量启动失败: ' + e.message);
    }
  });

  reg('post', '/services/batch/stop', async (req, res) => {
    const body = await readBody(req);
    const ids = body.services || [];
    try {
      const result = await serviceManager.batchStop(ids.length > 0 ? ids : null);
      ok(res, result);
    } catch (e) {
      console.error('[batch-stop]', e);
      fail(res, 500, '批量停止失败: ' + e.message);
    }
  });

  reg('post', '/services/batch/restart', async (req, res) => {
    const body = await readBody(req);
    const ids = body.services || [];
    try {
      const result = await serviceManager.batchRestart(ids.length > 0 ? ids : null);
      ok(res, result);
    } catch (e) {
      console.error('[batch-restart]', e);
      fail(res, 500, '批量重启失败: ' + e.message);
    }
  });

  reg('post', '/services/start-all', async (req, res) => {
    try {
      const result = await serviceManager.batchStart();
      ok(res, result);
    } catch (e) {
      console.error('[start-all]', e);
      fail(res, 500, '一键启动所有服务失败: ' + e.message);
    }
  });

  reg('post', '/services/stop-all', async (req, res) => {
    try {
      const result = await serviceManager.batchStop();
      ok(res, result);
    } catch (e) {
      console.error('[stop-all]', e);
      fail(res, 500, '一键停止所有服务失败: ' + e.message);
    }
  });

};
