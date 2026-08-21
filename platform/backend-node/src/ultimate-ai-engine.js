'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getGateway } = require('./llm-gateway');
const { getAlliance } = require('./expert-alliance');
const { getAIIntegrationEngine } = require('./ai-integration-engine');

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
    return false;
  }
}

class VectorMemoryStore {
  constructor(dimensions = 128) {
    this.dimensions = dimensions;
    this.vectors = new Map();
    this.metadata = new Map();
    this.indices = new Map();
    this._init();
  }

  _init() {
    const saved = readJSON('ultimate_vector_store.json', null);
    if (saved) {
      saved.forEach(v => {
        this.vectors.set(v.id, new Float32Array(v.vector));
        this.metadata.set(v.id, v.metadata);
      });
    }
  }

  _save() {
    if (this.vectors.size > 10000) {
      const toSave = Array.from(this.vectors.entries())
        .slice(-5000)
        .map(([id, vec]) => ({
          id,
          vector: Array.from(vec),
          metadata: this.metadata.get(id) || {}
        }));
      writeJSON('ultimate_vector_store.json', toSave);
    }
  }

  async _generateEmbedding(text, gateway) {
    if (gateway && gateway.activeProvider && typeof gateway.embed === 'function') {
      try {
        const response = await gateway.embed({
          input: text,
          model: 'text-embedding-3-small',
          dimensions: this.dimensions
        });
        if (response && response.embedding) {
          return new Float32Array(response.embedding);
        }
      } catch (e) {
      }
    }

    return this._generateLocalEmbedding(text);
  }

  _generateLocalEmbedding(text) {
    const vec = new Float32Array(this.dimensions);
    const lower = text.toLowerCase();
    const charCodes = Array.from(lower).map(c => c.charCodeAt(0));
    const freq = new Map();

    for (const code of charCodes) {
      freq.set(code, (freq.get(code) || 0) + 1);
    }

    let i = 0;
    for (const [code, count] of freq.entries()) {
      const idx = code % this.dimensions;
      vec[idx] += count / charCodes.length;
      i++;
      if (i > this.dimensions) break;
    }

    for (let j = 0; j < this.dimensions; j++) {
      const bytePattern = crypto.createHash('md5')
        .update(`${text}_${j}`)
        .digest();
      vec[j] += (bytePattern[0] / 255) * 0.3;
    }

    const norm = Math.sqrt(vec.reduce((a, b) => a + b * b, 0)) || 1;
    for (let j = 0; j < this.dimensions; j++) vec[j] /= norm;
    return vec;
  }

  cosineSimilarity(a, b) {
    let dotProduct = 0, normA = 0, normB = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    if (normA === 0 || normB === 0) return 0;
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  async store(id, content, metadata = {}) {
    const gateway = getGateway();
    const vec = await this._generateEmbedding(content, gateway);
    this.vectors.set(id, vec);
    this.metadata.set(id, {
      ...metadata,
      content,
      createdAt: new Date().toISOString(),
      dimensions: this.dimensions
    });
    this._save();
    return { id, stored: true, dimensions: this.dimensions };
  }

  async search(query, options = {}) {
    const { topK = 10, threshold = 0.3, filter = null } = options;
    const gateway = getGateway();
    const queryVec = await this._generateEmbedding(query, gateway);

    const results = [];
    for (const [id, vec] of this.vectors.entries()) {
      const meta = this.metadata.get(id);
      if (filter && meta && !this._matchFilter(meta, filter)) continue;
      const similarity = this.cosineSimilarity(queryVec, vec);
      if (similarity >= threshold) {
        results.push({
          id,
          similarity: parseFloat(similarity.toFixed(4)),
          metadata: meta
        });
      }
    }

    return results
      .sort((a, b) => b.similarity - a.similarity)
      .slice(0, topK);
  }

  _matchFilter(metadata, filter) {
    return Object.entries(filter).every(([key, value]) => metadata[key] === value);
  }

  delete(id) {
    this.vectors.delete(id);
    this.metadata.delete(id);
  }

  getStats() {
    return {
      totalVectors: this.vectors.size,
      dimensions: this.dimensions,
      metadataTypes: new Set(Array.from(this.metadata.values()).map(m => m.type || 'unknown')).size
    };
  }
}

class ReasoningEngine {
  constructor() {
    this.reasoningHistory = [];
    this.cognitiveState = {
      workingMemory: [],
      attention: new Map(),
      confidence: 1.0
    };
  }

  async multiStepReasoning(question, options = {}) {
    const gateway = getGateway();
    const {
      maxSteps = 5,
      temperature = 0.7,
      systemPrompt = null,
      context = {}
    } = options;

    const steps = [];
    let currentReasoning = question;
    let finalAnswer = null;
    let confidence = 1.0;

    const defaultSystemPrompt = `你是一个高级推理引擎，具备以下能力：
1. 逻辑推理：从前提推导结论
2. 因果分析：识别原因和结果
3. 类比推理：发现相似性和模式
4. 归纳总结：从具体到一般
5. 演绎验证：从一般到具体

请按照以下步骤进行推理：
1. 理解问题：分解核心要素
2. 分析关联：识别相关上下文
3. 推理扩展：逐步深入分析
4. 验证结论：检查一致性和合理性`;

    const system = systemPrompt || defaultSystemPrompt;

    for (let step = 1; step <= maxSteps; step++) {
      const stepPrompt = step === 1
        ? `问题：${question}\n\n请进行第一步推理，分析问题的核心要素和潜在方向。`
        : `基于之前的推理：\n${currentReasoning}\n\n请继续第${step}步推理，深入分析并逼近最终答案。`;

      try {
        if (gateway && gateway.activeProvider) {
          const response = await gateway.chat({
            messages: [
              { role: 'system', content: system },
              { role: 'user', content: stepPrompt }
            ],
            temperature: temperature,
            maxTokens: 1024
          });

          const stepResult = response.content || response;
          steps.push({
            step,
            reasoning: stepResult,
            insight: this._extractInsight(stepResult),
            confidence: this._assessStepConfidence(stepResult)
          });

          currentReasoning = `${currentReasoning}\n\n推理步骤${step}：${stepResult}`;

          if (step >= maxSteps) {
            finalAnswer = stepResult;
          }

          confidence *= steps[steps.length - 1].confidence;
        } else {
          steps.push({
            step,
            reasoning: `本地推理步骤${step}：模拟逻辑分析`,
            insight: '本地模式，无AI推理',
            confidence: 0.5
          });
          confidence *= 0.5;
        }
      } catch (e) {
        steps.push({
          step,
          reasoning: `推理错误: ${e.message}`,
          insight: '推理步骤失败',
          confidence: 0.3
        });
        confidence *= 0.3;
        break;
      }
    }

    return {
      question,
      steps,
      finalAnswer,
      overallConfidence: parseFloat(confidence.toFixed(3)),
      reasoningQuality: this._assessQuality(steps, confidence),
      tokensUsed: 0
    };
  }

  _extractInsight(reasoning) {
    const sentences = reasoning.split(/[。.!？?]/).filter(s => s.trim().length > 5);
    if (sentences.length === 0) return reasoning.slice(0, 100);
    return sentences[sentences.length - 1].trim().slice(0, 100);
  }

  _assessStepConfidence(step) {
    if (!step || step.length < 10) return 0.3;
    const signals = [];
    if (/因为|由于|所以|因此|首先|其次|最后|总之|综上/.test(step)) signals.push(0.2);
    if (/例如|比如|案例|实践|经验/.test(step)) signals.push(0.15);
    if (/结论|结果|答案|应该|建议|推荐/.test(step)) signals.push(0.15);
    if (step.length > 50) signals.push(0.1);
    return Math.min(1.0, 0.5 + signals.reduce((a, b) => a + b, 0));
  }

  _assessQuality(steps, confidence) {
    if (steps.length === 0) return 'insufficient';
    if (confidence >= 0.8 && steps.length >= 3) return 'excellent';
    if (confidence >= 0.6 && steps.length >= 2) return 'good';
    if (confidence >= 0.4) return 'adequate';
    return 'weak';
  }

  async selfReflect(reasoningResult, question, options = {}) {
    const gateway = getGateway();
    const { maxIterations = 3, targetQuality = 'good' } = options;

    let current = reasoningResult;
    let iteration = 0;

    while (iteration < maxIterations && current.reasoningQuality !== targetQuality) {
      iteration++;
      const issues = this._identifyIssues(current);

      if (issues.length === 0) break;

      try {
        if (gateway && gateway.activeProvider) {
          const reflectPrompt = `请反思以下推理过程，识别并修正其中的问题：

原问题：${question}
当前推理：${JSON.stringify(current.steps.slice(-3))}
识别的问题：${issues.join(', ')}

请提供修正后的推理。`;

          const response = await gateway.chat({
            messages: [
              { role: 'system', content: '你是一个严格的批判者和修正者。请识别推理中的问题并提供更好的推理。' },
              { role: 'user', content: reflectPrompt }
            ],
            temperature: 0.5,
            maxTokens: 1024
          });

          const reflection = response.content || response;

          current = {
            ...current,
            steps: [...current.steps, {
              step: current.steps.length + 1,
              reasoning: reflection,
              insight: `反思修正 ${iteration}`,
              confidence: 0.8
            }],
            finalAnswer: reflection,
            selfReflection: {
              iteration,
              issuesFound: issues,
              reflection: reflection.slice(0, 200)
            }
          };

          const totalConfidence = current.steps.reduce((acc, s) => acc * (s.confidence || 0.5), 1);
          current.overallConfidence = parseFloat(totalConfidence.toFixed(3));
          current.reasoningQuality = this._assessQuality(current.steps, current.overallConfidence);
        }
      } catch (e) {
        break;
      }
    }

    return current;
  }

  _identifyIssues(result) {
    const issues = [];
    if (result.overallConfidence < 0.5) issues.push('confidence_low');
    if (result.steps.length < 2) issues.push('insufficient_steps');
    if (!result.finalAnswer || result.finalAnswer.length < 20) issues.push('no_conclusion');
    return issues;
  }

  async analogicalReasoning(sourceDomain, targetDomain, question) {
    const gateway = getGateway();
    if (!gateway || !gateway.activeProvider) {
      return {
        sourceDomain,
        targetDomain,
        analogies: [],
        status: 'local_mode'
      };
    }

    const prompt = `请在"${sourceDomain}"和"${targetDomain}"之间寻找类比，以回答"${question}"：

请识别：
1. 结构相似性
2. 功能相似性
3. 过程相似性
4. 约束条件相似性

输出结构化的类比分析。`;

    const response = await gateway.chat({
      messages: [
        { role: 'system', content: '你是一个擅长跨领域类比推理的专家。' },
        { role: 'user', content: prompt }
      ],
      temperature: 0.8,
      maxTokens: 1024
    });

    return {
      sourceDomain,
      targetDomain,
      question,
      analogies: this._extractAnalogies(response.content || response),
      rawResponse: response.content || response,
      status: 'ai_powered'
    };
  }

  _extractAnalogies(text) {
    const analogies = [];
    const patterns = [
      /([^，。,\.]*?)类似([^，。,\.]*?)/g,
      /([^，。,\.]*?)好比([^，。,\.]*?)/g,
      /([^，。,\.]*?)如同([^，。,\.]*?)/g
    ];

    for (const pattern of patterns) {
      let match;
      while ((match = pattern.exec(text)) !== null && analogies.length < 5) {
        analogies.push({ source: match[1], target: match[2] });
      }
    }

    return analogies.length > 0 ? analogies : [{ source: text.slice(0, 100), target: '' }];
  }
}

class KnowledgeGraphReasoner {
  constructor() {
    this.kgCache = new Map();
    this.ruleEngine = [];
    this._init();
  }

  _init() {
    const rules = readJSON('ultimate_reasoning_rules.json', null);
    if (rules) {
      this.ruleEngine = rules;
    } else {
      this.ruleEngine = this._defaultRules();
      writeJSON('ultimate_reasoning_rules.json', this.ruleEngine);
    }
  }

  _defaultRules() {
    return [
      {
        id: 'rule_1',
        name: '传递性推理',
        pattern: { relation: 'depends_on' },
        action: 'if A depends_on B and B depends_on C then A depends_on C',
        confidence: 0.9
      },
      {
        id: 'rule_2',
        name: '组成关系推理',
        pattern: { relation: 'part_of' },
        action: 'if A is part_of B then B contains A',
        confidence: 0.95
      },
      {
        id: 'rule_3',
        name: '因果推理',
        pattern: { relation: 'causes' },
        action: 'if A causes B and B causes C then A causes C',
        confidence: 0.8
      },
      {
        id: 'rule_4',
        name: '实例推理',
        pattern: { relation: 'instance_of' },
        action: 'if A is instance_of B then A has all properties of B',
        confidence: 0.9
      },
      {
        id: 'rule_5',
        name: '同层推理',
        pattern: { relation: 'similar_to' },
        action: 'if A similar_to B then share properties with similar confidence',
        confidence: 0.7
      }
    ];
  }

  async reasonOverGraph(graph, query) {
    const nodes = graph.nodes || [];
    const edges = graph.edges || [];

    const results = {
      path: null,
      inferences: [],
      confidence: 1.0,
      query
    };

    const startNode = this._findStartNode(query, nodes);
    if (!startNode) {
      results.inferences.push({ type: 'no_start_node', message: '未找到起点节点' });
      return results;
    }

    const adjMap = new Map();
    for (const edge of edges) {
      if (!adjMap.has(edge.source)) adjMap.set(edge.source, []);
      adjMap.get(edge.source).push({ target: edge.target, relation: edge.relation || 'related', weight: edge.weight || 1 });
    }

    const visited = new Set();
    const queue = [{ node: startNode.id, path: [startNode.id], depth: 0, confidence: 1.0 }];
    const maxDepth = 5;
    const maxPaths = 10;

    while (queue.length > 0 && results.inferences.length < maxPaths) {
      const { node, path, depth, confidence: pathConf } = queue.shift();

      if (depth >= maxDepth) continue;
      if (visited.has(path.join(','))) continue;
      visited.add(path.join(','));

      const neighbors = adjMap.get(node) || [];
      for (const neighbor of neighbors) {
        const newPath = [...path, neighbor.target];
        const newConfidence = pathConf * (neighbor.weight || 0.5);

        const inferred = this._applyRules(nodes, newPath, neighbor.relation);
        if (inferred) {
          results.inferences.push({
            path: newPath,
            relation: neighbor.relation,
            confidence: parseFloat(newConfidence.toFixed(3)),
            inference: inferred
          });
        }

        queue.push({
          node: neighbor.target,
          path: newPath,
          depth: depth + 1,
          confidence: newConfidence
        });
      }
    }

    results.inferences.sort((a, b) => b.confidence - a.confidence);
    if (results.inferences.length > 0) {
      results.path = results.inferences[0].path;
      results.confidence = results.inferences[0].confidence;
    }

    return results;
  }

  _findStartNode(query, nodes) {
    const lower = query.toLowerCase();
    for (const node of nodes) {
      if (node.id && lower.includes(node.id.toLowerCase())) return node;
      if (node.label && lower.includes(node.label.toLowerCase())) return node;
      if (node.name && lower.includes(node.name.toLowerCase())) return node;
    }
    return nodes[0] || null;
  }

  _applyRules(nodes, path, relation) {
    for (const rule of this.ruleEngine) {
      if (rule.pattern.relation === relation) {
        return {
          rule: rule.name,
          action: rule.action,
          ruleConfidence: rule.confidence
        };
      }
    }
    return { rule: 'default', action: `沿${relation}关系传播`, ruleConfidence: 0.5 };
  }

  addRule(rule) {
    this.ruleEngine.push({
      id: `rule_${this.ruleEngine.length + 1}`,
      ...rule
    });
    writeJSON('ultimate_reasoning_rules.json', this.ruleEngine);
  }

  getStats() {
    return {
      rulesCount: this.ruleEngine.length,
      cacheSize: this.kgCache.size
    };
  }
}

class PerformanceOptimizer {
  constructor() {
    this.metrics = [];
    this.circuitBreaker = {
      failures: 0,
      threshold: 5,
      resetTimeout: 60000,
      lastFailure: 0,
      halfOpen: false
    };
    this.adaptiveCache = new Map();
  }

  _checkCircuitBreaker() {
    const now = Date.now();
    if (this.circuitBreaker.halfOpen) {
      if (now - this.circuitBreaker.lastFailure > this.circuitBreaker.resetTimeout) {
        this.circuitBreaker.halfOpen = false;
        return 'closed';
      }
      return 'half-open';
    }

    if (this.circuitBreaker.failures >= this.circuitBreaker.threshold) {
      if (now - this.circuitBreaker.lastFailure > this.circuitBreaker.resetTimeout) {
        this.circuitBreaker.halfOpen = true;
        return 'half-open';
      }
      return 'open';
    }

    return 'closed';
  }

  _recordSuccess() {
    this.circuitBreaker.failures = 0;
    this.circuitBreaker.halfOpen = false;
  }

  _recordFailure() {
    this.circuitBreaker.failures++;
    this.circuitBreaker.lastFailure = Date.now();
  }

  getCircuitStatus() {
    return {
      state: this._checkCircuitBreaker(),
      failures: this.circuitBreaker.failures,
      threshold: this.circuitBreaker.threshold,
      lastFailure: this.circuitBreaker.lastFailure
    };
  }

  async withOptimization(fn, options = {}) {
    const {
      cacheKey = null,
      ttl = 300000,
      timeout = 30000
    } = options;

    const breakerState = this._checkCircuitBreaker();
    if (breakerState === 'open') {
      throw new Error('Circuit breaker is open, request blocked');
    }

    if (cacheKey) {
      const cached = this.adaptiveCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < ttl) {
        return cached.data;
      }
    }

    const startTime = Date.now();
    try {
      const result = await Promise.race([
        fn(),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Timeout exceeded')), timeout)
        )
      ]);

      if (cacheKey) {
        this.adaptiveCache.set(cacheKey, {
          data: result,
          timestamp: Date.now()
        });
      }

      this._recordSuccess();
      this.metrics.push({
        success: true,
        duration: Date.now() - startTime,
        timestamp: Date.now()
      });
      return result;
    } catch (e) {
      this._recordFailure();
      this.metrics.push({
        success: false,
        duration: Date.now() - startTime,
        error: e.message,
        timestamp: Date.now()
      });
      throw e;
    }
  }

  optimizePrompt(prompt, target = 'concise') {
    const optimizations = {
      concise: this._conciseOptimize,
      detailed: this._detailedOptimize,
      creative: this._creativeOptimize,
      analytical: this._analyticalOptimize
    };
    const optimizer = optimizations[target] || optimizations.concise;
    return optimizer(prompt);
  }

  _conciseOptimize(prompt) {
    return prompt
      .replace(/\s+/g, ' ')
      .replace(/请/g, '')
      .replace(/你是/g, '系统指令：')
      .trim();
  }

  _detailedOptimize(prompt) {
    return `请详细分析以下问题，并提供：\n1. 背景分析\n2. 关键要素\n3. 解决方案\n4. 实施建议\n\n问题：${prompt}`;
  }

  _creativeOptimize(prompt) {
    return `请以创新性思维处理以下问题：\n- 探索非常规解法\n- 考虑多种可能性\n- 寻找突破点\n\n问题：${prompt}`;
  }

  _analyticalOptimize(prompt) {
    return `请以分析思维处理以下问题：\n1. 分解问题结构\n2. 识别关键变量\n3. 建立因果关系\n4. 量化影响\n\n问题：${prompt}`;
  }

  getPerformanceReport() {
    const recent = this.metrics.slice(-100);
    const total = recent.length;
    const successes = recent.filter(m => m.success).length;
    const avgDuration = total > 0
      ? Math.round(recent.reduce((sum, m) => sum + m.duration, 0) / total)
      : 0;

    return {
      totalRequests: total,
      successRate: total > 0 ? parseFloat((successes / total).toFixed(3)) : 0,
      avgDurationMs: avgDuration,
      circuitBreaker: this.getCircuitStatus(),
      cacheSize: this.adaptiveCache.size,
      cacheHitRate: this._calculateCacheHitRate()
    };
  }

  _calculateCacheHitRate() {
    if (this.adaptiveCache.size === 0) return 0;
    const now = Date.now();
    const valid = Array.from(this.adaptiveCache.values())
      .filter(c => now - c.timestamp < 300000);
    return parseFloat((valid.length / this.adaptiveCache.size).toFixed(3));
  }
}

class UltimateAIEngine {
  constructor() {
    this.vectorStore = new VectorMemoryStore(128);
    this.reasoningEngine = new ReasoningEngine();
    this.graphReasoner = new KnowledgeGraphReasoner();
    this.optimizer = new PerformanceOptimizer();
    this.baseEngine = getAIIntegrationEngine();
    this.gateway = getGateway();
    this.processingHistory = [];
    this._init();
  }

  _init() {
    const history = readJSON('ultimate_processing_history.json', []);
    this.processingHistory = history.slice(-200);
  }

  async processWithDeepIntelligence(question, options = {}) {
    const startTime = Date.now();
    const mode = options.mode || 'intelligent';
    const context = options.context || {};
    const graphData = options.graph || null;

    const result = {
      question,
      mode,
      processingLayers: [],
      finalAnswer: null,
      deepAnalysis: null,
      optimization: null
    };

    try {
      const intelligentResult = await this.optimizer.withOptimization(
        () => this.baseEngine.intelligentProcess(question, { mode: 'auto', context, graphData }),
        { cacheKey: `intelligent:${question.slice(0, 50)}:${Date.now() / 300000 | 0}` }
      );
      result.processingLayers.push({
        layer: 'base_intelligence',
        result: intelligentResult,
        status: 'completed'
      });
    } catch (e) {
      result.processingLayers.push({
        layer: 'base_intelligence',
        error: e.message,
        status: 'failed'
      });
    }

    try {
      const deepReasoning = await this.optimizer.withOptimization(
        () => this.reasoningEngine.multiStepReasoning(question, {
          maxSteps: 4,
          temperature: 0.7,
          context
        }),
        { cacheKey: `reasoning:${question.slice(0, 50)}:${Date.now() / 300000 | 0}` }
      );

      const reflected = await this.reasoningEngine.selfReflect(deepReasoning, question, {
        maxIterations: 2,
        targetQuality: 'good'
      });

      result.processingLayers.push({
        layer: 'deep_reasoning',
        result: reflected,
        status: 'completed',
        quality: reflected.reasoningQuality,
        confidence: reflected.overallConfidence
      });

      result.deepAnalysis = {
        reasoningSteps: reflected.steps.length,
        confidence: reflected.overallConfidence,
        quality: reflected.reasoningQuality,
        finalReasoning: reflected.finalAnswer
      };
    } catch (e) {
      result.processingLayers.push({
        layer: 'deep_reasoning',
        error: e.message,
        status: 'failed'
      });
    }

    if (graphData && graphData.nodes?.length > 0) {
      try {
        const graphReasoning = await this.graphReasoner.reasonOverGraph(graphData, question);
        result.processingLayers.push({
          layer: 'graph_reasoning',
          result: graphReasoning,
          status: 'completed'
        });
      } catch (e) {
        result.processingLayers.push({
          layer: 'graph_reasoning',
          error: e.message,
          status: 'failed'
        });
      }
    }

    try {
      const memories = await this.vectorStore.search(question, { topK: 5, threshold: 0.3 });
      result.processingLayers.push({
        layer: 'memory_recall',
        results: memories,
        status: 'completed'
      });
    } catch (e) {
      result.processingLayers.push({
        layer: 'memory_recall',
        error: e.message,
        status: 'failed'
      });
    }

    try {
      const synthesisPrompt = this._buildUltimateSynthesisPrompt(result, question);

      if (this.gateway && this.gateway.activeProvider) {
        const response = await this.gateway.chat({
          messages: [
            { role: 'system', content: '你是终极AI引擎，具备深度推理、记忆和分析能力。请综合所有处理层的结果，给出最终的高质量回答。' },
            { role: 'user', content: synthesisPrompt }
          ],
          temperature: 0.7,
          maxTokens: 4096
        });
        result.finalAnswer = response.content || response;
      } else {
        result.finalAnswer = this._fallbackSynthesis(result);
      }
    } catch (e) {
      result.finalAnswer = this._fallbackSynthesis(result);
      result.processingLayers.push({
        layer: 'synthesis',
        error: e.message,
        status: 'failed'
      });
    }

    try {
      await this.vectorStore.store(
        `qa_${Date.now()}`,
        question,
        { type: 'question', timestamp: new Date().toISOString() }
      );
      await this.vectorStore.store(
        `ans_${Date.now()}`,
        result.finalAnswer || '',
        { type: 'answer', timestamp: new Date().toISOString(), question }
      );
    } catch (e) {}

    result.optimization = this.optimizer.getPerformanceReport();
    result.processingTimeMs = Date.now() - startTime;
    result.success = result.processingLayers.filter(l => l.status === 'completed').length > 0;

    this.processingHistory.push({
      question,
      timeMs: result.processingTimeMs,
      success: result.success,
      layers: result.processingLayers.length,
      timestamp: new Date().toISOString()
    });

    if (this.processingHistory.length > 200) {
      writeJSON('ultimate_processing_history.json', this.processingHistory.slice(-200));
    }

    return result;
  }

  _buildUltimateSynthesisPrompt(result, question) {
    const parts = [`用户问题：${question}\n`];

    for (const layer of result.processingLayers) {
      if (layer.status !== 'completed') continue;

      if (layer.layer === 'base_intelligence' && layer.result) {
        parts.push(`【基础智能处理】\n步骤数: ${layer.result.steps?.length || 0}\n初步答案: ${(layer.result.finalAnswer || '').slice(0, 200)}\n`);
      }

      if (layer.layer === 'deep_reasoning' && layer.result) {
        parts.push(`【深度推理】\n推理质量: ${layer.quality}\n置信度: ${layer.confidence}\n最终推理: ${(layer.result.finalAnswer || '').slice(0, 300)}\n`);
      }

      if (layer.layer === 'graph_reasoning' && layer.result) {
        parts.push(`【图谱推理】\n推理路径: ${layer.result.path?.length || 0} 步\n推理数量: ${layer.result.inferences?.length || 0}\n`);
      }

      if (layer.layer === 'memory_recall' && layer.results) {
        parts.push(`【记忆召回】\n相关记忆: ${layer.results.length} 条\n`);
      }
    }

    parts.push(`\n请综合以上所有分析层次，提供最终的、高质量的回答。
要求：
1. 整合所有分析结果，避免重复
2. 标注信息来源和置信度
3. 给出可执行的建议
4. 回答要全面、深入、有洞察力`);

    return parts.join('\n');
  }

  _fallbackSynthesis(result) {
    const parts = [];
    for (const layer of result.processingLayers) {
      if (layer.status === 'completed' && layer.result?.finalAnswer) {
        parts.push(layer.result.finalAnswer);
      }
    }
    return parts.length > 0
      ? `基于多层分析的综合结果（本地模式）。\n\n${parts.join('\n\n')}`
      : '分析完成（本地模式，无AI辅助）。';
  }

  async storeKnowledge(id, content, metadata = {}) {
    return this.vectorStore.store(id, content, metadata);
  }

  async searchKnowledge(query, options = {}) {
    return this.vectorStore.search(query, options);
  }

  async reasonByAnalogy(sourceDomain, targetDomain, question) {
    return this.reasoningEngine.analogicalReasoning(sourceDomain, targetDomain, question);
  }

  async addReasoningRule(rule) {
    return this.graphReasoner.addRule(rule);
  }

  getUltimateStats() {
    return {
      engine: 'ultimate_ai_engine_v2',
      version: '2.0.0',
      vectorStore: this.vectorStore.getStats(),
      graphReasoner: this.graphReasoner.getStats(),
      performance: this.optimizer.getPerformanceReport(),
      processingHistory: {
        total: this.processingHistory.length,
        recent: this.processingHistory.slice(-5)
      },
      integrations: {
        baseEngine: 'active',
        reasoningEngine: 'active',
        graphReasoner: 'active',
        optimizer: 'active',
        vectorStore: 'active'
      }
    };
  }

  async performFullUltimateAnalysis(question, options = {}) {
    const result = await this.processWithDeepIntelligence(question, options);

    const reasoningOnly = await this.reasoningEngine.multiStepReasoning(question, {
      maxSteps: 3,
      temperature: 0.5
    });

    const memories = await this.vectorStore.search(question, { topK: 5 });

    return {
      ...result,
      alternateReasoning: reasoningOnly,
      relevantMemories: memories,
      ultimateAnalysis: true
    };
  }
}

let ultimateInstance = null;

function getUltimateEngine() {
  if (!ultimateInstance) {
    ultimateInstance = new UltimateAIEngine();
  }
  return ultimateInstance;
}

module.exports = {
  VectorMemoryStore,
  ReasoningEngine,
  KnowledgeGraphReasoner,
  PerformanceOptimizer,
  UltimateAIEngine,
  getUltimateEngine
};
