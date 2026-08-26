'use strict';

/**
 * MOX Enterprise · 应用启动编排器
 * ==============================
 * 统一编排整个应用的启动、运行、关闭流程
 *
 * 启动流程：
 *  1. pre-bootstrap：环境检查、配置加载、日志初始化
 *  2. bootstrap：核心基础设施（DB/Cache/Storage）
 *  3. module-init：按拓扑顺序初始化所有模块
 *  4. module-start：按拓扑顺序启动所有模块
 *  5. post-bootstrap：路由挂载、健康检查、事件监听
 *  6. ready：应用就绪，接收流量
 *
 * 关闭流程（优雅关闭）：
 *  1. pre-shutdown：停止接收新请求
 *  2. drain：等待进行中请求完成
 *  3. module-stop：按反向拓扑顺序停止模块
 *  4. shutdown：关闭基础设施
 *  5. exit：进程退出
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');
const { ModuleRegistry, MODULE_STATUS } = require('./module-registry');
const { DIContainer } = require('./di-container');
const { EnterpriseConfig } = require('./enterprise-config');
const { ModuleLifecycleManager } = require('./module-lifecycle');
const { EventBus } = require('./event-bus');
const { HealthAggregator, CHECK_TYPE } = require('./health-aggregator');
const { MiddlewareAssembler } = require('./middleware-assembler');
const { RouterAggregator } = require('./router-aggregator');
const { DependencyGraph } = require('./dependency-graph');

// ─── 应用状态 ───
const APP_STATE = {
  UNINITIALIZED: 'uninitialized',
  BOOTSTRAPPING: 'bootstrapping',
  INITIALIZING: 'initializing',
  STARTING: 'starting',
  READY: 'ready',
  DEGRADED: 'degraded',
  SHUTTING_DOWN: 'shutting_down',
  STOPPED: 'stopped',
  ERROR: 'error',
};

// ─── 启动阶段 ───
const BOOTSTRAP_PHASE = {
  PRE_BOOTSTRAP: 'pre_bootstrap',
  BOOTSTRAP: 'bootstrap',
  MODULE_INIT: 'module_init',
  MODULE_START: 'module_start',
  POST_BOOTSTRAP: 'post_bootstrap',
  READY: 'ready',
};

class AppBootstrap extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.appName       应用名称
   * @param {string} options.version       应用版本
   * @param {object} options.config        配置中心实例（不传则创建）
   * @param {object} options.registry      模块注册中心实例（不传则创建）
   * @param {object} options.diContainer   DI 容器实例（不传则创建）
   * @param {object} options.eventBus      事件总线实例（不传则创建）
   * @param {object} options.health        健康聚合器实例（不传则创建）
   * @param {object} options.middleware    中间件装配器实例（不传则创建）
   * @param {object} options.router        路由聚合器实例（不传则创建）
   * @param {number} options.shutdownTimeoutMs 优雅关闭超时（默认 30s）
   * @param {number} options.drainTimeoutMs    请求排空超时（默认 10s）
   * @param {Function[]} options.preBootstrapHooks  预启动钩子
   * @param {Function[]} options.postBootstrapHooks 后启动钩子
   * @param {Function[]} options.preShutdownHooks    预关闭钩子
   */
  constructor(options = {}) {
    super();
    this.appId = `app-${crypto.randomBytes(6).toString('hex')}`;
    this.appName = options.appName || 'MOX Enterprise';
    this.version = options.version || '1.0.0';
    this.shutdownTimeoutMs = options.shutdownTimeoutMs || 30000;
    this.drainTimeoutMs = options.drainTimeoutMs || 10000;

    // 核心组件
    this.config = options.config || new EnterpriseConfig();
    this.registry = options.registry || new ModuleRegistry();
    this.di = options.diContainer || new DIContainer();
    this.eventBus = options.eventBus || new EventBus();
    this.health = options.health || new HealthAggregator();
    this.middleware = options.middleware || new MiddlewareAssembler();
    this.router = options.router || new RouterAggregator();

    // 生命周期管理器
    this.lifecycle = new ModuleLifecycleManager({
      registry: this.registry,
      diContainer: this.di,
      config: this.config,
    });

    // 钩子
    this.preBootstrapHooks = options.preBootstrapHooks || [];
    this.postBootstrapHooks = options.postBootstrapHooks || [];
    this.preShutdownHooks = options.preShutdownHooks || [];

    // 状态
    this.state = APP_STATE.UNINITIALIZED;
    this.bootstrapReport = null;
    this.startTime = null;
    this.readyTime = null;
    this.httpServer = null;

    // 注册核心组件到 DI
    this._registerCoreComponents();

    // 监听进程信号
    this._setupSignalHandlers();
  }

  /**
   * 注册模块
   */
  registerModule(descriptor) {
    const mod = this.registry.register(descriptor);
    this.emit('app:module_registered', { name: descriptor.name });
    return mod;
  }

  /**
   * 批量注册模块
   */
  registerModules(descriptors) {
    const results = [];
    for (const desc of descriptors) {
      results.push(this.registerModule(desc));
    }
    return results;
  }

  /**
   * 启动应用（完整流程）
   */
  async bootstrap() {
    if (this.state !== APP_STATE.UNINITIALIZED && this.state !== APP_STATE.STOPPED) {
      throw new Error(`应用当前状态无法启动: ${this.state}`);
    }

    this.startTime = Date.now();
    this._setState(APP_STATE.BOOTSTRAPPING);
    this.emit('app:bootstrap_start', { appId: this.appId, appName: this.appName });

    const report = {
      appId: this.appId,
      appName: this.appName,
      version: this.version,
      phases: {},
      totalDurationMs: 0,
      errors: [],
    };

    try {
      // Phase 1: 预启动
      report.phases.preBootstrap = await this._phasePreBootstrap();

      // Phase 2: 基础设施启动
      report.phases.bootstrap = await this._phaseBootstrap();

      // Phase 3: 模块初始化
      this._setState(APP_STATE.INITIALIZING);
      report.phases.moduleInit = await this.lifecycle.initializeAll();

      // Phase 4: 模块启动
      this._setState(APP_STATE.STARTING);
      report.phases.moduleStart = await this.lifecycle.startAll();

      // Phase 5: 后启动
      report.phases.postBootstrap = await this._phasePostBootstrap();

      // Phase 6: 就绪
      this._setState(APP_STATE.READY);
      this.readyTime = Date.now();
      report.totalDurationMs = this.readyTime - this.startTime;
      this.bootstrapReport = report;

      this.emit('app:ready', {
        appId: this.appId,
        durationMs: report.totalDurationMs,
        moduleCount: this.registry.getStats().totalModules,
      });

      return report;

    } catch (err) {
      this._setState(APP_STATE.ERROR);
      report.errors.push(err.message);
      report.totalDurationMs = Date.now() - this.startTime;
      this.bootstrapReport = report;

      this.emit('app:bootstrap_failed', { error: err.message, report });
      throw err;
    }
  }

  async _phasePreBootstrap() {
    const start = Date.now();
    this.emit('app:phase:pre_bootstrap:start');

    // 执行预启动钩子
    for (const hook of this.preBootstrapHooks) {
      await hook(this);
    }

    // 加载配置
    await this.config.load();

    // 环境检查
    this._environmentCheck();

    const durationMs = Date.now() - start;
    this.emit('app:phase:pre_bootstrap:complete', { durationMs });
    return { durationMs, configLoaded: true, envChecked: true };
  }

  async _phaseBootstrap() {
    const start = Date.now();
    this.emit('app:phase:bootstrap:start');

    // 注册健康检查器
    this._registerCoreHealthChecks();

    // 注册核心事件
    this._registerCoreEventHandlers();

    const durationMs = Date.now() - start;
    this.emit('app:phase:bootstrap:complete', { durationMs });
    return { durationMs, healthChecksRegistered: true, eventHandlersRegistered: true };
  }

  async _phasePostBootstrap() {
    const start = Date.now();
    this.emit('app:phase:post_bootstrap:start');

    // 执行后启动钩子
    for (const hook of this.postBootstrapHooks) {
      await hook(this);
    }

    // 检测路由冲突
    const conflicts = this.router.detectConflicts();
    if (conflicts.length > 0) {
      this.emit('app:route_conflicts', { conflicts });
    }

    const durationMs = Date.now() - start;
    this.emit('app:phase:post_bootstrap:complete', { durationMs });
    return { durationMs, routeConflicts: conflicts.length, hooksExecuted: this.postBootstrapHooks.length };
  }

  /**
   * 优雅关闭
   */
  async shutdown(signal = 'SIGTERM') {
    if (this.state === APP_STATE.SHUTTING_DOWN || this.state === APP_STATE.STOPPED) {
      return;
    }

    this._setState(APP_STATE.SHUTTING_DOWN);
    this.emit('app:shutdown_start', { signal });

    const shutdownStart = Date.now();
    const report = { signal, phases: {}, errors: [] };

    try {
      // Phase 1: 预关闭钩子
      for (const hook of this.preShutdownHooks) {
        try { await hook(this); } catch (err) { report.errors.push(err.message); }
      }

      // Phase 2: 停止接收新请求 + 排空
      report.phases.drain = await this._drainConnections();

      // Phase 3: 停止模块（反向拓扑）
      report.phases.moduleStop = await this.lifecycle.stopAll();

      // Phase 4: 关闭核心组件
      report.phases.shutdown = await this._shutdownCore();

      this._setState(APP_STATE.STOPPED);
      report.totalDurationMs = Date.now() - shutdownStart;

      this.emit('app:shutdown_complete', report);
      return report;

    } catch (err) {
      report.errors.push(err.message);
      this.emit('app:shutdown_failed', { error: err.message, report });
      throw err;
    }
  }

  async _drainConnections() {
    const start = Date.now();

    if (this.httpServer) {
      this.httpServer.close();
      // 等待进行中请求完成
      await new Promise(resolve => {
        const timer = setTimeout(resolve, this.drainTimeoutMs);
        this.httpServer.once('close', () => {
          clearTimeout(timer);
          resolve();
        });
      });
    }

    return { durationMs: Date.now() - start, drained: true };
  }

  async _shutdownCore() {
    const start = Date.now();

    // 关闭事件总线
    await this.eventBus.destroy();

    // 关闭健康聚合器
    this.health.destroy();

    // 关闭 DI 容器
    await this.di.dispose();

    return { durationMs: Date.now() - start };
  }

  _registerCoreComponents() {
    this.di.registerValue('config', this.config);
    this.di.registerValue('registry', this.registry);
    this.di.registerValue('eventBus', this.eventBus);
    this.di.registerValue('health', this.health);
    this.di.registerValue('middleware', this.middleware);
    this.di.registerValue('router', this.router);
    this.di.registerValue('app', this);
  }

  _registerCoreHealthChecks() {
    // 应用自身健康
    this.health.register('app', async () => ({
      status: this.state === APP_STATE.READY ? 'healthy' : 'degraded',
      details: { state: this.state, uptimeMs: this.readyTime ? Date.now() - this.readyTime : 0 },
    }), { type: CHECK_TYPE.LIVENESS, criticality: 'critical', description: '应用存活检查' });

    // 模块注册中心健康
    this.health.register('module-registry', async () => {
      const stats = this.registry.getStats();
      return {
        status: stats.errorModules.length === 0 ? 'healthy' : 'degraded',
        details: { total: stats.totalModules, ready: stats.readyModules.length, errors: stats.errorModules },
      };
    }, { type: CHECK_TYPE.READINESS, criticality: 'important', description: '模块注册中心健康' });

    // 事件总线健康
    this.health.register('event-bus', async () => {
      const stats = this.eventBus.getStats();
      return {
        status: stats.deadLetterQueueSize < 100 ? 'healthy' : 'degraded',
        details: { queueSize: stats.queueSize, deadLetter: stats.deadLetterQueueSize },
      };
    }, { type: CHECK_TYPE.READINESS, criticality: 'optional', description: '事件总线健康' });
  }

  _registerCoreEventHandlers() {
    // 模块状态变更
    this.registry.on('module:status_changed', ({ name, newStatus }) => {
      if (newStatus === MODULE_STATUS.ERROR) {
        this.eventBus.publish('app.module.error', { module: name }, { priority: 1 });
      }
    });

    // 健康告警
    this.health.on('health:alert', ({ name, status }) => {
      this.eventBus.publish('app.health.alert', { checker: name, status }, { priority: 0 });
    });
  }

  _environmentCheck() {
    const checks = {
      nodeVersion: process.version,
      platform: process.platform,
      arch: process.arch,
      memory: process.memoryUsage(),
      cwd: process.cwd(),
      env: this.config.env,
    };

    // 检查必要环境变量
    const required = [];
    const missing = required.filter(v => !process.env[v]);
    if (missing.length > 0) {
      this.emit('app:env_warning', { missing });
    }

    this.emit('app:env_checked', checks);
    return checks;
  }

  _setState(state) {
    const oldState = this.state;
    this.state = state;
    this.emit('app:state_changed', { oldState, newState: state });
    this.emit(`app:${state}`, {});
  }

  _setupSignalHandlers() {
    const signals = ['SIGTERM', 'SIGINT', 'SIGUSR2'];
    for (const signal of signals) {
      process.on(signal, () => {
        this.emit('app:signal_received', { signal });
        this.shutdown(signal).catch(err => {
          this.emit('app:shutdown_error', { error: err.message });
          process.exit(1);
        });
      });
    }

    // 未捕获异常
    process.on('uncaughtException', (err) => {
      this.emit('app:uncaught_exception', { error: err.message, stack: err.stack });
      this._setState(APP_STATE.ERROR);
    });

    process.on('unhandledRejection', (reason) => {
      this.emit('app:unhandled_rejection', { reason: reason?.message || String(reason) });
    });
  }

  /**
   * 获取应用状态
   */
  getStatus() {
    return {
      appId: this.appId,
      appName: this.appName,
      version: this.version,
      state: this.state,
      startTime: this.startTime ? new Date(this.startTime).toISOString() : null,
      readyTime: this.readyTime ? new Date(this.readyTime).toISOString() : null,
      uptimeMs: this.readyTime ? Date.now() - this.readyTime : 0,
      moduleStats: this.registry.getStats(),
      healthStats: this.health.getStats(),
      eventBusStats: this.eventBus.getStats(),
      routerStats: this.router.getStats(),
      diStats: this.di.getStats(),
      configStats: this.config.getStats(),
    };
  }

  /**
   * 获取依赖图
   */
  getDependencyGraph() {
    return DependencyGraph.fromRegistry(this.registry);
  }

  /**
   * 获取启动报告
   */
  getBootstrapReport() {
    return this.bootstrapReport;
  }
}

module.exports = {
  AppBootstrap,
  APP_STATE,
  BOOTSTRAP_PHASE,
};
