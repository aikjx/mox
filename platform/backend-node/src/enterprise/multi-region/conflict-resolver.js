'use strict';

/**
 * MOX Enterprise · 多 Region 冲突解决器
 * ========================================
 * 处理跨 Region 并发写入导致的数据冲突
 *
 * 冲突类型：
 *  1. 内容冲突：同一 sha256 不同内容（理论上不可能，SHA-256 碰撞概率极低）
 *  2. 元数据冲突：同一对象不同元数据（文件名、标签、生命周期等）
 *  3. 版本冲突：同一对象不同版本号（并发更新）
 *  4. 删除冲突：一端删除一端更新
 *
 * 解决策略：
 *  - LWW（Last-Writer-Wins）：按时间戳最新者胜
 *  - 向量时钟（Vector Clock）：精确检测并发
 *  - 内容寻址兜底：SHA-256 相同则内容必然相同，无冲突
 *  - 人工介入：无法自动解决的冲突进入审核队列
 */

const crypto = require('crypto');
const { EventEmitter } = require('events');

// ─── 冲突类型 ───
const CONFLICT_TYPE = {
  METADATA: 'metadata',       // 元数据冲突
  VERSION: 'version',         // 版本冲突
  DELETE_UPDATE: 'delete_update', // 删除/更新冲突
  CONTENT: 'content',         // 内容冲突（极罕见）
  UNKNOWN: 'unknown',
};

// ─── 解决策略 ───
const RESOLUTION_STRATEGY = {
  LWW: 'lww',                   // Last-Writer-Wins
  VECTOR_CLOCK: 'vector_clock', // 向量时钟
  SOURCE_PRIORITY: 'source_priority', // 源 Region 优先级
  MANUAL: 'manual',             // 人工介入
};

// ─── 冲突状态 ───
const CONFLICT_STATUS = {
  DETECTED: 'detected',
  RESOLVING: 'resolving',
  RESOLVED: 'resolved',
  NEEDS_MANUAL: 'needs_manual',
  ESCALATED: 'escalated',
};

class ConflictResolver extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.defaultStrategy 默认解决策略
   * @param {string[]} options.regionPriority Region 优先级（高优先级在前）
   * @param {object} options.metadataStore   元数据存储
   * @param {number} options.manualThreshold 人工介入阈值（重试次数）
   */
  constructor(options = {}) {
    super();
    this.defaultStrategy = options.defaultStrategy || RESOLUTION_STRATEGY.LWW;
    this.regionPriority = options.regionPriority || [];
    this.metadataStore = options.metadataStore;
    this.manualThreshold = options.manualThreshold || 3;

    // 冲突队列
    this.conflictQueue = [];
    this.conflictHistory = new Map(); // conflictId -> conflict record
    this._processing = false;
  }

  /**
   * 检测冲突
   * @param {object} localEntry   本地写入记录
   * @param {object} remoteEntry  远程已有记录
   * @returns {object|null} 冲突信息（无冲突返回 null）
   */
  detectConflict(localEntry, remoteEntry) {
    // 内容寻址：SHA-256 相同则内容相同，无内容冲突
    if (localEntry.sha256 === remoteEntry.sha256) {
      // 检查元数据冲突
      return this._detectMetadataConflict(localEntry, remoteEntry);
    }

    // 不同 sha256 = 内容冲突（极罕见，可能是哈希碰撞或数据损坏）
    return {
      conflictId: this._generateId(),
      type: CONFLICT_TYPE.CONTENT,
      severity: 'critical',
      local: localEntry,
      remote: remoteEntry,
      detectedAt: new Date(),
      status: CONFLICT_STATUS.DETECTED,
    };
  }

  _detectMetadataConflict(localEntry, remoteEntry) {
    const localMeta = localEntry.metadata || {};
    const remoteMeta = remoteEntry.metadata || {};

    // 比较关键字段
    const conflictFields = [];
    const keyFields = ['filename', 'contentType', 'tier', 'lifecycle', 'tags', 'tenantId'];

    for (const field of keyFields) {
      const localVal = JSON.stringify(localMeta[field]);
      const remoteVal = JSON.stringify(remoteMeta[field]);
      if (localVal !== remoteVal && localVal !== undefined && remoteVal !== undefined) {
        conflictFields.push(field);
      }
    }

    if (conflictFields.length === 0) return null;

    // 检查是否为删除/更新冲突
    if (localEntry.opType === 'delete' && remoteEntry.opType === 'put') {
      return {
        conflictId: this._generateId(),
        type: CONFLICT_TYPE.DELETE_UPDATE,
        severity: 'high',
        conflictFields,
        local: localEntry,
        remote: remoteEntry,
        detectedAt: new Date(),
        status: CONFLICT_STATUS.DETECTED,
      };
    }

    // 版本冲突
    if (localEntry.version && remoteEntry.version && localEntry.version !== remoteEntry.version) {
      return {
        conflictId: this._generateId(),
        type: CONFLICT_TYPE.VERSION,
        severity: 'medium',
        conflictFields,
        local: localEntry,
        remote: remoteEntry,
        detectedAt: new Date(),
        status: CONFLICT_STATUS.DETECTED,
      };
    }

    // 普通元数据冲突
    return {
      conflictId: this._generateId(),
      type: CONFLICT_TYPE.METADATA,
      severity: 'low',
      conflictFields,
      local: localEntry,
      remote: remoteEntry,
      detectedAt: new Date(),
      status: CONFLICT_STATUS.DETECTED,
    };
  }

  /**
   * 解决冲突
   * @param {object} conflict 冲突信息
   * @param {string} [strategy] 解决策略（默认使用配置的默认策略）
   * @returns {object} 解决结果
   */
  async resolve(conflict, strategy = this.defaultStrategy) {
    conflict.status = CONFLICT_STATUS.RESOLVING;
    conflict.resolutionStrategy = strategy;
    conflict.resolutionStart = new Date();

    this.emit('conflict:resolving', { conflictId: conflict.conflictId, strategy });

    try {
      let winner;
      let resolution;

      switch (strategy) {
        case RESOLUTION_STRATEGY.LWW:
          resolution = this._resolveLWW(conflict);
          break;
        case RESOLUTION_STRATEGY.VECTOR_CLOCK:
          resolution = this._resolveVectorClock(conflict);
          break;
        case RESOLUTION_STRATEGY.SOURCE_PRIORITY:
          resolution = this._resolveSourcePriority(conflict);
          break;
        case RESOLUTION_STRATEGY.MANUAL:
        default:
          resolution = { needsManual: true, reason: '策略要求人工介入' };
      }

      if (resolution.needsManual) {
        conflict.status = CONFLICT_STATUS.NEEDS_MANUAL;
        conflict.manualReason = resolution.reason;
        this.conflictQueue.push(conflict);
        this.emit('conflict:needs_manual', conflict);
        return conflict;
      }

      winner = resolution.winner;
      conflict.winner = winner;
      conflict.winnerRegion = winner.sourceRegion;
      conflict.resolvedAt = new Date();
      conflict.status = CONFLICT_STATUS.RESOLVED;
      conflict.resolutionDetail = resolution.detail;

      // 应用解决结果（将胜出版本同步到所有 Region）
      await this._applyResolution(conflict, winner);

      this.conflictHistory.set(conflict.conflictId, conflict);
      this.emit('conflict:resolved', conflict);
      return conflict;

    } catch (err) {
      conflict.status = CONFLICT_STATUS.ESCALATED;
      conflict.error = err.message;
      this.emit('conflict:error', { conflictId: conflict.conflictId, error: err.message });
      throw err;
    }
  }

  /**
   * LWW：Last-Writer-Wins，按时间戳最新者胜
   */
  _resolveLWW(conflict) {
    const localTime = new Date(conflict.local.timestamp || conflict.local.createdAt).getTime();
    const remoteTime = new Date(conflict.remote.timestamp || conflict.remote.createdAt).getTime();

    const winner = localTime >= remoteTime ? conflict.local : conflict.remote;
    const loser = localTime >= remoteTime ? conflict.remote : conflict.local;

    return {
      winner,
      loser,
      detail: {
        strategy: 'LWW',
        localTimestamp: conflict.local.timestamp,
        remoteTimestamp: conflict.remote.timestamp,
        winnerTimestamp: winner.timestamp,
        timeDiffMs: Math.abs(localTime - remoteTime),
      },
    };
  }

  /**
   * 向量时钟：精确检测并发关系
   * 如果有因果关系则取较新者，如果是真并发则需要人工介入
   */
  _resolveVectorClock(conflict) {
    const localVC = conflict.local.vectorClock || {};
    const remoteVC = conflict.remote.vectorClock || {};

    // 判断向量时钟关系
    const relation = this._compareVectorClocks(localVC, remoteVC);

    if (relation === 'greater') {
      return { winner: conflict.local, detail: { strategy: 'vector_clock', relation: 'local_greater' } };
    }
    if (relation === 'less') {
      return { winner: conflict.remote, detail: { strategy: 'vector_clock', relation: 'remote_greater' } };
    }
    // concurrent（真并发）或 equal → 人工介入
    return {
      needsManual: true,
      reason: `向量时钟检测到真并发（concurrent），无法自动解决`,
    };
  }

  _compareVectorClocks(vc1, vc2) {
    const allKeys = new Set([...Object.keys(vc1), ...Object.keys(vc2)]);
    let hasGreater = false;
    let hasLess = false;

    for (const key of allKeys) {
      const v1 = vc1[key] || 0;
      const v2 = vc2[key] || 0;
      if (v1 > v2) hasGreater = true;
      if (v1 < v2) hasLess = true;
    }

    if (hasGreater && !hasLess) return 'greater';
    if (!hasGreater && hasLess) return 'less';
    if (!hasGreater && !hasLess) return 'equal';
    return 'concurrent';
  }

  /**
   * 源 Region 优先级：高优先级 Region 的写入胜出
   */
  _resolveSourcePriority(conflict) {
    const localIdx = this.regionPriority.indexOf(conflict.local.sourceRegion);
    const remoteIdx = this.regionPriority.indexOf(conflict.remote.sourceRegion);

    // 不在优先级列表中的 Region 排最后
    const localRank = localIdx === -1 ? Infinity : localIdx;
    const remoteRank = remoteIdx === -1 ? Infinity : remoteIdx;

    const winner = localRank <= remoteRank ? conflict.local : conflict.remote;
    return {
      winner,
      detail: {
        strategy: 'source_priority',
        localRegion: conflict.local.sourceRegion,
        remoteRegion: conflict.remote.sourceRegion,
        winnerRegion: winner.sourceRegion,
      },
    };
  }

  async _applyResolution(conflict, winner) {
    // 将胜出版本写入元数据存储
    if (this.metadataStore) {
      await this.metadataStore.resolveConflict(conflict.conflictId, winner);
    }
  }

  /**
   * 处理人工审核队列
   */
  async processManualQueue(handler) {
    const resolved = [];
    for (const conflict of this.conflictQueue) {
      if (conflict.status !== CONFLICT_STATUS.NEEDS_MANUAL) continue;
      const decision = await handler(conflict);
      if (decision) {
        conflict.status = CONFLICT_STATUS.RESOLVED;
        conflict.winner = decision.winner;
        conflict.resolvedAt = new Date();
        conflict.resolutionDetail = { strategy: 'manual', reviewer: decision.reviewer, comment: decision.comment };
        resolved.push(conflict);
      }
    }
    this.conflictQueue = this.conflictQueue.filter(c => c.status !== CONFLICT_STATUS.RESOLVED);
    return resolved;
  }

  /**
   * 获取统计
   */
  getStats() {
    const history = Array.from(this.conflictHistory.values());
    return {
      totalDetected: history.length + this.conflictQueue.length,
      totalResolved: history.filter(c => c.status === CONFLICT_STATUS.RESOLVED).length,
      pendingManual: this.conflictQueue.filter(c => c.status === CONFLICT_STATUS.NEEDS_MANUAL).length,
      escalated: history.filter(c => c.status === CONFLICT_STATUS.ESCALATED).length,
      byType: this._countBy(history, 'type'),
      byStrategy: this._countBy(history, 'resolutionStrategy'),
    };
  }

  _countBy(arr, field) {
    return arr.reduce((acc, item) => {
      const key = item[field] || 'unknown';
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
  }

  _generateId() {
    return `conflict-${crypto.randomBytes(8).toString('hex')}`;
  }
}

module.exports = {
  ConflictResolver,
  CONFLICT_TYPE,
  RESOLUTION_STRATEGY,
  CONFLICT_STATUS,
};
