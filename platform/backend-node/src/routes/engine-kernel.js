'use strict';

/**
 * 路由域：引擎内核（一切皆可插件化）
 * 槽位契约 + 瞬间切换 + 三层插件商城（系统/云端/本地）+ AI 自动配置。
 */
module.exports = function registerEngineKernelRoutes(ctx) {
  const { ok, fail, readBody, log, reg } = ctx;
  const kernel = require('../engine-kernel');

  // 槽位全景：契约摘要 + 当前绑定 + 候选引擎
  reg('get', '/engine-kernel/slots', (req, res) => {
    ok(res, { slots: kernel.getSlots() });
  });

  // 单槽位契约文档（接口规范原文）
  reg('get', '/engine-kernel/contracts/:slot', (req, res, params) => {
    const contract = kernel.getContract(params.slot);
    if (!contract) return fail(res, 404, `槽位不存在: ${params.slot}`);
    ok(res, contract);
  });

  // 当前绑定（持久化绑定 vs 适配器实时绑定一致性）
  reg('get', '/engine-kernel/bindings', (req, res) => {
    ok(res, kernel.getBindingsView());
  });

  // 瞬间切换引擎（校验→切换→探活→失败自动回滚）
  reg('post', '/engine-kernel/switch', async (req, res) => {
    const body = await readBody(req);
    if (!body.slot || !body.engineId) return fail(res, 400, 'slot 与 engineId 为必填');
    const result = await kernel.switchEngine(body.slot, body.engineId, { verify: body.verify });
    if (!result.ok) return fail(res, 400, result.error, result);
    ok(res, result);
  });

  // 契约兼容性预检（探活，不切换）
  reg('post', '/engine-kernel/validate', async (req, res) => {
    const body = await readBody(req);
    if (!body.slot || !body.engineId) return fail(res, 400, 'slot 与 engineId 为必填');
    ok(res, await kernel.validateEngine(body.slot, body.engineId));
  });

  // 三层插件商城（system/cloud/local）
  reg('get', '/engine-kernel/marketplace', async (req, res) => {
    ok(res, await kernel.getMarketplace());
  });

  // 云端注册表配置（可指向任意云端商城 JSON URL）
  reg('get', '/engine-kernel/marketplace/config', (req, res) => {
    ok(res, kernel.getMarketplaceConfig());
  });
  reg('post', '/engine-kernel/marketplace/config', async (req, res) => {
    const body = await readBody(req);
    if (!body.registryUrl) return fail(res, 400, 'registryUrl 为必填');
    kernel.saveMarketplaceConfig({ registryUrl: body.registryUrl });
    ok(res, kernel.getMarketplaceConfig());
  });

  // 安装插件（cloud: kind+installConfig / local: manifest）
  reg('post', '/engine-kernel/marketplace/install', async (req, res) => {
    const body = await readBody(req);
    const result = await kernel.installPlugin(body);
    if (!result.ok) return fail(res, 400, result.error, result);
    ok(res, result);
  });

  // 卸载插件（system 内置不可卸载）
  reg('post', '/engine-kernel/marketplace/uninstall', async (req, res) => {
    const body = await readBody(req);
    if (!body.id) return fail(res, 400, 'id 为必填');
    const result = await kernel.uninstallPlugin(body.id);
    if (!result.ok) return fail(res, 400, result.error, result);
    ok(res, result);
  });

  // AI 自动配置（自然语言需求 → 引擎绑定方案；dryRun 默认 true）
  reg('post', '/engine-kernel/ai-configure', async (req, res) => {
    const body = await readBody(req);
    if (!body.requirement) return fail(res, 400, 'requirement 为必填');
    const result = await kernel.aiConfigure(body.requirement, { dryRun: body.dryRun, llmEngineId: body.llmEngineId });
    if (!result.ok) return fail(res, 500, result.error, result);
    ok(res, result);
  });

  log('Engine kernel endpoints registered: slots, contracts, switch, validate, marketplace(3-layer), ai-configure');
};
