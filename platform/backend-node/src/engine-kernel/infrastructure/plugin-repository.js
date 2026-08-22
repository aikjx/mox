'use strict';

/**
 * 引擎适配器仓储（infrastructure 层 · 唯一触碰各引擎子系统与插件持久化的位置）
 * ------------------------------------------------------------------
 * 职责：
 *   1. 四类引擎适配器：list（候选清单）/ current（当前绑定）/ apply（切换）/ health（契约探活）
 *   2. 槽位绑定持久化：engine_bindings.json（复用 lib/json-store 单一真相源）
 *   3. 插件记录持久化：engine_plugins.json（本地安装的插件清单）
 *
 * 适配器是「代码之间只是调用」的落点：槽位契约 → 适配器 → 具体引擎子系统。
 * 换引擎 = 换绑定；新增引擎类型 = 新增一个 adapter，契约与调用方不变。
 */

const { readJSON, writeJSON } = require('../../lib/json-store');

// ============ 适配器注册表 ============

/**
 * llm-gateway 适配器（AI 对话引擎）
 * 延迟 require：运行时才触碰网关单例，避免装载期环。
 */
const llmGatewayAdapter = {
  list() {
    const gateway = require('../../llm-gateway').getGateway();
    return gateway.listProviders().map(p => ({
      id: p.id,
      name: p.name,
      active: !!p.active,
      enabled: p.enabled,
      provider: p.provider
    }));
  },
  current() {
    const gateway = require('../../llm-gateway').getGateway();
    return gateway.activeProvider;
  },
  apply(engineId) {
    const gateway = require('../../llm-gateway').getGateway();
    return gateway.setActiveProvider(engineId);
  },
  async health(engineId) {
    const gateway = require('../../llm-gateway').getGateway();
    const r = await gateway.testConnection(engineId);
    return r;
  }
};

/** 存储引擎适配器（SQLite/MySQL/PostgreSQL） */
const storageAdapter = {
  list() {
    const { listProviders } = require('../../config');
    return listProviders().map(p => ({
      id: p.name,
      name: p.name,
      active: !!p.current,
      enabled: true,
      provider: p.type
    }));
  },
  current() {
    const { config } = require('../../config');
    return config.storage.provider;
  },
  apply(engineId) {
    const { switchProvider } = require('../../config');
    switchProvider(engineId);
    return true;
  },
  async health() {
    // 存储链路探活：同数据往返写（文件 + SQLite 双写真实落盘，不产生探针文件）
    const t0 = Date.now();
    const bindings = readJSON('engine_bindings.json', {});
    writeJSON('engine_bindings.json', bindings);
    const back = readJSON('engine_bindings.json', null);
    return { ok: back !== null, latency_ms: Date.now() - t0 };
  }
};

/** 联网搜索适配器（Bing/DuckDuckGo/Tavily/博查/SearXNG） */
const webSearchAdapter = {
  list() {
    const svc = require('../../web-search-service').getWebSearchService();
    const engines = svc.getEngines() || []; // Array<{id, name, needKey, description}>
    const cur = svc.getConfig().engine;
    return engines.map(e => ({
      id: e.id,
      name: e.name || e.id,
      active: e.id === cur,
      enabled: !e.needKey || svc.isReady(),
      provider: 'web-search'
    }));
  },
  current() {
    const svc = require('../../web-search-service').getWebSearchService();
    return svc.getConfig().engine;
  },
  apply(engineId) {
    const svc = require('../../web-search-service').getWebSearchService();
    svc.updateConfig({ engine: engineId });
    return true;
  },
  async health() {
    const svc = require('../../web-search-service').getWebSearchService();
    return await svc.test();
  }
};

/** 音高检测适配器（Python FastAPI 子项目，backend 表单字段注入） */
const pitchDetectionAdapter = {
  CANDIDATES: [
    { id: 'auto', name: '自动降级（crepe_onnx→pyin）', provider: 'melody2score' },
    { id: 'crepe_onnx', name: 'CREPE ONNX（高精度神经网络）', provider: 'melody2score' },
    { id: 'pyin', name: 'pYIN（经典概率 YIN）', provider: 'melody2score' },
    { id: 'torchcrepe', name: 'TorchCrepe（PyTorch 原生）', provider: 'melody2score' }
  ],
  list() {
    const cur = this.current();
    return this.CANDIDATES.map(c => ({ ...c, active: c.id === cur, enabled: true }));
  },
  current() {
    return getBinding('pitch-detection') || 'auto';
  },
  apply(engineId) {
    if (!this.CANDIDATES.some(c => c.id === engineId)) return false;
    setBinding('pitch-detection', engineId);
    return true;
  },
  async health() {
    const http = require('http');
    const host = process.env.MELODY2SCORE_HOST || '127.0.0.1';
    const port = parseInt(process.env.MELODY2SCORE_PORT || '3008', 10);
    return await new Promise((resolve) => {
      const req = http.get({ hostname: host, port, path: '/api/melody2score/health', timeout: 3000 }, (res) => {
        resolve({ ok: res.statusCode === 200, latency_ms: 0 });
        res.resume();
      });
      req.on('error', () => resolve({ ok: false, latency_ms: -1 }));
      req.on('timeout', () => { req.destroy(); resolve({ ok: false, latency_ms: -1 }); });
    });
  }
};

const ADAPTERS = {
  'llm-gateway': llmGatewayAdapter,
  'storage-config': storageAdapter,
  'web-search': webSearchAdapter,
  'melody2score': pitchDetectionAdapter
};

// ============ 绑定持久化 ============

function getBindings() {
  return readJSON('engine_bindings.json', {});
}

// 内核数据文件幂等初始化（保证 W2 无破窗：注册表 ↔ data/ 目录双向一致）
(function _initKernelFiles() {
  // 自愈：对象形内核文件（engine_bindings/engine_marketplace）此前若被误写进
  // SQLite 实体表（readJSON 会得到数组包裹），迁移到 kv 单对象通道并清理污染行。
  try {
    const storage = require('../../storage').getStorage();
    ['engine_bindings', 'engine_marketplace'].forEach(t => {
      if (storage.kvGet(t, null) === null) {
        const polluted = storage.getList(t);
        if (polluted && polluted.length > 0) {
          storage.kvSet(t, polluted[0]);
          storage.deleteByType(t);
        }
      }
    });
  } catch (e) { /* 自愈尽力而为，不阻断内核装载 */ }
  if (readJSON('engine_bindings.json', null) === null) writeJSON('engine_bindings.json', {});
  if (readJSON('engine_plugins.json', null) === null) writeJSON('engine_plugins.json', []);
  if (readJSON('engine_marketplace.json', null) === null) writeJSON('engine_marketplace.json', { registryUrl: '' });
})();

function getBinding(slot) {
  return getBindings()[slot] || null;
}

function setBinding(slot, engineId) {
  const bindings = getBindings();
  bindings[slot] = engineId;
  writeJSON('engine_bindings.json', bindings);
  return bindings;
}

// ============ 插件记录持久化 ============

function getPlugins() {
  return readJSON('engine_plugins.json', []);
}

function savePlugins(plugins) {
  writeJSON('engine_plugins.json', plugins);
}

function addPlugin(plugin) {
  const plugins = getPlugins();
  const existing = plugins.findIndex(p => p.id === plugin.id);
  if (existing >= 0) plugins[existing] = { ...plugins[existing], ...plugin, updated_at: new Date().toISOString() };
  else plugins.push({ ...plugin, created_at: new Date().toISOString(), updated_at: new Date().toISOString() });
  savePlugins(plugins);
  return plugin;
}

function removePlugin(pluginId) {
  const plugins = getPlugins();
  const idx = plugins.findIndex(p => p.id === pluginId);
  if (idx < 0) return false;
  plugins.splice(idx, 1);
  savePlugins(plugins);
  return true;
}

module.exports = {
  ADAPTERS,
  getBindings, getBinding, setBinding,
  getPlugins, addPlugin, removePlugin
};
