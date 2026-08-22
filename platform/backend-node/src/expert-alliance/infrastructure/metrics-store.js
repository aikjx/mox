'use strict';

/**
 * 专家指标仓储（infrastructure 层）
 * ------------------------------------------------------------------
 * 职责：专家咨询指标（次数/成功率/时长/置信度窗口）的内存态与持久化。
 * 每 EXPERT_METRICS_INTERVAL 次咨询落盘一次（含专家表，同步 metrics 冗余字段）。
 */
const { readJSON, writeJSON } = require('../../lib/json-store');

const EXPERT_METRICS_INTERVAL = 100;

class MetricsStore {
  constructor() {
    this.stats = new Map();
    const persisted = readJSON('expert_stats.json', {});
    if (persisted) {
      Object.entries(persisted).forEach(([k, v]) => this.stats.set(k, v));
    }
  }

  /**
   * 记录一次咨询并回写专家冗余指标字段。
   * @param {Object} expert 目标专家（引用回写 metrics 字段）
   * @param {number} duration 耗时 ms
   * @param {boolean} success 是否成功
   * @param {number} [confidence] 置信度
   * @returns {boolean} 是否触发落盘周期
   */
  record(expert, duration, success, confidence) {
    const expertId = expert.id;
    const current = this.stats.get(expertId) || {
      total_consults: 0,
      successful_consults: 0,
      total_duration: 0,
      confidences: []
    };

    current.total_consults += 1;
    if (success) current.successful_consults += 1;
    current.total_duration += duration;
    if (confidence !== undefined) {
      current.confidences.push(confidence);
      if (current.confidences.length > 100) {
        current.confidences = current.confidences.slice(-50);
      }
    }

    expert.metrics = {
      total_consults: current.total_consults,
      avg_confidence: current.confidences.length > 0
        ? current.confidences.reduce((a, b) => a + b, 0) / current.confidences.length
        : 0.7,
      success_rate: current.total_consults > 0
        ? current.successful_consults / current.total_consults
        : 1.0,
      avg_duration: current.total_duration / current.total_consults
    };

    this.stats.set(expertId, current);

    if (current.total_consults % EXPERT_METRICS_INTERVAL === 0) {
      this.persist();
      return true;
    }
    return false;
  }

  persist() {
    const out = {};
    this.stats.forEach((v, k) => out[k] = v);
    writeJSON('expert_stats.json', out);
  }

  of(expertId) {
    return this.stats.get(expertId) || {};
  }
}

module.exports = { MetricsStore, EXPERT_METRICS_INTERVAL };
