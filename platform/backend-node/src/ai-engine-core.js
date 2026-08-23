'use strict';

/**
 * AI 引擎统一编排核心（AI Engine Core）
 * ========================================
 * 归一化设计依据：docs/modules/ai-engine-master-analysis.md
 *
 * 三层收口：
 *   输入收口 —— 统一 EngineRequest { question, capability?, options? }
 *   过程收口 —— 五步流水线：意图识别 → 能力路由 → 引擎执行 → 质量校验 → 指标反馈
 *   输出收口 —— 统一 EngineResponse { capability, intent, engine, result, quality, latency_ms }
 *
 * 四条不变式：
 *   1) 只编排不重造：不实现任何领域算法，全部委托既有引擎（算法单源）
 *   2) 降级单向：能力执行失败 → 降级 chat 能力，绝不让请求空手而归
 *   3) 指标必达：每次调用必产生一条指标记录（成功与失败同等记录）
 *   4) 显式覆盖：capability 显式指定时跳过意图识别（机器调用可预测）
 */

const fs = require('fs');
const path = require('path');
const { getGateway } = require('./llm-gateway');
const { getAIEngine } = require('./ai-engine');
const { getUltimateEngine } = require('./ultimate-ai-engine');
const { getAllianceEngine } = require('./expert-alliance-engine');
const { getAIFlowGraph } = require('./ai-flow-graph');
// [C3 单一真源] 意图识别 domain 层：ai-engine-core 不做独立重复实现
const { detectIntent: _domainDetectIntent } = require('./expert-alliance/domain/intent-classifier');

const DATA_DIR = path.join(__dirname, '..', 'data');
const METRICS_FILE = 'engine_core_metrics.json';

// ==================== 意图 → 能力矩阵（可解释关键词加权） ====================
const INTENT_KEYWORDS = {
  expert: [
    { kw: '专家', w: 3 }, { kw: '会诊', w: 3 }, { kw: '联盟', w: 2 }, { kw: '多专家', w: 3 },
    { kw: '协作会商', w: 3 }, { kw: '团队讨论', w: 2 }, { kw: '组队', w: 2 }
  ],
  reasoning: [
    { kw: '推理', w: 3 }, { kw: '逐步分析', w: 3 }, { kw: '为什么', w: 2 }, { kw: '论证', w: 3 },
    { kw: '深度思考', w: 3 }, { kw: '原因', w: 1 }, { kw: '原理', w: 2 }, { kw: '推演', w: 3 }
  ],
  memory: [
    { kw: '记住', w: 3 }, { kw: '回忆', w: 3 }, { kw: '之前说过', w: 3 }, { kw: '知识检索', w: 3 },
    { kw: '存量知识', w: 3 }, { kw: '之前讨论', w: 2 }, { kw: '历史知识', w: 2 }
  ],
  graph: [
    { kw: '图谱', w: 3 }, { kw: '节点关系', w: 3 }, { kw: 'pagerank', w: 3 }, { kw: 'PageRank', w: 3 },
    { kw: '中心性', w: 3 }, { kw: '社区结构', w: 3 }, { kw: '知识结构分析', w: 2 }, { kw: '关系网络', w: 2 }
  ],
  workflow: [
    { kw: '工作流', w: 3 }, { kw: '流程编排', w: 3 }, { kw: '依次执行', w: 3 }, { kw: '步骤执行', w: 2 },
    { kw: '流水线执行', w: 3 }, { kw: '编排任务', w: 2 }
  ]
  // chat 为默认兜底，无关键词表
};

const CAPABILITY_META = {
  expert: { engine: 'expert-alliance-engine', description: '专家联盟协作（意图→组队→辩论→综合→质量闸门）' },
  reasoning: { engine: 'ultimate-ai-engine', description: '深度推理（多步推理+自我反思+类比）' },
  memory: { engine: 'ultimate-ai-engine', description: '向量知识记忆检索' },
  graph: { engine: 'ai-engine', description: '图谱分析（统计+PageRank+社区+中心性+AI 结论）' },
  workflow: { engine: 'ai-engine', description: '工作流顺序执行（关键步中断）' },
  chat: { engine: 'llm-gateway', description: '通用对话（默认兜底）' }
};

function readMetrics() {
  try {
    const fp = path.join(DATA_DIR, METRICS_FILE);
    if (!fs.existsSync(fp)) return { total: 0, by_capability: {} };
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : { total: 0, by_capability: {} };
  } catch (e) {
    return { total: 0, by_capability: {} };
  }
}

function writeMetrics(m) {
  try {
    fs.mkdirSync(DATA_DIR, { recursive: true });
    fs.writeFileSync(path.join(DATA_DIR, METRICS_FILE), JSON.stringify(m, null, 2), 'utf8');
  } catch (e) {
    console.error('[engine-core] writeMetrics:', e.message);
  }
}

class AIEngineCore {
  constructor() {
    this.gateway = getGateway();
    this.aiEngine = getAIEngine(this.gateway);
    this.ultimateEngine = getUltimateEngine();
    this.allianceEngine = getAllianceEngine();
    // 流程图谱：注入能力矩阵（业务流程+算法流程统一承载于图谱引擎）
    this.flowGraph = getAIFlowGraph({ INTENT_KEYWORDS, CAPABILITY_META });
    this.metrics = readMetrics();
    this.recentCalls = []; // 最近 50 次调用明细（内存环形）
  }

  // ==================== ① 意图识别（图谱激活扩散，可解释） ====================
  /**
   * 图谱化意图识别：委托流程图谱的激活扩散（F8）。
   * 保留同步关键词打分作为降级路径（图谱引擎异常时兜底），不变式②的延伸。
   */
  detectIntent(question) {
    // [C3] SINGLE-SOURCE wrapper（真实定义见 expert-alliance/domain/intent-classifier.js）
    const r = _domainDetectIntent(question);
    return { intent: r.primary, score: r.allScores?.[r.primary] || 0, scores: r.allScores || {}, matched_keywords: r.matchedKeywords || [], method: 'keyword-scoring-domain' };
  }

  // 异步图谱版意图识别（激活扩散）：process 主路径
  async detectIntentByGraph(question) {
    try {
      return await this.flowGraph.detectIntentBySpread(question);
    } catch (e) {
      // 图谱引擎异常 → 降级关键词打分（绝不让意图识别失败）
      const fallback = this.detectIntent(question);
      return { ...fallback, activation: { method: 'keyword-scoring', degraded: true, error: e.message } };
    }
  }

  // ==================== 能力矩阵自描述 ====================
  getCapabilities() {
    return {
      capabilities: Object.entries(CAPABILITY_META).map(([id, meta]) => ({
        id,
        engine: meta.engine,
        description: meta.description,
        keywords: (INTENT_KEYWORDS[id] || []).map((r) => r.kw),
        is_default: id === 'chat'
      })),
      intent_keywords: INTENT_KEYWORDS,
      pipeline: ['意图识别', '能力路由', '引擎执行', '质量校验', '指标反馈'],
      invariants: ['只编排不重造', '降级单向（失败→chat）', '指标必达', '显式覆盖（capability 优先）']
    };
  }

  // ==================== 统一入口 ====================
  async process(request) {
    const question = String((request && request.question) || '').trim();
    if (!question) throw new Error('缺少 question 参数');

    // ① 意图识别（图谱激活扩散；显式 capability 覆盖 → 不变式④）
    const intent = request.capability
      ? { intent: request.capability, score: -1, scores: {}, matched_keywords: [], explicit: true }
      : await this.detectIntentByGraph(question);

    // ②③ 能力路由 + 引擎执行
    return this._execute(intent.intent, question, request.options || {}, intent);
  }

  // 显式能力执行（/ai/engine/analyze）
  async executeCapability(capability, question, options = {}) {
    if (!CAPABILITY_META[capability]) {
      throw new Error(`未知能力: ${capability}（可用: ${Object.keys(CAPABILITY_META).join(', ')}）`);
    }
    return this._execute(capability, String(question || ''), options, { intent: capability, explicit: true });
  }

  async _execute(capability, question, options, intentInfo) {
    const start = Date.now();
    let record = {
      capability,
      intent: intentInfo.intent,
      explicit: !!intentInfo.explicit,
      engine: CAPABILITY_META[capability].engine,
      success: false,
      degraded_to: null,
      latency_ms: 0
    };

    let result;
    try {
      result = await this._dispatch(capability, question, options);
      // ④ 质量校验：结果非空
      if (result === null || result === undefined) throw new Error('引擎返回空结果');
      record.success = true;
    } catch (e) {
      // 不变式②：降级单向 → chat
      if (capability !== 'chat') {
        try {
          result = await this._dispatch('chat', question, options);
          record.degraded_to = 'chat';
          record.success = true;
          record.error = e.message;
        } catch (e2) {
          record.error = e2.message;
          result = { error: e2.message, reply: null };
        }
      } else {
        record.error = e.message;
        result = { error: e.message, reply: null };
      }
    }

    record.latency_ms = Date.now() - start;

    // ⑤ 指标反馈（不变式③）
    this._recordMetric(record);

    return {
      capability: record.degraded_to || record.capability,
      requested_capability: record.capability,
      intent: record.intent,
      matched_keywords: intentInfo.matched_keywords || [],
      activation: intentInfo.activation || null, // 图谱激活扩散明细（F8）
      engine: record.degraded_to ? CAPABILITY_META[record.degraded_to].engine : record.engine,
      degraded: !!record.degraded_to,
      result,
      quality: {
        success: record.success,
        non_empty: this._resultNonEmpty(result)
      },
      latency_ms: record.latency_ms,
      metrics_ref: 'GET /ai/engine/metrics'
    };
  }

  // 语义化判空：字符串非空白 / 数组非空 / 对象含键（纯 error 载荷视为空）
  _resultNonEmpty(result) {
    if (result === null || result === undefined) return false;
    if (typeof result === 'string') return result.trim().length > 0;
    if (Array.isArray(result)) return result.length > 0;
    if (typeof result === 'object') {
      const keys = Object.keys(result);
      if (keys.length === 0) return false;
      return !(keys.length <= 2 && result.error !== undefined && !result.reply && !result.content);
    }
    return true;
  }

  // ==================== ③ 引擎执行（委托，不重造） ====================
  async _dispatch(capability, question, options) {
    switch (capability) {
      case 'expert':
        return this.allianceEngine.process(question, options);
      case 'reasoning':
        return this.ultimateEngine.processWithDeepIntelligence(question, options);
      case 'memory':
        return this.ultimateEngine.searchKnowledge(question, options);
      case 'graph': {
        const graphData = options.graphData || options.graph_data;
        if (!graphData || !Array.isArray(graphData.nodes)) {
          throw new Error('graph 能力需要 options.graphData（{nodes, edges}）');
        }
        return this.aiEngine.analyzeGraph(graphData, options);
      }
      case 'workflow': {
        const workflow = options.workflow;
        if (!workflow || !(workflow.steps || workflow.nodes)) {
          throw new Error('workflow 能力需要 options.workflow（{steps|nodes}）');
        }
        return this.aiEngine.executeWorkflow(workflow, options.inputs || {});
      }
      case 'chat':
      default: {
        const res = await this.gateway.chat({
          messages: (options.messages && options.messages.length ? options.messages : [{ role: 'user', content: question }]),
          sessionId: options.session_id || options.sessionId,
          expertType: options.expert_type || options.expertType,
          temperature: options.temperature,
          maxTokens: options.maxTokens
        });
        return { reply: res.content, model: res.model, provider: res.provider, usage: res.usage };
      }
    }
  }

  // ==================== ⑤ 指标 ====================
  _recordMetric(record) {
    this.metrics.total = (this.metrics.total || 0) + 1;
    const byCap = (this.metrics.by_capability = this.metrics.by_capability || {});
    const cap = byCap[record.capability] || (byCap[record.capability] = { calls: 0, success: 0, degraded: 0, total_latency_ms: 0, errors: 0 });
    cap.calls++;
    if (record.success) cap.success++;
    if (record.degraded_to) cap.degraded++;
    if (record.error) cap.errors++;
    cap.total_latency_ms += record.latency_ms;
    cap.last_error = record.error || null;
    cap.last_called_at = new Date().toISOString();

    this.recentCalls.push(record);
    if (this.recentCalls.length > 50) this.recentCalls.shift();

    if (this.metrics.total % 5 === 0 || !record.success) writeMetrics(this.metrics);
  }

  getMetrics() {
    const byCap = {};
    for (const [cap, m] of Object.entries(this.metrics.by_capability || {})) {
      byCap[cap] = {
        ...m,
        avg_latency_ms: m.calls ? Math.round(m.total_latency_ms / m.calls) : 0,
        success_rate: m.calls ? +(m.success / m.calls).toFixed(4) : 0
      };
    }
    return {
      total_calls: this.metrics.total || 0,
      by_capability: byCap,
      recent_calls: this.recentCalls.slice(-20).reverse()
    };
  }
}

let instance = null;

function getAIEngineCore() {
  if (!instance) {
    instance = new AIEngineCore();
  }
  return instance;
}

module.exports = { AIEngineCore, getAIEngineCore, INTENT_KEYWORDS, CAPABILITY_META };
