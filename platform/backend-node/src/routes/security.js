'use strict';

/**
 * 路由域：安全审计
 * /security/* API Key 管理、审计日志、安全校验
 */
module.exports = function registerSecurityRoutes(ctx) {
  const { url, security, ok, fail, readBody, log, appendLog, reg } = ctx;

  // ===== 安全与审计路由 =====
  
  reg('get', '/security/status', (req, res) => {
    ok(res, security.getSecurityStatus());
  });

  reg('get', '/security/api-keys', (req, res) => {
    ok(res, security.getApiKeys());
  });

  reg('post', '/security/api-keys', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    
    const key = security.createApiKey(body.name, body.permissions || ['read']);
    appendLog({ type: 'security', msg: 'API key created', keyId: key.id });
    ok(res, key);
  });

  reg('delete', '/security/api-keys/:id', async (req, res, params) => {
    const revoked = security.revokeApiKey(params.id);
    if (revoked) {
      appendLog({ type: 'security', msg: 'API key revoked', keyId: params.id });
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Key not found');
    }
  });

  reg('get', '/security/audit-log', (req, res) => {
    const q = url.parse(req.url, true).query;
    const filters = {
      action: q.action,
      actor: q.actor,
      since: q.since,
      limit: parseInt(q.limit) || 100
    };
    ok(res, security.getAuditLog(filters));
  });

  reg('post', '/security/validate', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.api_key) return fail(res, 400, 'api_key required');
    
    const result = security.validateApiKey(body.api_key);
    ok(res, result);
  });

};
