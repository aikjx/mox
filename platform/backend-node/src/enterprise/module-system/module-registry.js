'use strict';

/**
 * MOX Enterprise · 模块注册中心
 * =============================
 * 所有企业级模块的统一注册、发现、查询入口
 *
 * 设计原则：
 *  - 每个模块声明自身元数据（name/version/dependencies/capabilities）
 *  - 注册中心维护全局模块清单，支持按名称/标签/能力查询
 *  - 模块状态机：unregistered → registered → initializing → ready → degraded → stopped → error
 *  - 支持热插拔（运行时注册/卸载模块）
 *  - 模块间通过注册中心发现彼此，不直接 require
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 模块状态 ───
const MODULE_STATUS = {
  UNREGISTERED: 'unregistered',
  REGISTERED: 'registered',
  INITIALIZING: 'initializing',
  READY: 'ready',
  DEGRADED: 'degraded',
  STOPPING: 'stopping',
  STOPPED: 'stopped',
  ERROR: 'error',
};

// ─── 模块健康度 ───
const MODULE_HEALTH = {
  HEALTHY: 'healthy',
  DEGRADED: 'degraded',
  UNHEALTHY: 'unhealthy',
  UNKNOWN: 'unknown',
};

class ModuleRegistry extends EventEmitter {
  constructor(options = {}) {
    super();
    this.modules = new Map();       // moduleName -> moduleDescriptor
    this.capabilities = new Map();   // capabilityName -> Set(moduleName)
    this.tags = new Map();           // tag -> Set(moduleName)
    this.registryId = `registry-${crypto.randomBytes(4).toString('hex')}`;
    this.startedAt = new Date().toISOString();
    this._maxListeners = 100;
  }

  /**
   * 注册一个模块
   * @param {object} descriptor 模块描述符
   * @param {string} descriptor.name        模块唯一名称
   * @param {string} descriptor.version     语义化版本
   * @param {string[]} descriptor.dependencies 依赖的模块名列表
   * @param {string[]} descriptor.capabilities 提供的能力列表
   * @param {string[]} descriptor.tags      标签列表
   * @param {object} descriptor.config      默认配置
   * @param {Function} descriptor.init      初始化函数 (context) => Promise<instance>
   * @param {Function} descriptor.start     启动函数 (instance) => Promise<void>
   * @param {Function} descriptor.stop      停止函数 (instance) => Promise<void>
   * @param {Function} descriptor.healthCheck 健康检查函数 => Promise<{status, details}>
   * @param {string} descriptor.description 模块描述
   * @param {string} descriptor.category    分类（storage/compute/security/observability/...）
   */
  register(descriptor) {
    if (!descriptor.name) throw new Error('模块必须声明 name');
    if (!descriptor.version) throw new Error('模块必须声明 version');
    if (this.modules.has(descriptor.name)) {
      throw new Error(`模块已注册: ${descriptor.name}`);
    }

    const moduleDescriptor = {
      moduleId: `mod-${crypto.randomBytes(6).toString('hex')}`,
      name: descriptor.name,
      version: descriptor.version,
      description: descriptor.description || '',
      category: descriptor.category || 'uncategorized',
      dependencies: descriptor.dependencies || [],
      optionalDependencies: descriptor.optionalDependencies || [],
      capabilities: descriptor.capabilities || [],
      tags: descriptor.tags || [],
      config: descriptor.config || {},
      init: descriptor.init || null,
      start: descriptor.start || null,
      stop: descriptor.stop || null,
      healthCheck: descriptor.healthCheck || null,
      status: MODULE_STATUS.REGISTERED,
      health: MODULE_HEALTH.UNKNOWN,
      instance: null,
      registeredAt: new Date().toISOString(),
      initializedAt: null,
      startedAt: null,
      stoppedAt: null,
      error: null,
      stats: {
        initDurationMs: null,
        startDurationMs: null,
        healthCheckCount: 0,
        lastHealthCheckAt: null,
      },
    };

    this.modules.set(descriptor.name, moduleDescriptor);

    // 索引能力
    for (const cap of moduleDescriptor.capabilities) {
      if (!this.capabilities.has(cap)) this.capabilities.set(cap, new Set());
      this.capabilities.get(cap).add(descriptor.name);
    }

    // 索引标签
    for (const tag of moduleDescriptor.tags) {
      if (!this.tags.has(tag)) this.tags.set(tag, new Set());
      this.tags.get(tag).add(descriptor.name);
    }

    this.emit('module:registered', { name: descriptor.name, version: descriptor.version });
    return moduleDescriptor;
  }

  /**
   * 注销模块
   */
  unregister(moduleName) {
    const mod = this.modules.get(moduleName);
    if (!mod) return false;

    // 检查是否有其他模块依赖它
    const dependents = this.getDependents(moduleName);
    if (dependents.length > 0 && mod.status === MODULE_STATUS.READY) {
      throw new Error(`无法注销模块 ${moduleName}，以下模块依赖它: ${dependents.join(', ')}`);
    }

    // 移除能力索引
    for (const cap of mod.capabilities) {
      const set = this.capabilities.get(cap);
      if (set) {
        set.delete(moduleName);
        if (set.size === 0) this.capabilities.delete(cap);
      }
    }

    // 移除标签索引
    for (const tag of mod.tags) {
      const set = this.tags.get(tag);
      if (set) {
        set.delete(moduleName);
        if (set.size === 0) this.tags.delete(tag);
      }
    }

    this.modules.delete(moduleName);
    this.emit('module:unregistered', { name: moduleName });
    return true;
  }

  /**
   * 获取模块描述符
   */
  get(moduleName) {
    return this.modules.get(moduleName) || null;
  }

  /**
   * 获取模块实例
   */
  getInstance(moduleName) {
    const mod = this.modules.get(moduleName);
    return mod ? mod.instance : null;
  }

  /**
   * 检查模块是否存在且就绪
   */
  isReady(moduleName) {
    const mod = this.modules.get(moduleName);
    return mod ? mod.status === MODULE_STATUS.READY : false;
  }

  /**
   * 按能力查找模块
   */
  findByCapability(capability) {
    const names = this.capabilities.get(capability);
    if (!names) return [];
    return Array.from(names).map(n => this.modules.get(n)).filter(Boolean);
  }

  /**
   * 按标签查找模块
   */
  findByTag(tag) {
    const names = this.tags.get(tag);
    if (!names) return [];
    return Array.from(names).map(n => this.modules.get(n)).filter(Boolean);
  }

  /**
   * 按分类查找模块
   */
  findByCategory(category) {
    return Array.from(this.modules.values()).filter(m => m.category === category);
  }

  /**
   * 获取模块的依赖（递归）
   */
  getDependencies(moduleName, recursive = false) {
    const mod = this.modules.get(moduleName);
    if (!mod) return [];

    if (!recursive) return [...mod.dependencies];

    const result = new Set();
    const stack = [...mod.dependencies];
    while (stack.length > 0) {
      const dep = stack.pop();
      if (result.has(dep)) continue;
      result.add(dep);
      const depMod = this.modules.get(dep);
      if (depMod) stack.push(...depMod.dependencies);
    }
    return Array.from(result);
  }

  /**
   * 获取依赖该模块的所有模块
   */
  getDependents(moduleName) {
    const dependents = [];
    for (const [name, mod] of this.modules) {
      if (mod.dependencies.includes(moduleName)) dependents.push(name);
    }
    return dependents;
  }

  /**
   * 获取所有模块名
   */
  listNames() {
    return Array.from(this.modules.keys());
  }

  /**
   * 获取所有模块（可按状态过滤）
   */
  list(status = null) {
    let mods = Array.from(this.modules.values());
    if (status) mods = mods.filter(m => m.status === status);
    return mods;
  }

  /**
   * 更新模块状态
   */
  setStatus(moduleName, status, error = null) {
    const mod = this.modules.get(moduleName);
    if (!mod) throw new Error(`模块不存在: ${moduleName}`);

    const oldStatus = mod.status;
    mod.status = status;
    if (error) mod.error = error;

    if (status === MODULE_STATUS.READY) mod.startedAt = new Date().toISOString();
    if (status === MODULE_STATUS.STOPPED) mod.stoppedAt = new Date().toISOString();

    this.emit('module:status_changed', { name: moduleName, oldStatus, newStatus: status });
    this.emit(`module:${moduleName}:${status}`, { name: moduleName });
  }

  /**
   * 更新模块健康度
   */
  setHealth(moduleName, health, details = {}) {
    const mod = this.modules.get(moduleName);
    if (!mod) return;

    const oldHealth = mod.health;
    mod.health = health;
    mod.healthDetails = details;
    mod.stats.lastHealthCheckAt = new Date().toISOString();
    mod.stats.healthCheckCount++;

    if (oldHealth !== health) {
      this.emit('module:health_changed', { name: moduleName, oldHealth, newHealth: health, details });
    }
  }

  /**
   * 获取注册中心统计
   */
  getStats() {
    const all = Array.from(this.modules.values());
    return {
      registryId: this.registryId,
      startedAt: this.startedAt,
      totalModules: all.length,
      byStatus: all.reduce((acc, m) => {
        acc[m.status] = (acc[m.status] || 0) + 1;
        return acc;
      }, {}),
      byHealth: all.reduce((acc, m) => {
        acc[m.health] = (acc[m.health] || 0) + 1;
        return acc;
      }, {}),
      byCategory: all.reduce((acc, m) => {
        acc[m.category] = (acc[m.category] || 0) + 1;
        return acc;
      }, {}),
      totalCapabilities: this.capabilities.size,
      totalTags: this.tags.size,
      readyModules: all.filter(m => m.status === MODULE_STATUS.READY).map(m => m.name),
      errorModules: all.filter(m => m.status === MODULE_STATUS.ERROR).map(m => m.name),
    };
  }

  /**
   * 导出模块清单（用于诊断/文档）
   */
  exportManifest() {
    return {
      registryId: this.registryId,
      exportedAt: new Date().toISOString(),
      modules: Array.from(this.modules.values()).map(m => ({
        name: m.name,
        version: m.version,
        description: m.description,
        category: m.category,
        status: m.status,
        health: m.health,
        dependencies: m.dependencies,
        optionalDependencies: m.optionalDependencies,
        capabilities: m.capabilities,
        tags: m.tags,
        registeredAt: m.registeredAt,
        startedAt: m.startedAt,
      })),
    };
  }
}

// 全局单例
let _globalRegistry = null;
function getGlobalRegistry() {
  if (!_globalRegistry) _globalRegistry = new ModuleRegistry();
  return _globalRegistry;
}

module.exports = {
  ModuleRegistry,
  MODULE_STATUS,
  MODULE_HEALTH,
  getGlobalRegistry,
};
