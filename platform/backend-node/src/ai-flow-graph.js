'use strict';

/**
 * AI 流程图谱引擎（AI Flow Graph）
 * =================================
 * 设计依据：docs/modules/ai-flow-graph-design.md
 *
 * 核心命题：图谱即 AI 引擎的流程基础设施。
 *   - 业务流程（五步流水线/能力矩阵/降级链）建模为图谱的 step/capability/keyword/engine 节点与 4 类边
 *   - 算法流程（意图识别）= 图谱上的激活扩散（个性化 PageRank 特例，委托统一 PageRank 单源）
 *   - 图公式库（度/介数/紧密中心性/模块度）在此单源实现，ai-engine 委托调用（消除 D8 占位符缺陷）
 *
 * 依赖方向（无环）：ai-flow-graph → ai-integration-engine → llm-gateway
 * 配置注入式：不 require ai-engine-core（避免环），由调用方注入 {INTENT_KEYWORDS, CAPABILITY_META, PIPELINE}
 */

const { getAIIntegrationEngine } = require('./ai-integration-engine');

// ==================== 公式库（单源实现，人读公式 + 解读文案） ====================

const GraphFormulas = {
  /**
   * F1 密度：D = 2E / (N(N-1))（无向）
   * 人性化：返回公式与解读
   */
  density(nodeCount, edgeCount) {
    const N = nodeCount;
    const E = edgeCount;
    if (N < 2) return { value: 0, formula: 'D = 2E/(N(N-1))', interpretation: '节点数不足 2，密度无定义，按 0 处理' };
    const value = (2 * E) / (N * (N - 1));
    // 项目记忆硬性：禁用 toFixed 截断，保留全精度
    let interpretation = '稀疏图，存在大量未连接节点对';
    if (value >= 0.8) interpretation = '高度稠密图，接近完全图';
    else if (value >= 0.3) interpretation = '中等密度，连接适中';
    return { value, formula: 'D = 2E/(N(N-1))', interpretation };
  },

  /**
   * F2 度中心性：C_D(v) = deg(v) / (N-1)（无向度）
   */
  degreeCentrality(nodes, edges) {
    const n = nodes.length;
    if (n === 0) return {};
    const deg = new Map(nodes.map(nd => [nd.id, 0]));
    edges.forEach(e => {
      if (deg.has(e.source)) deg.set(e.source, deg.get(e.source) + 1);
      if (deg.has(e.target)) deg.set(e.target, deg.get(e.target) + 1);
    });
    const out = {};
    for (const [id, d] of deg) {
      out[id] = d / (n - 1 || 1);
    }
    return out;
  },

  /**
   * F4 介数中心性（Brandes 2001 算法，BFS 最短路计数）
   *   C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st
   *   归一化：无向除以 (N-1)(N-2)/2，有向除以 (N-1)(N-2)
   * 修复 D8：此前 ai-engine 中该指标为占位符恒 0。
   */
  betweennessCentrality(nodes, edges, { directed = false } = {}) {
    const n = nodes.length;
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const adj = Array.from({ length: n }, () => []);
    edges.forEach(e => {
      const s = idx.get(e.source);
      const t = idx.get(e.target);
      if (s === undefined || t === undefined || s === t) return;
      adj[s].push(t);
      if (!directed) adj[t].push(s);
    });

    const cb = new Array(n).fill(0);

    // Brandes：对每个源点 s 做 BFS，反向累积依赖
    for (let s = 0; s < n; s++) {
      const stack = [];
      const queue = [s];
      const dist = new Array(n).fill(-1);
      const sigma = new Array(n).fill(0);
      const preds = Array.from({ length: n }, () => []);
      dist[s] = 0;
      sigma[s] = 1;

      while (queue.length) {
        const v = queue.shift();
        stack.push(v);
        for (const w of adj[v]) {
          if (dist[w] < 0) { // w 首次被发现
            dist[w] = dist[v] + 1;
            queue.push(w);
          }
          if (dist[w] === dist[v] + 1) { // v 是 w 的最短路前驱
            sigma[w] += sigma[v];
            preds[w].push(v);
          }
        }
      }

      // 反向累积依赖（δ）
      const delta = new Array(n).fill(0);
      while (stack.length) {
        const w = stack.pop();
        for (const v of preds[w]) {
          delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
        }
        if (w !== s) cb[w] += delta[w];
      }
    }

    // 归一化 + 无向图 Brandes 累积了两次方向，除 2（保留全精度，展示层再格式化）
    const norm = (n > 2) ? (directed ? (n - 1) * (n - 2) : ((n - 1) * (n - 2)) / 2) : 1;
    const scale = directed ? 1 : 0.5;
    const out = {};
    ids.forEach((id, i) => {
      out[id] = norm > 0 ? (cb[i] * scale) / norm : 0;
    });
    return out;
  },

  /**
   * F5 紧密中心性（harmonic 版本，对不可达节点稳健）
   *   C_C(v) = ( Σ_{u≠v} 1/d(v,u) ) / (N-1)   （不可达贡献 0）
   */
  closenessCentrality(nodes, edges, { directed = false } = {}) {
    const n = nodes.length;
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const adj = Array.from({ length: n }, () => []);
    edges.forEach(e => {
      const s = idx.get(e.source);
      const t = idx.get(e.target);
      if (s === undefined || t === undefined || s === t) return;
      adj[s].push(t);
      if (!directed) adj[t].push(s);
    });

    const out = {};
    for (let v = 0; v < n; v++) {
      // BFS 求单源最短路
      const dist = new Array(n).fill(-1);
      dist[v] = 0;
      const queue = [v];
      while (queue.length) {
        const x = queue.shift();
        for (const y of adj[x]) {
          if (dist[y] < 0) { dist[y] = dist[x] + 1; queue.push(y); }
        }
      }
      let harmonic = 0;
      for (let u = 0; u < n; u++) {
        if (u !== v && dist[u] > 0) harmonic += 1 / dist[u];
      }
      out[ids[v]] = n > 1 ? harmonic / (n - 1) : 0;
    }
    return out;
  },

  /**
   * F7 模块度（Newman-Girvan，社区划分质量评估）
   *   Q = Σ_c [ e_c/m − (d_c/(2m))² ]
   *   e_c：社区 c 内部边数；d_c：社区 c 的度数和；m：总边数（无向计 1 次）
   */
  modularity(nodes, edges, communities) {
    // communities: [{members:[id,...]}]
    const commOf = new Map();
    communities.forEach((c, ci) => c.members.forEach(id => commOf.set(id, ci)));
    const m = edges.filter(e => commOf.has(e.source) && commOf.has(e.target) && e.source !== e.target).length;
    if (m === 0) return 0;
    const eIn = new Array(communities.length).fill(0);
    const dSum = new Array(communities.length).fill(0);
    edges.forEach(e => {
      if (e.source === e.target) return;
      const cs = commOf.get(e.source);
      const ct = commOf.get(e.target);
      if (cs !== undefined) dSum[cs] += 1;
      if (ct !== undefined) dSum[ct] += 1;
      if (cs !== undefined && cs === ct) eIn[cs] += 1;
    });
    let q = 0;
    for (let c = 0; c < communities.length; c++) {
      q += eIn[c] / m - Math.pow(dSum[c] / (2 * m), 2);
    }
    return q;
  }
};

// ==================== AI 流程图谱 ====================

const DEFAULT_PIPELINE = ['intent', 'route', 'execute', 'verify', 'feedback'];
const DEFAULT_STEPS = {
  intent: { title: '意图识别', desc: '激活扩散：命中关键词→能力激活（图谱算法 F8）' },
  route: { title: '能力路由', desc: '取激活值最高的能力节点' },
  execute: { title: '引擎执行', desc: '沿 delegates_to 边委托引擎（失败沿 degrades_to 降级）' },
  verify: { title: '质量校验', desc: '语义化判空 + 异常捕获' },
  feedback: { title: '指标反馈', desc: 'per-capability 指标记录（不变式③）' }
};

class AIFlowGraph {
  /**
   * @param {object} config 注入式配置（避免循环依赖）
   *   { INTENT_KEYWORDS, CAPABILITY_META, PIPELINE }
   */
  constructor(config = {}) {
    this.intentKeywords = config.INTENT_KEYWORDS || {};
    this.capabilityMeta = config.CAPABILITY_META || {};
    this.pipeline = config.PIPELINE || DEFAULT_PIPELINE;
    this._graph = this._build();
  }

  // ---------------- 构建流程图谱（业务流程图谱化） ----------------
  _build() {
    const nodes = [];
    const edges = [];

    // step 节点 + flows_to 边（业务流程骨架）
    this.pipeline.forEach((step, i) => {
      const meta = DEFAULT_STEPS[step] || { title: step, desc: '' };
      nodes.push({ id: `step:${step}`, type: 'step', label: `${i + 1}. ${meta.title}`, title: meta.title, desc: meta.desc, order: i });
      if (i > 0) edges.push({ source: `step:${this.pipeline[i - 1]}`, target: `step:${step}`, type: 'flows_to', weight: 1.0 });
    });

    // keyword 节点 + triggers 边（意图触发）
    for (const [capability, rules] of Object.entries(this.intentKeywords)) {
      for (const { kw, w } of rules) {
        const kwId = `kw:${kw}`;
        if (!nodes.find(nd => nd.id === kwId)) {
          nodes.push({ id: kwId, type: 'keyword', label: kw, weight: w, capability });
        }
        edges.push({ source: kwId, target: `cap:${capability}`, type: 'triggers', weight: w });
      }
    }

    // capability 节点 + delegates_to / degrades_to 边
    for (const [capId, meta] of Object.entries(this.capabilityMeta)) {
      nodes.push({ id: `cap:${capId}`, type: 'capability', label: capId, desc: meta.description, is_default: capId === 'chat' });
      if (meta.engine) {
        edges.push({ source: `cap:${capId}`, target: `eng:${meta.engine}`, type: 'delegates_to', weight: 1.0 });
      }
      if (capId !== 'chat') {
        edges.push({ source: `cap:${capId}`, target: 'cap:chat', type: 'degrades_to', weight: 0.5 });
      }
    }

    // engine 节点（去重）
    const engines = [...new Set(Object.values(this.capabilityMeta).map(m => m.engine).filter(Boolean))];
    engines.forEach(eng => {
      nodes.push({ id: `eng:${eng}`, type: 'engine', label: eng });
    });

    return { nodes, edges };
  }

  getGraph() {
    return this._graph;
  }

  // 可视化友好输出（前端力导向图直接可用）
  toVisFormat() {
    const g = this._graph;
    const stats = {
      node_count: g.nodes.length,
      edge_count: g.edges.length,
      by_type: g.nodes.reduce((acc, nd) => { acc[nd.type] = (acc[nd.type] || 0) + 1; return acc; }, {}),
      by_edge_type: g.edges.reduce((acc, ed) => { acc[ed.type] = (acc[ed.type] || 0) + 1; return acc; }, {})
    };
    return {
      nodes: g.nodes,
      edges: g.edges,
      legend: {
        node_types: {
          step: '流水线步骤（业务流程骨架）',
          keyword: '意图关键词',
          capability: 'AI 能力',
          engine: '委托引擎'
        },
        edge_types: {
          triggers: '关键词→能力（触发，weight=词权重）',
          flows_to: '步骤→步骤（流水线顺序）',
          delegates_to: '能力→引擎（委托执行）',
          degrades_to: '能力→chat（失败单向降级）'
        }
      },
      stats,
      formulas: {
        activation_spread: 'a_i = (1-d)·p_i + d·Σ_{j→i} a_j·W(j,i)/outW(j)',
        note: '意图识别=图谱激活扩散（个性化 PageRank 特例，委托统一 PageRank 单源实现）'
      }
    };
  }

  // ---------------- 算法流程图谱化：激活扩散意图识别（F8） ----------------
  /**
   * 把"关键词打分循环"升级为图谱上的带权激活扩散。
   * d=0 时严格等价于旧打分算法（排序不变），向后兼容。
   */
  async detectIntentBySpread(question, { damping = 0.85 } = {}) {
    const text = String(question || '');
    const g = this._graph;
    const capNodes = g.nodes.filter(nd => nd.type === 'capability');
    const kwNodes = g.nodes.filter(nd => nd.type === 'keyword');

    // ① 命中检测
    const hits = kwNodes.filter(nd => text.includes(nd.label));
    if (hits.length === 0) {
      return {
        intent: 'chat', score: 0, scores: {}, matched_keywords: [],
        activation: { method: 'spread', hit_count: 0, note: '无关键词命中 → 默认能力 chat' }
      };
    }

    // ② 个性化向量（命中关键词按权重归一）
    const totalW = hits.reduce((s, nd) => s + (nd.weight || 1), 0);
    const personalization = {};
    hits.forEach(nd => { personalization[nd.id] = (nd.weight || 1) / totalW; });

    // ③ 激活扩散：委托统一 PageRank 单源（算法流程放在图谱引擎上）
    // G5 修复：maxIterations 从 50 改为 30，与 flow-registry 声明"30 轮收敛"一致
    //   硬约束：激活扩散（method=spread, d=0.85, 30 轮收敛）作为个性化 PageRank 特例
    const integration = getAIIntegrationEngine();
    const spread = await integration.graphEngine.computePersonalizedPageRank(
      { nodes: g.nodes, edges: g.edges },
      { damping, personalization, maxIterations: 30, topK: g.nodes.length }
    );

    // ④ 能力排序
    const scoreMap = {};
    (spread.scores || []).forEach(r => { scoreMap[r.id] = r.score; });
    const capScores = {};
    capNodes.forEach(nd => { capScores[nd.label] = Number(scoreMap[nd.id] || 0); });

    let best = 'chat';
    let bestScore = 0;
    for (const [cap, sc] of Object.entries(capScores)) {
      if (sc > bestScore) { best = cap; bestScore = sc; }
    }

    return {
      intent: best,
      score: bestScore,
      scores: capScores,
      matched_keywords: hits.map(nd => nd.label),
      activation: {
        method: 'spread',
        damping,
        hit_count: hits.length,
        personalization,
        converged: spread.convergence,
        iterations: spread.iterations
      }
    };
  }
}

let flowGraphInstance = null;

/**
 * 获取 AI 流程图谱单例。
 * @param {object} config 首次调用时注入（后续调用忽略），默认用 ai-engine-core 的矩阵
 */
function getAIFlowGraph(config) {
  if (!flowGraphInstance) {
    let cfg = config;
    if (!cfg) {
      // 延迟 require：仅在单例未建且未显式注入时拉取 core 的矩阵（运行时已无环）
      const core = require('./ai-engine-core');
      cfg = { INTENT_KEYWORDS: core.INTENT_KEYWORDS, CAPABILITY_META: core.CAPABILITY_META };
    }
    flowGraphInstance = new AIFlowGraph(cfg);
  }
  return flowGraphInstance;
}

module.exports = { AIFlowGraph, getAIFlowGraph, GraphFormulas, DEFAULT_PIPELINE };
