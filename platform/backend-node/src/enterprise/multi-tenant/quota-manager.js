'use strict';

/**
 * MOX Enterprise · 多租户配额管理器
 * ==================================
 * 租户级资源配额管理与实时限流
 *
 * 配额维度：
 *  - 存储容量（GB/TB）
 *  - 对象数量
 *  - API 调用速率（QPS）
 *  - 出口流量（GB/月）
 *  - 计算资源（Spark 核数/内存）
 *  - 并发连接数
 *
 * 配额层级：
 *  - 全局默认配额
 *  - 租户套餐配额（free/pro/enterprise）
 *  - 自定义配额（覆盖套餐）
 *
 * 超限行为：
 *  - 软限制：告警通知，允许继续使用
 *  - 硬限制：拒绝请求（429 Too Many Requests / 403 Forbidden）
 *  - 分级限流：逐步降低速率
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 配额类型 ───
const QUOTA_TYPE = {
  STORAGE_BYTES: 'storage_bytes',
  OBJECT_COUNT: 'object_count',
  API_QPS: 'api_qps',
  EGRESS_BYTES: 'egress_bytes',
  COMPUTE_CORES: 'compute_cores',
  CONCURRENT_CONNECTIONS: 'concurrent_connections',
};

// ─── 配额套餐 ───
const QUOTA_PLANS = {
  free: {
    name: '免费版',
    quotas: {
      storage_bytes: 10 * 1024 ** 3,       // 10 GB
      object_count: 100000,
      api_qps: 10,
      egress_bytes: 10 * 1024 ** 3,        // 10 GB/月
      compute_cores: 0,
      concurrent_connections: 10,
    },
    price_monthly: 0,
  },
  pro: {
    name: '专业版',
    quotas: {
      storage_bytes: 1 * 1024 ** 4,         // 1 TB
      object_count: 10000000,
      api_qps: 100,
      egress_bytes: 100 * 1024 ** 3,       // 100 GB/月
      compute_cores: 4,
      concurrent_connections: 100,
    },
    price_monthly: 999,
  },
  enterprise: {
    name: '企业版',
    quotas: {
      storage_bytes: 100 * 1024 ** 4,       // 100 TB
      object_count: 1000000000,
      api_qps: 1000,
      egress_bytes: 1024 ** 4,               // 1 TB/月
      compute_cores: 64,
      concurrent_connections: 1000,
    },
    price_monthly: 9999,
  },
  unlimited: {
    name: '无限版',
    quotas: {
      storage_bytes: Infinity,
      object_count: Infinity,
      api_qps: Infinity,
      egress_bytes: Infinity,
      compute_cores: Infinity,
      concurrent_connections: Infinity,
    },
    price_monthly: -1, // 定制报价
  },
};

// ─── 超限行为 ───
const OVER_QUOTA_ACTION = {
  WARN: 'warn',               // 仅告警
  THROTTLE: 'throttle',       // 限流（降低速率）
  REJECT: 'reject',           // 拒绝新请求
  REJECT_WRITE: 'reject_write', // 仅拒绝写入，允许读取
  DOWNGRADE: 'downgrade',     // 降级服务
};

class QuotaManager extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.usageMeter   用量采集器实例
   * @param {string} options.defaultPlan   默认套餐（默认 free）
   * @param {number} options.checkIntervalMs 检查间隔（默认 60 秒）
   * @param {object} options.thresholds    告警阈值 { warn: 0.8, critical: 0.95 }
   */
  constructor(options = {}) {
    super();
    this.usageMeter = options.usageMeter;
    this.defaultPlan = options.defaultPlan || 'free';
    this.checkIntervalMs = options.checkIntervalMs || 60000;
    this.thresholds = options.thresholds || { warn: 0.8, critical: 0.95 };

    // 租户配额：tenantId -> { plan, customQuotas, overQuotaAction, status }
    this.tenantQuotas = new Map();

    // 实时 QPS 计数器（滑动窗口）
    this.qpsCounters = new Map(); // tenantId -> { timestamps: [], count }

    // 超限状态
    this.overQuotaStatus = new Map(); // tenantId -> { type, level, since }

    this._startCheckLoop();
  }

  /**
   * 初始化租户配额
   */
  initTenant(tenantId, plan = this.defaultPlan, customQuotas = null) {
    if (!QUOTA_PLANS[plan]) throw new Error(`未知套餐: ${plan}`);

    const tenantQuota = {
      tenantId,
      plan,
      planQuotas: { ...QUOTA_PLANS[plan].quotas },
      customQuotas: customQuotas ? { ...customQuotas } : null,
      overQuotaAction: OVER_QUOTA_ACTION.REJECT_WRITE,
      status: 'active',
      createdAt: new Date().toISOString(),
    };

    this.tenantQuotas.set(tenantId, tenantQuota);
    this.qpsCounters.set(tenantId, { timestamps: [], count: 0 });

    this.emit('quota:tenant_initialized', { tenantId, plan });
    return tenantQuota;
  }

  /**
   * 获取租户有效配额（custom 覆盖 plan）
   */
  getEffectiveQuota(tenantId, quotaType) {
    const tenant = this.tenantQuotas.get(tenantId);
    if (!tenant) return QUOTA_PLANS[this.defaultPlan].quotas[quotaType] || 0;

    if (tenant.customQuotas && tenant.customQuotas[quotaType] !== undefined) {
      return tenant.customQuotas[quotaType];
    }
    return tenant.planQuotas[quotaType] || 0;
  }

  /**
   * 获取租户所有有效配额
   */
  getAllEffectiveQuotas(tenantId) {
    const tenant = this.tenantQuotas.get(tenantId);
    if (!tenant) return { ...QUOTA_PLANS[this.defaultPlan].quotas };

    return {
      ...tenant.planQuotas,
      ...(tenant.customQuotas || {}),
    };
  }

  /**
   * 检查租户是否超过配额
   * @returns {object} { allowed: boolean, quotaType, usage, limit, percentage, action }
   */
  async checkQuota(tenantId, quotaType, requestedAmount = 1) {
    const limit = this.getEffectiveQuota(tenantId, quotaType);
    if (limit === Infinity) {
      return { allowed: true, quotaType, usage: 0, limit: Infinity, percentage: 0 };
    }

    // 获取当前用量
    const usage = await this._getCurrentUsage(tenantId, quotaType);
    const projectedUsage = usage + requestedAmount;
    const percentage = limit > 0 ? projectedUsage / limit : 1;

    const allowed = projectedUsage <= limit;
    const tenant = this.tenantQuotas.get(tenantId);
    const action = tenant?.overQuotaAction || OVER_QUOTA_ACTION.REJECT_WRITE;

    // 检查告警阈值
    if (percentage >= this.thresholds.critical) {
      this._triggerAlert(tenantId, quotaType, usage, limit, percentage, 'critical');
    } else if (percentage >= this.thresholds.warn) {
      this._triggerAlert(tenantId, quotaType, usage, limit, percentage, 'warn');
    }

    if (!allowed) {
      this._handleOverQuota(tenantId, quotaType, usage, limit, action);
    }

    return { allowed, quotaType, usage, limit, percentage, action, requestedAmount };
  }

  /**
   * 记录一次 API 调用（QPS 限流）
   * @returns {boolean} 是否允许
   */
  recordApiCall(tenantId) {
    const limit = this.getEffectiveQuota(tenantId, QUOTA_TYPE.API_QPS);
    if (limit === Infinity) return true;

    const counter = this.qpsCounters.get(tenantId) || { timestamps: [], count: 0 };
    const now = Date.now();

    // 滑动窗口：移除 1 秒前的记录
    counter.timestamps = counter.timestamps.filter(t => now - t < 1000);
    counter.timestamps.push(now);
    counter.count = counter.timestamps.length;

    this.qpsCounters.set(tenantId, counter);

    if (counter.count > limit) {
      this.emit('quota:qps_exceeded', { tenantId, currentQps: counter.count, limit });
      return false;
    }
    return true;
  }

  /**
   * Express 中间件：配额检查
   */
  quotaMiddleware(quotaType = QUOTA_TYPE.API_QPS) {
    return async (req, res, next) => {
      const tenantId = req.user?.tenantId || req.headers['x-tenant-id'];
      if (!tenantId) {
        return res.status(401).json({ error: 'missing_tenant_id' });
      }

      // QPS 检查
      if (quotaType === QUOTA_TYPE.API_QPS) {
        if (!this.recordApiCall(tenantId)) {
          return res.status(429).json({
            error: 'rate_limit_exceeded',
            message: 'API 调用频率超过配额',
            tenantId,
          });
        }
        return next();
      }

      // 其他配额检查
      const result = await this.checkQuota(tenantId, quotaType);
      if (!result.allowed) {
        return res.status(403).json({
          error: 'quota_exceeded',
          message: `配额不足: ${quotaType}`,
          quotaType,
          usage: result.usage,
          limit: result.limit,
          percentage: result.percentage,
        });
      }
      next();
    };
  }

  async _getCurrentUsage(tenantId, quotaType) {
    if (this.usageMeter) {
      return this.usageMeter.getCurrentUsage(tenantId, quotaType);
    }
    return 0;
  }

  _triggerAlert(tenantId, quotaType, usage, limit, percentage, level) {
    const alertId = `quota-alert-${crypto.randomBytes(6).toString('hex')}`;
    this.emit('quota:alert', {
      alertId,
      tenantId,
      quotaType,
      usage,
      limit,
      percentage,
      level,
      timestamp: new Date().toISOString(),
    });
  }

  _handleOverQuota(tenantId, quotaType, usage, limit, action) {
    this.overQuotaStatus.set(tenantId, { type: quotaType, level: 'breach', since: new Date() });
    this.emit('quota:exceeded', { tenantId, quotaType, usage, limit, action });

    switch (action) {
      case OVER_QUOTA_ACTION.REJECT:
      case OVER_QUOTA_ACTION.REJECT_WRITE:
        // 由中间件处理拒绝
        break;
      case OVER_QUOTA_ACTION.THROTTLE:
        // 降低 QPS 限制
        break;
      case OVER_QUOTA_ACTION.DOWNGRADE:
        // 降级到低频服务
        break;
    }
  }

  /**
   * 更新租户套餐
   */
  updatePlan(tenantId, newPlan, customQuotas = null) {
    const tenant = this.tenantQuotas.get(tenantId);
    if (!tenant) throw new Error(`租户不存在: ${tenantId}`);
    if (!QUOTA_PLANS[newPlan]) throw new Error(`未知套餐: ${newPlan}`);

    tenant.plan = newPlan;
    tenant.planQuotas = { ...QUOTA_PLANS[newPlan].quotas };
    if (customQuotas) tenant.customQuotas = { ...customQuotas };
    tenant.updatedAt = new Date().toISOString();

    this.emit('quota:plan_updated', { tenantId, newPlan });
    return tenant;
  }

  /**
   * 设置自定义配额
   */
  setCustomQuota(tenantId, quotaType, value) {
    const tenant = this.tenantQuotas.get(tenantId);
    if (!tenant) throw new Error(`租户不存在: ${tenantId}`);

    if (!tenant.customQuotas) tenant.customQuotas = {};
    tenant.customQuotas[quotaType] = value;
    tenant.updatedAt = new Date().toISOString();

    this.emit('quota:custom_set', { tenantId, quotaType, value });
    return tenant;
  }

  /**
   * 获取租户配额使用情况
   */
  async getTenantQuotaStatus(tenantId) {
    const quotas = this.getAllEffectiveQuotas(tenantId);
    const status = { tenantId, quotas: {}, timestamp: new Date().toISOString() };

    for (const [type, limit] of Object.entries(quotas)) {
      const usage = await this._getCurrentUsage(tenantId, type);
      status.quotas[type] = {
        limit,
        usage,
        remaining: limit === Infinity ? Infinity : Math.max(0, limit - usage),
        percentage: limit === Infinity ? 0 : (limit > 0 ? usage / limit : 1),
      };
    }

    return status;
  }

  _startCheckLoop() {
    setInterval(async () => {
      for (const [tenantId] of this.tenantQuotas) {
        try {
          await this.checkQuota(tenantId, QUOTA_TYPE.STORAGE_BYTES);
        } catch (err) {
          this.emit('quota:check_error', { tenantId, error: err.message });
        }
      }
    }, this.checkIntervalMs);
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      totalTenants: this.tenantQuotas.size,
      activeTenants: Array.from(this.tenantQuotas.values()).filter(t => t.status === 'active').length,
      overQuotaTenants: this.overQuotaStatus.size,
      plans: Object.keys(QUOTA_PLANS),
      tenantsByPlan: Array.from(this.tenantQuotas.values()).reduce((acc, t) => {
        acc[t.plan] = (acc[t.plan] || 0) + 1;
        return acc;
      }, {}),
    };
  }
}

module.exports = {
  QuotaManager,
  QUOTA_TYPE,
  QUOTA_PLANS,
  OVER_QUOTA_ACTION,
};
