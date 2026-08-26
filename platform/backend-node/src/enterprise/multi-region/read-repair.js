'use strict';

/**
 * MOX Enterprise · 读修复（Read Repair）
 * ========================================
 * 在读取时检测跨 Region 数据不一致，并自动修复
 *
 * 工作原理：
 *  1. 读取时同时请求主 Region 和一个/多个副本 Region
 *  2. 对比各 Region 返回的数据（SHA-256 / 版本号 / 元数据）
 *  3. 如果发现不一致，触发异步修复（将正确版本同步到落后 Region）
 *  4. 返回最新/最一致的版本给用户
 *
 * 触发条件：
 *  - 读取时指定 readConsistency = 'quorum' 或 'all'
 *  - 后台定期巡检（anti-entropy）
 *  - CRR 同步失败后的补偿修复
 */

const crypto = require('crypto');
const { EventEmitter } = require('events');

// ─── 读一致性级别 ───
const READ_CONSISTENCY = {
  ONE: 'one',         // 只读一个 Region（最低延迟，可能读到旧数据）
  QUORUM: 'quorum',   // 读多数 Region（N/2+1），保证一致性
  ALL: 'all',         // 读所有 Region，最强一致性
};

// ─── 修复状态 ───
const REPAIR_STATUS = {
  DETECTED: 'detected',
  REPAIRING: 'repairing',
  REPAIRED: 'repaired',
  FAILED: 'failed',
  SKIPPED: 'skipped',
};

class ReadRepair extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.regionBackends  Region -> 存储后端 Map
   * @param {string} options.primaryRegion   主 Region
   * @param {string} options.defaultConsistency 默认读一致性级别
   * @param {number} options.repairBatchSize  修复批大小
   * @param {number} options.maxRepairRetries 最大修复重试次数
   * @param {boolean} options.backgroundScan   是否启用后台巡检
   * @param {number} options.scanIntervalMs    巡检间隔（默认 1 小时）
   */
  constructor(options = {}) {
    super();
    this.regionBackends = options.regionBackends || new Map();
    this.primaryRegion = options.primaryRegion || 'ap-southeast-1';
    this.defaultConsistency = options.defaultConsistency || READ_CONSISTENCY.QUORUM;
    this.repairBatchSize = options.repairBatchSize || 50;
    this.maxRepairRetries = options.maxRepairRetries || 3;
    this.backgroundScan = options.backgroundScan !== false;
    this.scanIntervalMs = options.scanIntervalMs || 3600000;

    // 修复队列
    this.repairQueue = [];
    this.repairHistory = new Map();
    this._repairing = false;

    // 统计
    this.stats = {
      totalReads: 0,
      totalInconsistencies: 0,
      totalRepairs: 0,
      totalRepairFailures: 0,
      totalBytesRepaired: 0,
      quorumReads: 0,
    };

    if (this.backgroundScan) {
      this._startBackgroundScan();
    }
    this._startRepairLoop();
  }

  /**
   * 注册 Region 存储后端
   */
  registerRegion(region, backend) {
    this.regionBackends.set(region, backend);
  }

  /**
   * 一致性读取
   * @param {string} sha256    chunk 哈希
   * @param {string} [consistency] 一致性级别
   * @returns {Promise<{data: Buffer, metadata: object, consistency: string, repaired: boolean}>}
   */
  async read(sha256, consistency = this.defaultConsistency) {
    this.stats.totalReads++;

    const regions = this._getReadRegions(consistency);
    if (regions.length === 0 || consistency === READ_CONSISTENCY.ONE) {
      // 单 Region 读
      const backend = this.regionBackends.get(this.primaryRegion);
      if (!backend) throw new Error(`主 Region ${this.primaryRegion} 未配置`);
      const data = await backend.readChunk(sha256);
      return { data, metadata: {}, consistency: READ_CONSISTENCY.ONE, repaired: false };
    }

    // 多 Region 并发读
    this.stats.quorumReads++;
    const readResults = await Promise.allSettled(
      regions.map(region => this._readFromRegion(region, sha256))
    );

    // 分析结果
    const successful = readResults
      .filter(r => r.status === 'fulfilled')
      .map(r => r.value);

    if (successful.length === 0) {
      throw new Error(`所有 Region 读取失败: ${sha256}`);
    }

    // 对比各 Region 返回的数据
    const { consistent, majorityData, inconsistentRegions, latestVersion } = this._compareResults(successful);

    if (!consistent && inconsistentRegions.length > 0) {
      // 检测到不一致，触发修复
      this.stats.totalInconsistencies++;
      this._enqueueRepair(sha256, majorityData, inconsistentRegions, latestVersion);
    }

    return {
      data: majorityData.data,
      metadata: majorityData.metadata || {},
      consistency,
      repaired: !consistent,
      regionsRead: regions.length,
      regionsConsistent: successful.length - inconsistentRegions.length,
    };
  }

  async _readFromRegion(region, sha256) {
    const backend = this.regionBackends.get(region);
    if (!backend) throw new Error(`Region ${region} 未配置`);

    const startTime = Date.now();
    const data = await backend.readChunk(sha256);
    const hash = crypto.createHash('sha256').update(data).digest('hex');

    return {
      region,
      data,
      hash,
      size: data.length,
      latencyMs: Date.now() - startTime,
      readAt: new Date().toISOString(),
    };
  }

  _getReadRegions(consistency) {
    const allRegions = Array.from(this.regionBackends.keys());

    if (consistency === READ_CONSISTENCY.ALL) {
      return allRegions;
    }
    if (consistency === READ_CONSISTENCY.QUORUM) {
      // 读多数：N/2+1 个 Region，优先主 Region + 最近的副本
      const quorumCount = Math.floor(allRegions.length / 2) + 1;
      const sorted = allRegions.sort((a, b) => {
        if (a === this.primaryRegion) return -1;
        if (b === this.primaryRegion) return 1;
        return 0;
      });
      return sorted.slice(0, quorumCount);
    }
    return [this.primaryRegion];
  }

  _compareResults(results) {
    // 按数据哈希分组
    const byHash = new Map();
    for (const r of results) {
      if (!byHash.has(r.hash)) byHash.set(r.hash, []);
      byHash.get(r.hash).push(r);
    }

    // 找多数派
    let majority = null;
    let maxCount = 0;
    for (const [hash, group] of byHash) {
      if (group.length > maxCount) {
        maxCount = group.length;
        majority = group[0];
      }
    }

    // 找不一致的 Region
    const inconsistentRegions = [];
    for (const [hash, group] of byHash) {
      if (hash !== majority.hash) {
        group.forEach(r => inconsistentRegions.push(r.region));
      }
    }

    return {
      consistent: inconsistentRegions.length === 0,
      majorityData: majority,
      inconsistentRegions,
      latestVersion: majority,
    };
  }

  /**
   * 入队修复任务
   */
  _enqueueRepair(sha256, correctData, targetRegions, versionInfo) {
    const repairId = `repair-${crypto.randomBytes(8).toString('hex')}`;
    const task = {
      repairId,
      sha256,
      correctData: correctData.data,
      correctHash: correctData.hash,
      targetRegions,
      sourceRegion: correctData.region,
      versionInfo,
      retries: 0,
      status: REPAIR_STATUS.DETECTED,
      createdAt: new Date(),
    };

    this.repairQueue.push(task);
    this.repairHistory.set(repairId, task);
    this.emit('repair:enqueued', { repairId, sha256, targetRegions });
  }

  async _processRepairQueue() {
    if (this._repairing || this.repairQueue.length === 0) return;
    this._repairing = true;

    try {
      while (this.repairQueue.length > 0) {
        const batch = this.repairQueue.splice(0, this.repairBatchSize);
        await this._processRepairBatch(batch);
      }
    } finally {
      this._repairing = false;
    }
  }

  async _processRepairBatch(batch) {
    const promises = batch.map(task => this._repairOne(task));
    await Promise.allSettled(promises);
  }

  async _repairOne(task) {
    task.status = REPAIR_STATUS.REPAIRING;
    task.retries++;

    try {
      for (const region of task.targetRegions) {
        const backend = this.regionBackends.get(region);
        if (!backend) {
          this.emit('repair:skip', { repairId: task.repairId, region, reason: 'backend_not_configured' });
          continue;
        }

        // 写入正确数据
        await backend.writeChunk(task.sha256, task.correctData);

        // 验证修复结果
        const verifyData = await backend.readChunk(task.sha256);
        const verifyHash = crypto.createHash('sha256').update(verifyData).digest('hex');

        if (verifyHash !== task.correctHash) {
          throw new Error(`修复验证失败: region=${region}, expected=${task.correctHash}, actual=${verifyHash}`);
        }
      }

      task.status = REPAIR_STATUS.REPAIRED;
      task.completedAt = new Date();
      this.stats.totalRepairs++;
      this.stats.totalBytesRepaired += task.correctData.length;
      this.emit('repair:completed', { repairId: task.repairId, sha256: task.sha256 });

    } catch (err) {
      if (task.retries < this.maxRepairRetries) {
        // 重新入队
        task.status = REPAIR_STATUS.DETECTED;
        this.repairQueue.unshift(task);
        this.emit('repair:retry', { repairId: task.repairId, attempt: task.retries, error: err.message });
      } else {
        task.status = REPAIR_STATUS.FAILED;
        task.error = err.message;
        this.stats.totalRepairFailures++;
        this.emit('repair:failed', { repairId: task.repairId, error: err.message });
      }
    }
  }

  _startRepairLoop() {
    setInterval(() => this._processRepairQueue(), 5000);
  }

  /**
   * 后台巡检：定期扫描随机样本，检测静默不一致
   */
  async _startBackgroundScan() {
    setInterval(async () => {
      try {
        await this._antiEntropyScan();
      } catch (err) {
        this.emit('scan:error', { error: err.message });
      }
    }, this.scanIntervalMs);
  }

  async _antiEntropyScan() {
    // 从元数据存储获取随机样本（生产环境实现）
    // const sampleKeys = await this.metadataStore.getRandomSample(1000);
    // for (const key of sampleKeys) {
    //   await this.read(key, READ_CONSISTENCY.ALL);
    // }
    this.emit('scan:completed', { scanned: 0, timestamp: new Date() });
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      ...this.stats,
      repairQueueLength: this.repairQueue.length,
      repairing: this._repairing,
      configuredRegions: Array.from(this.regionBackends.keys()),
      primaryRegion: this.primaryRegion,
      defaultConsistency: this.defaultConsistency,
    };
  }

  /**
   * 获取修复历史
   */
  getRepairHistory(limit = 100) {
    return Array.from(this.repairHistory.values())
      .sort((a, b) => new Date(b.createdAt) - new Date(a.createdAt))
      .slice(0, limit);
  }

  async close() {
    // 等待修复队列排空
    while (this.repairQueue.length > 0) {
      await new Promise(r => setTimeout(r, 1000));
    }
    this.removeAllListeners();
  }
}

module.exports = {
  ReadRepair,
  READ_CONSISTENCY,
  REPAIR_STATUS,
};
