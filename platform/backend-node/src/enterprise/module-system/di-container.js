'use strict';

/**
 * MOX Enterprise · 依赖注入容器
 * =============================
 * 企业级 DI 容器，管理模块间的依赖解析与实例生命周期
 *
 * 核心能力：
 *  - 服务注册（singleton/transient/request-scoped）
 *  - 构造函数注入 / 属性注入 / 工厂注入
 *  - 循环依赖检测
 *  - 依赖图可视化
 *  - 服务装饰器（中间件/拦截器）
 *  - 作用域隔离（tenant-scoped / request-scoped）
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 服务生命周期 ───
const SERVICE_SCOPE = {
  SINGLETON: 'singleton',     // 全局单例
  TRANSIENT: 'transient',     // 每次解析新建
  REQUEST: 'request',         // 请求级单例
  TENANT: 'tenant',           // 租户级单例
};

class DIContainer extends EventEmitter {
  constructor(options = {}) {
    super();
    this.registrations = new Map();  // token -> registration
    this.instances = new Map();       // token -> instance (singleton)
    this.scopedInstances = new Map(); // scopeId -> Map(token -> instance)
    this.containerId = `di-${crypto.randomBytes(4).toString('hex')}`;
    this.parent = options.parent || null;
    this._resolutionStack = [];       // 用于循环依赖检测
  }

  /**
   * 注册服务
   * @param {string|symbol} token  服务标识符
   * @param {Function|object} implementation 实现类/工厂函数/实例
   * @param {object} options
   * @param {string} options.scope       生命周期（默认 singleton）
   * @param {string[]} options.deps      依赖的 token 列表
   * @param {Function} options.factory   工厂函数 (container) => instance
   * @param {object} options.value       直接值（常量注入）
   * @param {string[]} options.tags      标签
   */
  register(token, implementation, options = {}) {
    const tokenStr = this._tokenToString(token);
    if (this.registrations.has(tokenStr)) {
      throw new Error(`服务已注册: ${tokenStr}`);
    }

    const registration = {
      token: tokenStr,
      implementation,
      scope: options.scope || SERVICE_SCOPE.SINGLETON,
      deps: options.deps || [],
      factory: options.factory || null,
      value: options.value !== undefined ? options.value : undefined,
      tags: options.tags || [],
      registeredAt: new Date().toISOString(),
      instanceCount: 0,
    };

    // 如果是直接值，立即存储
    if (registration.value !== undefined) {
      this.instances.set(tokenStr, registration.value);
    }

    this.registrations.set(tokenStr, registration);
    this.emit('di:registered', { token: tokenStr, scope: registration.scope });
    return this;
  }

  /**
   * 注册单例（便捷方法）
   */
  registerSingleton(token, implementation, deps = []) {
    return this.register(token, implementation, { scope: SERVICE_SCOPE.SINGLETON, deps });
  }

  /**
   * 注册工厂
   */
  registerFactory(token, factory, scope = SERVICE_SCOPE.SINGLETON) {
    return this.register(token, null, { scope, factory });
  }

  /**
   * 注册常量值
   */
  registerValue(token, value) {
    return this.register(token, null, { scope: SERVICE_SCOPE.SINGLETON, value });
  }

  /**
   * 解析服务
   * @param {string|symbol} token
   * @param {object} scopeContext 作用域上下文（requestId/tenantId）
   */
  resolve(token, scopeContext = {}) {
    const tokenStr = this._tokenToString(token);

    // 循环依赖检测
    if (this._resolutionStack.includes(tokenStr)) {
      throw new Error(`检测到循环依赖: ${[...this._resolutionStack, tokenStr].join(' → ')}`);
    }

    const registration = this.registrations.get(tokenStr);
    if (!registration) {
      // 尝试从父容器解析
      if (this.parent) return this.parent.resolve(token, scopeContext);
      throw new Error(`服务未注册: ${tokenStr}`);
    }

    this._resolutionStack.push(tokenStr);

    try {
      const instance = this._resolveInstance(registration, scopeContext);
      return instance;
    } finally {
      this._resolutionStack.pop();
    }
  }

  _resolveInstance(registration, scopeContext) {
    const { token, scope, value } = registration;

    // 常量值
    if (value !== undefined) return value;

    // 单例：已创建则直接返回
    if (scope === SERVICE_SCOPE.SINGLETON) {
      if (this.instances.has(token)) return this.instances.get(token);
    }

    // 作用域实例
    if (scope === SERVICE_SCOPE.REQUEST || scope === SERVICE_SCOPE.TENANT) {
      const scopeId = scope === SERVICE_SCOPE.REQUEST
        ? scopeContext.requestId
        : scopeContext.tenantId;
      if (scopeId) {
        if (!this.scopedInstances.has(scopeId)) this.scopedInstances.set(scopeId, new Map());
        const scopeMap = this.scopedInstances.get(scopeId);
        if (scopeMap.has(token)) return scopeMap.get(token);
      }
    }

    // 创建实例
    const instance = this._createInstance(registration, scopeContext);
    registration.instanceCount++;

    // 缓存
    if (scope === SERVICE_SCOPE.SINGLETON) {
      this.instances.set(token, instance);
    }
    if ((scope === SERVICE_SCOPE.REQUEST || scope === SERVICE_SCOPE.TENANT)) {
      const scopeId = scope === SERVICE_SCOPE.REQUEST
        ? scopeContext.requestId
        : scopeContext.tenantId;
      if (scopeId) {
        const scopeMap = this.scopedInstances.get(scopeId);
        if (scopeMap) scopeMap.set(token, instance);
      }
    }

    return instance;
  }

  _createInstance(registration, scopeContext) {
    const { implementation, factory, deps } = registration;

    // 工厂模式
    if (factory) {
      return factory(this, scopeContext);
    }

    // 构造函数注入
    if (typeof implementation === 'function') {
      const resolvedDeps = deps.map(dep => this.resolve(dep, scopeContext));
      return new implementation(...resolvedDeps);
    }

    // 对象实例（属性注入）
    if (typeof implementation === 'object' && implementation !== null) {
      const instance = { ...implementation };
      for (const dep of deps) {
        const depToken = typeof dep === 'string' ? dep : dep.token;
        const propName = typeof dep === 'string' ? dep : dep.property;
        instance[propName] = this.resolve(depToken, scopeContext);
      }
      return instance;
    }

    throw new Error(`无法创建服务实例: ${registration.token}`);
  }

  /**
   * 批量解析
   */
  resolveAll(tokens, scopeContext = {}) {
    return tokens.map(t => this.resolve(t, scopeContext));
  }

  /**
   * 按标签解析所有服务
   */
  resolveByTag(tag) {
    const results = [];
    for (const [token, reg] of this.registrations) {
      if (reg.tags.includes(tag)) {
        results.push({ token, instance: this.resolve(token) });
      }
    }
    return results;
  }

  /**
   * 检查服务是否已注册
   */
  has(token) {
    const tokenStr = this._tokenToString(token);
    return this.registrations.has(tokenStr) || (this.parent?.has(token));
  }

  /**
   * 销毁作用域实例
   */
  disposeScope(scopeId) {
    const scopeMap = this.scopedInstances.get(scopeId);
    if (scopeMap) {
      for (const [, instance] of scopeMap) {
        if (typeof instance.dispose === 'function') {
          try { instance.dispose(); } catch {}
        }
      }
      this.scopedInstances.delete(scopeId);
    }
  }

  /**
   * 销毁所有单例
   */
  async dispose() {
    for (const [, instance] of this.instances) {
      if (typeof instance.stop === 'function') {
        try { await instance.stop(); } catch {}
      }
      if (typeof instance.dispose === 'function') {
        try { instance.dispose(); } catch {}
      }
    }
    this.instances.clear();
    this.scopedInstances.clear();
    this.emit('di:disposed');
  }

  /**
   * 获取依赖图
   */
  getDependencyGraph() {
    const graph = {};
    for (const [token, reg] of this.registrations) {
      graph[token] = {
        scope: reg.scope,
        deps: reg.deps,
        tags: reg.tags,
        instanceCount: reg.instanceCount,
      };
    }
    return graph;
  }

  /**
   * 检测循环依赖（静态分析）
   */
  detectCircularDependencies() {
    const cycles = [];
    const visited = new Set();
    const recStack = new Set();

    const dfs = (token, path) => {
      visited.add(token);
      recStack.add(token);
      path.push(token);

      const reg = this.registrations.get(token);
      if (reg) {
        for (const dep of reg.deps) {
          if (!visited.has(dep)) {
            dfs(dep, path);
          } else if (recStack.has(dep)) {
            const cycleStart = path.indexOf(dep);
            cycles.push([...path.slice(cycleStart), dep]);
          }
        }
      }

      path.pop();
      recStack.delete(token);
    };

    for (const token of this.registrations.keys()) {
      if (!visited.has(token)) dfs(token, []);
    }

    return cycles;
  }

  /**
   * 获取容器统计
   */
  getStats() {
    return {
      containerId: this.containerId,
      totalRegistrations: this.registrations.size,
      singletonInstances: this.instances.size,
      activeScopes: this.scopedInstances.size,
      byScope: Array.from(this.registrations.values()).reduce((acc, r) => {
        acc[r.scope] = (acc[r.scope] || 0) + 1;
        return acc;
      }, {}),
      circularDependencies: this.detectCircularDependencies().length,
      hasParent: !!this.parent,
    };
  }

  _tokenToString(token) {
    if (typeof token === 'symbol') return token.toString();
    return String(token);
  }

  /**
   * 创建子容器
   */
  createChild() {
    const child = new DIContainer({ parent: this });
    return child;
  }
}

// 全局单例
let _globalContainer = null;
function getGlobalContainer() {
  if (!_globalContainer) _globalContainer = new DIContainer();
  return _globalContainer;
}

module.exports = {
  DIContainer,
  SERVICE_SCOPE,
  getGlobalContainer,
};
