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

  // G4 修复：AI 引擎四端点（process/analyze/capabilities/metrics）已统一收敛到 ai-engine.js 域
  //   本域（auto-dev）不再跨域注册 engine 核心端点，避免域注册顺序导致的后注册覆盖语义风险。
  //   注：endpoint 路径仍是 /ai/engine/*，由 routes/index.js 中先注册的 ai-engine 域（序37）承载。

};
