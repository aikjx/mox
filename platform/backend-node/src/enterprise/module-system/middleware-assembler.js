'use strict';

/**
 * MOX Enterprise · 中间件装配器
 * ============================
 * 统一管理 Express/Koa 中间件的注册、排序、组合与热更新
 *
 * 中间件分类：
 *  - ingress（入口）：请求日志、请求 ID、限流
 *  - auth（认证）：API Key、JWT、RBAC
 *  - security（安全）：CORS、Helmet、输入校验
 *  - business（业务）：租户隔离、配额检查
 *  - egress（出口）：响应压缩、响应头、错误处理
 *
 * 核心能力：
 *  - 中间件优先级排序（数字越小越先执行）
 *  - 条件挂载（按路径/方法/环境）
 *  - 中间件链组合（洋葱模型）
 *  - 中间件热替换（运行时更新）
 *  - 中间件性能监控（耗时统计）
 *  - 中间件依赖声明与排序
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 中间件阶段 ───
const MIDDLEWARE_PHASE = {
  INGRESS: 100,     // 入口层
  SECURITY: 200,    // 安全层
  AUTH: 300,         // 认证层
  TENANT: 400,       // 租户层
  QUOTA: 500,        // 配额层
  BUSINESS: 600,     // 业务层
  EGRESS: 700,       // 出口层
  ERROR: 999,        // 错误处理层
};

// ─── 中间件状态 ───
const MIDDLEWARE_STATUS = {
  REGISTERED: 'registered',
  ACTIVE: 'active',
  DISABLED: 'disabled',
  ERROR: 'error',
};

class MiddlewareAssembler extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.framework  框架类型（express/koa）
   * @param {boolean} options.enableMetrics 启用性能监控
   */
  constructor(options = {}) {
    super();
    this.framework = options.framework || 'express';
    this.enableMetrics = options.enableMetrics !== false;

    // 中间件注册表：name -> descriptor
    this.middlewares = new Map();

    // 已挂载的中间件（按顺序）
    this.mounted = [];

    // 性能统计
    this.metrics = new Map(); // name -> { count, totalDurationMs, avgDurationMs, errors }

    this._assemblerId = `mw-${crypto.randomBytes(4).toString('hex')}`;
  }

  /**
   * 注册中间件
   * @param {string} name  中间件名称
   * @param {Function} factory 中间件工厂函数 (options) => middleware
   * @param {object} options
   * @param {number} options.priority  优先级（数字越小越先执行）
   * @param {string} options.phase     阶段（用于自动排序）
   * @param {object} options.config    中间件配置
   * @param {string[]} options.dependencies 依赖的中间件名称
   * @param {object} options.condition  挂载条件 { path, method, env }
   * @param {boolean} options.enabled   是否启用（默认 true）
   * @param {string} options.description 描述
   */
  register(name, factory, options = {}) {
    if (this.middlewares.has(name)) {
      throw new Error(`中间件已注册: ${name}`);
    }

    const descriptor = {
      name,
      factory,
      priority: options.priority || (options.phase ? MIDDLEWARE_PHASE[options.phase] : MIDDLEWARE_PHASE.BUSINESS),
      phase: options.phase || 'BUSINESS',
      config: options.config || {},
      dependencies: options.dependencies || [],
      condition: options.condition || null,
      enabled: options.enabled !== false,
      description: options.description || '',
      status: MIDDLEWARE_STATUS.REGISTERED,
      instance: null,
      registeredAt: new Date().toISOString(),
      mountedAt: null,
    };

    this.middlewares.set(name, descriptor);
    this.metrics.set(name, { count: 0, totalDurationMs: 0, avgDurationMs: 0, errors: 0 });

    this.emit('middleware:registered', { name, priority: descriptor.priority });
    return this;
  }

  /**
   * 注销中间件
   */
  unregister(name) {
    const descriptor = this.middlewares.get(name);
    if (!descriptor) return false;

    this.middlewares.delete(name);
    this.metrics.delete(name);
    this.mounted = this.mounted.filter(m => m.name !== name);

    this.emit('middleware:unregistered', { name });
    return true;
  }

  /**
   * 启用/禁用中间件
   */
  setEnabled(name, enabled) {
    const descriptor = this.middlewares.get(name);
    if (!descriptor) throw new Error(`中间件不存在: ${name}`);
    descriptor.enabled = enabled;
    descriptor.status = enabled ? MIDDLEWARE_STATUS.REGISTERED : MIDDLEWARE_STATUS.DISABLED;
    this.emit('middleware:enabled_changed', { name, enabled });
    return this;
  }

  /**
   * 更新中间件配置（热更新）
   */
  updateConfig(name, config) {
    const descriptor = this.middlewares.get(name);
    if (!descriptor) throw new Error(`中间件不存在: ${name}`);

    descriptor.config = { ...descriptor.config, ...config };

    // 如果已挂载，重新创建实例
    if (descriptor.instance) {
      try {
        descriptor.instance = descriptor.factory(descriptor.config);
        this.emit('middleware:config_updated', { name, config: descriptor.config });
      } catch (err) {
        descriptor.status = MIDDLEWARE_STATUS.ERROR;
        this.emit('middleware:update_failed', { name, error: err.message });
      }
    }

    return this;
  }

  /**
   * 组装所有中间件（按优先级排序）
   * @returns {Function[]} 排序后的中间件数组
   */
  assemble() {
    // 筛选启用的中间件
    const enabled = Array.from(this.middlewares.values())
      .filter(m => m.enabled);

    // 检查依赖
    for (const mw of enabled) {
      for (const dep of mw.dependencies) {
        if (!this.middlewares.has(dep) || !this.middlewares.get(dep).enabled) {
          throw new Error(`中间件 ${mw.name} 依赖未注册或未启用: ${dep}`);
        }
      }
    }

    // 按优先级排序
    enabled.sort((a, b) => a.priority - b.priority);

    // 创建实例
    const assembled = [];
    for (const descriptor of enabled) {
      try {
        if (!descriptor.instance) {
          descriptor.instance = descriptor.factory(descriptor.config);
        }
        descriptor.status = MIDDLEWARE_STATUS.ACTIVE;
        descriptor.mountedAt = new Date().toISOString();

        // 包装性能监控
        const wrapped = this.enableMetrics
          ? this._wrapWithMetrics(descriptor)
          : descriptor.instance;

        assembled.push(wrapped);
        this.mounted.push({ name: descriptor.name, priority: descriptor.priority, mountedAt: descriptor.mountedAt });
      } catch (err) {
        descriptor.status = MIDDLEWARE_STATUS.ERROR;
        this.emit('middleware:mount_failed', { name: descriptor.name, error: err.message });
        throw err;
      }
    }

    this.emit('middleware:assembled', { count: assembled.length });
    return assembled;
  }

  /**
   * 挂载到 Express 应用
   */
  mountToApp(app) {
    const middlewares = this.assemble();
    for (const mw of middlewares) {
      app.use(mw);
    }
    this.emit('middleware:mounted_to_app', { count: middlewares.length });
    return this;
  }

  /**
   * 获取条件匹配的中间件（用于路由级挂载）
   */
  getConditionalMiddleware(condition) {
    return Array.from(this.middlewares.values())
      .filter(m => m.enabled && this._matchCondition(m.condition, condition))
      .sort((a, b) => a.priority - b.priority)
      .map(m => m.instance || m.factory(m.config));
  }

  _wrapWithMetrics(descriptor) {
    const self = this;
    const metrics = this.metrics.get(descriptor.name);
    const instance = descriptor.instance;

    // Express 中间件签名: (req, res, next)
    return function metricsMiddleware(req, res, next) {
      const start = Date.now();
      metrics.count++;

      // 监听响应完成
      const originalEnd = res.end;
      res.end = function (...args) {
        const durationMs = Date.now() - start;
        metrics.totalDurationMs += durationMs;
        metrics.avgDurationMs = metrics.totalDurationMs / metrics.count;
        return originalEnd.apply(this, args);
      };

      try {
        instance(req, res, (err) => {
          if (err) metrics.errors++;
          next(err);
        });
      } catch (err) {
        metrics.errors++;
        next(err);
      }
    };
  }

  _matchCondition(mwCondition, requestCondition) {
    if (!mwCondition) return true;
    if (!requestCondition) return true;

    if (mwCondition.path && !requestCondition.path?.startsWith(mwCondition.path)) return false;
    if (mwCondition.method && mwCondition.method !== requestCondition.method) return false;
    if (mwCondition.env && mwCondition.env !== process.env.NODE_ENV) return false;

    return true;
  }

  /**
   * 获取中间件执行顺序
   */
  getExecutionOrder() {
    return Array.from(this.middlewares.values())
      .filter(m => m.enabled)
      .sort((a, b) => a.priority - b.priority)
      .map(m => ({
        name: m.name,
        priority: m.priority,
        phase: m.phase,
        status: m.status,
        dependencies: m.dependencies,
        description: m.description,
      }));
  }

  /**
   * 获取性能统计
   */
  getMetrics() {
    const result = {};
    for (const [name, metrics] of this.metrics) {
      result[name] = { ...metrics };
    }
    return result;
  }

  /**
   * 获取统计
   */
  getStats() {
    const all = Array.from(this.middlewares.values());
    return {
      assemblerId: this._assemblerId,
      framework: this.framework,
      totalRegistered: all.length,
      active: all.filter(m => m.status === MIDDLEWARE_STATUS.ACTIVE).length,
      disabled: all.filter(m => m.status === MIDDLEWARE_STATUS.DISABLED).length,
      errors: all.filter(m => m.status === MIDDLEWARE_STATUS.ERROR).length,
      mounted: this.mounted.length,
      byPhase: all.reduce((acc, m) => { acc[m.phase] = (acc[m.phase] || 0) + 1; return acc; }, {}),
      executionOrder: this.getExecutionOrder(),
    };
  }
}

// 全局单例
let _globalAssembler = null;
function getGlobalMiddlewareAssembler() {
  if (!_globalAssembler) _globalAssembler = new MiddlewareAssembler();
  return _globalAssembler;
}

module.exports = {
  MiddlewareAssembler,
  MIDDLEWARE_PHASE,
  MIDDLEWARE_STATUS,
  getGlobalMiddlewareAssembler,
};
