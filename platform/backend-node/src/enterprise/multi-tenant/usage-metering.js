'use strict';

/**
 * MOX Enterprise · 多租户用量采集器
 * ==================================
 * 实时采集各租户的资源使用量，用于配额检查和计费
 *
 * 采集维度：
 *  - 存储用量（字节数 + 对象数）
 *  - API 调用次数（按接口分类）
 *  - 出口流量（字节数）
 *  - 计算资源（Spark 核秒数）
 *  - 并发连接数
 *
 * 采集方式：
 *  - 实时计数器（内存 + Redis）
 *  - 定时聚合（分钟/小时/天）
 *  - 写入数据湖（Iceberg usage_metrics 表）
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 用量类型 ───
const USAGE_TYPE = {
  STORAGE_BYTES: 'storage_bytes',
  OBJECT_COUNT: 'object_count',
  API_CALLS: 'api_calls',
  EGRESS_BYTES: 'egress_bytes',
  INGRESS_BYTES: 'ingress_bytes',
  COMPUTE_CORE_SECONDS: 'compute_core_seconds',
  CONCURRENT_CONNECTIONS: 'concurrent_connections',
  DATA_TRANSFER: 'data_transfer',
};

// ─── 聚合粒度 ───
const AGGREGATION_GRANULARITY = {
  MINUTE: 'minute',
  HOUR: 'hour',
  DAY: 'day',
  MONTH: 'month',
};

class UsageMeter extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.redisClient   Redis 客户端（用于分布式计数器）
   * @param {object} options.icebergWriter Iceberg 写入器（用于持久化用量数据）
   * @param {number} options.aggregateIntervalMs 聚合间隔（默认 60 秒）
   * @param {number} options.flushIntervalMs     刷新到数据湖间隔（默认 3600 秒）
   * @param {string} options.defaultGranularity   默认聚合粒度
   */
  constructor(options = {}) {
    super();
    this.redisClient = options.redisClient;
    this.icebergWriter = options.icebergWriter;
    this.aggregateIntervalMs = options.aggregateIntervalMs || 60000;
    this.flushIntervalMs = options.flushIntervalMs || 3600000;
    this.defaultGranularity = options.defaultGranularity || AGGREGATION_GRANULARITY.HOUR;

    // 内存计数器：tenantId -> usageType -> { count, bytes, timestamps[] }
    this.counters = new Map();

    // 历史用量缓存
    this.historyCache = new Map(); // `${tenantId}:${type}:${period}` -> value

    this._startAggregateLoop();
    this._startFlushLoop();
  }

  /**
   * 记录一次用量
   * @param {string} tenantId  租户 ID
   * @param {string} usageType 用量类型
   * @param {number} amount    用量数量（默认 1）
   * @param {object} [metadata] 附加元数据
   */
  record(tenantId, usageType, amount = 1, metadata = {}) {
    const key = `${tenantId}:${usageType}`;
    if (!this.counters.has(key)) {
      this.counters.set(key, { count: 0, bytes: 0, timestamps: [], metadata: {} });
    }

    const counter = this.counters.get(key);
    counter.count += amount;
    if (metadata.bytes) counter.bytes += metadata.bytes;
    counter.timestamps.push(Date.now());
    if (counter.timestamps.length > 1000) {
      counter.timestamps = counter.timestamps.slice(-1000);
    }
    Object.assign(counter.metadata, metadata);

    // Redis 分布式计数（如果配置了）
    if (this.redisClient) {
      this.redisClient.incrby(`usage:${tenantId}:${usageType}:count`, amount).catch(() => {});
      if (metadata.bytes) {
        this.redisClient.incrby(`usage:${tenantId}:${usageType}:bytes`, metadata.bytes).catch(() => {});
      }
    }

    this.emit('usage:recorded', { tenantId, usageType, amount, metadata });
  }

  /**
   * 记录存储用量（对象写入时调用）
   */
  recordStorage(tenantId, sizeBytes, objectCount = 1) {
    this.record(tenantId, USAGE_TYPE.STORAGE_BYTES, sizeBytes, { bytes: sizeBytes });
    this.record(tenantId, USAGE_TYPE.OBJECT_COUNT, objectCount);
  }

  /**
   * 记录 API 调用
   */
  recordApiCall(tenantId, apiPath, method, statusCode, durationMs) {
    this.record(tenantId, USAGE_TYPE.API_CALLS, 1, {
      apiPath,
      method,
      statusCode,
      durationMs,
    });
  }

  /**
   * 记录出口流量
   */
  recordEgress(tenantId, bytes) {
    this.record(tenantId, USAGE_TYPE.EGRESS_BYTES, bytes, { bytes });
  }

  /**
   * 记录计算用量
   */
  recordCompute(tenantId, coreSeconds) {
    this.record(tenantId, USAGE_TYPE.COMPUTE_CORE_SECONDS, coreSeconds);
  }

  /**
   * 获取当前周期用量
   * @param {string} tenantId
   * @param {string} usageType
   * @param {string} [period] 周期（current_minute/current_hour/current_day/current_month）
   */
  async getCurrentUsage(tenantId, usageType, period = 'current_hour') {
    // 优先从 Redis 获取
    if (this.redisClient) {
      try {
        const periodKey = this._getPeriodKey(period);
        const count = await this.redisClient.get(`usage:${tenantId}:${usageType}:${periodKey}:count`);
        const bytes = await this.redisClient.get(`usage:${tenantId}:${usageType}:${periodKey}:bytes`);
        if (count !== null) return parseInt(count, 10);
        if (bytes !== null) return parseInt(bytes, 10);
      } catch {}
    }

    // 从内存计数器获取
    const key = `${tenantId}:${usageType}`;
    const counter = this.counters.get(key);
    return counter ? counter.count : 0;
  }

  /**
   * 获取租户用量汇总
   */
  async getTenantUsageSummary(tenantId, period = 'current_day') {
    const types = Object.values(USAGE_TYPE);
    const summary = { tenantId, period, timestamp: new Date().toISOString(), usage: {} };

    for (const type of types) {
      summary.usage[type] = await this.getCurrentUsage(tenantId, type, period);
    }

    return summary;
  }

  /**
   * 获取用量趋势（按时间序列）
   */
  async getUsageTrend(tenantId, usageType, granularity = AGGREGATION_GRANULARITY.DAY, days = 30) {
    // 从数据湖查询
    // const result = await this.icebergQuery.execute(
    //   `SELECT date_trunc('${granularity}', metric_date) as period, SUM(usage_amount) as amount
    //    FROM usage_metrics WHERE tenant_id = ? AND usage_type = ?
    //    AND metric_date >= current_date - interval '${days}' day
    //    GROUP BY 1 ORDER BY 1`,
    //   { params: [tenantId, usageType] }
    // );
    return { tenantId, usageType, granularity, days, trend: [] };
  }

  /**
   * 聚合当前计数器到历史数据
   */
  async _aggregate() {
    const now = new Date();
    const periodKey = this._getPeriodKey('current_minute');
    const aggregated = [];

    for (const [key, counter] of this.counters) {
      const [tenantId, usageType] = key.split(':');
      const record = {
        tenant_id: tenantId,
        usage_type: usageType,
        usage_amount: counter.count,
        usage_bytes: counter.bytes,
        metric_date: now.toISOString().slice(0, 10),
        metric_hour: now.getHours(),
        metric_minute: now.getMinutes(),
        period: periodKey,
        timestamp: now.toISOString(),
      };
      aggregated.push(record);

      // 缓存到历史
      this.historyCache.set(`${tenantId}:${usageType}:${periodKey}`, counter.count);

      // 重置计数器
      counter.count = 0;
      counter.bytes = 0;
      counter.timestamps = [];
    }

    if (aggregated.length > 0) {
      this.emit('usage:aggregated', { count: aggregated.length, period: periodKey });

      // 写入数据湖
      if (this.icebergWriter) {
        try {
          await this.icebergWriter.append('usage_metrics', aggregated);
        } catch (err) {
          this.emit('usage:flush_error', { error: err.message });
        }
      }
    }

    return aggregated;
  }

  /**
   * 刷新到数据湖（持久化）
   */
  async _flushToDataLake() {
    // 聚合所有待刷新数据
    const aggregated = await this._aggregate();
    if (aggregated.length > 0) {
      this.emit('usage:flushed', { count: aggregated.length });
    }
    return aggregated;
  }

  _getPeriodKey(period) {
    const now = new Date();
    switch (period) {
      case 'current_minute':
        return now.toISOString().slice(0, 16);
      case 'current_hour':
        return now.toISOString().slice(0, 13);
      case 'current_day':
        return now.toISOString().slice(0, 10);
      case 'current_month':
        return now.toISOString().slice(0, 7);
      default:
        return now.toISOString().slice(0, 13);
    }
  }

  _startAggregateLoop() {
    setInterval(() => this._aggregate().catch(err => {
      this.emit('aggregate:error', { error: err.message });
    }), this.aggregateIntervalMs);
  }

  _startFlushLoop() {
    setInterval(() => this._flushToDataLake().catch(err => {
      this.emit('flush:error', { error: err.message });
    }), this.flushIntervalMs);
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      activeCounters: this.counters.size,
      historyCacheSize: this.historyCache.size,
      redisEnabled: !!this.redisClient,
      icebergEnabled: !!this.icebergWriter,
      aggregateIntervalMs: this.aggregateIntervalMs,
      flushIntervalMs: this.flushIntervalMs,
    };
  }

  async close() {
    await this._flushToDataLake();
    this.removeAllListeners();
  }
}

module.exports = {
  UsageMeter,
  USAGE_TYPE,
  AGGREGATION_GRANULARITY,
};
