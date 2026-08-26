'use strict';

/**
 * MOX Enterprise · 模块生命周期管理器
 * ==================================
 * 统一管理所有模块的初始化、启动、健康检查、优雅关闭
 *
 * 生命周期阶段：
 *  register → configure → initialize → start → ready → (health-check loop) → stop → destroy
 *
 * 核心能力：
 *  - 按依赖拓扑顺序并行/串行启动
 *  - 启动超时控制与失败回滚
 *  - 健康检查调度（间隔/超时/重试）
 *  - 优雅关闭（反向拓扑顺序，超时强制终止）
 *  - 模块降级（单个模块失败不影响整体）
 *  - 生命周期事件钩子（before/after each phase）
 *  - 启动进度追踪与报告
 */

const { EventEmitter } = require('events');
const { MODULE_STATUS, MODULE_HEALTH } = require('./module-registry');
const { DependencyGraph } = require('./dependency-graph');

// ─── 生命周期阶段 ───
const LIFECYCLE_PHASE = {
  REGISTER: 'register',
  CONFIGURE: 'configure',
  INITIALIZE: 'initialize',
  START: 'start',
  READY: 'ready',
  HEALTH_CHECK: 'health_check',
  STOP: 'stop',
  DESTROY: 'destroy',
};

// ─── 启动策略 ───
const START_STRATEGY = {
  PARALLEL: 'parallel',     // 同层并行
  SERIAL: 'serial',         // 全部串行
  PARALLEL_WITH_BARRIER: 'parallel_with_barrier', // 同层并行，层间屏障
};

class ModuleLifecycleManager extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.registry     模块注册中心
   * @param {object} options.diContainer  DI 容器
   * @param {object} options.config       配置中心
   * @param {string} options.startStrategy 启动策略（默认 parallel_with_barrier）
   * @param {number} options.initTimeoutMs  单个模块初始化超时（默认 30s）
   * @param {number} options.startTimeoutMs 单个模块启动超时（默认 30s）
   * @param {number} options.stopTimeoutMs  单个模块停止超时（默认 15s）
   * @param {number} options.healthCheckIntervalMs 健康检查间隔（默认 30s）
   * @param {number} options.healthCheckTimeoutMs  健康检查超时（默认 5s）
   * @param {boolean} options.failFast    启动失败是否立即终止（默认 false，降级模式）
   */
  constructor(options = {}) {
    super();
    this.registry = options.registry;
    this.diContainer = options.diContainer;
    this.config = options.config;
    this.startStrategy = options.startStrategy || START_STRATEGY.PARALLEL_WITH_BARRIER;
    this.initTimeoutMs = options.initTimeoutMs || 30000;
    this.startTimeoutMs = options.startTimeoutMs || 30000;
    this.stopTimeoutMs = options.stopTimeoutMs || 15000;
    this.healthCheckIntervalMs = options.healthCheckIntervalMs || 30000;
    this.healthCheckTimeoutMs = options.healthCheckTimeoutMs || 5000;
    this.failFast = options.failFast || false;

    this._started = false;
    this._stopping = false;
    this._healthCheckTimer = null;
    this._startReport = null;
  }

  /**
   * 初始化所有模块（按拓扑顺序）
   */
  async initializeAll() {
    this.emit('lifecycle:initialize_all:start');

    const graph = DependencyGraph.fromRegistry(this.registry);
    const layers = graph.computeLayers();
    const report = { phase: 'initialize', layers: [], totalDurationMs: 0, failed: [] };
    const startTime = Date.now();

    for (let layerIndex = 0; layerIndex < layers.length; layerIndex++) {
      const layer = layers[layerIndex];
      const layerStart = Date.now();
      const layerResults = [];

      this.emit('lifecycle:layer:start', { layerIndex, modules: layer });

      if (this.startStrategy === START_STRATEGY.SERIAL) {
        for (const moduleName of layer) {
          const result = await this._initializeModule(moduleName);
          layerResults.push(result);
        }
      } else {
        // 并行
        const promises = layer.map(name => this._initializeModule(name));
        layerResults.push(...await Promise.allSettled(promises));
      }

      const layerDuration = Date.now() - layerStart;
      report.layers.push({
        layerIndex,
        modules: layer,
        durationMs: layerDuration,
        results: layerResults.map(r => r.status === 'fulfilled' ? r.value : { name: r.reason?.moduleName || 'unknown', success: false, error: r.reason?.message }),
      });

      // 检查失败
      const failures = layerResults.filter(r => r.status === 'rejected' || (r.value && !r.value.success));
      if (failures.length > 0) {
        report.failed.push(...failures.map(f => f.reason?.moduleName || f.value?.name));
        if (this.failFast) {
          this.emit('lifecycle:initialize_all:failed', { report });
          throw new Error(`初始化失败，模块: ${report.failed.join(', ')}`);
        }
      }

      this.emit('lifecycle:layer:complete', { layerIndex, durationMs: layerDuration });
    }

    report.totalDurationMs = Date.now() - startTime;
    this.emit('lifecycle:initialize_all:complete', report);
    return report;
  }

  /**
   * 启动所有模块
   */
  async startAll() {
    if (this._started) throw new Error('模块已启动');

    this.emit('lifecycle:start_all:start');

    const graph = DependencyGraph.fromRegistry(this.registry);
    const layers = graph.computeLayers();
    const report = { phase: 'start', layers: [], totalDurationMs: 0, failed: [] };
    const startTime = Date.now();

    for (let layerIndex = 0; layerIndex < layers.length; layerIndex++) {
      const layer = layers[layerIndex];
      const layerStart = Date.now();

      this.emit('lifecycle:start_layer:start', { layerIndex, modules: layer });

      const promises = layer.map(name => this._startModule(name));
      const results = await Promise.allSettled(promises);

      const layerDuration = Date.now() - layerStart;
      report.layers.push({
        layerIndex,
        modules: layer,
        durationMs: layerDuration,
        results: results.map(r => r.status === 'fulfilled' ? r.value : { name: 'unknown', success: false, error: r.reason?.message }),
      });

      const failures = results.filter(r => r.status === 'rejected');
      if (failures.length > 0) {
        report.failed.push(...failures.map(f => f.reason?.moduleName || 'unknown'));
        if (this.failFast) {
          this.emit('lifecycle:start_all:failed', { report });
          throw new Error(`启动失败，模块: ${report.failed.join(', ')}`);
        }
      }

      this.emit('lifecycle:start_layer:complete', { layerIndex, durationMs: layerDuration });
    }

    report.totalDurationMs = Date.now() - startTime;
    this._started = true;
    this._startReport = report;

    // 启动健康检查循环
    this._startHealthCheckLoop();

    this.emit('lifecycle:start_all:complete', report);
    this.emit('lifecycle:all_ready', { report });

    return report;
  }

  /**
   * 停止所有模块（反向拓扑顺序）
   */
  async stopAll() {
    if (!this._started || this._stopping) return;
    this._stopping = true;

    this.emit('lifecycle:stop_all:start');

    // 停止健康检查
    if (this._healthCheckTimer) {
      clearInterval(this._healthCheckTimer);
      this._healthCheckTimer = null;
    }

    const graph = DependencyGraph.fromRegistry(this.registry);
    const reverseOrder = graph.reverseTopologicalSort();
    const report = { phase: 'stop', modules: [], totalDurationMs: 0, failed: [] };
    const startTime = Date.now();

    for (const moduleName of reverseOrder) {
      try {
        const result = await this._stopModule(moduleName);
        report.modules.push(result);
      } catch (err) {
        report.failed.push(moduleName);
        this.emit('lifecycle:stop_module:failed', { moduleName, error: err.message });
      }
    }

    report.totalDurationMs = Date.now() - startTime;
    this._started = false;
    this._stopping = false;

    this.emit('lifecycle:stop_all:complete', report);
    return report;
  }

  /**
   * 重启单个模块
   */
  async restartModule(moduleName) {
    this.emit('lifecycle:restart:start', { moduleName });
    await this._stopModule(moduleName);
    await this._initializeModule(moduleName);
    await this._startModule(moduleName);
    this.emit('lifecycle:restart:complete', { moduleName });
  }

  async _initializeModule(moduleName) {
    const mod = this.registry.get(moduleName);
    if (!mod) return { name: moduleName, success: false, error: '模块不存在' };
    if (mod.status === MODULE_STATUS.READY || mod.status === MODULE_STATUS.INITIALIZING) {
      return { name: moduleName, success: true, skipped: true };
    }

    this.registry.setStatus(moduleName, MODULE_STATUS.INITIALIZING);
    const start = Date.now();

    try {
      if (mod.init) {
        const context = {
          config: this.config,
          di: this.diContainer,
          registry: this.registry,
          moduleName,
        };
        const instance = await this._withTimeout(
          mod.init(context),
          this.initTimeoutMs,
          `模块 ${moduleName} 初始化超时`
        );
        mod.instance = instance;
      }

      mod.stats.initDurationMs = Date.now() - start;
      mod.initializedAt = new Date().toISOString();
      this.registry.setStatus(moduleName, MODULE_STATUS.REGISTERED); // 初始化完成但未启动

      this.emit('lifecycle:module_initialized', { moduleName, durationMs: mod.stats.initDurationMs });
      return { name: moduleName, success: true, durationMs: mod.stats.initDurationMs };

    } catch (err) {
      this.registry.setStatus(moduleName, MODULE_STATUS.ERROR, err.message);
      this.emit('lifecycle:module_init_failed', { moduleName, error: err.message });
      throw { moduleName, message: err.message };
    }
  }

  async _startModule(moduleName) {
    const mod = this.registry.get(moduleName);
    if (!mod) return { name: moduleName, success: false, error: '模块不存在' };
    if (mod.status === MODULE_STATUS.READY) {
      return { name: moduleName, success: true, skipped: true };
    }

    const start = Date.now();

    try {
      if (mod.start && mod.instance) {
        await this._withTimeout(
          mod.start(mod.instance),
          this.startTimeoutMs,
          `模块 ${moduleName} 启动超时`
        );
      }

      mod.stats.startDurationMs = Date.now() - start;
      this.registry.setStatus(moduleName, MODULE_STATUS.READY);
      this.registry.setHealth(moduleName, MODULE_HEALTH.HEALTHY);

      this.emit('lifecycle:module_started', { moduleName, durationMs: mod.stats.startDurationMs });
      return { name: moduleName, success: true, durationMs: mod.stats.startDurationMs };

    } catch (err) {
      this.registry.setStatus(moduleName, MODULE_STATUS.ERROR, err.message);
      this.registry.setHealth(moduleName, MODULE_HEALTH.UNHEALTHY);
      this.emit('lifecycle:module_start_failed', { moduleName, error: err.message });
      throw { moduleName, message: err.message };
    }
  }

  async _stopModule(moduleName) {
    const mod = this.registry.get(moduleName);
    if (!mod || mod.status === MODULE_STATUS.STOPPED) {
      return { name: moduleName, success: true, skipped: true };
    }

    this.registry.setStatus(moduleName, MODULE_STATUS.STOPPING);
    const start = Date.now();

    try {
      if (mod.stop && mod.instance) {
        await this._withTimeout(
          mod.stop(mod.instance),
          this.stopTimeoutMs,
          `模块 ${moduleName} 停止超时`
        );
      }

      this.registry.setStatus(moduleName, MODULE_STATUS.STOPPED);
      this.registry.setHealth(moduleName, MODULE_HEALTH.UNKNOWN);

      const durationMs = Date.now() - start;
      this.emit('lifecycle:module_stopped', { moduleName, durationMs });
      return { name: moduleName, success: true, durationMs };

    } catch (err) {
      this.registry.setStatus(moduleName, MODULE_STATUS.ERROR, err.message);
      this.emit('lifecycle:module_stop_failed', { moduleName, error: err.message });
      return { name: moduleName, success: false, error: err.message };
    }
  }

  _startHealthCheckLoop() {
    this._healthCheckTimer = setInterval(() => {
      this._runHealthChecks().catch(err => {
        this.emit('lifecycle:health_check_loop:error', { error: err.message });
      });
    }, this.healthCheckIntervalMs);
  }

  async _runHealthChecks() {
    const readyModules = this.registry.list(MODULE_STATUS.READY);
    const results = [];

    for (const mod of readyModules) {
      if (!mod.healthCheck) continue;

      try {
        const result = await this._withTimeout(
          mod.healthCheck(mod.instance),
          this.healthCheckTimeoutMs,
          `模块 ${mod.name} 健康检查超时`
        );

        const health = result?.status || MODULE_HEALTH.HEALTHY;
        this.registry.setHealth(mod.name, health, result?.details || {});
        results.push({ module: mod.name, health, details: result?.details });

        // 健康降级处理
        if (health === MODULE_HEALTH.UNHEALTHY) {
          this.emit('lifecycle:module_unhealthy', { module: mod.name, details: result?.details });
        }
      } catch (err) {
        this.registry.setHealth(mod.name, MODULE_HEALTH.UNHEALTHY, { error: err.message });
        results.push({ module: mod.name, health: MODULE_HEALTH.UNHEALTHY, error: err.message });
        this.emit('lifecycle:health_check_failed', { module: mod.name, error: err.message });
      }
    }

    this.emit('lifecycle:health_check:complete', { results, timestamp: new Date().toISOString() });
    return results;
  }

  async _withTimeout(promise, timeoutMs, timeoutMessage) {
    return Promise.race([
      promise,
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs)
      ),
    ]);
  }

  /**
   * 获取启动报告
   */
  getStartReport() {
    return this._startReport;
  }

  /**
   * 获取生命周期状态
   */
  getStatus() {
    return {
      started: this._started,
      stopping: this._stopping,
      healthCheckRunning: !!this._healthCheckTimer,
      startStrategy: this.startStrategy,
      timeouts: {
        init: this.initTimeoutMs,
        start: this.startTimeoutMs,
        stop: this.stopTimeoutMs,
        healthCheck: this.healthCheckTimeoutMs,
      },
      intervals: {
        healthCheck: this.healthCheckIntervalMs,
      },
    };
  }
}

module.exports = {
  ModuleLifecycleManager,
  LIFECYCLE_PHASE,
  START_STRATEGY,
};
