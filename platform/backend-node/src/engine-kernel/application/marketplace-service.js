'use strict';

/**
 * 三层插件商城用例（application 层 · mixin 用例族）
 * ------------------------------------------------------------------
 * 三层商城（L1-L3）：
 *   system 系统商城：随版本发布的内置引擎（全部自研，开箱即用）
 *   cloud  云端商城：云端插件目录（registryUrl 可指向任意注册表；安装即注册）
 *   local  本地商城：本地 JSON 清单安装的插件（落盘 engine_plugins.json）
 *
 * 安装语义（插件 kind）：
 *   llm-provider  → 注册为 ai-chat 槽位候选引擎（gateway.addProvider）
 *   web-search-key→ 写入 web-search 槽位引擎密钥（updateConfig）
 *   binding       → 仅记录槽位绑定（供 AI 配置/回放）
 */

const { readJSON, writeJSON } = require('../../lib/json-store');
const { SLOT_INDEX, MARKETPLACE_LAYERS } = require('../domain/contract-registry');
const { ADAPTERS, getPlugins, addPlugin, removePlugin, getBinding, setBinding } = require('../infrastructure/plugin-repository');

/** 云端注册表配置（engine_marketplace.json） */
function getMarketplaceConfig() {
  return readJSON('engine_marketplace.json', { registryUrl: '' });
}

function saveMarketplaceConfig(cfg) {
  writeJSON('engine_marketplace.json', cfg);
}

/** L1 系统商城：全部槽位的内置引擎（自动生成，永不过期） */
function listSystemMarket() {
  const items = [];
  Object.values(SLOT_INDEX).forEach(slot => {
    const adapter = ADAPTERS[slot.adapter];
    if (!adapter) return;
    adapter.list().forEach(c => {
      items.push({
        id: `system:${slot.id}:${c.id}`,
        slot: slot.id,
        engineId: c.id,
        name: c.name,
        layer: 'system',
        installed: true,
        active: !!c.active,
        description: `${slot.name} · 内置引擎（${c.provider || slot.adapter}）`
      });
    });
  });
  return items;
}

/** L2 云端商城：预置云端目录（LLM 预设 + 密钥型搜索引擎），支持 registryUrl 扩展 */
async function listCloudMarket() {
  const { PROVIDER_PRESETS } = require('../../llm-gateway');
  const items = [];

  // 云端 LLM 引擎目录（安装 = 注册 provider + 填 Key）
  Object.entries(PROVIDER_PRESETS).forEach(([provider, preset]) => {
    if (provider === 'custom') return;
    items.push({
      id: `cloud:ai-chat:${provider}`,
      slot: 'ai-chat',
      kind: 'llm-provider',
      name: preset.name,
      layer: 'cloud',
      installed: _isLLMProviderInstalled(provider),
      installConfig: { provider, base_url: preset.base_url, model: preset.models[0], name: preset.name },
      models: preset.models,
      description: preset.description
    });
  });

  // 云端密钥型搜索引擎目录
  const webEngines = ADAPTERS['web-search'].list().filter(c => ['tavily', 'bocha', 'searxng'].includes(c.id));
  webEngines.forEach(c => {
    items.push({
      id: `cloud:web-search:${c.id}`,
      slot: 'web-search',
      kind: 'web-search-key',
      name: c.name,
      layer: 'cloud',
      installed: _isWebSearchKeyConfigured(c.id),
      installConfig: { engine: c.id },
      description: '密钥型搜索引擎（安装后自动切换为当前引擎）'
    });
  });

  // 扩展：外部注册表目录（registryUrl 指向任意云端商城 JSON）
  const { registryUrl } = getMarketplaceConfig();
  if (registryUrl) {
    try {
      const remote = await _fetchJSON(registryUrl);
      (Array.isArray(remote) ? remote : remote.items || []).forEach(p => {
        items.push({ ...p, layer: 'cloud', external: true, installed: false });
      });
    } catch (e) {
      // 云端不可达时静默降级为预置目录（银行级可用性）
    }
  }
  return items;
}

/** L3 本地商城：本地安装的插件清单 */
function listLocalMarket() {
  const plugins = getPlugins();
  return plugins.map(p => ({ ...p, layer: 'local', installed: true, active: getBinding(p.slot) === p.engineId }));
}

/** 三层商城总览 */
async function getMarketplace() {
  return {
    layers: MARKETPLACE_LAYERS,
    system: listSystemMarket(),
    cloud: await listCloudMarket(),
    local: listLocalMarket()
  };
}

/**
 * 安装插件（三层统一入口）
 * @param {object} body { layer, slot, kind?, installConfig?, id? }
 *  - cloud 安装：kind + installConfig（llm-provider 需 api_key）
 *  - local 安装：manifest（本地 JSON 清单：{id, name, slot, kind, installConfig}）
 */
async function installPlugin(body) {
  const layer = body.layer || 'cloud';

  // ---- local 层：本地清单安装 ----
  if (layer === 'local') {
    const m = body.manifest || body;
    if (!m.id || !m.slot || !SLOT_INDEX[m.slot]) return { ok: false, error: '本地清单必须含 id/slot（且 slot 合法）' };
    const record = { id: m.id, name: m.name || m.id, slot: m.slot, kind: m.kind || 'binding', installConfig: m.installConfig || {}, layer: 'local' };
    const applied = await _applyInstall(record, m.installConfig || {});
    addPlugin(record);
    return { ok: true, layer: 'local', plugin: record, applied };
  }

  // ---- cloud 层：云端目录安装 ----
  if (layer === 'cloud') {
    const kind = body.kind || (body.installConfig && body.installConfig.provider ? 'llm-provider' : null);
    if (!kind) return { ok: false, error: 'cloud 安装需提供 kind 或 installConfig.provider' };
    const record = {
      id: body.id || `cloud:${body.slot}:${Date.now()}`,
      name: body.name || body.installConfig?.name || body.installConfig?.provider || '云端插件',
      slot: body.slot,
      kind,
      installConfig: body.installConfig || {},
      layer: 'cloud'
    };
    const applied = await _applyInstall(record, body.installConfig || {}, body.api_key);
    addPlugin(record);
    return { ok: true, layer: 'cloud', plugin: record, applied };
  }

  return { ok: false, error: `不支持的商业层: ${layer}（可用 system/cloud/local；system 内置无需安装）` };
}

/** 卸载插件（仅 local/cloud 安装的；system 内置不可卸载） */
async function uninstallPlugin(pluginId) {
  const plugins = getPlugins();
  const target = plugins.find(p => p.id === pluginId);
  if (!target) return { ok: false, error: `插件不存在: ${pluginId}` };
  if (target.layer === 'system') return { ok: false, error: '系统内置引擎不可卸载' };
  removePlugin(pluginId);
  return { ok: true, uninstalled: pluginId, slot: target.slot };
}

// ============ 内部：安装落点 ============

async function _applyInstall(record, installConfig, apiKey) {
  if (record.kind === 'llm-provider') {
    const gateway = require('../../llm-gateway').getGateway();
    const id = gateway.addProvider({
      ...installConfig,
      api_key: apiKey || installConfig.api_key || '',
      enabled: true
    });
    return { registeredEngineId: id, slot: 'ai-chat' };
  }
  if (record.kind === 'web-search-key') {
    const svc = require('../../web-search-service').getWebSearchService();
    svc.updateConfig({ engine: installConfig.engine, ...(installConfig.api_key ? { api_key: installConfig.api_key } : {}), ...(installConfig.base_url ? { base_url: installConfig.base_url } : {}) });
    setBinding('web-search', installConfig.engine);
    return { switchedEngine: installConfig.engine, slot: 'web-search' };
  }
  if (record.kind === 'binding') {
    if (installConfig.engineId) {
      setBinding(record.slot, installConfig.engineId);
      return { bound: installConfig.engineId, slot: record.slot };
    }
    return { note: '纯记录型插件（无引擎落点）' };
  }
  return { note: `未知 kind: ${record.kind}（仅记录）` };
}

function _isLLMProviderInstalled(provider) {
  const gateway = require('../../llm-gateway').getGateway();
  return gateway.listProviders().some(p => p.provider === provider);
}

function _isWebSearchKeyConfigured(engineId) {
  const svc = require('../../web-search-service').getWebSearchService();
  const cfg = svc.getConfig();
  return cfg.engine === engineId && !!cfg.api_key;
}

async function _fetchJSON(url) {
  const r = await fetch(url, { signal: AbortSignal.timeout(5000) });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return await r.json();
}

module.exports = { getMarketplace, listSystemMarket, listCloudMarket, listLocalMarket, installPlugin, uninstallPlugin, getMarketplaceConfig, saveMarketplaceConfig };
