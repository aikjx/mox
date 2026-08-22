'use strict';

/**
 * 路由域：模块与存储管理
 * /modules 模块清单 / /storage/* 存储提供方切换
 */
module.exports = function registerModulesAdminRoutes(ctx) {
  const { storage, modules, config, ok, fail, readBody, reg } = ctx;

  // ===== 模块化系统管理 =====
  reg('get', '/modules', (req, res) => {
    const { listModules } = require('../modules');
    ok(res, listModules().map(m => ({
      name: m.name,
      description: m.options?.description || '',
      version: m.options?.version || '1.0',
      routes: m.routes ? m.routes.length : 0
    })));
  });

  reg('get', '/storage/providers', (req, res) => {
    const { listProviders } = require('../storage');
    ok(res, listProviders());
  });

  reg('post', '/storage/switch', async (req, res) => {
    const body = await readBody(req);
    const provider = body.provider;
    if (!provider) return fail(res, 400, 'provider 为必填项');
    try {
      const { switchDatabase } = require('../storage');
      const newStorage = switchDatabase(provider);
      ok(res, { success: true, provider: newStorage.name, message: `已切换到 ${provider}` });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/storage/status', (req, res) => {
    const s = getStorage();
    const all = s.listAllEntities();
    const byType = {};
    all.forEach(e => { byType[e.type] = (byType[e.type] || 0) + 1; });
    ok(res, {
      provider: config.storage.provider,
      name: s.name,
      totalEntities: all.length,
      entitiesByType: Object.entries(byType).map(([type, cnt]) => ({ entity_type: type, cnt })),
      features: config.features
    });
  });

};
