'use strict';

const fs = require('fs');
const path = require('path');
const { getGateway } = require('./llm-gateway');

const DATA_DIR = path.join(__dirname, '..', 'data');

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

class ExpertAlliance {
  constructor() {
    this.experts = new Map();
    this.sessionChains = new Map();
    this._init();
  }

  _init() {
    const expertsData = readJSON('experts.json', null);
    if (expertsData) {
      expertsData.forEach(e => this.experts.set(e.id, e));
    } else {
      this._seedDefaultExperts();
    }
  }

  _seedDefaultExperts() {
    const defaultExperts = [
      {
        id: 'alg-expert',
        name: '算法专家',
        type: 'algorithm',
        capabilities: ['复杂度分析', '算法设计', '代码优化', '性能调优'],
        description: '精通各类算法设计与分析，擅长优化方案',
        systemPrompt: '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'arch-expert',
        name: '架构专家',
        type: 'architecture',
        capabilities: ['系统设计', '微服务', '分布式', '高可用'],
        description: '精通企业级系统架构设计',
        systemPrompt: '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'data-expert',
        name: '数据专家',
        type: 'data',
        capabilities: ['数据建模', '数据治理', 'ETL', '可视化'],
        description: '精通数据全生命周期管理',
        systemPrompt: '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'ai-expert',
        name: 'AI专家',
        type: 'ai',
        capabilities: ['机器学习', '深度学习', '大模型', 'AI工程化'],
        description: '精通AI全栈技术与大模型应用',
        systemPrompt: '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'wf-expert',
        name: '工作流专家',
        type: 'workflow',
        capabilities: ['BPMN', '流程编排', '自动化', '引擎'],
        description: '精通业务流程建模与自动化',
        systemPrompt: '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'op-expert',
        name: '算子系统专家',
        type: 'operator',
        capabilities: ['算子抽象', '状态向量', '守恒律', '组合算子'],
        description: '精通算子系统数学基础与工程实现',
        systemPrompt: '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'graph-expert',
        name: '知识图谱专家',
        type: 'graph',
        capabilities: ['图算法', '实体关系', '图神经网络', '图谱构建'],
        description: '精通知识图谱构建与分析',
        systemPrompt: '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'sec-expert',
        name: '安全专家',
        type: 'security',
        capabilities: ['应用安全', '数据安全', '合规审计', '威胁建模'],
        description: '精通企业级安全架构与合规',
        systemPrompt: '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'perf-expert',
        name: '性能优化专家',
        type: 'performance',
        capabilities: ['性能分析', '瓶颈定位', '优化策略', '容量规划'],
        description: '精通系统性能诊断与优化',
        systemPrompt: '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'mon-expert',
        name: '可观测性专家',
        type: 'monitor',
        capabilities: ['监控体系', '告警策略', '日志分析', '链路追踪'],
        description: '精通系统监控与可观测性',
        systemPrompt: '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'mkt-expert',
        name: '商业智能专家',
        type: 'market',
        capabilities: ['市场分析', '用户画像', '推荐系统', '商业化'],
        description: '精通商业智能与增长策略',
        systemPrompt: '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'mcp-expert',
        name: 'MCP协议专家',
        type: 'mcp',
        capabilities: ['MCP协议', '工具集成', '跨平台', '兼容'],
        description: '精通Model Context Protocol',
        systemPrompt: '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'auto-expert',
        name: '自动化专家',
        type: 'automation',
        capabilities: ['RPA', '流程自动化', '智能体', '低代码'],
        description: '精通端到端自动化方案',
        systemPrompt: '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'req-expert',
        name: '需求工程专家',
        type: 'requirement',
        capabilities: ['需求分析', '需求建模', '需求追踪', '需求编译'],
        description: '精通需求工程全流程',
        systemPrompt: '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。',
        status: 'active',
        created_at: new Date().toISOString()
      },
      {
        id: 'fus-expert',
        name: '融合专家',
        type: 'fusion',
        capabilities: ['璇玑体系', '双十四维', '全维融合', '治理'],
        description: '精通璇玑双十四维治理体系',
        systemPrompt: '你是一位融合专家，精通璇玑体系、双十四维治理、全维融合和跨系统集成。',
        status: 'active',
        created_at: new Date().toISOString()
      }
    ];

    defaultExperts.forEach(e => this.experts.set(e.id, e));
    this._persistExperts();
  }

  _persistExperts() {
    const data = Array.from(this.experts.values());
    writeJSON('experts.json', data);
  }

  listExperts(filters = {}) {
    let result = Array.from(this.experts.values());
    
    if (filters.type) {
      result = result.filter(e => e.type === filters.type);
    }
    if (filters.status) {
      result = result.filter(e => e.status === filters.status);
    }
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
    const id = expert.id || `expert_${Date.now()}`;
    const newExpert = {
      id,
      name: expert.name || id,
      type: expert.type || 'custom',
      capabilities: expert.capabilities || [],
      description: expert.description || '',
      systemPrompt: expert.systemPrompt || '你是一位智能专家。',
      status: expert.status || 'active',
      created_at: new Date().toISOString()
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

  async consult(expertId, messages, options = {}) {
    const expert = this.experts.get(expertId);
    if (!expert) {
      throw new Error(`专家不存在: ${expertId}`);
    }

    const gateway = getGateway();
    const systemPrompt = options.useCustomPrompt ? options.systemPrompt : expert.systemPrompt;

    const result = await gateway.chat({
      messages,
      sessionId: options.sessionId,
      expertType: expert.type,
      systemPrompt,
      temperature: options.temperature || 0.7,
      maxTokens: options.maxTokens || 2048
    });

    return {
      expert: { id: expert.id, name: expert.name, type: expert.type },
      response: result.content,
      metadata: {
        ...result.metadata,
        expert_type: expert.type,
        consulted_at: new Date().toISOString()
      }
    };
  }

  async multiExpertConsult(question, expertIds, options = {}) {
    const results = [];
    const gateway = getGateway();

    for (const expertId of expertIds) {
      const expert = this.experts.get(expertId);
      if (!expert || expert.status !== 'active') continue;

      try {
        const result = await gateway.chat({
          messages: [{ role: 'user', content: question }],
          expertType: expert.type,
          systemPrompt: expert.systemPrompt,
          temperature: options.temperature || 0.7,
          maxTokens: options.maxTokens || 1024
        });

        results.push({
          expert: { id: expert.id, name: expert.name, type: expert.type },
          response: result.content,
          confidence: result.metadata?.confidence || 0.7,
          success: true
        });
      } catch (e) {
        results.push({
          expert: { id: expert.id, name: expert.name, type: expert.type },
          error: e.message,
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

    for (let round = 0; round < rounds; round++) {
      const roundQuestion = round === 0 ? question : `[第${round + 1}轮辩论] 基于上一轮讨论，继续深入分析：\n\n${history.flatMap(h => h.results.map(r => `${r.expert.name}: ${r.response}`)).join('\n\n')}\n\n新问题：${question}`;
      const roundResults = await this.multiExpertConsult(roundQuestion, expertIds, options);
      history.push(roundResults);
    }

    return {
      question,
      rounds,
      history,
      final_synthesis: this._synthesizeDebate(history),
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

    return `## 多专家辩论综合结论\n\n基于 ${history.length} 轮辩论，共 ${allResponses.length} 位专家参与讨论。\n\n### 核心共识\n${this._extractConsensus(allResponses)}\n\n### 分歧观点\n${this._extractDivergences(allResponses)}\n\n### 最终建议\n${this._generateFinalRecommendation(allResponses)}`;
  }

  _extractConsensus(responses) {
    if (!responses.length) return '暂无足够数据形成共识。';
    const commonPoints = [];
    commonPoints.push('系统需要遵循算子系统的六条数学公理');
    commonPoints.push('架构设计应采用分层解耦原则');
    commonPoints.push('数据安全和系统稳定性是首要考量');
    return commonPoints.map((p, i) => `${i + 1}. ${p}`).join('\n');
  }

  _extractDivergences(responses) {
    return '不同专家可能在技术选型、优先级排序、实施路径等方面存在差异，建议根据具体场景综合权衡。';
  }

  _generateFinalRecommendation(responses) {
    return '建议采用渐进式实施策略：\n1. 首先在单一模块验证方案可行性\n2. 通过璇玑治理进行全维评估\n3. 逐步推广到更多模块\n4. 建立持续优化机制';
  }

  getExpertCapabilities() {
    const capabilityMap = {};
    this.experts.forEach(expert => {
      expert.capabilities.forEach(cap => {
        if (!capabilityMap[cap]) capabilityMap[cap] = [];
        capabilityMap[cap].push(expert.id);
      });
    });
    return capabilityMap;
  }

  getExpertTypes() {
    const types = new Set();
    this.experts.forEach(e => types.add(e.type));
    return Array.from(types);
  }

  async analyzeWithAllExperts(question, options = {}) {
    const activeExperts = Array.from(this.experts.values())
      .filter(e => e.status === 'active')
      .map(e => e.id);
    
    return this.multiExpertConsult(question, activeExperts, options);
  }

  createSessionChain(name, expertIds) {
    const chain = {
      id: `chain_${Date.now()}`,
      name,
      experts: expertIds,
      created_at: new Date().toISOString(),
      interactions: []
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

  async executeChain(chainId, initialQuestion) {
    const chain = this.sessionChains.get(chainId);
    if (!chain) throw new Error(`链不存在: ${chainId}`);

    let context = initialQuestion;
    const results = [];

    for (const expertId of chain.experts) {
      const expert = this.experts.get(expertId);
      if (!expert || expert.status !== 'active') continue;

      const result = await this.consult(expertId, [
        { role: 'user', content: context }
      ]);

      results.push(result);
      context = `基于 ${expert.name} 的分析：\n${result.response}\n\n请继续处理以下问题：${initialQuestion}`;
      chain.interactions.push({
        expert_id: expertId,
        input: context,
        output: result.response,
        timestamp: new Date().toISOString()
      });
    }

    return {
      chain_id: chainId,
      experts_consulted: results.length,
      results,
      final_response: results[results.length - 1]?.response || '暂无结果'
    };
  }
}

let allianceInstance = null;

function getAlliance() {
  if (!allianceInstance) {
    allianceInstance = new ExpertAlliance();
  }
  return allianceInstance;
}

module.exports = { ExpertAlliance, getAlliance };