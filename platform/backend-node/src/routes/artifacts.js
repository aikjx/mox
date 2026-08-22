'use strict';

/**
 * 路由域：本地制品
 * /ai/artifact/* 文档与代码制品创建
 */
module.exports = function registerArtifactsRoutes(ctx) {
  const { artifactService, config, ok, fail, readBody, appendLog, reg } = ctx;

  // ==================== 本地制品引擎（文档/代码自动创建） ====================
  reg('get', '/ai/artifact/config', async (req, res) => {
    ok(res, artifactService.getConfig());
  });

  reg('get', '/ai/artifact/list', async (req, res) => {
    ok(res, artifactService.listArtifacts());
  });

  reg('post', '/ai/artifact/create', async (req, res) => {
    const body = await readBody(req);
    if (!body.message || !String(body.message).trim()) {
      fail(res, 400, '缺少 message 参数');
      return;
    }
    if (body.artifact_mode !== 'document' && body.artifact_mode !== 'code') {
      fail(res, 400, 'artifact_mode 必须为 document 或 code');
      return;
    }
    try {
      const result = await artifactService.process({
        mode: body.artifact_mode,
        message: body.message,
        session_id: body.session_id || body.sessionId || null,
        overwrite: !!body.overwrite
      });
      appendLog({
        type: 'artifact',
        msg: 'create',
        mode: body.artifact_mode,
        created: result.created.length,
        skipped: result.skipped.length
      });
      ok(res, result);
    } catch (e) {
      fail(res, 500, '制品创建失败: ' + e.message);
    }
  });

};
