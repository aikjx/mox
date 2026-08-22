'use strict';

/**
 * 专家仓储（infrastructure 层 · 唯一触碰专家持久化的位置）
 * ------------------------------------------------------------------
 * 职责：专家 Map 内存态 + experts.json 持久化 + 默认专家种子。
 * 持久化原语复用 lib/json-store（A3 单一真相源，享受 SQLite 双写）。
 */
const crypto = require('crypto');
const { readJSON, writeJSON } = require('../../lib/json-store');

function _makeExpert(id, name, type, capabilities, description, systemPrompt) {
  return {
    id, name, type, capabilities, description, systemPrompt,
    status: 'active',
    created_at: new Date().toISOString(),
    updated_at: null,
    metrics: { total_consults: 0, avg_confidence: 0.7, success_rate: 1.0, avg_duration: 0 }
  };
}

class ExpertRepository {
  constructor() {
    this.experts = new Map();
    this._load();
  }

  _load() {
    const expertsData = readJSON('experts.json', null);
    if (expertsData && expertsData.length) {
      expertsData.forEach(e => this.experts.set(e.id, e));
    } else {
      this._seedDefaults();
    }
    // 幂等补种：项目总架构师（全息图谱知识源专家）
    if (!this.experts.has('atlas-expert')) {
      this.experts.set('atlas-expert', _makeExpert('atlas-expert', '项目总架构师', 'architecture',
        ['项目全息图谱', '引擎宇宙', '架构规范', '全链路验证', '模块化设计', '影响面分析'],
        '以项目全息图谱（129 节点）为知识源，通晓 24 业务域/18 引擎/15 算法/34 数据资产的全部关联',
        '你是本项目首席架构师，精通项目全息图谱（Project Atlas）与引擎宇宙图谱。回答任何项目架构问题时，必须基于提供的图谱上下文（业务域/引擎/算法/数据/文档的关联关系与代码路径）作答，引用具体的域 ID、引擎 ID 与代码路径。'));
      this.persist();
    }
  }

  _seedDefaults() {
    const defaultExperts = [
      _makeExpert('alg-expert', '算法专家', 'algorithm',
        ['复杂度分析', '算法设计', '代码优化', '性能调优', '动态规划', '贪心策略'],
        '精通各类算法设计与分析，擅长优化方案和复杂度评估',
        '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。请提供最优算法建议，注意时间和空间复杂度权衡。'),
      _makeExpert('arch-expert', '架构专家', 'architecture',
        ['系统设计', '微服务', '分布式', '高可用', '服务治理', '分层解耦'],
        '精通企业级系统架构设计，擅长复杂系统的模块化与可扩展性设计',
        '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。请提供清晰的架构分层和模块划分建议。'),
      _makeExpert('data-expert', '数据专家', 'data',
        ['数据建模', '数据治理', 'ETL', '可视化', '数据仓库', '主数据'],
        '精通数据全生命周期管理，从建模到治理的完整方案',
        '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。请提供规范化的数据架构建议。'),
      _makeExpert('ai-expert', 'AI专家', 'ai',
        ['机器学习', '深度学习', '大模型', 'AI工程化', 'RAG', 'Agent'],
        '精通AI全栈技术与大模型应用，从算法到工程落地',
        '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。请提供前沿的AI技术方案和最佳实践。'),
      _makeExpert('wf-expert', '工作流专家', 'workflow',
        ['BPMN', '流程编排', '自动化', '引擎', '服务任务', '事件驱动'],
        '精通业务流程建模与自动化，擅长BPMN标准和流程引擎',
        '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。请提供规范化的流程设计建议。'),
      _makeExpert('op-expert', '算子系统专家', 'operator',
        ['算子抽象', '状态向量', '守恒律', '组合算子', '代数结构', '幺正变换'],
        '精通算子系统数学基础与工程实现，擅长代数结构设计',
        '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。请提供严谨的数学化算子方案。'),
      _makeExpert('graph-expert', '知识图谱专家', 'graph',
        ['图算法', '实体关系', '图神经网络', '图谱构建', 'PageRank', '中心性'],
        '精通知识图谱构建与分析，擅长图算法和实体关系抽取',
        '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。请提供深入的图谱分析和构建方案。'),
      _makeExpert('sec-expert', '安全专家', 'security',
        ['应用安全', '数据安全', '合规审计', '威胁建模', '加密', 'RBAC'],
        '精通企业级安全架构与合规，擅长威胁建模和安全加固',
        '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。请提供全面的安全评估和加固建议。'),
      _makeExpert('perf-expert', '性能优化专家', 'performance',
        ['性能分析', '瓶颈定位', '优化策略', '容量规划', '缓存', '索引'],
        '精通系统性能诊断与优化，擅长瓶颈定位和性能调优',
        '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。请提供量化的性能优化方案。'),
      _makeExpert('mon-expert', '可观测性专家', 'monitor',
        ['监控体系', '告警策略', '日志分析', '链路追踪', 'Metrics', 'SLA'],
        '精通系统监控与可观测性，擅长构建全面的监控体系',
        '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。请提供完整的可观测性架构方案。'),
      _makeExpert('mkt-expert', '商业智能专家', 'market',
        ['市场分析', '用户画像', '推荐系统', '商业化', '增长策略', '竞品分析'],
        '精通商业智能与增长策略，擅长数据驱动的商业决策',
        '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。请提供数据驱动的商业建议。'),
      _makeExpert('mcp-expert', 'MCP协议专家', 'mcp',
        ['MCP协议', '工具集成', '跨平台', '兼容', 'Model Context', 'Server'],
        '精通Model Context Protocol设计与实现',
        '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。请提供标准的MCP实现方案。'),
      _makeExpert('auto-expert', '自动化专家', 'automation',
        ['RPA', '流程自动化', '智能体', '低代码', 'Agent', '机器人'],
        '精通端到端自动化方案，擅长RPA和智能体工作流',
        '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。请提供高效的自动化方案。'),
      _makeExpert('req-expert', '需求工程专家', 'requirement',
        ['需求分析', '需求建模', '需求追踪', '需求编译', '用例', '验收标准'],
        '精通需求工程全流程，从需求获取到追踪管理',
        '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。请提供结构化的需求分析文档。'),
      _makeExpert('fus-expert', '融合专家', 'fusion',
        ['璇玑体系', '双十四维', '全维融合', '治理', '归一化', '统一架构'],
        '精通璇玑双十四维治理体系，擅长全维融合与治理',
        '你是一位融合专家，精通璇玑体系、双十四维治理、全维融合和跨系统集成。请提供全维融合的治理方案。')
    ];

    defaultExperts.forEach(e => this.experts.set(e.id, e));
    this.persist();
  }

  persist() {
    writeJSON('experts.json', Array.from(this.experts.values()));
  }

  list(filters = {}) {
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

  get(id) {
    return this.experts.get(id) || null;
  }

  active() {
    return Array.from(this.experts.values()).filter(e => e.status === 'active');
  }

  register(expert) {
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
    this.persist();
    return newExpert;
  }

  update(id, updates) {
    const expert = this.experts.get(id);
    if (!expert) return null;
    Object.assign(expert, updates, { updated_at: new Date().toISOString() });
    this.persist();
    return expert;
  }

  remove(id) {
    if (this.experts.has(id)) {
      this.experts.delete(id);
      this.persist();
      return true;
    }
    return false;
  }

  capabilities() {
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

  types() {
    const types = new Set();
    this.experts.forEach(e => types.add(e.type));
    return Array.from(types);
  }
}

module.exports = { ExpertRepository };
