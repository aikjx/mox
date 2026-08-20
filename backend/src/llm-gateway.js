'use strict';

const fs = require('fs');
const path = require('path');

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
    console.error('[llm-gateway] writeJSON', file, e.message);
    return false;
  }
}

class LLMGateway {
  constructor() {
    this.providers = {};
    this.activeProvider = null;
    this.conversations = new Map();
    this._init();
  }

  _init() {
    const config = readJSON('llm_config.json', []);
    if (Array.isArray(config) && config.length) {
      config.forEach((p) => {
        this.providers[p.id || p.provider] = p;
        if (p.enabled && !this.activeProvider) {
          this.activeProvider = p.id || p.provider;
        }
      });
    }
    if (!this.activeProvider && Object.keys(this.providers).length) {
      this.activeProvider = Object.keys(this.providers)[0];
    }
  }

  async chat(params) {
    const { messages, sessionId, expertType, systemPrompt, temperature = 0.7, maxTokens = 2048 } = params;

    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
    
    const enhancedMessages = [];
    
    if (systemPrompt) {
      enhancedMessages.push({ role: 'system', content: systemPrompt });
    } else if (expertType) {
      const expertPrompt = this._getExpertSystemPrompt(expertType);
      enhancedMessages.push({ role: 'system', content: expertPrompt });
    }

    const convHistory = sessionId ? this.conversations.get(sessionId) || [] : [];
    const allMessages = [...enhancedMessages, ...convHistory, ...messages];

    if (provider && provider.enabled && provider.provider !== 'local') {
      return await this._callExternalProvider(provider, allMessages, temperature, maxTokens);
    }

    return this._generateIntelligentResponse(messages, expertType, convHistory);
  }

  _getExpertSystemPrompt(expertType) {
    const prompts = {
      algorithm: '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。请以专业、精准的方式回答。',
      architecture: '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。请提供清晰、可落地的架构建议。',
      data: '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。请给出专业的数据方案。',
      ai: '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。请提供前沿且实用的AI建议。',
      workflow: '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。请设计高效的工作流方案。',
      operator: '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。请提供符合算子系统数学公理的方案。',
      graph: '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。请提供专业的图谱方案。',
      security: '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。请给出全面的安全建议。',
      performance: '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。请提供量化的性能方案。',
      monitor: '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。请设计完善的监控方案。',
      market: '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。请提供商业洞察。',
      mcp: '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。请提供标准的MCP方案。',
      automation: '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。请提供端到端自动化方案。',
      requirement: '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。请提供结构化的需求方案。',
      fusion: '你是一位融合专家，精通璇玑体系、双十四维治理、全维融合和跨系统集成。请提供全维融合方案。',
      default: '你是一位智能助手，可以帮助进行系统分析、代码实现、架构设计和问题解决。请以专业、精准的方式回答。'
    };
    return prompts[expertType] || prompts.default;
  }

  async _callExternalProvider(provider, messages, temperature, maxTokens) {
    const url = provider.base_url || 'https://api.openai.com/v1';
    const model = provider.model || 'gpt-4';
    const apiKey = provider.api_key;

    const payload = {
      model,
      messages,
      temperature,
      max_tokens: maxTokens
    };

    try {
      const response = await fetch(`${url}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify(payload)
      });

      if (!response.ok) {
        throw new Error(`LLM API error: ${response.status}`);
      }

      const data = await response.json();
      return {
        content: data.choices[0].message.content,
        usage: data.usage,
        model: data.model,
        provider: provider.id
      };
    } catch (error) {
      console.warn('[llm-gateway] External provider failed, falling back to local:', error.message);
      return this._generateIntelligentResponse(messages, null, []);
    }
  }

  _generateIntelligentResponse(messages, expertType, history) {
    const lastMsg = messages.filter(m => m.role === 'user').pop()?.content || '';
    const context = [...history, ...messages];
    
    const expertInsights = this._getExpertInsights(expertType);
    const intentAnalysis = this._analyzeIntent(lastMsg);
    const relatedOperators = this._findRelatedOperators(lastMsg);
    
    const response = {
      content: this._composeResponse(lastMsg, expertType, intentAnalysis, expertInsights, relatedOperators),
      usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
      model: 'ous-internal-v3',
      provider: 'local-intelligent',
      metadata: {
        intent: intentAnalysis.primary,
        confidence: intentAnalysis.confidence,
        expert_type: expertType || 'general',
        related_operators: relatedOperators
      }
    };

    return response;
  }

  _analyzeIntent(text) {
    const lower = text.toLowerCase();
    const intents = {
      operator_recommendation: /推荐|算子|operator|建议|用什么/.test(lower),
      algorithm_analysis: /算法|复杂度|时间|空间|algorithm|complexity/.test(lower),
      graph_analysis: /图谱|节点|边|中心性|社区|pagerank|graph|node/.test(lower),
      workflow: /工作流|编排|流程|pipeline|workflow|flow/.test(lower),
      automation: /自动化|automation|自动|机器人/.test(lower),
      requirement: /需求|编译|草莓|caomei|requirement/.test(lower),
      browser: /浏览器|页面|爬取|抓取|browser|web/.test(lower),
      market: /商城|市场|下载|购买|market/.test(lower),
      mcp: /mcp|兼容|协议|protocol/.test(lower),
      monitor: /监控|日志|指标|报警|monitor|metric/.test(lower),
      security: /安全|权限|审计|security|permission/.test(lower),
      performance: /性能|优化|加速|快|performance|optimize/.test(lower),
      fusion: /融合|璇玑|全维|治理|fusion|xuanji/.test(lower),
      ai_chat: /你好|hello|hi|介绍|说明/.test(lower)
    };

    const matched = Object.entries(intents)
      .filter(([, v]) => v)
      .map(([k]) => k);

    if (matched.length === 0) {
      return { primary: 'general', confidence: 0.6 };
    }

    return { primary: matched[0], secondary: matched.slice(1), confidence: 0.85 };
  }

  _getExpertInsights(expertType) {
    const insights = {
      algorithm: ['时间复杂度分析', '空间复杂度评估', '边界条件处理', '优化建议'],
      architecture: ['分层架构设计', '服务拆分策略', '数据流向分析', '扩展性评估'],
      data: ['数据模型设计', '数据质量规则', '数据血缘追踪', '数据安全策略'],
      ai: ['模型选型建议', '训练策略', '推理优化', 'AI治理框架'],
      workflow: ['流程建模', '节点编排', '异常处理', '性能调优'],
      operator: ['算子抽象层次', '组合算子设计', '守恒律验证', '状态向量管理'],
      graph: ['图构建策略', '实体关系抽取', '图算法选型', '图谱应用场景'],
      security: ['威胁建模', '防护策略', '审计日志', '合规检查'],
      performance: ['瓶颈分析', '优化方案', '基准测试', '容量规划'],
      monitor: ['监控指标', '告警规则', '日志采集', '链路追踪'],
      market: ['市场分析', '竞品对比', '定价策略', '增长模型'],
      mcp: ['工具协议', '能力描述', '会话管理', '错误处理'],
      automation: ['自动化策略', '触发条件', '执行监控', '异常恢复'],
      requirement: ['需求拆解', '优先级排序', '验收标准', '追踪矩阵'],
      fusion: ['璇玑配置', '维度权重', '融合策略', '治理闸门']
    };
    return insights[expertType] || insights.default || ['系统分析', '方案设计', '实施建议'];
  }

  _findRelatedOperators(text) {
    const lower = text.toLowerCase();
    const operators = [
      { id: 'normalize', name: 'L2归一化', tags: ['归一化', 'normalize', '向量'] },
      { id: 'relu', name: 'ReLU激活', tags: ['激活', 'relu', '非线性'] },
      { id: 'sigmoid', name: 'Sigmoid', tags: ['压缩', 'sigmoid', '概率'] },
      { id: 'softmax', name: 'Softmax', tags: ['概率', 'softmax', '分类'] },
      { id: 'linear', name: '线性变换', tags: ['线性', '缩放', '变换'] },
      { id: 'pagerank', name: 'PageRank', tags: ['排名', 'pagerank', '影响力'] },
      { id: 'label_propagation', name: '标签传播', tags: ['社区', '传播', '聚类'] },
      { id: 'bfs', name: '广度优先搜索', tags: ['路径', '搜索', 'bfs'] },
      { id: 'activate', name: '激活传播', tags: ['传播', '能量', '激活'] },
      { id: 'degree_centrality', name: '度中心性', tags: ['中心性', '度', 'degree'] }
    ];

    return operators.filter(op => 
      op.tags.some(tag => lower.includes(tag.toLowerCase()))
    ).slice(0, 3);
  }

  _composeResponse(userText, expertType, intentAnalysis, insights, relatedOperators) {
    const text = userText.trim();
    const expertLabel = expertType ? {
      algorithm: '算法专家',
      architecture: '架构专家',
      data: '数据专家',
      ai: 'AI专家',
      workflow: '工作流专家',
      operator: '算子系统专家',
      graph: '知识图谱专家',
      security: '安全专家',
      performance: '性能优化专家',
      monitor: '可观测性专家',
      market: '商业智能专家',
      mcp: 'MCP协议专家',
      automation: '自动化专家',
      requirement: '需求工程专家',
      fusion: '融合专家'
    }[expertType] : '智能助手';

    const greetingPatterns = [/你好/, /hello/i, /hi/i, /在吗/, /介绍/];
    if (greetingPatterns.some(p => p.test(text))) {
      return `你好！我是${expertLabel}。\n\n我可以帮你：\n${this._getCapabilitiesList(expertType)}\n\n请告诉我你的具体需求，我会以专业的方式为你解答。`;
    }

    const parts = [];
    
    parts.push(`## ${expertLabel}分析\n\n针对你的问题"${text.slice(0, 50)}${text.length > 50 ? '...' : ''}"，我的分析如下：\n`);

    if (intentAnalysis.primary && intentAnalysis.primary !== 'general') {
      parts.push(`**识别意图**: ${this._translateIntent(intentAnalysis.primary)}（置信度 ${Math.round(intentAnalysis.confidence * 100)}%）\n`);
    }

    parts.push(`### 核心分析\n`);
    parts.push(this._generateExpertAnalysis(text, expertType, intentAnalysis));

    if (insights && insights.length) {
      parts.push(`\n### 专家洞察\n`);
      insights.slice(0, 4).forEach((insight, i) => {
        parts.push(`${i + 1}. **${insight}**: ${this._expandInsight(insight, expertType)}`);
      });
    }

    if (relatedOperators && relatedOperators.length) {
      parts.push(`\n### 推荐算子\n`);
      relatedOperators.forEach(op => {
        parts.push(`- \`${op.id}\` (${op.name}) — 可用于解决此类问题`);
      });
    }

    parts.push(`\n### 下一步建议\n`);
    parts.push(this._getNextSteps(text, expertType));

    return parts.join('\n');
  }

  _getCapabilitiesList(expertType) {
    const caps = {
      algorithm: ['算法复杂度分析', '优化方案设计', '代码实现建议', '边界条件检查'],
      architecture: ['系统架构设计', '微服务拆分', '技术选型建议', '扩展性评估'],
      data: ['数据模型设计', '数据治理方案', 'ETL流程设计', '数据安全策略'],
      ai: ['AI模型选型', '训练策略建议', '推理优化方案', 'AI应用设计'],
      workflow: ['工作流建模', '节点编排建议', '异常处理设计', '性能优化'],
      operator: ['算子抽象设计', '组合算子开发', '守恒律验证', '状态向量管理'],
      graph: ['图谱构建方案', '实体关系抽取', '图算法选型', '应用场景设计'],
      security: ['威胁建模', '安全防护策略', '审计日志设计', '合规检查方案'],
      performance: ['性能瓶颈分析', '优化方案设计', '基准测试建议', '容量规划'],
      monitor: ['监控体系设计', '告警规则配置', '日志采集方案', '链路追踪设计'],
      market: ['市场趋势分析', '竞品对比', '定价策略建议', '增长模型设计'],
      mcp: ['MCP工具设计', '协议兼容方案', '能力描述规范', '错误处理设计'],
      automation: ['自动化策略设计', '触发条件配置', '执行监控方案', '异常恢复设计'],
      requirement: ['需求拆解分析', '优先级排序', '验收标准定义', '追踪矩阵设计'],
      fusion: ['璇玑配置方案', '维度权重设计', '融合策略制定', '治理闸门配置']
    };
    return (caps[expertType] || caps.default || ['系统分析', '方案设计', '实施建议'])
      .map(c => `- ${c}`).join('\n');
  }

  _translateIntent(intent) {
    const map = {
      operator_recommendation: '算子推荐',
      algorithm_analysis: '算法分析',
      graph_analysis: '图谱分析',
      workflow: '工作流',
      automation: '自动化',
      requirement: '需求编译',
      browser: '浏览器自动化',
      market: '算子商城',
      mcp: 'MCP兼容',
      monitor: '系统监控',
      security: '安全审计',
      performance: '性能优化',
      fusion: '全维融合',
      ai_chat: 'AI对话',
      general: '通用咨询'
    };
    return map[intent] || intent;
  }

  _generateExpertAnalysis(text, expertType, intent) {
    const analyses = {
      operator_recommendation: '这是一个算子推荐场景。我会根据你的需求特征（输入类型、输出目标、性能要求），从算子库中筛选最合适的算子组合。关键考量维度包括：算子类型匹配度、参数适配性、守恒律兼容性。',
      algorithm_analysis: '算法分析需要从时间复杂度和空间复杂度两个维度进行评估。我建议采用大O表示法进行标准化度量，并考虑实际运行环境的硬件约束。对于大规模数据场景，还需要评估常数因子和缓存友好性。',
      graph_analysis: '知识图谱分析涉及图结构的多个维度：节点影响力（PageRank）、社区结构（标签传播）、关键路径（BFS最短路径）、节点中心性（度/中介中心性）。我建议根据具体业务场景选择合适的分析方法。',
      workflow: '工作流编排需要考虑：节点依赖关系、并行执行路径、异常处理机制、状态回滚策略。我建议采用有向无环图（DAG）进行流程建模，并为每个节点定义明确的输入输出契约。',
      general: '基于你的问题，我会从以下维度进行分析：问题理解、方案设计、实施路径、风险评估。让我逐步为你展开。'
    };
    return analyses[intent.primary] || analyses.general;
  }

  _expandInsight(insight, expertType) {
    const expansions = {
      '时间复杂度分析': '使用大O表示法，关注最坏情况和平均情况的差异，考虑数据规模增长对性能的影响',
      '空间复杂度评估': '分析内存使用模式，评估缓存友好性，考虑空间换时间的可行性',
      '分层架构设计': '采用关注点分离原则，将系统划分为表现层、业务层、数据层、基础设施层',
      '数据模型设计': '遵循第三范式，同时考虑查询性能进行适度反范式化设计',
      '算子抽象层次': '定义清晰的算子接口，支持组合和复用，确保类型安全',
      '流程建模': '使用BPMN 2.0标准建模，明确网关类型（排他/并行/包容）',
      '图谱构建策略': '采用增量构建方式，定义实体和关系的语义类型',
      '威胁建模': '使用STRIDE模型识别威胁，定义攻击面和防护措施',
      '瓶颈分析': '通过性能剖析定位热点代码，使用火焰图可视化性能分布',
      '监控指标': '定义黄金指标（延迟/流量/错误/饱和度），设计分层监控体系',
      'MCP工具设计': '遵循Model Context Protocol规范，定义工具能力描述',
      '自动化策略': '基于事件驱动设计自动化触发器，定义执行条件和后置动作',
      '需求拆解': '将高层需求逐层分解为可执行的用户故事，定义验收标准',
      '璇玑配置': '根据业务特性配置双十四维权重，定义治理闸门阈值'
    };
    return expansions[insight] || '需要根据具体场景进行深入分析和定制化设计';
  }

  _getNextSteps(text, expertType) {
    const steps = {
      operator: [
        '1. 明确输入数据类型和维度',
        '2. 选择算子基类（FunctionOperator/LinearOperator）',
        '3. 定义算子元数据（输入/输出类型、参数）',
        '4. 实现算子逻辑并注册到算子中心',
        '5. 在工作流编排中测试算子'
      ],
      workflow: [
        '1. 使用流程图编辑器设计节点和连线',
        '2. 为每个节点选择合适的算子',
        '3. 配置节点参数和条件分支',
        '4. 验证流程并执行测试',
        '5. 在璇玑治理中进行全维优化'
      ],
      graph: [
        '1. 定义节点类型和关系类型',
        '2. 添加初始节点和边',
        '3. 运行图谱分析算法（PageRank/社区发现）',
        '4. 查看可视化结果',
        '5. 在图谱管理中进行持续维护'
      ],
      default: [
        '1. 明确需求边界和目标',
        '2. 选择相关模块开始实施',
        '3. 在AI助手对话框中获取更多帮助',
        '4. 查看系统文档了解详细用法',
        '5. 联系专家获得一对一咨询'
      ]
    };
    return (steps[expertType] || steps.default).join('\n');
  }

  listProviders() {
    return Object.entries(this.providers).map(([id, p]) => ({
      id,
      name: p.name || id,
      type: p.provider,
      enabled: p.enabled,
      active: id === this.activeProvider,
      model: p.model
    }));
  }

  setActiveProvider(providerId) {
    if (this.providers[providerId]) {
      this.activeProvider = providerId;
      return true;
    }
    return false;
  }

  addProvider(provider) {
    const id = provider.id || `llm_${Date.now()}`;
    this.providers[id] = {
      id,
      provider: provider.provider || 'custom',
      base_url: provider.base_url || '',
      model: provider.model || 'default',
      api_key: provider.api_key || '',
      enabled: provider.enabled || false,
      name: provider.name
    };
    
    const config = Object.values(this.providers);
    writeJSON('llm_config.json', config);
    
    if (!this.activeProvider && provider.enabled) {
      this.activeProvider = id;
    }
    return id;
  }

  removeProvider(providerId) {
    if (this.providers[providerId]) {
      delete this.providers[providerId];
      const config = Object.values(this.providers);
      writeJSON('llm_config.json', config);
      if (this.activeProvider === providerId) {
        this.activeProvider = Object.keys(this.providers)[0] || null;
      }
      return true;
    }
    return false;
  }
}

let gatewayInstance = null;

function getGateway() {
  if (!gatewayInstance) {
    gatewayInstance = new LLMGateway();
  }
  return gatewayInstance;
}

module.exports = { LLMGateway, getGateway };