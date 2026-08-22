'use strict';

/**
 * 路由域：自动开发引擎
 * /ai/engine/auto-dev/* 需求 → 架构图谱 → 代码渲染 → 预览
 */
module.exports = function registerAutoDevRoutes(ctx) {
  const { engineCore, artifactService, ok, fail, readBody, appendLog, reg, getAutoDevEngine } = ctx;

  // ===== 自动开发引擎路由（需求 → 业务架构图谱 → 代码 → 预览） =====
  const autoDevEngine = getAutoDevEngine();

  // POST /ai/engine/auto-dev —— 一句话需求全自动开发（架构图谱→代码渲染→落盘）
  reg('post', '/ai/engine/auto-dev', async (req, res) => {
    const body = await readBody(req);
    if (!body.requirement) {
      fail(res, 400, '缺少 requirement 参数（例如：开发一个企业官网）');
      return;
    }
    try {
      const result = await autoDevEngine.develop(body);
      appendLog({ type: 'auto-dev', msg: 'develop complete', project: result.project, files: result.files.length });
      ok(res, result);
    } catch (e) {
      console.error('[auto-dev]', e);
      appendLog({ type: 'auto-dev', msg: 'develop failed', error: e.message });
      fail(res, 500, '自动开发失败: ' + e.message);
    }
  });

  // GET /ai/engine/auto-dev/projects —— 已生成项目列表
  reg('get', '/ai/engine/auto-dev/projects', (req, res) => {
    try {
      ok(res, autoDevEngine.listProjects());
    } catch (e) {
      fail(res, 500, '获取项目列表失败: ' + e.message);
    }
  });

  // GET /ai/engine/auto-dev/preview/:project/:file —— 生成站点在线预览（安全静态服务）
  reg('get', '/ai/engine/auto-dev/preview/:project/:file', async (req, res, params) => {
    const rel = `${params.project}/${params.file}`;
    const result = artifactService.readFileSafe(rel);
    if (!result.ok) {
      res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('Not Found: ' + result.reason);
      return;
    }
    res.writeHead(200, {
      'Content-Type': result.content_type,
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff'
    });
    res.end(result.content);
  });

  // POST /ai/engine/analyze —— 显式能力执行（跳过意图识别，可预测）
  reg('post', '/ai/engine/analyze', async (req, res) => {
    const body = await readBody(req);
    if (!body.capability) {
      fail(res, 400, '缺少 capability 参数');
      return;
    }
    try {
      const result = await engineCore.executeCapability(body.capability, body.question, body.options);
      ok(res, result);
    } catch (e) {
      console.error('[engine-core-analyze]', e);
      fail(res, 400, e.message);
    }
  });

  // GET /ai/engine/capabilities —— 能力矩阵自描述
  reg('get', '/ai/engine/capabilities', (req, res) => {
    try {
      ok(res, engineCore.getCapabilities());
    } catch (e) {
      console.error('[engine-core-capabilities]', e);
      fail(res, 500, '获取能力矩阵失败: ' + e.message);
    }
  });

  // GET /ai/engine/metrics —— 性能指标（成功率/降级率/平均延迟）
  reg('get', '/ai/engine/metrics', (req, res) => {
    try {
      ok(res, engineCore.getMetrics());
    } catch (e) {
      console.error('[engine-core-metrics]', e);
      fail(res, 500, '获取引擎指标失败: ' + e.message);
    }
  });

};
