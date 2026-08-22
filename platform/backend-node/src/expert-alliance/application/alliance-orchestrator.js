'use strict';

/**
 * 企业级「专家联盟」编排器（Application Layer · 组合根）
 * ------------------------------------------------------------------
 * 职责：用例编排（咨询/辩论/路由）+ 引擎协作 + mixin 用例族装配。
 * 架构（AINA-STD-001 域包模式）：
 *   domain/         纯算法内核（意图/匹配/辩论综合，零 IO）
 *   infrastructure/ 仓储（专家/指标/会话链，唯一 IO 边界）
 *   本文件          application 编排：组合 domain 算法 + infra 仓储 + 外部引擎
 *   *-service.js    同层 mixin 用例族（算法分析 / 会话链 / V2 编排代理）
 *
 * 对外契约不变式：class ExpertAlliance 全部公开方法签名保持不变（消费方零改动）。
 * 历史收口：A16 意图模式单一真相源、A18 PageRank 单源委托、
 *           A20 辩论共识真实提取 —— 算法实现均已在 domain 层固化。
 */

const { getGateway } = require('../../llm-gateway');
const { getOrchestrationEngine } = require('../../orchestration-engine');

const { detectIntent } = require('../domain/intent-classifier');
const { matchExperts, scoreExperts } = require('../domain/expert-matcher');
const {
  synthesizeDebate,
  extractConsensus,
  extractDivergences,
  generateFinalRecommendation
} = require('../domain/debate-synthesis');
const { ExpertRepository } = require('../infrastructure/expert-repository');
const { MetricsStore } = require('../infrastructure/metrics-store');
const { SessionChainStore } = require('../infrastructure/session-chain-store');

// Application 层 mixin 用例族（AINA §5 用例族分组，单文件 ≤400 行）
const algorithmAnalysis = require('./algorithm-analysis-service');
const sessionService = require('./session-service');
const orchestrationProxy = require('./orchestration-proxy');
const atlasConsult = require('./atlas-consult-service');

class ExpertAlliance {
  constructor() {
    // infrastructure 仓储装配
    this.repo = new ExpertRepository();
    this.metricsStore = new MetricsStore();
    this.store = new SessionChainStore();
    // 兼容视图（历史公开字段）
    this.experts = this.repo.experts;
    this.sessions = this.store.sessions;
    this.sessionChains = this.store.sessionChains;
    this.expertStats = this.metricsStore.stats;

    this.orchestrationEngine = null;
    this._initOrchestration();
  }

  _initOrchestration() {
    try {
      this.orchestrationEngine = getOrchestrationEngine();
    } catch (e) {
      console.warn('[expert-alliance] orchestration engine init failed:', e.message);
    }
  }

  // ==================== 专家管理（委托仓储） ====================

  listExperts(filters = {}) {
    return this.repo.list(filters);
  }

  getExpert(id) {
    return this.repo.get(id);
  }

  registerExpert(expert) {
    const created = this.repo.register(expert);
    this.experts = this.repo.experts;
    return created;
  }

  updateExpert(id, updates) {
    return this.repo.update(id, updates);
  }

  removeExpert(id) {
    const removed = this.repo.remove(id);
    this.experts = this.repo.experts;
    return removed;
  }

  getExpertCapabilities() {
    return this.repo.capabilities();
  }

  getExpertTypes() {
    return this.repo.types();
  }

  // ==================== 智能路由（委托 domain 算法） ====================

  async routeExperts(question, options = {}) {
    const startTime = Date.now();
    const intentResult = detectIntent(question);
    const candidateExperts = matchExperts(this.repo.active(), question, intentResult);
    const scoredExperts = scoreExperts(candidateExperts, (id) => this.metricsStore.of(id));
    const selected = scoredExperts.slice(0, options.maxExperts || 3);

    return {
      intent: intentResult,
      candidates: scoredExperts,
      selected,
      routing_time_ms: Date.now() - startTime,
      strategy: options.strategy || 'score_based'
    };
  }

  // ==================== 对话咨询 ====================

  async consult(expertId, messages, options = {}) {
    const expert = this.repo.get(expertId);
    if (!expert) throw new Error(`专家不存在: ${expertId}`);
    if (expert.status !== 'active') throw new Error(`专家 ${expert.name} 当前不在线`);

    const startTime = Date.now();
    const gateway = getGateway();
    const systemPrompt = options.useCustomPrompt ? options.systemPrompt : expert.systemPrompt;

    const contextMessages = this._buildContextMessages(expert, messages, options);

    const result = await gateway.chat({
      messages: contextMessages,
      sessionId: options.sessionId || `sess_${Date.now()}`,
      expertType: expert.type,
      systemPrompt,
      webSearchContext: options.webSearchContext || null,
      temperature: options.temperature || 0.7,
      maxTokens: options.maxTokens || 2048
    });

    const duration = Date.now() - startTime;
    this._updateExpertMetrics(expertId, duration, true, result.metadata?.confidence);

    return {
      expert: { id: expert.id, name: expert.name, type: expert.type },
      response: result.content,
      metadata: {
        ...result.metadata,
        expert_type: expert.type,
        consulted_at: new Date().toISOString(),
        duration_ms: duration
      }
    };
  }

  _buildContextMessages(expert, messages, options = {}) {
    const enhancedSystem = options.useCustomPrompt ? options.systemPrompt : expert.systemPrompt;

    const contextParts = [];
    if (options.problemContext) {
      contextParts.push(`## 背景上下文\n${options.problemContext}`);
    }
    if (options.businessConstraints) {
      contextParts.push(`## 业务约束\n${options.businessConstraints}`);
    }

    const enhancedMessages = [{
      role: 'system',
      content: contextParts.length > 0
        ? `${enhancedSystem}\n\n${contextParts.join('\n\n')}`
        : enhancedSystem
    }];

    if (options.includeExpertContext !== false) {
      enhancedMessages[0].content += `\n\n专家能力: ${expert.capabilities.join(', ')}`;
    }

    enhancedMessages.push(...messages);
    return enhancedMessages;
  }

  async intelligentConsult(question, options = {}) {
    const routing = await this.routeExperts(question, options);
    const mode = options.mode || (routing.selected.length > 1 ? 'multi' : 'single');

    if (mode === 'single' || routing.selected.length === 1) {
      const expertId = routing.selected[0].expert.id;
      const result = await this.consult(expertId, [{ role: 'user', content: question }], options);
      return { ...result, routing, mode: 'single' };
    }

    if (mode === 'multi') {
      const expertIds = routing.selected.map(s => s.expert.id);
      const multiResult = await this.multiExpertConsult(question, expertIds, options);
      return { ...multiResult, routing, mode: 'multi' };
    }

    if (mode === 'debate') {
      const expertIds = routing.selected.map(s => s.expert.id);
      const debateResult = await this.debate(question, expertIds, options);
      return { ...debateResult, routing, mode: 'debate' };
    }

    const expertId = routing.selected[0].expert.id;
    const result = await this.consult(expertId, [{ role: 'user', content: question }], options);
    return { ...result, routing, mode: 'single' };
  }

  async multiExpertConsult(question, expertIds, options = {}) {
    const results = [];
    const gateway = getGateway();

    for (const expertId of expertIds) {
      const expert = this.repo.get(expertId);
      if (!expert || expert.status !== 'active') continue;

      const startTime = Date.now();
      try {
        const result = await gateway.chat({
          messages: [{ role: 'user', content: question }],
          expertType: expert.type,
          systemPrompt: expert.systemPrompt,
          temperature: options.temperature || 0.7,
          maxTokens: options.maxTokens || 1024
        });

        const duration = Date.now() - startTime;
        this._updateExpertMetrics(expertId, duration, true, result.metadata?.confidence);

        results.push({
          expert: { id: expert.id, name: expert.name, type: expert.type },
          response: result.content,
          confidence: result.metadata?.confidence || 0.7,
          duration_ms: duration,
          success: true
        });
      } catch (e) {
        const duration = Date.now() - startTime;
        this._updateExpertMetrics(expertId, duration, false);
        results.push({
          expert: { id: expert.id, name: expert.name, type: expert.type },
          error: e.message,
          duration_ms: duration,
          success: false
        });
      }
    }

    return {
      question,
      total: results.length,
      successful: results.filter(r => r.success).length,
      results,
      synthesized_at: new Date().toISOString()
    };
  }

  async debate(question, expertIds, options = {}) {
    const rounds = options.rounds || 2;
    const history = [];
    const startTime = Date.now();

    for (let round = 0; round < rounds; round++) {
      const roundQuestion = round === 0
        ? question
        : `[第${round + 1}轮辩论] 基于上一轮讨论，继续深入分析：\n\n${history.flatMap(h => h.results.map(r => `${r.expert.name}: ${r.response}`)).join('\n\n')}\n\n新问题：${question}`;

      const roundResults = await this.multiExpertConsult(roundQuestion, expertIds, options);
      history.push(roundResults);
    }

    return {
      question,
      rounds,
      history,
      final_synthesis: synthesizeDebate(history),
      total_duration_ms: Date.now() - startTime,
      completed_at: new Date().toISOString()
    };
  }

  // ============ 算法联盟集成：见 algorithm-analysis-service.js（mixin） ============

  // ==================== 指标与统计（委托仓储） ====================

  _updateExpertMetrics(expertId, duration, success, confidence) {
    const expert = this.repo.get(expertId);
    if (!expert) return;
    const persisted = this.metricsStore.record(expert, duration, success, confidence);
    if (persisted) this.repo.persist();
  }

  getExpertMetrics(expertId) {
    const expert = this.repo.get(expertId);
    if (!expert) return null;
    const stats = this.metricsStore.of(expertId);
    return {
      expert: { id: expert.id, name: expert.name, type: expert.type },
      metrics: expert.metrics,
      detailed: {
        total_consults: stats.total_consults || 0,
        successful_consults: stats.successful_consults || 0,
        total_duration: stats.total_duration || 0,
        recent_confidences: (stats.confidences || []).slice(-10)
      }
    };
  }

  getAllMetrics() {
    return this.repo.active().map(e => this.getExpertMetrics(e.id));
  }

  // ==================== 会话链与会话管理（委托仓储） ====================

  createSessionChain(name, expertIds, options = {}) {
    const chain = this.store.createChain(name, expertIds, options);
    this.sessionChains = this.store.sessionChains;
    return chain;
  }

  getSessionChain(id) {
    return this.store.getChain(id);
  }

  listSessionChains() {
    return this.store.listChains();
  }

  // executeChain：见 session-service.js（mixin）

  createSession(options = {}) {
    const session = this.store.createSession(options);
    this.sessions = this.store.sessions;
    return session;
  }

  getSession(sessionId) {
    return this.store.getSession(sessionId);
  }

  listSessions() {
    return this.store.listSessions();
  }

  appendMessage(sessionId, message) {
    return this.store.appendMessage(sessionId, message);
  }

  // processSessionMessage：见 session-service.js（mixin）

  // ==================== V2 编排引擎代理 ====================

  getOrchestrationEngine() {
    return this.orchestrationEngine;
  }

  // orchestrate / generatePlan / getOrchestrationStats / listPlugins /
  // runPlanExecution：见 orchestration-proxy.js（mixin）

  // ==================== 便捷方法 ====================

  async analyzeWithAllExperts(question, options = {}) {
    const activeExperts = this.repo.active().map(e => e.id);
    return this.multiExpertConsult(question, activeExperts, options);
  }

  getSystemOverview() {
    const experts = Array.from(this.repo.experts.values());
    const activeExperts = experts.filter(e => e.status === 'active');
    const totalConsults = experts.reduce((sum, e) => sum + (e.metrics?.total_consults || 0), 0);
    const avgSuccessRate = activeExperts.length > 0
      ? activeExperts.reduce((sum, e) => sum + (e.metrics?.success_rate || 1.0), 0) / activeExperts.length
      : 0;

    return {
      total_experts: experts.length,
      active_experts: activeExperts.length,
      expert_types: this.getExpertTypes(),
      total_consults: totalConsults,
      avg_success_rate: Math.round(avgSuccessRate * 100) / 100,
      capabilities_count: Object.keys(this.getExpertCapabilities()).length,
      session_chains: this.store.chainCount(),
      uptime_since: this._initTime || new Date().toISOString()
    };
  }
}

// mixin 用例族装配（对外契约不变：方法直接挂到原型）
Object.assign(ExpertAlliance.prototype, algorithmAnalysis, sessionService, orchestrationProxy, atlasConsult);

// 历史私有方法契约（A16/A18/A20 测试与消费方直调）：委托 domain 单源实现
Object.assign(ExpertAlliance.prototype, {
  _detectIntent: detectIntent,
  _extractConsensus: extractConsensus,
  _extractDivergences: extractDivergences,
  _generateFinalRecommendation: generateFinalRecommendation
});

module.exports = { ExpertAlliance };
