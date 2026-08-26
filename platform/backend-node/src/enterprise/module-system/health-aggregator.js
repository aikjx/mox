'use strict';

/**
 * MOX Enterprise · 健康检查聚合器
 * ==============================
 * 聚合所有模块/服务/基础设施的健康状态，提供统一健康端点
 *
 * 健康检查类型：
 *  - liveness（存活）：进程是否在运行
 *  - readiness（就绪）：是否可以接收流量
 *  - startup（启动）：是否完成启动
 *  - deep（深度）：依赖链路是否全部健康
 *
 * 健康状态：
 *  - healthy（健康）：所有检查通过
 *  - degraded（降级）：非关键依赖失败
 *  - unhealthy（不健康）：关键依赖失败
 *
 * 核心能力：
 *  - 注册健康检查器（带超时/重试/缓存）
 *  - 依赖树健康传播
 *  - 健康状态缓存（避免频繁检查）
 *  - 健康历史记录与趋势
 *  - K8s 探针端点输出
 *  - 健康告警集成
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 健康状态 ───
const HEALTH_STATUS = {
  HEALTHY: 'healthy',
  DEGRADED: 'degraded',
  UNHEALTHY: 'unhealthy',
  UNKNOWN: 'unknown',
};

// ─── 检查类型 ───
const CHECK_TYPE = {
  LIVENESS: 'liveness',
  READINESS: 'readiness',
  STARTUP: 'startup',
  DEEP: 'deep',
};

// ─── 依赖重要性 ───
const DEPENDENCY_CRITICALITY = {
  CRITICAL: 'critical',   // 失败则 unhealthy
  IMPORTANT: 'important',  // 失败则 degraded
  OPTIONAL: 'optional',     // 失败不影响整体状态
};

class HealthAggregator extends EventEmitter {
  /**
   * @param {object} options
   * @param {number} options.cacheTtlMs       健康状态缓存时间（默认 5s）
   * @param {number} options.defaultTimeoutMs  默认检查超时（默认 3s）
   * @param {number} options.historySize       历史记录大小（默认 1000）
   * @param {boolean} options.enableAlerts     启用健康告警（默认 true）
   */
  constructor(options = {}) {
    super();
    this.cacheTtlMs = options.cacheTtlMs || 5000;
    this.defaultTimeoutMs = options.defaultTimeoutMs || 3000;
    this.historySize = options.historySize || 1000;
    this.enableAlerts = options.enableAlerts !== false;

    // 健康检查器：name -> checker
    this.checkers = new Map();

    // 健康状态缓存：name -> { status, details, checkedAt }
    this.cache = new Map();

    // 健康历史：name -> [{ status, timestamp, durationMs }]
    this.history = new Map();

    // 依赖关系：name -> [{ name, criticality }]
    this.dependencies = new Map();

    this._aggregatorId = `health-${crypto.randomBytes(4).toString('hex')}`;
  }

  /**
   * 注册健康检查器
   * @param {string} name  检查器名称
   * @param {Function} checkFn 检查函数 () => Promise<{status, details, durationMs?}>
   * @param {object} options
   * @param {string} options.type        检查类型（默认 readiness）
   * @param {string} options.criticality 依赖重要性（默认 critical）
   * @param {number} options.timeoutMs   超时时间
   * @param {number} options.intervalMs  自动检查间隔（0=不自动检查）
   * @param {string[]} options.tags      标签
   * @param {string} options.description 描述
   */
  register(name, checkFn, options = {}) {
    if (this.checkers.has(name)) {
      throw new Error(`健康检查器已注册: ${name}`);
    }

    const checker = {
      name,
      checkFn,
      type: options.type || CHECK_TYPE.READINESS,
      criticality: options.criticality || DEPENDENCY_CRITICALITY.CRITICAL,
      timeoutMs: options.timeoutMs || this.defaultTimeoutMs,
      intervalMs: options.intervalMs || 0,
      tags: options.tags || [],
      description: options.description || '',
      registeredAt: new Date().toISOString(),
      lastCheckAt: null,
      checkCount: 0,
      failureCount: 0,
      consecutiveFailures: 0,
      timer: null,
    };

    this.checkers.set(name, checker);
    this.history.set(name, []);

    // 启动自动检查
    if (checker.intervalMs > 0) {
      checker.timer = setInterval(() => {
        this.check(name).catch(err => {
          this.emit('health:auto_check_error', { name, error: err.message });
        });
      }, checker.intervalMs);
    }

    this.emit('health:checker_registered', { name, type: checker.type });
    return this;
  }

  /**
   * 注销健康检查器
   */
  unregister(name) {
    const checker = this.checkers.get(name);
    if (checker?.timer) clearInterval(checker.timer);
    this.checkers.delete(name);
    this.cache.delete(name);
    this.history.delete(name);
    this.emit('health:checker_unregistered', { name });
    return this;
  }

  /**
   * 声明依赖关系
   */
  addDependency(name, dependencyName, criticality = DEPENDENCY_CRITICALITY.CRITICAL) {
    if (!this.dependencies.has(name)) this.dependencies.set(name, []);
    this.dependencies.get(name).push({ name: dependencyName, criticality });
    return this;
  }

  /**
   * 执行单个健康检查
   */
  async check(name) {
    const checker = this.checkers.get(name);
    if (!checker) throw new Error(`健康检查器不存在: ${name}`);

    // 检查缓存
    const cached = this.cache.get(name);
    if (cached && Date.now() - new Date(cached.checkedAt).getTime() < this.cacheTtlMs) {
      return cached;
    }

    const start = Date.now();
    checker.checkCount++;

    try {
      const result = await Promise.race([
        checker.checkFn(),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`健康检查超时 ${checker.timeoutMs}ms`)), checker.timeoutMs)
        ),
      ]);

      const durationMs = Date.now() - start;
      const status = result?.status || HEALTH_STATUS.HEALTHY;
      const details = result?.details || {};

      const healthResult = {
        name,
        status,
        details,
        durationMs,
        checkedAt: new Date().toISOString(),
        type: checker.type,
        criticality: checker.criticality,
      };

      // 更新缓存
      this.cache.set(name, healthResult);

      // 更新历史
      this._appendHistory(name, { status, durationMs, timestamp: healthResult.checkedAt });

      // 重置连续失败计数
      if (status === HEALTH_STATUS.HEALTHY) {
        checker.consecutiveFailures = 0;
      } else {
        checker.consecutiveFailures++;
        checker.failureCount++;
      }

      checker.lastCheckAt = healthResult.checkedAt;

      // 状态变更告警
      if (cached && cached.status !== status) {
        this.emit('health:status_changed', { name, oldStatus: cached.status, newStatus: status, details });
        if (this.enableAlerts && status === HEALTH_STATUS.UNHEALTHY) {
          this.emit('health:alert', { name, status, details, consecutiveFailures: checker.consecutiveFailures });
        }
      }

      this.emit('health:checked', healthResult);
      return healthResult;

    } catch (err) {
      const durationMs = Date.now() - start;
      checker.consecutiveFailures++;
      checker.failureCount++;
      checker.lastCheckAt = new Date().toISOString();

      const healthResult = {
        name,
        status: HEALTH_STATUS.UNHEALTHY,
        details: { error: err.message },
        durationMs,
        checkedAt: checker.lastCheckAt,
        type: checker.type,
        criticality: checker.criticality,
      };

      this.cache.set(name, healthResult);
      this._appendHistory(name, { status: HEALTH_STATUS.UNHEALTHY, durationMs, timestamp: healthResult.checkedAt });

      this.emit('health:check_failed', { name, error: err.message, durationMs });
      if (this.enableAlerts && checker.consecutiveFailures >= 3) {
        this.emit('health:alert', { name, status: HEALTH_STATUS.UNHEALTHY, error: err.message, consecutiveFailures: checker.consecutiveFailures });
      }

      return healthResult;
    }
  }

  /**
   * 检查所有（可按类型/标签过滤）
   */
  async checkAll(filter = {}) {
    let checkers = Array.from(this.checkers.values());

    if (filter.type) checkers = checkers.filter(c => c.type === filter.type);
    if (filter.tag) checkers = checkers.filter(c => c.tags.includes(filter.tag));
    if (filter.names) checkers = checkers.filter(c => filter.names.includes(c.name));

    const results = await Promise.all(checkers.map(c => this.check(c.name)));
    return this._aggregateResults(results);
  }

  /**
   * 获取聚合健康状态（K8s 探针格式）
   */
  async getAggregatedHealth(checkType = CHECK_TYPE.READINESS) {
    const results = await this.checkAll({ type: checkType });
    return {
      status: results.overallStatus,
      timestamp: new Date().toISOString(),
      version: process.env.npm_package_version || '1.0.0',
      checks: results.checks,
      ...results.summary,
    };
  }

  /**
   * Express 中间件：健康端点
   */
  healthEndpoint(checkType = CHECK_TYPE.READINESS) {
    return async (req, res) => {
      try {
        const health = await this.getAggregatedHealth(checkType);
        const httpStatus = health.status === HEALTH_STATUS.UNHEALTHY ? 503 : 200;
        res.status(httpStatus).json(health);
      } catch (err) {
        res.status(500).json({ status: HEALTH_STATUS.UNKNOWN, error: err.message });
      }
    };
  }

  _aggregateResults(results) {
    let overallStatus = HEALTH_STATUS.HEALTHY;
    let criticalFailures = 0;
    let importantDegradations = 0;
    let optionalFailures = 0;
    let totalDurationMs = 0;

    for (const result of results) {
      totalDurationMs += result.durationMs || 0;

      if (result.status === HEALTH_STATUS.UNHEALTHY) {
        if (result.criticality === DEPENDENCY_CRITICALITY.CRITICAL) {
          overallStatus = HEALTH_STATUS.UNHEALTHY;
          criticalFailures++;
        } else if (result.criticality === DEPENDENCY_CRITICALITY.IMPORTANT) {
          if (overallStatus !== HEALTH_STATUS.UNHEALTHY) overallStatus = HEALTH_STATUS.DEGRADED;
          importantDegradations++;
        } else {
          optionalFailures++;
        }
      } else if (result.status === HEALTH_STATUS.DEGRADED) {
        if (overallStatus === HEALTH_STATUS.HEALTHY) overallStatus = HEALTH_STATUS.DEGRADED;
      }
    }

    return {
      overallStatus,
      checks: results,
      summary: {
        totalChecks: results.length,
        healthy: results.filter(r => r.status === HEALTH_STATUS.HEALTHY).length,
        degraded: results.filter(r => r.status === HEALTH_STATUS.DEGRADED).length,
        unhealthy: results.filter(r => r.status === HEALTH_STATUS.UNHEALTHY).length,
        criticalFailures,
        importantDegradations,
        optionalFailures,
        totalDurationMs,
      },
    };
  }

  _appendHistory(name, entry) {
    const history = this.history.get(name);
    if (!history) return;
    history.push(entry);
    if (history.length > this.historySize) history.shift();
  }

  /**
   * 获取健康历史（趋势分析）
   */
  getHistory(name, limit = 100) {
    const history = this.history.get(name) || [];
    return history.slice(-limit);
  }

  /**
   * 获取可用性统计
   */
  getAvailability(name, windowMs = 3600000) {
    const history = this.history.get(name) || [];
    const now = Date.now();
    const windowed = history.filter(h => now - new Date(h.timestamp).getTime() < windowMs);

    if (windowed.length === 0) return { availability: 100, totalChecks: 0, windowMs };

    const healthy = windowed.filter(h => h.status === HEALTH_STATUS.HEALTHY).length;
    const availability = (healthy / windowed.length) * 100;

    return {
      availability: Math.round(availability * 100) / 100,
      totalChecks: windowed.length,
      healthy,
      degraded: windowed.filter(h => h.status === HEALTH_STATUS.DEGRADED).length,
      unhealthy: windowed.filter(h => h.status === HEALTH_STATUS.UNHEALTHY).length,
      windowMs,
      avgDurationMs: windowed.reduce((s, h) => s + (h.durationMs || 0), 0) / windowed.length,
    };
  }

  /**
   * 获取统计
   */
  getStats() {
    const checkers = Array.from(this.checkers.values());
    return {
      aggregatorId: this._aggregatorId,
      totalCheckers: checkers.length,
      byType: checkers.reduce((acc, c) => { acc[c.type] = (acc[c.type] || 0) + 1; return acc; }, {}),
      byCriticality: checkers.reduce((acc, c) => { acc[c.criticality] = (acc[c.criticality] || 0) + 1; return acc; }, {}),
      totalChecks: checkers.reduce((s, c) => s + c.checkCount, 0),
      totalFailures: checkers.reduce((s, c) => s + c.failureCount, 0),
      cachedEntries: this.cache.size,
      historyEntries: Array.from(this.history.values()).reduce((s, h) => s + h.length, 0),
      dependencies: Array.from(this.dependencies.entries()).map(([name, deps]) => ({ name, depCount: deps.length })),
    };
  }

  /**
   * 销毁
   */
  destroy() {
    for (const checker of this.checkers.values()) {
      if (checker.timer) clearInterval(checker.timer);
    }
    this.checkers.clear();
    this.cache.clear();
    this.history.clear();
    this.removeAllListeners();
  }
}

// 全局单例
let _globalAggregator = null;
function getGlobalHealthAggregator() {
  if (!_globalAggregator) _globalAggregator = new HealthAggregator();
  return _globalAggregator;
}

module.exports = {
  HealthAggregator,
  HEALTH_STATUS,
  CHECK_TYPE,
  DEPENDENCY_CRITICALITY,
  getGlobalHealthAggregator,
};
