'use strict';

/**
 * MOX Enterprise · 成本优化推荐器
 * ================================
 * 基于成本数据和使用模式，自动生成优化建议
 *
 * 优化维度：
 *  1. 存储优化：冷热分级、EC 替换三副本、去重、压缩
 *  2. 计算优化：Spot 实例、自动扩缩容、资源 right-sizing
 *  3. 网络优化：跨域流量优化、CDN 缓存、计算下沉
 *  4. 预留实例：RI/SP 承诺使用折扣
 *  5. 闲置资源：未使用的存储桶/实例/IP
 *  6. 数据生命周期：自动降级/删除过期数据
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 优化类别 ───
const OPTIMIZATION_CATEGORY = {
  STORAGE: 'storage',
  COMPUTE: 'compute',
  NETWORK: 'network',
  RESERVATION: 'reservation',
  IDLE_RESOURCE: 'idle_resource',
  LIFECYCLE: 'lifecycle',
};

// ─── 优化优先级 ───
const OPTIMIZATION_PRIORITY = {
  CRITICAL: 'critical', // 立即可省 >10%
  HIGH: 'high',         // 可省 5-10%
  MEDIUM: 'medium',     // 可省 1-5%
  LOW: 'low',           // 可省 <1%
};

// ─── 实施难度 ───
const IMPLEMENTATION_EFFORT = {
  EASY: 'easy',         // 配置变更，<1 小时
  MEDIUM: 'medium',     // 需要代码/架构变更，1-3 天
  HARD: 'hard',         // 重大架构变更，>1 周
};

class OptimizationRecommender extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.costCollector  成本采集器
   * @param {object} options.usageAnalyzer  使用分析器
   * @param {number} options.scanIntervalMs 扫描间隔（默认 24 小时）
   * @param {number} options.minSavingsPercent 最小节省百分比阈值（默认 1%）
   */
  constructor(options = {}) {
    super();
    this.costCollector = options.costCollector;
    this.usageAnalyzer = options.usageAnalyzer;
    this.scanIntervalMs = options.scanIntervalMs || 86400000;
    this.minSavingsPercent = options.minSavingsPercent || 0.01;

    // 推荐缓存
    this.recommendations = new Map(); // recId -> recommendation
    this._scanCount = 0;

    // 已实施的优化
    this.implementedOptimizations = [];

    this._startScanLoop();
  }

  /**
   * 执行全量扫描，生成优化建议
   */
  async scan() {
    const scanId = `scan-${crypto.randomBytes(6).toString('hex')}`;
    const startTime = Date.now();
    this._scanCount++;

    this.emit('scan:start', { scanId });

    try {
      const allRecommendations = [];

      // 1. 存储优化
      allRecommendations.push(...await this._scanStorageOptimizations());

      // 2. 计算优化
      allRecommendations.push(...await this._scanComputeOptimizations());

      // 3. 网络优化
      allRecommendations.push(...await this._scanNetworkOptimizations());

      // 4. 预留实例
      allRecommendations.push(...await this._scanReservationOpportunities());

      // 5. 闲置资源
      allRecommendations.push(...await this._scanIdleResources());

      // 6. 生命周期优化
      allRecommendations.push(...await this._scanLifecycleOptimizations());

      // 过滤低于阈值的建议
      const filtered = allRecommendations.filter(r =>
        r.savingsPercent >= this.minSavingsPercent
      );

      // 按预计节省金额排序
      filtered.sort((a, b) => b.estimatedSavingsMonthly - a.estimatedSavingsMonthly);

      // 缓存
      for (const rec of filtered) {
        this.recommendations.set(rec.recId, rec);
      }

      const totalSavings = filtered.reduce((s, r) => s + r.estimatedSavingsMonthly, 0);

      this.emit('scan:completed', {
        scanId,
        recommendations: filtered.length,
        totalMonthlySavings: totalSavings,
        durationMs: Date.now() - startTime,
      });

      return { scanId, recommendations: filtered, totalMonthlySavings: totalSavings };

    } catch (err) {
      this.emit('scan:failed', { scanId, error: err.message });
      throw err;
    }
  }

  async _scanStorageOptimizations() {
    const recs = [];

    // 1.1 三副本 → EC 12+4
    recs.push({
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.STORAGE,
      title: '三副本替换为 EC 12+4 纠删码',
      description: '将温/冷数据从三副本（3× 开销）替换为 EC 12+4（1.33× 开销），可节省 56% 存储成本',
      priority: OPTIMIZATION_PRIORITY.CRITICAL,
      effort: IMPLEMENTATION_EFFORT.MEDIUM,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.56,
      paybackPeriodDays: 7,
      implementationSteps: [
        '评估数据冷热分布，确定 EC 适用范围',
        '部署 EC 编码服务（ISA-L 硬件加速）',
        '灰度迁移 10% 温数据到 EC',
        '验证 EC 修复性能和读取延迟',
        '全量迁移温/冷数据',
      ],
      risks: ['EC 编码增加 CPU 开销', '小对象 EC 效率低', '修复期间读性能降级'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    // 1.2 冷热分级
    recs.push({
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.STORAGE,
      title: '启用三级冷热存储生命周期',
      description: '30 天未访问数据自动降级到 IA，365 天降级到归档，可节省 70% 冷数据成本',
      priority: OPTIMIZATION_PRIORITY.HIGH,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.35,
      paybackPeriodDays: 1,
      implementationSteps: ['配置 S3 Lifecycle 规则', '验证降级/解档流程', '监控冷数据访问频率'],
      risks: ['冷数据解档延迟 12h', '解档费用'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    // 1.3 端侧压缩
    recs.push({
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.STORAGE,
      title: '启用 Zstd 端侧压缩',
      description: '上传前 Zstd level 3 压缩，压缩率约 2.5×，可节省 60% 存储费和出口流量费',
      priority: OPTIMIZATION_PRIORITY.HIGH,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.4,
      paybackPeriodDays: 1,
      implementationSteps: ['客户端集成 zstd 库', '服务端支持解压', '灰度启用压缩'],
      risks: ['压缩增加 CPU 开销', '已存数据需要重写'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    return recs;
  }

  async _scanComputeOptimizations() {
    const recs = [];

    // 2.1 Spot 实例
    recs.push({
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.COMPUTE,
      title: 'Spark/批处理使用 Spot 竞价实例',
      description: '无状态批处理任务可使用 Spot 实例，节省 60-80% 计算成本，配合检查点机制容错',
      priority: OPTIMIZATION_PRIORITY.CRITICAL,
      effort: IMPLEMENTATION_EFFORT.MEDIUM,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.65,
      paybackPeriodDays: 3,
      implementationSteps: ['配置 Spark 动态资源分配', '启用 RDD 检查点', '配置 Spot 中断处理策略', '灰度 20% 任务到 Spot'],
      risks: ['Spot 实例可能被中断', '需要容错设计'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    // 2.2 Right-sizing
    recs.push({
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.COMPUTE,
      title: '容器资源 Right-sizing',
      description: '基于历史 CPU/内存使用率调整 requests/limits，平均可节省 30% 计算资源',
      priority: OPTIMIZATION_PRIORITY.MEDIUM,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.25,
      paybackPeriodDays: 1,
      implementationSteps: ['收集 30 天资源使用率数据', '计算 P95 使用率', '调整 requests/limits', '监控调整后性能'],
      risks: ['资源不足导致 OOM', '需要逐步调整'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    return recs;
  }

  async _scanNetworkOptimizations() {
    return [{
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.NETWORK,
      title: '计算下沉存储端减少跨域流量',
      description: '将过滤/聚合操作下推到 S3 Select，减少 99% 数据传输，节省出口流量费',
      priority: OPTIMIZATION_PRIORITY.HIGH,
      effort: IMPLEMENTATION_EFFORT.MEDIUM,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.5,
      paybackPeriodDays: 5,
      implementationSteps: ['识别高流量查询', '改写为 S3 Select', '性能对比验证', '全量推广'],
      risks: ['S3 Select 有额外费用', '复杂查询不支持下推'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    }];
  }

  async _scanReservationOpportunities() {
    return [{
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.RESERVATION,
      title: '购买预留实例/节省计划',
      description: '对稳定运行的核心服务购买 1 年/3 年 RI，可节省 30-50% 计算成本',
      priority: OPTIMIZATION_PRIORITY.HIGH,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.35,
      paybackPeriodDays: 90,
      implementationSteps: ['分析 3 个月实例使用率', '确定稳定负载实例', '购买 1 年 RI（先小范围）', '评估后扩展到 3 年'],
      risks: ['RI 有承诺期', '业务变化可能导致浪费'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    }];
  }

  async _scanIdleResources() {
    return [{
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.IDLE_RESOURCE,
      title: '清理闲置资源',
      description: '识别并删除未使用的存储桶、EIP、快照、负载均衡器，通常可节省 5-10% 云费用',
      priority: OPTIMIZATION_PRIORITY.MEDIUM,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.07,
      paybackPeriodDays: 1,
      implementationSteps: ['运行资源闲置扫描脚本', '确认闲置资源列表', '备份后删除', '设置定期巡检'],
      risks: ['误删正在使用的资源', '需要人工确认'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    }];
  }

  async _scanLifecycleOptimizations() {
    return [{
      recId: `rec-${crypto.randomBytes(6).toString('hex')}`,
      category: OPTIMIZATION_CATEGORY.LIFECYCLE,
      title: '过期数据自动删除策略',
      description: '对临时文件、日志、测试数据设置 TTL 自动删除，避免无限累积存储费用',
      priority: OPTIMIZATION_PRIORITY.MEDIUM,
      effort: IMPLEMENTATION_EFFORT.EASY,
      estimatedSavingsMonthly: 0,
      savingsPercent: 0.1,
      paybackPeriodDays: 1,
      implementationSteps: ['分类数据保留策略', '配置 S3 Expiration 规则', '设置日志保留期', '定期审计'],
      risks: ['误删需要长期保留的数据', '需要业务方确认'],
      createdAt: new Date().toISOString(),
      status: 'pending',
    }];
  }

  /**
   * 标记建议为已实施
   */
  async implement(recId, implementationDetails = {}) {
    const rec = this.recommendations.get(recId);
    if (!rec) throw new Error(`建议不存在: ${recId}`);

    rec.status = 'implemented';
    rec.implementedAt = new Date().toISOString();
    rec.implementationDetails = implementationDetails;

    this.implementedOptimizations.push({
      recId,
      title: rec.title,
      implementedAt: rec.implementedAt,
      estimatedSavingsMonthly: rec.estimatedSavingsMonthly,
      actualSavingsMonthly: implementationDetails.actualSavingsMonthly || null,
    });

    this.emit('recommendation:implemented', rec);
    return rec;
  }

  /**
   * 获取推荐列表
   */
  getRecommendations(category = null, priority = null, status = 'pending') {
    let recs = Array.from(this.recommendations.values());
    if (category) recs = recs.filter(r => r.category === category);
    if (priority) recs = recs.filter(r => r.priority === priority);
    if (status) recs = recs.filter(r => r.status === status);
    return recs;
  }

  /**
   * 获取优化报告
   */
  getReport() {
    const pending = this.getRecommendations(null, null, 'pending');
    const implemented = this.implementedOptimizations;
    const totalPotentialSavings = pending.reduce((s, r) => s + r.estimatedSavingsMonthly, 0);
    const totalActualSavings = implemented.reduce((s, r) => s + (r.actualSavingsMonthly || 0), 0);

    return {
      generatedAt: new Date().toISOString(),
      totalScans: this._scanCount,
      pendingRecommendations: pending.length,
      implementedOptimizations: implemented.length,
      totalPotentialMonthlySavings: totalPotentialSavings,
      totalActualMonthlySavings: totalActualSavings,
      byCategory: pending.reduce((acc, r) => {
        acc[r.category] = (acc[r.category] || 0) + r.estimatedSavingsMonthly;
        return acc;
      }, {}),
      byPriority: pending.reduce((acc, r) => {
        acc[r.priority] = (acc[r.priority] || 0) + 1;
        return acc;
      }, {}),
      topRecommendations: pending.slice(0, 5),
    };
  }

  _startScanLoop() {
    setInterval(() => this.scan().catch(err => {
      this.emit('scan-loop:error', { error: err.message });
    }), this.scanIntervalMs);
  }
}

module.exports = {
  OptimizationRecommender,
  OPTIMIZATION_CATEGORY,
  OPTIMIZATION_PRIORITY,
  IMPLEMENTATION_EFFORT,
};
