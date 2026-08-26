'use strict';

/**
 * MOX Enterprise · FinOps 成本采集器
 * ====================================
 * 采集多云/多服务的成本数据，统一聚合到数据湖
 *
 * 采集来源：
 *  - AWS Cost Explorer / CUR（成本和使用报告）
 *  - 阿里云账单 API
 *  - 腾讯云账单 API
 *  - Kubernetes 资源成本（基于请求/限制估算）
 *  - 内部服务计量（存储/计算/网络）
 *
 * 输出：Iceberg 表 cost_records（按天/月分区）
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 云服务商 ───
const CLOUD_PROVIDERS = {
  AWS: 'aws',
  ALIYUN: 'aliyun',
  TENCENT: 'tencent',
  INTERNAL: 'internal',
};

// ─── 成本类别 ───
const COST_CATEGORIES = {
  STORAGE: 'storage',       // 存储（S3/EC/磁带）
  COMPUTE: 'compute',       // 计算（EC2/Spark/容器）
  NETWORK: 'network',       // 网络（出口流量/跨域复制）
  DATABASE: 'database',     // 数据库（TiKV/PG/Redis）
  MONITORING: 'monitoring', // 监控（Prometheus/ClickHouse）
  SECURITY: 'security',     // 安全（KMS/WAF/审计）
  OTHER: 'other',
};

class CostCollector extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.warehousePath   数据湖路径
   * @param {object} options.providers       云服务商配置
   * @param {string} options.collectionMode  采集模式（daily/monthly/realtime）
   * @param {number} options.lookbackDays    回溯天数（默认 7 天）
   * @param {object} options.icebergWriter   Iceberg 写入器实例
   */
  constructor(options = {}) {
    super();
    this.warehousePath = options.warehousePath || './data-lake';
    this.providers = options.providers || {};
    this.collectionMode = options.collectionMode || 'daily';
    this.lookbackDays = options.lookbackDays || 7;
    this.icebergWriter = options.icebergWriter;

    // 成本缓存
    this.costCache = new Map(); // date -> costRecords
    this._lastCollection = null;
    this._collecting = false;

    // 统计
    this.stats = {
      totalCollections: 0,
      totalRecordsCollected: 0,
      totalCostAmount: 0,
      lastCollectionTime: null,
      providersCollected: new Set(),
    };
  }

  /**
   * 采集指定日期范围的成本数据
   * @param {Date} [startDate] 开始日期（默认 lookbackDays 前）
   * @param {Date} [endDate]   结束日期（默认今天）
   */
  async collect(startDate, endDate) {
    if (this._collecting) {
      this.emit('collect:skip', { reason: 'already_collecting' });
      return null;
    }

    this._collecting = true;
    const collectionId = `cost-${crypto.randomBytes(6).toString('hex')}`;
    const startTime = Date.now();

    startDate = startDate || this._daysAgo(this.lookbackDays);
    endDate = endDate || new Date();

    this.emit('collect:start', { collectionId, startDate, endDate });

    try {
      const allRecords = [];

      // 采集各云服务商
      for (const [provider, config] of Object.entries(this.providers)) {
        try {
          const records = await this._collectFromProvider(provider, config, startDate, endDate);
          allRecords.push(...records);
          this.stats.providersCollected.add(provider);
        } catch (err) {
          this.emit('collect:provider_error', { provider, error: err.message });
        }
      }

      // 采集内部服务成本
      const internalRecords = await this._collectInternalCosts(startDate, endDate);
      allRecords.push(...internalRecords);

      // 采集 Kubernetes 资源成本
      const k8sRecords = await this._collectK8sCosts(startDate, endDate);
      allRecords.push(...k8sRecords);

      // 写入数据湖
      if (this.icebergWriter && allRecords.length > 0) {
        await this.icebergWriter.append('cost_records', allRecords);
      }

      // 缓存
      const dateKey = startDate.toISOString().slice(0, 10);
      this.costCache.set(dateKey, allRecords);

      // 更新统计
      this.stats.totalCollections++;
      this.stats.totalRecordsCollected += allRecords.length;
      this.stats.totalCostAmount += allRecords.reduce((s, r) => s + (r.cost_amount || 0), 0);
      this.stats.lastCollectionTime = new Date().toISOString();
      this._lastCollection = { collectionId, records: allRecords.length, date: new Date() };

      this.emit('collect:completed', {
        collectionId,
        records: allRecords.length,
        totalCost: this.stats.totalCostAmount,
        durationMs: Date.now() - startTime,
      });

      return { collectionId, records: allRecords, count: allRecords.length };

    } catch (err) {
      this.emit('collect:failed', { collectionId, error: err.message });
      throw err;
    } finally {
      this._collecting = false;
    }
  }

  async _collectFromProvider(provider, config, startDate, endDate) {
    // 各云服务商成本采集（生产环境调用对应 API）
    // AWS: CostExplorer / CUR S3 报告
    // 阿里云: DescribeBill / QueryAccountBalance
    // 腾讯云: DescribeBillDetail
    return [];
  }

  async _collectInternalCosts(startDate, endDate) {
    // 内部服务成本估算（基于用量 × 单价）
    return [];
  }

  async _collectK8sCosts(startDate, endDate) {
    // Kubernetes 资源成本（基于 requests/limits × 节点单价）
    return [];
  }

  /**
   * 获取成本汇总（按维度聚合）
   * @param {string} dimension 聚合维度（service/resource_type/region/tenant/category）
   * @param {Date} [startDate]
   * @param {Date} [endDate]
   */
  async getCostSummary(dimension = 'service', startDate, endDate) {
    startDate = startDate || this._daysAgo(30);
    endDate = endDate || new Date();

    // 从数据湖查询
    // const result = await this.icebergQuery.execute(
    //   `SELECT ${dimension}, SUM(cost_amount) as total_cost, COUNT(*) as records
    //    FROM cost_records WHERE record_date BETWEEN ? AND ? GROUP BY ${dimension} ORDER BY total_cost DESC`,
    //   { params: [startDate, endDate] }
    // );
    return { dimension, startDate, endDate, summary: [] };
  }

  /**
   * 获取成本趋势（按天）
   */
  async getCostTrend(startDate, endDate) {
    return this.getCostSummary('record_date', startDate, endDate);
  }

  /**
   * 获取 Top N 成本项
   */
  async getTopCosts(n = 10, dimension = 'service', startDate, endDate) {
    const summary = await this.getCostSummary(dimension, startDate, endDate);
    return summary.summary?.slice(0, n) || [];
  }

  /**
   * 计算成本预测（基于历史趋势）
   * @param {number} daysAhead 预测天数
   */
  async forecastCost(daysAhead = 30) {
    // 简单线性回归预测（生产环境使用 Prophet/ARIMA）
    const history = await this.getCostTrend(this._daysAgo(90), new Date());
    // ... 线性回归计算
    return {
      forecastDays: daysAhead,
      projectedCost: 0,
      confidence: 'medium',
      method: 'linear_regression',
    };
  }

  _daysAgo(days) {
    const d = new Date();
    d.setDate(d.getDate() - days);
    return d;
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      ...this.stats,
      providersCollected: Array.from(this.stats.providersCollected),
      cachedDates: Array.from(this.costCache.keys()),
      collecting: this._collecting,
      collectionMode: this.collectionMode,
    };
  }
}

module.exports = {
  CostCollector,
  CLOUD_PROVIDERS,
  COST_CATEGORIES,
};
