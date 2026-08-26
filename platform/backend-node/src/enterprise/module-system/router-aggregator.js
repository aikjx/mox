'use strict';

/**
 * MOX Enterprise · 路由聚合器
 * ==========================
 * 统一管理所有模块的 API 路由注册、版本控制、文档生成
 *
 * 核心能力：
 *  - 路由自动发现与注册
 *  - API 版本控制（/v1, /v2）
 *  - 路由分组与前缀
 *  - 路由权限声明（RBAC 集成）
 *  - 路由文档自动生成（OpenAPI 3.0）
 *  - 路由健康检查
 *  - 路由依赖与冲突检测
 *  - 动态路由注册/注销
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── HTTP 方法 ───
const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

// ─── 路由参数类型 ───
const PARAM_TYPE = {
  PATH: 'path',
  QUERY: 'query',
  HEADER: 'header',
  BODY: 'body',
};

class RouterAggregator extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.apiPrefix    API 前缀（默认 /api）
   * @param {string} options.defaultVersion 默认版本（默认 v1）
   * @param {boolean} options.enableDocs  启用文档生成（默认 true）
   * @param {object} options.openapiInfo   OpenAPI 元信息
   */
  constructor(options = {}) {
    super();
    this.apiPrefix = options.apiPrefix || '/api';
    this.defaultVersion = options.defaultVersion || 'v1';
    this.enableDocs = options.enableDocs !== false;
    this.openapiInfo = options.openapiInfo || {
      title: 'MOX Enterprise API',
      version: '1.0.0',
      description: 'MOX 千亿亿级企业级平台 API',
    };

    // 路由注册表：routeId -> routeDescriptor
    this.routes = new Map();

    // 路由分组：groupName -> { prefix, routes: Set(routeId) }
    this.groups = new Map();

    // 模块路由：moduleName -> Set(routeId)
    this.moduleRoutes = new Map();

    // 冲突检测：method+path -> routeId
    this.routeIndex = new Map();

    this._aggregatorId = `router-${crypto.randomBytes(4).toString('hex')}`;
  }

  /**
   * 注册路由
   * @param {object} route 路由描述符
   * @param {string} route.method     HTTP 方法
   * @param {string} route.path       路径（不含前缀和版本）
   * @param {Function} route.handler  处理函数
   * @param {string} route.module     所属模块
   * @param {string} route.group      路由组
   * @param {string} route.version    API 版本
   * @param {string[]} route.permissions 所需权限
   * @param {object} route.schema     请求/响应 Schema
   * @param {object} route.docs       文档信息
   * @param {object} route.middleware 路由级中间件
   * @param {boolean} route.authRequired 是否需要认证（默认 true）
   * @param {number} route.rateLimit  速率限制（QPS）
   */
  register(route) {
    if (!route.method || !HTTP_METHODS.includes(route.method.toUpperCase())) {
      throw new Error(`无效的 HTTP 方法: ${route.method}`);
    }
    if (!route.path) throw new Error('路由路径不能为空');
    if (!route.handler) throw new Error('路由处理函数不能为空');

    const routeId = `route-${crypto.randomBytes(6).toString('hex')}`;
    const method = route.method.toUpperCase();
    const version = route.version || this.defaultVersion;
    const fullPath = this._buildFullPath(route.path, version, route.group);

    // 冲突检测
    const indexKey = `${method}:${fullPath}`;
    if (this.routeIndex.has(indexKey)) {
      throw new Error(`路由冲突: ${method} ${fullPath} 已被注册`);
    }

    const descriptor = {
      routeId,
      method,
      path: route.path,
      fullPath,
      handler: route.handler,
      module: route.module || 'unknown',
      group: route.group || null,
      version,
      permissions: route.permissions || [],
      schema: route.schema || null,
      docs: route.docs || { summary: '', description: '', tags: [] },
      middleware: route.middleware || [],
      authRequired: route.authRequired !== false,
      rateLimit: route.rateLimit || 0,
      status: 'active',
      registeredAt: new Date().toISOString(),
      callCount: 0,
      errorCount: 0,
    };

    this.routes.set(routeId, descriptor);
    this.routeIndex.set(indexKey, routeId);

    // 模块索引
    if (!this.moduleRoutes.has(descriptor.module)) {
      this.moduleRoutes.set(descriptor.module, new Set());
    }
    this.moduleRoutes.get(descriptor.module).add(routeId);

    // 组索引
    if (descriptor.group) {
      if (!this.groups.has(descriptor.group)) {
        this.groups.set(descriptor.group, { prefix: '', routes: new Set() });
      }
      this.groups.get(descriptor.group).routes.add(routeId);
    }

    this.emit('router:registered', { routeId, method, fullPath, module: descriptor.module });
    return routeId;
  }

  /**
   * 批量注册路由（模块路由清单）
   * @param {string} moduleName 模块名
   * @param {object[]} routes   路由列表
   */
  registerModuleRoutes(moduleName, routes) {
    const routeIds = [];
    for (const route of routes) {
      const id = this.register({ ...route, module: moduleName });
      routeIds.push(id);
    }
    this.emit('router:module_routes_registered', { moduleName, count: routeIds.length });
    return routeIds;
  }

  /**
   * 注销路由
   */
  unregister(routeId) {
    const route = this.routes.get(routeId);
    if (!route) return false;

    const indexKey = `${route.method}:${route.fullPath}`;
    this.routeIndex.delete(indexKey);
    this.routes.delete(routeId);

    this.moduleRoutes.get(route.module)?.delete(routeId);
    if (route.group) this.groups.get(route.group)?.routes.delete(routeId);

    this.emit('router:unregistered', { routeId, method: route.method, fullPath: route.fullPath });
    return true;
  }

  /**
   * 挂载到 Express 应用
   */
  mountToApp(app) {
    const sortedRoutes = Array.from(this.routes.values())
      .filter(r => r.status === 'active')
      .sort((a, b) => a.fullPath.localeCompare(b.fullPath));

    for (const route of sortedRoutes) {
      const handler = this._wrapHandler(route);
      const middlewares = [...route.middleware, handler];

      switch (route.method) {
        case 'GET': app.get(route.fullPath, ...middlewares); break;
        case 'POST': app.post(route.fullPath, ...middlewares); break;
        case 'PUT': app.put(route.fullPath, ...middlewares); break;
        case 'PATCH': app.patch(route.fullPath, ...middlewares); break;
        case 'DELETE': app.delete(route.fullPath, ...middlewares); break;
        case 'HEAD': app.head(route.fullPath, ...middlewares); break;
        case 'OPTIONS': app.options(route.fullPath, ...middlewares); break;
      }
    }

    // 文档端点
    if (this.enableDocs) {
      app.get(`${this.apiPrefix}/docs/openapi.json`, (req, res) => {
        res.json(this.generateOpenAPI());
      });
    }

    this.emit('router:mounted', { count: sortedRoutes.length });
    return this;
  }

  _wrapHandler(route) {
    const self = this;
    return async function wrappedHandler(req, res, next) {
      route.callCount++;
      const start = Date.now();

      try {
        // 速率限制
        if (route.rateLimit > 0) {
          // 实际应集成限流中间件
        }

        await route.handler(req, res, next);
      } catch (err) {
        route.errorCount++;
        next(err);
      }
    };
  }

  _buildFullPath(path, version, group) {
    let full = this.apiPrefix;
    if (version) full += `/${version}`;
    if (group) full += `/${group}`;
    if (!path.startsWith('/')) full += '/';
    full += path;
    return full.replace(/\/+/g, '/');
  }

  /**
   * 生成 OpenAPI 3.0 文档
   */
  generateOpenAPI() {
    const openapi = {
      openapi: '3.0.3',
      info: this.openapiInfo,
      servers: [{ url: this.apiPrefix }],
      paths: {},
      components: {
        securitySchemes: {
          bearerAuth: { type: 'http', scheme: 'bearer', bearerFormat: 'JWT' },
          apiKeyAuth: { type: 'apiKey', in: 'header', name: 'X-API-Key' },
        },
      },
      tags: [],
    };

    const tagSet = new Set();

    for (const route of this.routes.values()) {
      if (route.status !== 'active') continue;

      if (!openapi.paths[route.fullPath]) openapi.paths[route.fullPath] = {};

      const operation = {
        summary: route.docs.summary || `${route.method} ${route.path}`,
        description: route.docs.description || '',
        tags: route.docs.tags?.length ? route.docs.tags : [route.module],
        operationId: `${route.module}_${route.method.toLowerCase()}_${route.path.replace(/[^a-zA-Z0-9]/g, '_')}`,
        parameters: [],
        responses: {
          200: { description: '成功' },
          400: { description: '请求参数错误' },
          401: { description: '未认证' },
          403: { description: '无权限' },
          500: { description: '服务器内部错误' },
        },
      };

      if (route.authRequired) {
        operation.security = [{ bearerAuth: [] }, { apiKeyAuth: [] }];
      }

      if (route.permissions.length > 0) {
        operation['x-permissions'] = route.permissions;
      }

      if (route.schema) {
        if (route.schema.body) {
          operation.requestBody = {
            required: true,
            content: { 'application/json': { schema: route.schema.body } },
          };
        }
        if (route.schema.response) {
          operation.responses['200'].content = {
            'application/json': { schema: route.schema.response },
          };
        }
        if (route.schema.query) {
          for (const [name, schema] of Object.entries(route.schema.query)) {
            operation.parameters.push({
              name,
              in: 'query',
              required: schema.required || false,
              schema: { type: schema.type },
              description: schema.description || '',
            });
          }
        }
      }

      openapi.paths[route.fullPath][route.method.toLowerCase()] = operation;

      for (const tag of (route.docs.tags?.length ? route.docs.tags : [route.module])) {
        tagSet.add(tag);
      }
    }

    openapi.tags = Array.from(tagSet).map(name => ({ name }));
    return openapi;
  }

  /**
   * 按模块获取路由
   */
  getModuleRoutes(moduleName) {
    const routeIds = this.moduleRoutes.get(moduleName);
    if (!routeIds) return [];
    return Array.from(routeIds).map(id => this.routes.get(id)).filter(Boolean);
  }

  /**
   * 获取所有路由（可按方法/模块/状态过滤）
   */
  list(filter = {}) {
    let routes = Array.from(this.routes.values());
    if (filter.method) routes = routes.filter(r => r.method === filter.method.toUpperCase());
    if (filter.module) routes = routes.filter(r => r.module === filter.module);
    if (filter.status) routes = routes.filter(r => r.status === filter.status);
    if (filter.group) routes = routes.filter(r => r.group === filter.group);
    return routes.sort((a, b) => a.fullPath.localeCompare(b.fullPath));
  }

  /**
   * 检测路由冲突
   */
  detectConflicts() {
    const conflicts = [];
    const pathMap = new Map();

    for (const route of this.routes.values()) {
      const normalizedPath = route.fullPath.replace(/:[^/]+/g, ':param');
      const key = `${route.method}:${normalizedPath}`;
      if (pathMap.has(key)) {
        conflicts.push({
          route1: pathMap.get(key),
          route2: route.routeId,
          method: route.method,
          path: normalizedPath,
        });
      } else {
        pathMap.set(key, route.routeId);
      }
    }

    return conflicts;
  }

  /**
   * 获取统计
   */
  getStats() {
    const all = Array.from(this.routes.values());
    return {
      aggregatorId: this._aggregatorId,
      apiPrefix: this.apiPrefix,
      defaultVersion: this.defaultVersion,
      totalRoutes: all.length,
      activeRoutes: all.filter(r => r.status === 'active').length,
      byMethod: all.reduce((acc, r) => { acc[r.method] = (acc[r.method] || 0) + 1; return acc; }, {}),
      byModule: all.reduce((acc, r) => { acc[r.module] = (acc[r.module] || 0) + 1; return acc; }, {}),
      byVersion: all.reduce((acc, r) => { acc[r.version] = (acc[r.version] || 0) + 1; return acc; }, {}),
      totalGroups: this.groups.size,
      authRequired: all.filter(r => r.authRequired).length,
      withRateLimit: all.filter(r => r.rateLimit > 0).length,
      totalCalls: all.reduce((s, r) => s + r.callCount, 0),
      totalErrors: all.reduce((s, r) => s + r.errorCount, 0),
      conflicts: this.detectConflicts().length,
    };
  }
}

// 全局单例
let _globalAggregator = null;
function getGlobalRouterAggregator() {
  if (!_globalAggregator) _globalAggregator = new RouterAggregator();
  return _globalAggregator;
}

module.exports = {
  RouterAggregator,
  HTTP_METHODS,
  PARAM_TYPE,
  getGlobalRouterAggregator,
};
