'use strict';

/**
 * 专家联盟域门面（AINA-STD-001 域包入口 · 唯一对外契约）
 * ------------------------------------------------------------------
 * 组装结构：
 *   domain/         纯算法内核（意图/匹配/辩论综合，零 IO）
 *   application/    编排器（ExpertAlliance 类：用例编排 + 引擎协作）
 *   infrastructure/ 仓储（专家/指标/会话链，唯一 IO 边界）
 *
 * 对外契约（历史消费方零改动）：
 *   require('./expert-alliance') → { ExpertAlliance, getAlliance, INTENT_PATTERNS }
 *   注意：平级不存在同名 .js 文件，Node 将目录 index.js 作为解析目标（无自引用循环）。
 */

// domain 层（纯算法，零 IO）
const { INTENT_PATTERNS } = require('./domain/intent-patterns');
const { detectIntent } = require('./domain/intent-classifier');
const { matchExperts, scoreExperts } = require('./domain/expert-matcher');
const {
  keywordsOf, extractConsensus, extractDivergences,
  generateFinalRecommendation, synthesizeDebate
} = require('./domain/debate-synthesis');

// application 层（编排器）
const { ExpertAlliance } = require('./application/alliance-orchestrator');

// infrastructure 层（IO 适配）
const { ExpertRepository } = require('./infrastructure/expert-repository');
const { MetricsStore } = require('./infrastructure/metrics-store');
const { SessionChainStore } = require('./infrastructure/session-chain-store');

let allianceInstance = null;

function getAlliance() {
  if (!allianceInstance) {
    allianceInstance = new ExpertAlliance();
    allianceInstance._initTime = new Date().toISOString();
  }
  return allianceInstance;
}

module.exports = {
  // 对外稳定契约（历史消费方依赖）
  ExpertAlliance,
  getAlliance,
  INTENT_PATTERNS,
  // domain（供六阶段流水线 engine 等跨模块复用）
  detectIntent,
  matchExperts,
  scoreExperts,
  keywordsOf,
  extractConsensus,
  extractDivergences,
  generateFinalRecommendation,
  synthesizeDebate,
  // infrastructure（供测试与新用例装配）
  ExpertRepository,
  MetricsStore,
  SessionChainStore
};
