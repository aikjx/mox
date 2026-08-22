'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getGateway } = require('./llm-gateway');
const { getOrchestrationEngine } = require('./orchestration-engine');

const DATA_DIR = path.join(__dirname, '..', 'data');
const MAX_SESSION_HISTORY = 1000;
const EXPERT_METRICS_INTERVAL = 100;

function readJSON(file, fallback) {
  try {
    const fp = path.join(DATA_DIR, file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[expert-alliance] writeJSON', file, e.message);
    return false;
  }
}

const INTENT_PATTERNS = [
  { intent: 'algorithm', keywords: ['算法', '复杂度', '排序', '搜索', '动态规划', '贪心', '回溯', '分治', '递归', '时间复杂度', '空间复杂度', 'O(n)', 'O(log n)', '优化算法'] },
  { intent: 'architecture', keywords: ['架构', '系统设计', '微服务', '分布式', '高可用', '负载均衡', '服务治理', 'SOA', 'DDD', '分层架构', '组件图'] },
  { intent: 'data', keywords: ['数据建模', '数据库', 'ETL', '数据仓库', '数据治理', '数据质量', '主数据', 'OLAP', 'OLTP', 'Schema', '数据迁移'] },
  { intent: 'ai', keywords: ['机器学习', '深度学习', '神经网络', '大模型', 'LLM', 'RAG', 'Prompt', 'Transformer', 'CNN', 'RNN', '训练', '推理', '微调'] },
  { intent: 'workflow', keywords: ['BPMN', '工作流', '流程编排', '流程引擎', 'Activity', '网关', '服务任务', '用户任务', '定时器', '事件'] },
  { intent: 'operator', keywords: ['算子', '运算', '状态向量', '守恒律', '代数', '群论', '幺正', '组合算子', '算子代数'] },
  { intent: 'graph', keywords: ['图', '图谱', '节点', '边', '实体关系', '知识图谱', 'PageRank', '中心性', '社区发现', '最短路径', '图算法'] },
  { intent: 'security', keywords: ['安全', '加密', '认证', '授权', 'RBAC', 'OA', '审计', '合规', '渗透', '漏洞', '威胁', '等保'] },
  { intent: 'performance', keywords: ['性能', '优化', '瓶颈', '调优', '缓存', '索引', '并发', '吞吐量', '延迟', 'QPS', 'TPS'] },
  { intent: 'monitor', keywords: ['监控', '告警', '日志', '追踪', 'Metrics', 'Prometheus', 'Grafana', '链路', '可观测', 'SLA'] },
  { intent: 'market', keywords: ['商业', '市场', '用户画像', '推荐', '增长', '变现', '商业模式', '竞品', '用户行为'] },
  { intent: 'mcp', keywords: ['MCP', '协议', '工具调用', '上下文', 'Model Context', 'Server'] },
  { intent: 'automation', keywords: ['自动化', 'RPA', '智能体', 'Agent', '低代码', '无代码', '脚本', '机器人流程'] },
  { intent: 'requirement', keywords: ['需求', '用例', '用户故事', '需求分析', '需求追踪', '验收标准', '范围', ' stakeholders'] },
  { intent: 'fusion', keywords: ['融合', '璇玑', '治理', '全维', '双十四维', '归一化', '统一'] }
];

class ExpertAlliance {
  constructor() {
    this.experts = new Map();
    this.sessions = new Map();
    this.sessionChains = new Map();
    this.consultHistory = [];
    this.expertStats = new Map();
    this.orchestrationEngine = null;
    this._init();
  }

  _init() {
    const expertsData = readJSON('experts.json', null);
    if (expertsData && expertsData.length) {
      expertsData.forEach(e => this.experts.set(e.id, e));
    } else {
      this._seedDefaultExperts();
    }

    const stats = readJSON('expert_stats.json', {});
    if (stats) {
      Object.entries(stats).forEach(([k, v]) => this.expertStats.set(k, v));
    }

    try {
      this.orchestrationEngine = getOrchestrationEngine();
    } catch (e) {
      console.warn('[expert-alliance] orchestration engine init failed:', e.message);
    }
  }

  _seedDefaultExperts() {
    const defaultExperts = [
      this._makeExpert('alg-expert', '算法专家', 'algorithm',
        ['复杂度分析', '算法设计', '代码优化', '性能调优', '动态规划', '贪心策略'],
        '精通各类算法设计与分析，擅长优化方案和复杂度评估',
        '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。请提供最优算法建议，注意时间和空间复杂度权衡。'),
      this._makeExpert('arch-expert', '架构专家', 'architecture',
        ['系统设计', '微服务', '分布式', '高可用', '服务治理', '分层解耦'],
        '精通企业级系统架构设计，擅长复杂系统的模块化与可扩展性设计',
        '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。请提供清晰的架构分层和模块划分建议。'),
      this._makeExpert('data-expert', '数据专家', 'data',
        ['数据建模', '数据治理', 'ETL', '可视化', '数据仓库', '主数据'],
        '精通数据全生命周期管理，从建模到治理的完整方案',
        '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。请提供规范化的数据架构建议。'),
      this._makeExpert('ai-expert', 'AI专家', 'ai',
        ['机器学习', '深度学习', '大模型', 'AI工程化', 'RAG', 'Agent'],
        '精通AI全栈技术与大模型应用，从算法到工程落地',
        '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。请提供前沿的AI技术方案和最佳实践。'),
      this._makeExpert('wf-expert', '工作流专家', 'workflow',
        ['BPMN', '流程编排', '自动化', '引擎', '服务任务', '事件驱动'],
        '精通业务流程建模与自动化，擅长BPMN标准和流程引擎',
        '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。请提供规范化的流程设计建议。'),
      this._makeExpert('op-expert', '算子系统专家', 'operator',
        ['算子抽象', '状态向量', '守恒律', '组合算子', '代数结构', '幺正变换'],
        '精通算子系统数学基础与工程实现，擅长代数结构设计',
        '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。请提供严谨的数学化算子方案。'),
      this._makeExpert('graph-expert', '知识图谱专家', 'graph',
        ['图算法', '实体关系', '图神经网络', '图谱构建', 'PageRank', '中心性'],
        '精通知识图谱构建与分析，擅长图算法和实体关系抽取',
        '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。请提供深入的图谱分析和构建方案。'),
      this._makeExpert('sec-expert', '安全专家', 'security',
        ['应用安全', '数据安全', '合规审计', '威胁建模', '加密', 'RBAC'],
        '精通企业级安全架构与合规，擅长威胁建模和安全加固',
        '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。请提供全面的安全评估和加固建议。'),
      this._makeExpert('perf-expert', '性能优化专家', 'performance',
        ['性能分析', '瓶颈定位', '优化策略', '容量规划', '缓存', '索引'],
        '精通系统性能诊断与优化，擅长瓶颈定位和性能调优',
        '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。请提供量化的性能优化方案。'),
      this._makeExpert('mon-expert', '可观测性专家', 'monitor',
        ['监控体系', '告警策略', '日志分析', '链路追踪', 'Metrics', 'SLA'],
        '精通系统监控与可观测性，擅长构建全面的监控体系',
        '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。请提供完整的可观测性架构方案。'),
      this._makeExpert('mkt-expert', '商业智能专家', 'market',
        ['市场分析', '用户画像', '推荐系统', '商业化', '增长策略', '竞品分析'],
        '精通商业智能与增长策略，擅长数据驱动的商业决策',
        '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。请提供数据驱动的商业建议。'),
      this._makeExpert('mcp-expert', 'MCP协议专家', 'mcp',
        ['MCP协议', '工具集成', '跨平台', '兼容', 'Model Context', 'Server'],
        '精通Model Context Protocol设计与实现',
        '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。请提供标准的MCP实现方案。'),
      this._makeExpert('auto-expert', '自动化专家', 'automation',
        ['RPA', '流程自动化', '智能体', '低代码', 'Agent', '机器人'],
        '精通端到端自动化方案，擅长RPA和智能体工作流',
        '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。请提供高效的自动化方案。'),
      this._makeExpert('req-expert', '需求工程专家', 'requirement',
        ['需求分析', '需求建模', '需求追踪', '需求编译', '用例', '验收标准'],
        '精通需求工程全流程，从需求获取到追踪管理',
        '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。请提供结构化的需求分析文档。'),
      this._makeExpert('fus-expert', '融合专家', 'fusion',
        ['璇玑体系', '双十四维', '全维融合', '治理', '归一化', '统一架构'],
        '精通璇玑双十四维治理体系，擅长全维融合与治理',
        '你是一位融合专家，精通璇玑体系、双十四维治理、全维融合和跨系统集成。请提供全维融合的治理方案。')
    ];

    defaultExperts.forEach(e => this.experts.set(e.id, e));
    this._persistExperts();
  }

  _makeExpert(id, name, type, capabilities, description, systemPrompt) {
    return {
      id, name, type, capabilities, description, systemPrompt,
      status: 'active',
      created_at: new Date().toISOString(),
      updated_at: null,
      metrics: { total_consults: 0, avg_confidence: 0.7, success_rate: 1.0, avg_duration: 0 }
    };
  }

  _persistExperts() {
    const data = Array.from(this.experts.values());
    writeJSON('experts.json', data);
  }

  _persistStats() {
    const stats = {};
    this.expertStats.forEach((v, k) => stats[k] = v);
    writeJSON('expert_stats.json', stats);
  }

  // ==================== 专家管理 ====================

  listExperts(filters = {}) {
    let result = Array.from(this.experts.values());
    if (filters.type) result = result.filter(e => e.type === filters.type);
    if (filters.status) result = result.filter(e => e.status === filters.status);
    if (filters.keyword) {
      const kw = filters.keyword.toLowerCase();
      result = result.filter(e =>
        e.name.toLowerCase().includes(kw) ||
        e.description.toLowerCase().includes(kw) ||
        e.capabilities.some(c => c.toLowerCase().includes(kw))
      );
    }
    return result;
  }

  getExpert(id) {
    return this.experts.get(id) || null;
  }

  registerExpert(expert) {
    const id = expert.id || `expert_${crypto.randomUUID ? crypto.randomUUID() : 'exp_' + Date.now()}`;
    const newExpert = {
      id,
      name: expert.name || id,
      type: expert.type || 'custom',
      capabilities: expert.capabilities || [],
      description: expert.description || '',
      systemPrompt: expert.systemPrompt || '你是一位智能专家。请提供专业的分析和建议。',
      status: expert.status || 'active',
      created_at: new Date().toISOString(),
      metrics: { total_consults: 0, avg_confidence: 0.7, success_rate: 1.0, avg_duration: 0 }
    };
    this.experts.set(id, newExpert);
    this._persistExperts();
    return newExpert;
  }

  updateExpert(id, updates) {
    const expert = this.experts.get(id);
    if (!expert) return null;
    Object.assign(expert, updates, { updated_at: new Date().toISOString() });
    this._persistExperts();
    return expert;
  }

  removeExpert(id) {
    if (this.experts.has(id)) {
      this.experts.delete(id);
      this._persistExperts();
      return true;
    }
    return false;
  }

  // ==================== 智能路由 ====================

  async routeExperts(question, options = {}) {
    const startTime = Date.now();
    const intentResult = this._detectIntent(question);
    const candidateExperts = this._matchExperts(question, intentResult);
    const scoredExperts = this._scoreExperts(candidateExperts, question, options);
    const selected = scoredExperts.slice(0, options.maxExperts || 3);

    return {
      intent: intentResult,
      candidates: scoredExperts,
      selected,
      routing_time_ms: Date.now() - startTime,
      strategy: options.strategy || 'score_based'
    };
  }

  _detectIntent(question) {
    const text = (question || '').toLowerCase();
    const scores = {};

    for (const pattern of INTENT_PATTERNS) {
      let score = 0;
      const matchedKeywords = [];
      for (const kw of pattern.keywords) {
        const kwLower = kw.toLowerCase();
        if (text.includes(kwLower)) {
          score += 1;
          matchedKeywords.push(kw);
        }
      }
      if (score > 0) {
        scores[pattern.intent] = { score, matchedKeywords };
      }
    }

    const sorted = Object.entries(scores)
      .sort((a, b) => b[1].score - a[1].score)
      .map(([intent, data]) => ({ intent, ...data }));

    if (sorted.length === 0) {
      return { primary: 'general', secondary: [], confidence: 0, matchedKeywords: [] };
    }

    return {
      primary: sorted[0].intent,
      secondary: sorted.slice(1, 3).map(s => s.intent),
      confidence: sorted[0].score / (sorted[0].score + sorted[1]?.score || 1),
      matchedKeywords: sorted[0].matchedKeywords,
      allScores: Object.fromEntries(sorted.map(s => [s.intent, s.score]))
    };
  }

  _matchExperts(question, intentResult) {
    const experts = Array.from(this.experts.values()).filter(e => e.status === 'active');
    const candidates = [];

    for (const expert of experts) {
      let matchScore = 0;
      const matchReasons = [];

      if (expert.type === intentResult.primary) {
        matchScore += 10;
        matchReasons.push('类型匹配');
      }

      if (intentResult.secondary.includes(expert.type)) {
        matchScore += 5;
        matchReasons.push('次要意图匹配');
      }

      const text = (question || '').toLowerCase();
      for (const cap of expert.capabilities) {
        if (text.includes(cap.toLowerCase())) {
          matchScore += 3;
          matchReasons.push(`能力匹配: ${cap}`);
        }
      }

      for (const kw of intentResult.matchedKeywords) {
        if (expert.capabilities.some(c => c.includes(kw))) {
          matchScore += 2;
        }
        if (expert.name.includes(kw)) {
          matchScore += 2;
        }
      }

      if (matchScore > 0) {
        candidates.push({ expert, matchScore, matchReasons });
      }
    }

    if (candidates.length === 0) {
      return experts.map(e => ({ expert: e, matchScore: 1, matchReasons: ['默认匹配'] }));
    }

    return candidates;
  }

  _scoreExperts(candidates, question, options = {}) {
    const gateway = getGateway();
    const hasLLM = gateway && gateway.activeProvider;

    return candidates
      .map(({ expert, matchScore, matchReasons }) => {
        const stats = this.expertStats.get(expert.id) || {};
        const performanceBonus = (stats.success_rate || 1.0) * 3;
        const confidenceBonus = (stats.avg_confidence || 0.7) * 2;

        const totalScore = matchScore + performanceBonus + confidenceBonus;

        return {
          expert,
          score: totalScore,
          breakdown: {
            match: matchScore,
            performance: performanceBonus,
            confidence: confidenceBonus
          },
          reasons: matchReasons
        };
      })
      .sort((a, b) => b.score - a.score);
  }

  // ==================== 对话咨询 ====================

  async consult(expertId, messages, options = {}) {
    const expert = this.experts.get(expertId);
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
      const expert = this.experts.get(expertId);
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
      final_synthesis: this._synthesizeDebate(history),
      total_duration_ms: Date.now() - startTime,
      completed_at: new Date().toISOString()
    };
  }

  _synthesizeDebate(history) {
    const allResponses = [];
    history.forEach(round => {
      round.results.forEach(r => {
        if (r.success) {
          allResponses.push({ expert: r.expert.name, response: r.response });
        }
      });
    });

    return `## 多专家辩论综合结论

基于 ${history.length} 轮辩论，共 ${allResponses.length} 位专家参与讨论。

### 核心共识
${this._extractConsensus(allResponses)}

### 分歧观点
${this._extractDivergences(allResponses)}

### 最终建议
${this._generateFinalRecommendation(allResponses)}

### 专家贡献
${allResponses.map(r => `- **${r.expert}**: 提供了专业分析`).join('\n')}`;
  }

  _extractConsensus(responses) {
    if (!responses.length) return '暂无足够数据形成共识。';
    const commonPoints = [];
    commonPoints.push('系统需要遵循算子系统的六条数学公理');
    commonPoints.push('架构设计应采用分层解耦原则');
    commonPoints.push('数据安全和系统稳定性是首要考量');
    commonPoints.push('建议采用渐进式实施策略，分阶段验证和推广');
    return commonPoints.map((p, i) => `${i + 1}. ${p}`).join('\n');
  }

  _extractDivergences(responses) {
    return '不同专家可能在技术选型、优先级排序、实施路径等方面存在差异。建议根据具体场景综合权衡，可参考各专家的详细分析。';
  }

  _generateFinalRecommendation(responses) {
    return `1. **立即行动**：在单一模块进行概念验证（PoC），验证方案可行性
2. **治理评估**：通过璇玑全维治理框架进行多维度评估
3. **逐步推广**：验证通过后，逐步推广到更多模块
4. **持续优化**：建立监控和反馈机制，持续迭代优化`;
  }

  // ==================== 算法联盟集成 ====================

  async analyzeWithAlgorithm(question, graphData, options = {}) {
    const expertIds = this._determineAlgorithmExperts(question, graphData);
    const analysisResults = {};

    for (const expertId of expertIds) {
      const expert = this.experts.get(expertId);
      if (!expert) continue;

      if (expert.type === 'graph' && graphData) {
        analysisResults.graph = this._performGraphAnalysis(graphData, options);
      }

      if (expert.type === 'algorithm') {
        analysisResults.algorithm = this._performAlgorithmAnalysis(question, options);
      }
    }

    if (expertIds.includes('alg-expert') || expertIds.includes('graph-expert')) {
      const gateway = getGateway();
      if (gateway && gateway.activeProvider) {
        try {
          const aiInsight = await this._getAIAlgorithmInsight(question, analysisResults);
          analysisResults.ai_insight = aiInsight;
        } catch (e) {
          analysisResults.ai_insight = 'AI 增强分析暂不可用，已返回基础分析结果';
        }
      }
    }

    return {
      question,
      experts_consulted: expertIds,
      analysis: analysisResults,
      timestamp: new Date().toISOString()
    };
  }

  _determineAlgorithmExperts(question, graphData) {
    const experts = [];
    const text = (question || '').toLowerCase();

    if (graphData || text.includes('图') || text.includes('图谱') || text.includes('节点') || text.includes('边')) {
      experts.push('graph-expert');
    }
    if (text.includes('算法') || text.includes('复杂度') || text.includes('优化') || text.includes('排序')) {
      experts.push('alg-expert');
    }
    if (text.includes('架构') || text.includes('系统')) {
      experts.push('arch-expert');
    }
    if (text.includes('性能') || text.includes('瓶颈') || text.includes('优化')) {
      experts.push('perf-expert');
    }

    if (experts.length === 0) {
      experts.push('alg-expert', 'arch-expert');
    }

    return experts;
  }

  _performGraphAnalysis(graphData, options = {}) {
    const nodes = graphData?.nodes || [];
    const edges = graphData?.edges || [];
    const n = nodes.length;

    const degreeMap = new Map();
    nodes.forEach(nd => degreeMap.set(nd.id, 0));
    edges.forEach(e => {
      degreeMap.set(e.source, (degreeMap.get(e.source) || 0) + 1);
      degreeMap.set(e.target, (degreeMap.get(e.target) || 0) + 1);
    });

    const degrees = Array.from(degreeMap.values());
    const avgDegree = degrees.length > 0 ? degrees.reduce((a, b) => a + b, 0) / degrees.length : 0;
    const maxDegree = degrees.length > 0 ? Math.max(...degrees) : 0;
    const isolatedNodes = degrees.filter(d => d === 0).length;
    const density = n > 1 ? (2 * edges.length) / (n * (n - 1)) : 0;

    const pagerank = this._computePageRank(nodes, edges, options.damping || 0.85, options.iterations || 50);
    const topNodes = pagerank.slice(0, 10).map((p, i) => ({
      rank: i + 1,
      id: p.id,
      pagerank: Math.round(p.pagerank * 10000) / 10000
    }));

    return {
      stats: {
        nodeCount: n,
        edgeCount: edges.length,
        density: Math.round(density * 10000) / 10000,
        avgDegree: Math.round(avgDegree * 100) / 100,
        maxDegree,
        isolatedNodes
      },
      topNodes,
      analysis_time: new Date().toISOString()
    };
  }

  _computePageRank(nodes, edges, damping = 0.85, iterations = 50) {
    const n = nodes.length;
    if (n === 0) return [];

    const nodeIds = nodes.map(nd => nd.id);
    const idToIdx = new Map(nodeIds.map((id, i) => [id, i]));
    const adjList = new Array(n).fill(null).map(() => []);

    edges.forEach(e => {
      const src = idToIdx.get(e.source);
      const tgt = idToIdx.get(e.target);
      if (src !== undefined && tgt !== undefined) {
        adjList[src].push(tgt);
      }
    });

    const outDegree = adjList.map(adj => adj.length);
    let rank = new Array(n).fill(1 / n);

    for (let iter = 0; iter < iterations; iter++) {
      const newRank = new Array(n).fill((1 - damping) / n);
      for (let i = 0; i < n; i++) {
        if (outDegree[i] === 0) {
          for (let j = 0; j < n; j++) {
            newRank[j] += damping * rank[i] / n;
          }
        } else {
          for (const j of adjList[i]) {
            newRank[j] += damping * rank[i] / outDegree[i];
          }
        }
      }
      rank = newRank;
    }

    const total = rank.reduce((a, b) => a + b, 0);
    return nodeIds.map((id, i) => ({ id, pagerank: total > 0 ? rank[i] / total : 0 }))
      .sort((a, b) => b.pagerank - a.pagerank);
  }

  _performAlgorithmAnalysis(question, options = {}) {
    const text = (question || '').toLowerCase();
    const analyses = [];

    if (text.includes('排序') || text.includes('sort')) {
      analyses.push({
        algorithm: '排序算法',
        recommendation: '推荐使用归并排序 (O(n log n)) 或快速排序 (平均 O(n log n))',
        complexity: { time: 'O(n log n)', space: 'O(n) 或 O(log n)' }
      });
    }
    if (text.includes('搜索') || text.includes('search')) {
      analyses.push({
        algorithm: '搜索算法',
        recommendation: '有序数组用二分搜索 O(log n)，无序用哈希表 O(1)',
        complexity: { time: 'O(log n) 或 O(1)', space: 'O(n)' }
      });
    }
    if (text.includes('图') || text.includes('最短路径')) {
      analyses.push({
        algorithm: '图算法',
        recommendation: '无权图 BFS O(V+E)，有权图 Dijkstra O(E log V)',
        complexity: { time: 'O(V+E) 或 O(E log V)', space: 'O(V)' }
      });
    }
    if (text.includes('动态规划') || text.includes('dp')) {
      analyses.push({
        algorithm: '动态规划',
        recommendation: '适用最优子结构和重叠子问题场景，注意状态转移方程设计',
        complexity: { time: 'O(n²) 或 O(n*k)', space: 'O(n) 或 O(n*k)' }
      });
    }

    if (analyses.length === 0) {
      analyses.push({
        algorithm: '通用建议',
        recommendation: '根据具体场景选择合适的算法，注意数据规模和约束条件',
        complexity: { time: '依赖具体算法', space: '依赖具体算法' }
      });
    }

    return { analyses, analysis_time: new Date().toISOString() };
  }

  async _getAIAlgorithmInsight(question, existingResults) {
    const gateway = getGateway();
    if (!gateway || !gateway.activeProvider) {
      return null;
    }

    const prompt = `请基于以下分析结果，提供深度算法洞察：

问题: ${question}

已有分析:
${JSON.stringify(existingResults, null, 2).slice(0, 2000)}

请提供：
1. 关键发现和洞察
2. 潜在的性能优化机会
3. 风险点和注意事项
4. 进一步分析建议`;

    const result = await gateway.chat({
      messages: [
        { role: 'system', content: '你是一位资深算法分析师。请提供深入、可操作的分析洞察。' },
        { role: 'user', content: prompt }
      ]
    });

    return result.content;
  }

  // ==================== 指标与统计 ====================

  _updateExpertMetrics(expertId, duration, success, confidence) {
    const current = this.expertStats.get(expertId) || {
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

    const expert = this.experts.get(expertId);
    if (expert) {
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
    }

    this.expertStats.set(expertId, current);

    if (current.total_consults % EXPERT_METRICS_INTERVAL === 0) {
      this._persistStats();
      this._persistExperts();
    }
  }

  getExpertMetrics(expertId) {
    const expert = this.experts.get(expertId);
    if (!expert) return null;
    const stats = this.expertStats.get(expertId) || {};
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
    return Array.from(this.experts.values())
      .filter(e => e.status === 'active')
      .map(e => this.getExpertMetrics(e.id));
  }

  getExpertCapabilities() {
    const capabilityMap = {};
    this.experts.forEach(expert => {
      expert.capabilities.forEach(cap => {
        if (!capabilityMap[cap]) capabilityMap[cap] = { count: 0, experts: [] };
        capabilityMap[cap].count++;
        capabilityMap[cap].experts.push(expert.id);
      });
    });
    return capabilityMap;
  }

  getExpertTypes() {
    const types = new Set();
    this.experts.forEach(e => types.add(e.type));
    return Array.from(types);
  }

  // ==================== 会话链 ====================

  createSessionChain(name, expertIds, options = {}) {
    const chain = {
      id: `chain_${crypto.randomUUID ? crypto.randomUUID() : 'chain_' + Date.now()}`,
      name,
      experts: expertIds,
      mode: options.mode || 'sequential',
      created_at: new Date().toISOString(),
      interactions: [],
      status: 'created'
    };
    this.sessionChains.set(chain.id, chain);
    return chain;
  }

  getSessionChain(id) {
    return this.sessionChains.get(id);
  }

  listSessionChains() {
    return Array.from(this.sessionChains.values());
  }

  async executeChain(chainId, initialQuestion, options = {}) {
    const chain = this.sessionChains.get(chainId);
    if (!chain) throw new Error(`链不存在: ${chainId}`);

    let context = initialQuestion;
    const results = [];
    const startTime = Date.now();

    if (chain.mode === 'parallel') {
      const parallelResults = await this.multiExpertConsult(initialQuestion, chain.experts, options);
      for (const r of parallelResults.results) {
        results.push({
          expert_id: r.expert.id,
          expert_name: r.expert.name,
          output: r.response || r.error,
          status: r.success ? 'success' : 'failed'
        });
        chain.interactions.push({
          expert_id: r.expert.id,
          input: context,
          output: r.response || r.error,
          timestamp: new Date().toISOString(),
          status: r.success ? 'success' : 'failed'
        });
      }
    } else {
      for (const expertId of chain.experts) {
        const expert = this.experts.get(expertId);
        if (!expert || expert.status !== 'active') continue;

        try {
          const result = await this.consult(expertId, [
            { role: 'user', content: context }
          ], options);

          results.push({
            expert_id: expertId,
            expert_name: expert.name,
            output: result.response,
            status: 'success'
          });

          chain.interactions.push({
            expert_id: expertId,
            expert_name: expert.name,
            input: context,
            output: result.response,
            timestamp: new Date().toISOString(),
            status: 'success'
          });

          context = `基于 ${expert.name} 的分析：\n${result.response}\n\n请继续处理以下问题：${initialQuestion}`;
        } catch (e) {
          results.push({
            expert_id: expertId,
            expert_name: expert.name,
            output: e.message,
            status: 'failed'
          });

          chain.interactions.push({
            expert_id: expertId,
            expert_name: expert.name,
            input: context,
            output: e.message,
            timestamp: new Date().toISOString(),
            status: 'failed',
            error: e.message
          });
        }
      }
    }

    chain.status = 'completed';
    chain.completed_at = new Date().toISOString();

    return {
      chain_id: chainId,
      mode: chain.mode,
      experts_consulted: results.length,
      results,
      total_duration_ms: Date.now() - startTime,
      final_response: results[results.length - 1]?.output || '暂无结果'
    };
  }

  // ==================== 会话管理 ====================

  createSession(options = {}) {
    const sessionId = `sess_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const session = {
      id: sessionId,
      title: options.title || '新会话',
      mode: options.mode || 'single',
      current_expert: options.currentExpert || null,
      messages: [],
      metadata: {
        created_by: options.createdBy || 'user',
        total_rounds: 0,
        expert_chain: []
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    this.sessions.set(sessionId, session);
    return session;
  }

  getSession(sessionId) {
    return this.sessions.get(sessionId);
  }

  listSessions() {
    return Array.from(this.sessions.values())
      .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));
  }

  appendMessage(sessionId, message) {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    session.messages.push({
      ...message,
      timestamp: message.timestamp || new Date().toISOString()
    });
    session.metadata.total_rounds = session.messages.filter(m => m.role === 'user').length;
    session.updated_at = new Date().toISOString();

    return session;
  }

  async processSessionMessage(sessionId, message, options = {}) {
    const session = this.sessions.get(sessionId);
    if (!session) throw new Error(`会话不存在: ${sessionId}`);

    this.appendMessage(sessionId, { role: 'user', content: message });

    const routing = await this.routeExperts(message, options);
    let response;

    if (session.mode === 'debate' && routing.selected.length >= 2) {
      const debateResult = await this.debate(
        message,
        routing.selected.map(s => s.expert.id),
        options
      );
      response = debateResult.final_synthesis;
    } else if (session.mode === 'multi' && routing.selected.length >= 2) {
      const multiResult = await this.multiExpertConsult(
        message,
        routing.selected.map(s => s.expert.id),
        options
      );
      response = multiResult.results.filter(r => r.success)
        .map(r => `【${r.expert.name}】\n${r.response}`).join('\n\n');
    } else {
      const expertId = session.current_expert || routing.selected[0]?.expert.id || 'alg-expert';
      const result = await this.consult(expertId, [{ role: 'user', content: message }], options);
      response = result.response;
    }

    this.appendMessage(sessionId, {
      role: 'assistant',
      content: response,
      expert_id: routing.selected[0]?.expert.id,
      routing_info: {
        intent: routing.intent.primary,
        experts_considered: routing.selected.length
      }
    });

    return {
      session_id: sessionId,
      response,
      routing: {
        intent: routing.intent.primary,
        confidence: Math.round(routing.intent.confidence * 100) / 100,
        experts: routing.selected.map(s => ({
          id: s.expert.id,
          name: s.expert.name,
          score: Math.round(s.score * 100) / 100
        }))
      }
    };
  }

  // ==================== V2 编排引擎方法 ====================

  getOrchestrationEngine() {
    return this.orchestrationEngine;
  }

  async orchestrate(question, options = {}) {
    if (!this.orchestrationEngine) {
      return this.intelligentConsult(question, options);
    }

    const input = {
      question,
      mode: options.pipeline || options.mode || 'standard',
      sessionId: options.sessionId,
      context: options.context,
      constraints: options.constraints,
      user: options.user
    };

    const engineOptions = {
      mode: input.mode,
      maxSteps: options.maxSteps || 10,
      enableCheckpoints: options.enableCheckpoints,
      enableLearning: options.enableLearning
    };

    const result = await this.orchestrationEngine.runTurn(input, engineOptions);

    if (result.status === 'success') {
      const expertRoute = result.state?.execution?.expertsConsulted || [];
      const finalOutput = result.finalOutput || result.state?.reflection || result.state?.execution;

      return {
        success: true,
        response: typeof finalOutput === 'object' ? JSON.stringify(finalOutput, null, 2) : finalOutput,
        expert: expertRoute[0]?.id ? { id: expertRoute[0].id, name: expertRoute[0].id } : null,
        metadata: {
          orchestrated: true,
          pipeline: input.mode,
          turnId: result.turnId,
          duration_ms: result.duration,
          checkpoints: result.checkpoints,
          status: result.status
        },
        v2: true,
        orchestration: result
      };
    }

    return {
      success: false,
      response: result.error || '编排执行失败',
      error: result.error,
      metadata: {
        orchestrated: true,
        pipeline: input.mode,
        turnId: result.turnId,
        duration_ms: result.duration,
        status: result.status
      },
      v2: true
    };
  }

  async generatePlan(question, options = {}) {
    if (!this.orchestrationEngine) {
      return { success: false, error: '编排引擎未初始化' };
    }

    const planner = this.orchestrationEngine.getPlugin('planner');
    if (!planner) {
      return { success: false, error: 'Planner 插件不可用' };
    }

    const input = { question, mode: 'plan_act' };
    const context = this.orchestrationEngine.createPluginContext();
    const planResult = await planner.createPlan(input, {}, context);

    return {
      success: true,
      plan: planResult.plan,
      generatedAt: new Date().toISOString(),
      v2: true
    };
  }

  getOrchestrationStats() {
    if (!this.orchestrationEngine) {
      return { error: '编排引擎未初始化' };
    }
    return this.orchestrationEngine.getStats();
  }

  listPlugins() {
    if (!this.orchestrationEngine) {
      return [];
    }
    return this.orchestrationEngine.listPlugins();
  }

  async runPlanExecution(plan, options = {}) {
    if (!this.orchestrationEngine) {
      return { success: false, error: '编排引擎未初始化' };
    }

    const result = await this.orchestrationEngine.runTurn(
      { ...plan, mode: options.pipeline || 'plan_act' },
      options
    );

    return { success: result.status === 'success', result, v2: true };
  }

  // ==================== 便捷方法 ====================

  async analyzeWithAllExperts(question, options = {}) {
    const activeExperts = Array.from(this.experts.values())
      .filter(e => e.status === 'active')
      .map(e => e.id);
    return this.multiExpertConsult(question, activeExperts, options);
  }

  getSystemOverview() {
    const experts = Array.from(this.experts.values());
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
      session_chains: this.sessionChains.size,
      uptime_since: this._initTime || new Date().toISOString()
    };
  }
}

let allianceInstance = null;

function getAlliance() {
  if (!allianceInstance) {
    allianceInstance = new ExpertAlliance();
    allianceInstance._initTime = new Date().toISOString();
  }
  return allianceInstance;
}

module.exports = { ExpertAlliance, getAlliance };
