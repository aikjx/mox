'use strict';

/**
 * 璇玑算子统一智能平台 · API 服务入口（组合根）
 *
 * 架构分层（企业级规范化）：
 *   本文件            组合根：依赖装配 + 自研路由器 + HTTP 服务器（不含业务路由）
 *   src/lib/          跨域共享基础设施：http 响应 / json 存储 / 日志 / 图算法
 *   src/routes/       23 个业务域路由模块（routes/index.js 装配清单登记）
 *   src/modules/      可插拔 16 模块系统（installAll 自动注册）
 *   src/<engine>.js   领域引擎单例（llm-gateway / expert-alliance / ai-engine / ...）
 *
 * 业务处理流程不变式：请求 → 鉴权(OUS_API_TOKEN) → match 路由 → 域 handler → ok/fail 统一响应。
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');

// ===== 引擎与领域服务（依赖方向：网关 → 领域引擎，无环）=====
const { getGateway } = require('./llm-gateway');
const { getAlliance } = require('./expert-alliance');
const { getSessionStore } = require('./session-store');
const { getDispatcher, STRATEGY_TYPES } = require('./expert-dispatcher');
const { getExpertGraph } = require('./expert-graph');
const { getStorage } = require('./storage');
const { getAIEngine } = require('./ai-engine');
const { getAIIntegrationEngine } = require('./ai-integration-engine');
const { getUltimateEngine } = require('./ultimate-ai-engine');
const { getAllianceEngine } = require('./expert-alliance-engine');
const { getServiceManager } = require('./service-manager');
const { getWebSearchService } = require('./web-search-service');
const { getInfiniteOptimizer } = require('./infinite-dimension-optimizer');
const { getLocalArtifactService } = require('./local-artifact-service');
const { getAIEngineCore } = require('./ai-engine-core');
const { getAutoDevEngine } = require('./auto-dev-engine');
const { getSecurityManager } = require('./security');
const { config, DATA_DIR } = require('./config');
const { uid } = require('./utils');

// 加载模块化系统（自动注册模块）
require('./modules/graph');
require('./modules/task');
require('./modules/storage');
require('./modules/melody2score');
const modules = require('./modules');

// ===== 跨域共享基础设施（src/lib/，经 ctx 注入路由域）=====
const { send, ok, fail, readBody } = require('./lib/http');
const { p, readJSON, writeJSON } = require('./lib/json-store');
const { log, appendLog } = require('./lib/logger');
const {
  graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality,
  labelPropagation, activateSpread
} = require('./lib/graph-algos');

const PORT = config.app.port;

// ===== 引擎单例装配（顺序即依赖序）=====
const gateway = getGateway();
const alliance = getAlliance();
const sessionStore = getSessionStore();
const dispatcher = getDispatcher(alliance);
const expertGraph = getExpertGraph(alliance);
const storage = getStorage();
const aiEngine = getAIEngine(gateway);
const aiIntegration = getAIIntegrationEngine();
const ultimateEngine = getUltimateEngine();
const engineCore = getAIEngineCore();
const serviceManager = getServiceManager();
const webSearchService = getWebSearchService();
const infiniteOptimizer = getInfiniteOptimizer();
const artifactService = getLocalArtifactService();
const security = getSecurityManager();

// ===== 自研 HTTP 路由器（注册 + 参数化匹配，handler 第三参为 params）=====
const handlers = {
  get: {},
  post: {},
  put: {},
  patch: {},
  delete: {}
};

function reg(method, pattern, fn) {
  handlers[method][pattern] = fn;
}

function match(method, urlPath) {
  const map = handlers[method];
  if (!map) return null;
  // 企业级匹配语义：静态路由优先于参数化路由（参数段少者优先），
  // 同为静态/同参数数时长路径优先（保留前缀精确匹配语义）。
  // 修复缺陷：纯长度排序会让 /res/:id 抢先 /res/stats 类静态子路径。
  const dynCount = (k) => (k.match(/\/:/g) || []).length;
  const keys = Object.keys(map).sort((a, b) => {
    const da = dynCount(a), db = dynCount(b);
    if (da !== db) return da - db;
    return b.length - a.length;
  });
  for (let i = 0; i < keys.length; i++) {
    const k = keys[i];
    if (k === urlPath) return { fn: map[k], params: {} };
    const kp = k.split('/');
    const up = urlPath.split('/');
    if (kp.length !== up.length) continue;
    const params = {};
    let ok = true;
    for (let j = 0; j < kp.length; j++) {
      if (kp[j].startsWith(':')) {
        params[kp[j].slice(1)] = decodeURIComponent(up[j]);
      } else if (kp[j] !== up[j]) {
        ok = false; break;
      }
    }
    if (ok) return { fn: map[k], params: params };
  }
  return null;
}

// ===== 路由域装配（组合根依赖注入 → 23 个业务域）=====
function registerRoutes() {
  const ctx = {
    // Node 内置
    http, fs, path, url,
    // 基础设施
    reg, send, ok, fail, readBody, p, readJSON, writeJSON, log, appendLog,
    // 引擎单例
    gateway, alliance, sessionStore, dispatcher, expertGraph, storage,
    aiEngine, aiIntegration, ultimateEngine, engineCore, serviceManager,
    webSearchService, infiniteOptimizer, artifactService, security, modules,
    // 配置与工厂
    config, DATA_DIR, PORT, uid, getAllianceEngine, getAutoDevEngine,
    // 图算法库
    graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality,
    labelPropagation, activateSpread
  };
  const { registerAllRoutes } = require('./routes');
  registerAllRoutes(ctx);
  // 可插拔模块路由注册（在业务域之后，可覆盖同路径默认实现）
  modules.installAll(reg);
}

registerRoutes();

// ===== HTTP 服务器（鉴权 → 路由分发 → 统一错误响应）=====
const server = http.createServer(async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET,POST,PUT,DELETE,OPTIONS,PATCH');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type,Authorization,Accept,X-Requested-With,Origin');
  res.setHeader('Access-Control-Max-Age', '86400');
  if (req.method === 'OPTIONS') { res.writeHead(204); return res.end(); }

  const parsed = url.parse(req.url, true);
  const pathname = parsed.pathname.replace(/\/+$/, '') || '/';
  const method = req.method.toLowerCase();

  if (parsed.query.pretty !== undefined) res._pretty = true;

  const matched = match(method, pathname);
  if (matched) {
    try {
      await matched.fn(req, res, matched.params);
    } catch (e) {
      console.error('[handler-error]', pathname, e);
      fail(res, 500, 'internal error: ' + e.message);
    }
  } else {
    fail(res, 404, 'not found: ' + req.method + ' ' + pathname);
  }
});

server.listen(PORT, () => {
  console.log('[api-server] 璇玑系统 API server running on http://localhost:' + PORT);
});

module.exports = server;
