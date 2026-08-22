'use strict';

/**
 * 无穷维度优化引擎（Infinite Dimension Optimizer）
 * =====================================================
 *
 * 核心思想：
 *   将 AI 管线的配置空间视为连续高维空间（维度 = 采样温度 × 专家路由强度 ×
 *   上下文深度 × 各引擎路由权重……），理论上维度可无限扩展（"无穷维度"），
 *   在该空间中搜索使多目标评分最大的最优配置。
 *
 * 优化算法：交叉熵方法 CEM（Cross-Entropy Method）
 *   1. 初始化每维的高斯分布 N(mu, sigma^2)
 *   2. 每轮采样 population 个候选配置（截断到 [0,1]）
 *   3. 在基准测试集上真实评估每个配置（严格调用，不降级）
 *   4. 多目标评分 = w_q×质量 + w_l×速度 + w_t×token效率 + w_s×稳定性
 *   5. 取评分前 elite 比例的精英，平滑更新 mu / sigma
 *   6. 自动迭代直至收敛（sigma 收缩 或 连续 patience 轮无改进）
 *
 * 验证方法（科学性）：
 *   - 收敛曲线：每轮最优 / 均值 / 标准差，验证优化过程收敛而非震荡
 *   - 维度敏感度：Pearson 相关性分析各维度取值与评分的关系
 *   - 跨类别交叉验证：7 类基准任务分别打分，检验配置的泛化性
 *   - 多引擎横向对比：同一基准集对所有已配置引擎独立评测
 */

const fs = require('fs');
const path = require('path');
const { getGateway } = require('./llm-gateway');

const DATA_DIR = path.join(__dirname, '..', 'data');
const RUNS_FILE = 'infinite_optimization_runs.json';

// ==================== 基准测试集 ====================
// 7 大能力维度，全部支持确定性校验（不依赖主观打分，结果可复现）
function buildBenchmarkTasks() {
  const year = new Date().getFullYear();
  return [
    {
      id: 'math_compute',
      category: '数学计算',
      prompt: '计算：3^4 + 12×7 - 56÷4 = ? 只输出最终数字，不要任何其他文字。',
      check: { type: 'contains', keywords: ['151'] },
      expert_type: 'algorithm',
      weight: 1.0
    },
    {
      id: 'logic_reasoning',
      category: '逻辑推理',
      prompt: '判断以下三段论是否有效：「所有的玫瑰都是花，有些花很快凋谢，所以有些玫瑰很快凋谢。」只回答"有效"或"无效"，并简要说明理由（一句话）。',
      check: { type: 'contains', keywords: ['无效'] },
      expert_type: 'algorithm',
      weight: 1.0
    },
    {
      id: 'knowledge_fact',
      category: '知识问答',
      prompt: '中国的国土面积约为多少万平方公里？只输出数字。',
      check: { type: 'contains', keywords: ['960'] },
      expert_type: 'data',
      weight: 0.8
    },
    {
      id: 'code_generation',
      category: '代码生成',
      prompt: '用 JavaScript 写一个判断回文字符串的函数，函数名必须是 isPalindrome。只输出代码。',
      check: { type: 'contains', keywords: ['isPalindrome'] },
      expert_type: 'algorithm',
      weight: 1.0
    },
    {
      id: 'chinese_nlu',
      category: '中文理解',
      prompt: '补全王勃《滕王阁序》中的名句："落霞与孤鹜齐飞，______"。只输出下半句。',
      check: { type: 'contains', keywords: ['秋水共长天一色'] },
      expert_type: 'fusion',
      weight: 0.8
    },
    {
      id: 'time_awareness',
      category: '时效认知',
      prompt: '今天是哪一年？只输出年份数字。',
      check: { type: 'contains', keywords: [String(year)] },
      expert_type: 'default',
      weight: 1.2 // 时效性权重最高：验证实时上下文注入
    },
    {
      id: 'instruction_following',
      category: '指令遵循',
      prompt: '把这句话翻译成英文："知识就是力量"。只输出英文译文，不要标点以外的任何内容。',
      check: { type: 'regex', pattern: /knowledge\s+is\s+power/i },
      expert_type: 'default',
      weight: 1.0
    }
  ];
}

// ==================== 多目标评分权重 ====================
const OBJECTIVE_WEIGHTS = {
  quality: 0.55, // 回答质量（确定性校验）
  latency: 0.20, // 响应速度
  token_efficiency: 0.10, // token 效率
  stability: 0.15 // 稳定性（成功率）
};

// ==================== 工具函数 ====================
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
    console.error('[infinite-optimizer] writeJSON', file, e.message);
    return false;
  }
}

function mean(arr) {
  return arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0;
}

function std(arr) {
  if (arr.length < 2) return 0;
  const m = mean(arr);
  return Math.sqrt(mean(arr.map((v) => (v - m) * (v - m))));
}

// Box-Muller 正态采样
function gaussianRandom(mu, sigma) {
  const u = Math.random() || 1e-9;
  const v = Math.random();
  return mu + sigma * Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

function clamp01(x) {
  return Math.max(0, Math.min(1, x));
}

function softmax(xs, scale = 4) {
  const max = Math.max(...xs);
  const exps = xs.map((x) => Math.exp(scale * (x - max)));
  const sum = exps.reduce((a, b) => a + b, 0) || 1;
  return exps.map((e) => e / sum);
}

// Pearson 相关（维度敏感度分析）
function pearson(xs, ys) {
  const n = Math.min(xs.length, ys.length);
  if (n < 3) return 0;
  const mx = mean(xs);
  const my = mean(ys);
  let num = 0;
  let dx = 0;
  let dy = 0;
  for (let i = 0; i < n; i++) {
    num += (xs[i] - mx) * (ys[i] - my);
    dx += (xs[i] - mx) ** 2;
    dy += (ys[i] - my) ** 2;
  }
  const den = Math.sqrt(dx * dy);
  return den === 0 ? 0 : num / den;
}

// ==================== 优化引擎 ====================
class InfiniteDimensionOptimizer {
  constructor(gateway) {
    this.gateway = gateway || getGateway();
    this.run = null; // 当前运行状态（内存）
    this.comparison = null; // 最近一次引擎对比结果
    this._loadHistory();
  }

  _loadHistory() {
    const data = readJSON(RUNS_FILE, { runs: [], best: null });
    this.history = data;
  }

  _saveHistory() {
    writeJSON(RUNS_FILE, this.history);
  }

  getBenchmarks() {
    return buildBenchmarkTasks().map((t) => ({
      id: t.id,
      category: t.category,
      prompt: t.prompt,
      weight: t.weight,
      check_type: t.check.type
    }));
  }

  // ---------- 维度空间构建 ----------
  // 每次运行时根据当前可用引擎动态构建维度（引擎数可变 → 维度可无限扩展）
  _buildDimensions() {
    const providers = this.gateway.listAvailableProviders();
    const dims = [
      { key: 'temperature', label: '采样温度', min: 0.1, max: 1.2, map: (x) => 0.1 + 1.1 * x },
      { key: 'expert_routing', label: '专家路由强度', min: 0, max: 1, map: (x) => x },
      { key: 'context_depth', label: '上下文深度', min: 0, max: 6, map: (x) => Math.round(6 * x) }
    ];
    providers.forEach((p) => {
      dims.push({
        key: `w_${p.id}`,
        label: `引擎权重·${p.name}`,
        min: 0,
        max: 1,
        map: null, // softmax 归一化处理
        provider: p.id
      });
    });
    return { dims, providers };
  }

  // 归一化向量 → 实际配置
  _mapVector(x, dims, providers) {
    const config = {};
    const providerWeights = [];
    dims.forEach((d, i) => {
      if (d.key === 'temperature') config.temperature = +(0.1 + 1.1 * x[i]).toFixed(2);
      else if (d.key === 'expert_routing') config.expert_routing = +x[i].toFixed(3);
      else if (d.key === 'context_depth') config.context_depth = Math.round(6 * x[i]);
      else if (d.key.startsWith('w_')) providerWeights.push(x[i]);
    });
    const sm = softmax(providerWeights.length ? providerWeights : [1]);
    config.provider_weights = {};
    providers.forEach((p, i) => {
      config.provider_weights[p.id] = +(sm[i] || 0).toFixed(4);
    });
    return config;
  }

  // ---------- 生命周期 ----------
  start(options = {}) {
    if (this.run && this.run.status === 'running') {
      throw new Error('优化已在运行中，请先停止或等待收敛');
    }
    const { dims, providers } = this._buildDimensions();
    if (!providers.length) {
      throw new Error('无可用 AI 引擎：请先在「LLM 配置」页启用并配置至少一个引擎的 API Key');
    }

    const iterations = Math.min(Math.max(parseInt(options.iterations, 10) || 6, 1), 30);
    const population = Math.min(Math.max(parseInt(options.population, 10) || 5, 3), 12);
    const evaluationMode = options.evaluation_mode === 'full' ? 'full' : 'fast';

    this.run = {
      id: `run_${Date.now()}`,
      started_at: new Date().toISOString(),
      status: 'running',
      stop_requested: false,
      options: { iterations, population, evaluationMode, elite_ratio: 0.3, patience: 3 },
      dimensions: dims.map((d) => ({ key: d.key, label: d.label })),
      providers: providers.map((p) => ({ id: p.id, name: p.name, model: p.model })),
      // CEM 状态
      state: {
        mu: new Array(dims.length).fill(0.5),
        sigma: new Array(dims.length).fill(0.3)
      },
      // 进度
      iteration: 0,
      evaluated_configs: 0,
      best: null, // { score, config, details }
      samples: [], // 全部评估样本（供敏感度分析）
      convergence: [], // 每轮 { iteration, best, mean, sigma_avg }
      iterations_log: []
    };

    // 异步自动运行直到收敛
    this._runLoop().catch((e) => {
      console.error('[infinite-optimizer] run failed:', e.message);
      if (this.run) {
        this.run.status = 'failed';
        this.run.error = e.message;
        this.run.finished_at = new Date().toISOString();
      }
    });

    return {
      run_id: this.run.id,
      dimensions: this.run.dimensions.length,
      providers: this.run.providers,
      iterations,
      population,
      evaluation_mode: evaluationMode
    };
  }

  stop() {
    if (this.run && this.run.status === 'running') {
      this.run.stop_requested = true;
      return { stopping: true };
    }
    return { stopping: false, note: '当前无运行中的优化任务' };
  }

  getStatus() {
    if (!this.run) {
      return { status: 'idle', note: '尚未启动优化', history_count: this.history.runs.length };
    }
    const r = this.run;
    return {
      run_id: r.id,
      status: r.status,
      iteration: r.iteration,
      total_iterations: r.options.iterations,
      evaluated_configs: r.evaluated_configs,
      dimensions: r.dimensions,
      state: {
        mu: r.state.mu.map((v) => +v.toFixed(4)),
        sigma: r.state.sigma.map((v) => +v.toFixed(4))
      },
      best: r.best ? { score: +r.best.score.toFixed(4), config: r.best.config } : null,
      convergence: r.convergence,
      latest_iteration_log: r.iterations_log[r.iterations_log.length - 1] || null,
      started_at: r.started_at,
      finished_at: r.finished_at || null,
      error: r.error || null
    };
  }

  getResults() {
    return {
      best: this.history.best,
      runs: this.history.runs.slice(-10).reverse(),
      objective_weights: OBJECTIVE_WEIGHTS,
      benchmarks: this.getBenchmarks()
    };
  }

  // ---------- CEM 主循环 ----------
  async _runLoop() {
    const r = this.run;
    const { dims, providers } = this._buildDimensions();
    // 以运行时快照为准（避免运行中配置变化）
    r._dims = dims;
    r._providers = providers;
    const tasks = buildBenchmarkTasks();
    let noImprovement = 0;
    let bestScore = -1;

    for (let iter = 1; iter <= r.options.iterations; iter++) {
      if (r.stop_requested) break;

      // 1. 采样 population 个候选配置
      const candidates = [];
      for (let i = 0; i < r.options.population; i++) {
        const x = r.state.mu.map((mu, d) => clamp01(gaussianRandom(mu, r.state.sigma[d])));
        candidates.push({ x, config: this._mapVector(x, dims, providers), score: null, details: [] });
      }
      // 保证首轮含均匀基线配置（0.5 向量），用于锚定对比
      if (iter === 1) {
        const x0 = new Array(dims.length).fill(0.5);
        candidates[0] = { x: x0, config: this._mapVector(x0, dims, providers), score: null, details: [] };
      }

      // 2. 逐个评估（顺序执行，避免引擎限流）
      for (const cand of candidates) {
        if (r.stop_requested) break;
        const evaluation = await this._evaluateConfig(cand.config, tasks, r.options.evaluation_mode);
        cand.score = evaluation.score;
        cand.details = evaluation.details;
        r.evaluated_configs++;
      }

      const valid = candidates.filter((c) => c.score != null);
      if (!valid.length) throw new Error('本轮所有候选评估失败，请检查引擎配置');

      // 3. 精英选择 + 分布更新（CEM 核心）
      valid.sort((a, b) => b.score - a.score);
      const eliteCount = Math.max(2, Math.floor(valid.length * r.options.elite_ratio));
      const elites = valid.slice(0, eliteCount);
      const alpha = 0.7; // 学习率（平滑更新）
      r.state.mu = r.state.mu.map((mu, d) => alpha * mean(elites.map((e) => e.x[d])) + (1 - alpha) * mu);
      r.state.sigma = r.state.sigma.map((sigma, d) =>
        Math.max(0.02, alpha * std(elites.map((e) => e.x[d])) + (1 - alpha) * sigma)
      );

      // 4. 记录收敛数据
      const iterBest = valid[0];
      if (iterBest.score > bestScore + 0.005) {
        bestScore = iterBest.score;
        noImprovement = 0;
      } else {
        noImprovement++;
      }
      if (!r.best || iterBest.score > r.best.score) {
        r.best = { score: iterBest.score, config: iterBest.config, details: iterBest.details };
      }
      r.convergence.push({
        iteration: iter,
        best: +iterBest.score.toFixed(4),
        mean: +mean(valid.map((c) => c.score)).toFixed(4),
        sigma_avg: +mean(r.state.sigma).toFixed(4)
      });
      r.iterations_log.push({
        iteration: iter,
        candidates: valid.map((c) => ({
          score: +c.score.toFixed(4),
          config: c.config,
          per_task: c.details.map((d) => ({ task: d.task_id, category: d.category, quality: +d.quality.toFixed(2), latency_ms: d.latency_ms, ok: d.ok }))
        }))
      });
      // 样本留存（敏感度分析用）
      valid.forEach((c) => r.samples.push({ x: c.x, score: c.score }));
      r.iteration = iter;

      // 5. 收敛判定
      const sigmaAvg = mean(r.state.sigma);
      if (sigmaAvg < 0.06 || noImprovement >= r.options.patience) {
        r.converged = true;
        r.convergence_reason = sigmaAvg < 0.06 ? `分布已收缩（σ̄=${sigmaAvg.toFixed(4)} < 0.06）` : `连续 ${noImprovement} 轮无显著改进`;
        break;
      }
    }

    // 6. 收尾：敏感度分析 + 持久化
    r.status = r.stop_requested ? 'stopped' : 'completed';
    r.finished_at = new Date().toISOString();
    r.sensitivity = this._sensitivityAnalysis(r);
    if (r.best) {
      r.best.scores = this._aggregateScores(r.best.details);
    }

    const summary = {
      id: r.id,
      started_at: r.started_at,
      finished_at: r.finished_at,
      status: r.status,
      converged: !!r.converged,
      convergence_reason: r.convergence_reason || (r.status === 'completed' ? '达到最大迭代次数' : '手动停止'),
      iterations: r.iteration,
      evaluated_configs: r.evaluated_configs,
      options: r.options,
      dimensions: r.dimensions,
      providers: r.providers,
      best: r.best,
      convergence: r.convergence,
      sensitivity: r.sensitivity
    };
    this.history.runs.push(summary);
    if (!this.history.best || summary.best.score > this.history.best.score) {
      this.history.best = { run_id: summary.id, score: summary.best.score, config: summary.best.config, at: summary.finished_at };
    }
    this._saveHistory();
    return summary;
  }

  // ---------- 配置评估 ----------
  async _evaluateConfig(config, tasks, evaluationMode) {
    const details = [];
    const historyMsgs = [];
    let totalQuality = 0;
    let totalWeight = 0;

    for (const task of tasks) {
      // 上下文深度：注入前 N 轮历史（模拟多轮对话）
      const ctx = historyMsgs.slice(-Math.max(0, config.context_depth * 2));
      const messages = [...ctx, { role: 'user', content: task.prompt }];

      // 专家路由强度：按概率为任务匹配专家 system prompt
      let systemPrompt = null;
      if (Math.random() < config.expert_routing && task.expert_type) {
        systemPrompt = this.gateway._getExpertSystemPrompt(task.expert_type);
      }

      // 引擎路由：按 softmax 权重采样引擎
      const providerId = this._sampleProvider(config.provider_weights);

      try {
        const result = await this.gateway.chatWithProvider(providerId, {
          messages,
          temperature: config.temperature,
          maxTokens: 400,
          systemPrompt
        });
        const quality = this._scoreReply(task, result.content, evaluationMode);
        details.push({
          task_id: task.id,
          category: task.category,
          provider: providerId,
          ok: true,
          quality,
          latency_ms: result.latency_ms,
          tokens: (result.usage && result.usage.total_tokens) || 0,
          reply: String(result.content || '').slice(0, 200)
        });
        totalQuality += quality * task.weight;
        totalWeight += task.weight;
        historyMsgs.push({ role: 'user', content: task.prompt }, { role: 'assistant', content: String(result.content || '').slice(0, 300) });
      } catch (e) {
        details.push({
          task_id: task.id,
          category: task.category,
          provider: providerId,
          ok: false,
          quality: 0,
          latency_ms: 0,
          tokens: 0,
          error: e.message
        });
        totalWeight += task.weight;
      }
    }

    const scores = this._aggregateScores(details);
    const score =
      OBJECTIVE_WEIGHTS.quality * scores.quality +
      OBJECTIVE_WEIGHTS.latency * scores.latency +
      OBJECTIVE_WEIGHTS.token_efficiency * scores.token_efficiency +
      OBJECTIVE_WEIGHTS.stability * scores.stability;

    return { score: +score.toFixed(4), details, scores };
  }

  _sampleProvider(weights) {
    const entries = Object.entries(weights).filter(([, w]) => w > 0);
    if (!entries.length) throw new Error('无可用引擎权重');
    const total = entries.reduce((a, [, w]) => a + w, 0);
    let r = Math.random() * total;
    for (const [id, w] of entries) {
      r -= w;
      if (r <= 0) return id;
    }
    return entries[entries.length - 1][0];
  }

  // 确定性质量评分（可复现，不依赖主观判断）
  _scoreReply(task, reply, evaluationMode) {
    const text = String(reply || '');
    const check = task.check;
    if (check.type === 'contains') {
      const hits = check.keywords.filter((k) => text.includes(k)).length;
      if (hits === check.keywords.length) return 1;
      if (hits > 0) return 0.5;
    } else if (check.type === 'regex') {
      if (check.pattern.test(text)) return 1;
    }
    // 未命中确定性校验：full 模式下给长度合理性弱分（防空白回复），fast 模式记 0
    if (evaluationMode === 'full' && text.trim().length >= 10) return 0.2;
    return 0;
  }

  _aggregateScores(details) {
    const okList = details.filter((d) => d.ok);
    const latencies = okList.map((d) => d.latency_ms);
    const tokens = okList.map((d) => d.tokens);
    return {
      quality: +mean(details.map((d) => d.quality)).toFixed(4),
      latency: +Math.max(0, Math.min(1, 1 - (mean(latencies) - 800) / 12000)).toFixed(4),
      token_efficiency: +Math.max(0, Math.min(1, 1 - (mean(tokens) - 60) / 600)).toFixed(4),
      stability: +(details.length ? okList.length / details.length : 0).toFixed(4),
      avg_latency_ms: Math.round(mean(latencies)),
      avg_tokens: Math.round(mean(tokens))
    };
  }

  // ---------- 维度敏感度分析 ----------
  _sensitivityAnalysis(run) {
    const samples = run.samples || [];
    if (samples.length < 4) return [];
    const ys = samples.map((s) => s.score);
    return run.dimensions
      .map((d, i) => ({
        dimension: d.label,
        key: d.key,
        correlation: +pearson(samples.map((s) => s.x[i]), ys).toFixed(4),
        mu: +run.state.mu[i].toFixed(4),
        sigma: +run.state.sigma[i].toFixed(4)
      }))
      .sort((a, b) => Math.abs(b.correlation) - Math.abs(a.correlation));
  }

  // ---------- 多引擎横向对比 ----------
  async runComparison() {
    const providers = this.gateway.listAvailableProviders();
    const tasks = buildBenchmarkTasks();
    const catalog = ['deepseek', 'openai', 'anthropic', 'volcengine', 'qwen', 'kimi', 'zhipu', 'google', 'ollama'];
    const presetNames = {
      deepseek: 'DeepSeek', openai: 'OpenAI', anthropic: 'Anthropic Claude', volcengine: '豆包（火山引擎）',
      qwen: '阿里云千问', kimi: 'Kimi（月之暗面）', zhipu: '智谱 GLM', google: 'Google Gemini', ollama: 'Ollama（本地）'
    };

    const rows = [];
    for (const p of providers) {
      const details = [];
      for (const task of tasks) {
        try {
          const result = await this.gateway.chatWithProvider(p.id, {
            messages: [{ role: 'user', content: task.prompt }],
            temperature: 0.3,
            maxTokens: 400
          });
          details.push({
            task_id: task.id,
            category: task.category,
            ok: true,
            quality: this._scoreReply(task, result.content, 'fast'),
            latency_ms: result.latency_ms,
            tokens: (result.usage && result.usage.total_tokens) || 0
          });
        } catch (e) {
          details.push({ task_id: task.id, category: task.category, ok: false, quality: 0, latency_ms: 0, tokens: 0, error: e.message });
        }
      }
      const scores = this._aggregateScores(details);
      const total =
        OBJECTIVE_WEIGHTS.quality * scores.quality +
        OBJECTIVE_WEIGHTS.latency * scores.latency +
        OBJECTIVE_WEIGHTS.token_efficiency * scores.token_efficiency +
        OBJECTIVE_WEIGHTS.stability * scores.stability;
      rows.push({
        provider_id: p.id,
        provider_type: p.provider,
        name: p.name,
        model: p.model,
        configured: true,
        total_score: +total.toFixed(4),
        scores,
        per_category: details.reduce((acc, d) => {
          acc[d.category] = acc[d.category] !== undefined ? acc[d.category] : d.quality;
          return acc;
        }, {})
      });
    }

    // 未配置的引擎也列出（全景对比，提示可接入）
    const configuredTypes = new Set(providers.map((p) => p.provider));
    catalog.forEach((type) => {
      if (!configuredTypes.has(type)) {
        rows.push({
          provider_id: null,
          provider_type: type,
          name: presetNames[type] || type,
          model: null,
          configured: false,
          total_score: null,
          scores: null,
          per_category: {}
        });
      }
    });

    rows.sort((a, b) => (b.total_score ?? -1) - (a.total_score ?? -1));
    this.comparison = {
      at: new Date().toISOString(),
      tasks: tasks.map((t) => ({ id: t.id, category: t.category })),
      objective_weights: OBJECTIVE_WEIGHTS,
      rows
    };
    return this.comparison;
  }

  getComparison() {
    return this.comparison;
  }

  // ---------- 应用最优配置 ----------
  applyBest(runId) {
    const run = runId
      ? this.history.runs.find((r) => r.id === runId)
      : this.history.runs[this.history.runs.length - 1];
    if (!run || !run.best || !run.best.config) {
      throw new Error('未找到可应用的最优配置');
    }
    const config = run.best.config;
    const applied = [];

    // 1. 应用最优引擎（权重最高者）
    const weights = Object.entries(config.provider_weights || {}).sort((a, b) => b[1] - a[1]);
    if (weights.length && weights[0][1] > 0) {
      const okSet = this.gateway.setActiveProvider(weights[0][0]);
      if (okSet) applied.push(`激活引擎: ${weights[0][0]}（权重 ${weights[0][1]}）`);
    }
    // 2. 应用最优温度到该引擎
    const provider = this.gateway.getProvider && this.gateway.getProvider(weights[0][0]);
    if (provider && config.temperature != null) {
      this.gateway.updateProvider(weights[0][0], { temperature: config.temperature });
      applied.push(`采样温度: ${config.temperature}`);
    }
    // 3. 路由权重写入 llm_routing.json
    const routing = {
      strategy: 'weighted',
      providers: weights.filter(([, w]) => w > 0.01).map(([id]) => id),
      fallback: true,
      load_balance: true,
      weights: Object.fromEntries(weights.filter(([, w]) => w > 0.01)),
      applied_from: run.id,
      applied_at: new Date().toISOString()
    };
    this.gateway.updateRoutingConfig(routing);
    applied.push(`路由策略: weighted（${routing.providers.length} 个引擎按权重分配）`);

    return { applied, config, run_id: run.id };
  }
}

let instance = null;

function getInfiniteOptimizer() {
  if (!instance) {
    instance = new InfiniteDimensionOptimizer();
  }
  return instance;
}

module.exports = { InfiniteDimensionOptimizer, getInfiniteOptimizer, OBJECTIVE_WEIGHTS, buildBenchmarkTasks };
